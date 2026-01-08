//! SQL Import functionality
//!
//! Provides parsing of CREATE TABLE statements from various SQL dialects.
//!
//! Uses `sqlparser` to parse CREATE TABLE statements into SDK import primitives.
//!
//! # Validation
//!
//! All imported table and column names are validated for:
//! - Valid identifier format
//! - Maximum length limits
//! - SQL reserved word detection

use super::{ColumnData, ImportError, ImportResult, TableData};
use crate::validation::input::{validate_column_name, validate_data_type, validate_table_name};
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlparser::ast::{ColumnDef, ColumnOption, ObjectName, Statement, TableConstraint};
use sqlparser::dialect::{
    AnsiDialect, BigQueryDialect, DatabricksDialect as OfficialDatabricksDialect, Dialect,
    GenericDialect, HiveDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect,
};
use sqlparser::parser::Parser;
use std::collections::HashMap;

// Static regex patterns compiled once for performance
static RE_IDENTIFIER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)IDENTIFIER\s*\(\s*([^)]+)\s*\)").expect("Invalid regex"));
static RE_LITERAL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?:'([^']*)'|"([^"]*)")"#).expect("Invalid regex"));
static RE_MATERIALIZED_VIEW: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)CREATE\s+MATERIALIZED\s+VIEW").expect("Invalid regex"));
static RE_TABLE_COMMENT_SINGLE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)\)\s+COMMENT\s+'[^']*'"#).expect("Invalid regex"));
static RE_TABLE_COMMENT_DOUBLE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)\)\s+COMMENT\s+"[^"]*""#).expect("Invalid regex"));
static RE_TBLPROPERTIES: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)TBLPROPERTIES\s*\(").expect("Invalid regex"));
static RE_CLUSTER_BY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\s+CLUSTER\s+BY\s+(?:AUTO|\([^)]*\)|[\w,\s]+)").expect("Invalid regex")
});
// USING clause (e.g., USING DELTA, USING PARQUET, USING CSV, etc.)
static RE_USING_FORMAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\s+USING\s+(?:DELTA|PARQUET|CSV|JSON|ORC|AVRO|TEXT|BINARYFILE|JDBC|ICEBERG)\b")
        .expect("Invalid regex")
});
static RE_VARIABLE_TYPE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r":\s*:([a-zA-Z_][a-zA-Z0-9_]*)").expect("Invalid regex"));
static RE_ARRAY_VARIABLE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ARRAY\s*<\s*:([a-zA-Z_][a-zA-Z0-9_]*)\s*>").expect("Invalid regex"));
static RE_FIELD_VARIABLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\w+)\s+:\w+\s+([A-Z][A-Z0-9_]*(?:<[^>]*>)?)").expect("Invalid regex")
});
static RE_COMPLEX_TYPE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\w+)\s+(STRUCT<|ARRAY<|MAP<)").expect("Invalid regex"));

/// Custom Databricks SQL dialect implementation
///
/// Extends the official DatabricksDialect to support additional Databricks-specific syntax patterns:
/// - Variable references (`:variable_name`) in identifiers
/// - Enhanced backtick-quoted identifiers
#[derive(Debug)]
struct CustomDatabricksDialect {
    official: OfficialDatabricksDialect,
}

impl CustomDatabricksDialect {
    fn new() -> Self {
        Self {
            official: OfficialDatabricksDialect {},
        }
    }
}

impl Dialect for CustomDatabricksDialect {
    fn is_identifier_start(&self, ch: char) -> bool {
        // Allow ':' as identifier start for variable references like :variable_name
        // Delegate to official dialect first, then add our custom support
        self.official.is_identifier_start(ch) || ch == ':'
    }

    fn is_identifier_part(&self, ch: char) -> bool {
        // Allow ':' as identifier part for variable references
        self.official.is_identifier_part(ch) || ch == ':'
    }

    fn is_delimited_identifier_start(&self, ch: char) -> bool {
        // Support backtick-quoted identifiers (Databricks style)
        // Use official dialect's support, which should include backticks
        self.official.is_delimited_identifier_start(ch)
    }
}

/// Tracks preprocessing transformations applied to SQL
#[derive(Debug)]
struct PreprocessingState {
    /// Maps placeholder table names to original IDENTIFIER() expressions
    identifier_replacements: HashMap<String, String>,
}

impl PreprocessingState {
    fn new() -> Self {
        Self {
            identifier_replacements: HashMap::new(),
        }
    }
}

/// SQL Importer - parses CREATE TABLE statements
pub struct SQLImporter {
    /// SQL dialect to use for parsing
    pub dialect: String,
}

impl Default for SQLImporter {
    fn default() -> Self {
        Self {
            dialect: "generic".to_string(),
        }
    }
}

impl SQLImporter {
    /// Create a new SQL importer with the specified dialect
    ///
    /// # Arguments
    ///
    /// * `dialect` - SQL dialect name (see supported dialects below)
    ///
    /// # Supported Dialects
    ///
    /// - **ansi**: ANSI SQL dialect
    /// - **bigquery**: Google BigQuery dialect (natively supports STRUCT/ARRAY types)
    /// - **databricks**: Databricks SQL dialect with support for:
    ///   - `IDENTIFIER()` function calls in table/view names
    ///   - Variable references (`:variable_name`) in type definitions, column definitions, and metadata
    ///   - `STRUCT` and `ARRAY` complex types
    ///   - `CREATE VIEW` and `CREATE MATERIALIZED VIEW` statements
    /// - **hive**: Apache Hive dialect (natively supports STRUCT/ARRAY types)
    /// - **mssql** / **sqlserver**: Microsoft SQL Server dialect
    /// - **mysql**: MySQL dialect
    /// - **postgres** / **postgresql**: PostgreSQL dialect
    /// - **sqlite**: SQLite dialect
    /// - **generic**: Generic SQL dialect (default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::sql::SQLImporter;
    ///
    /// // Standard SQL dialect
    /// let importer = SQLImporter::new("postgres");
    ///
    /// // Databricks SQL dialect
    /// let databricks_importer = SQLImporter::new("databricks");
    /// ```
    pub fn new(dialect: &str) -> Self {
        Self {
            dialect: dialect.to_string(),
        }
    }

    /// Preprocess Databricks SQL to handle IDENTIFIER() expressions
    ///
    /// Replaces IDENTIFIER() function calls with placeholder table names
    /// and tracks the original expressions for later extraction.
    fn preprocess_identifier_expressions(sql: &str, state: &mut PreprocessingState) -> String {
        let mut counter = 0;

        RE_IDENTIFIER
            .replace_all(sql, |caps: &regex::Captures| {
                let expr = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                counter += 1;
                let placeholder = format!("__databricks_table_{}__", counter);

                // Store the mapping for later extraction
                state
                    .identifier_replacements
                    .insert(placeholder.clone(), expr.to_string());

                placeholder
            })
            .to_string()
    }

    /// Extract table name from IDENTIFIER() expression
    ///
    /// If the expression contains string literals, extracts and constructs the table name.
    /// Returns None if expression contains only variables.
    fn extract_identifier_table_name(expr: &str) -> Option<String> {
        let mut parts = Vec::new();

        // Extract all string literals
        for cap in RE_LITERAL.captures_iter(expr) {
            if let Some(m) = cap.get(1) {
                parts.push(m.as_str().to_string());
            } else if let Some(m) = cap.get(2) {
                parts.push(m.as_str().to_string());
            }
        }

        if parts.is_empty() {
            // No literals found - expression contains only variables
            return None;
        }

        // Join literals and remove leading/trailing dots
        let result = parts.join("");
        Some(result.trim_matches('.').to_string())
    }

    /// Preprocess CREATE MATERIALIZED VIEW to CREATE VIEW
    ///
    /// sqlparser may not support MATERIALIZED VIEW directly, so we convert it to CREATE VIEW
    /// This allows parsing to succeed while preserving the intent.
    fn preprocess_materialized_views(sql: &str) -> String {
        RE_MATERIALIZED_VIEW
            .replace_all(sql, "CREATE VIEW")
            .to_string()
    }

    /// Preprocess table-level COMMENT clause removal
    ///
    /// sqlparser does not support table-level COMMENT clauses, so we remove them before parsing.
    /// Handles both single-quoted and double-quoted COMMENT strings.
    /// Table-level COMMENT appears after the closing parenthesis, not inside column definitions.
    fn preprocess_table_comment(sql: &str) -> String {
        // Replace with just the closing parenthesis
        let result = RE_TABLE_COMMENT_SINGLE.replace_all(sql, ")");
        RE_TABLE_COMMENT_DOUBLE
            .replace_all(&result, ")")
            .to_string()
    }

    /// Preprocess TBLPROPERTIES clause removal
    ///
    /// sqlparser does not support TBLPROPERTIES, so we remove it before parsing.
    /// This preserves the rest of the SQL structure while allowing parsing to succeed.
    fn preprocess_tblproperties(sql: &str) -> String {
        // Remove TBLPROPERTIES clause (may span multiple lines)
        // Pattern matches: TBLPROPERTIES ( ... ) where ... can contain nested parentheses
        // We need to match balanced parentheses
        let mut result = sql.to_string();

        // Find all TBLPROPERTIES occurrences and remove them with balanced parentheses
        let mut search_start = 0;
        while let Some(m) = RE_TBLPROPERTIES.find_at(&result, search_start) {
            let start = m.start();
            let mut pos = m.end();
            let mut paren_count = 1;

            // Find matching closing parenthesis (using byte positions)
            let bytes = result.as_bytes();
            while pos < bytes.len() && paren_count > 0 {
                if let Some(ch) = result[pos..].chars().next() {
                    if ch == '(' {
                        paren_count += 1;
                    } else if ch == ')' {
                        paren_count -= 1;
                    }
                    pos += ch.len_utf8();
                } else {
                    break;
                }
            }

            if paren_count == 0 {
                // Remove TBLPROPERTIES clause including the closing parenthesis
                result.replace_range(start..pos, "");
                search_start = start;
            } else {
                // Unbalanced parentheses, skip this match
                search_start = pos;
            }
        }

        result
    }

    /// Preprocess CLUSTER BY clause removal
    ///
    /// sqlparser does not support CLUSTER BY, so we remove it before parsing.
    fn preprocess_cluster_by(sql: &str) -> String {
        RE_CLUSTER_BY.replace_all(sql, "").to_string()
    }

    /// Preprocess USING format clause removal
    ///
    /// Databricks supports USING DELTA, USING PARQUET, etc. which sqlparser doesn't support.
    /// We remove these clauses before parsing as they don't affect the table structure.
    fn preprocess_using_format(sql: &str) -> String {
        RE_USING_FORMAT.replace_all(sql, "").to_string()
    }

    /// Normalize SQL while preserving quoted strings
    ///
    /// Converts multiline SQL to single line, but preserves quoted strings
    /// (both single and double quotes) to avoid breaking COMMENT clauses and other string literals.
    /// Handles escape sequences: `\'` (escaped quote) and `\\` (escaped backslash).
    fn normalize_sql_preserving_quotes(sql: &str) -> String {
        let mut result = String::with_capacity(sql.len());
        let mut chars = sql.chars().peekable();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut last_char_was_space = false;

        while let Some(ch) = chars.next() {
            match ch {
                '\\' if in_single_quote || in_double_quote => {
                    // Handle escape sequences inside quoted strings
                    // \' or \" or \\ - preserve the escape sequence
                    if let Some(&next_ch) = chars.peek() {
                        result.push(ch);
                        result.push(next_ch);
                        chars.next(); // Consume the escaped character
                        last_char_was_space = false;
                    } else {
                        // Backslash at end of string - just add it
                        result.push(ch);
                        last_char_was_space = false;
                    }
                }
                '\'' if !in_double_quote => {
                    // Check if this is an escaped quote (doubled quotes: '')
                    // In SQL standard, '' inside a string means a single quote
                    if in_single_quote && chars.peek() == Some(&'\'') {
                        // Doubled quote - this is an escaped quote, not the end of string
                        result.push(ch);
                        result.push(ch);
                        chars.next(); // Consume the second quote
                        last_char_was_space = false;
                    } else {
                        // Regular quote - toggle quote state
                        in_single_quote = !in_single_quote;
                        result.push(ch);
                        last_char_was_space = false;
                    }
                }
                '"' if !in_single_quote => {
                    // Check if this is an escaped quote (doubled quotes: "")
                    if in_double_quote && chars.peek() == Some(&'"') {
                        // Doubled quote - this is an escaped quote, not the end of string
                        result.push(ch);
                        result.push(ch);
                        chars.next(); // Consume the second quote
                        last_char_was_space = false;
                    } else {
                        // Regular quote - toggle quote state
                        in_double_quote = !in_double_quote;
                        result.push(ch);
                        last_char_was_space = false;
                    }
                }
                '\n' | '\r' => {
                    if in_single_quote || in_double_quote {
                        // Replace newlines inside quoted strings with space
                        // sqlparser doesn't support multiline string literals
                        if !last_char_was_space {
                            result.push(' ');
                            last_char_was_space = true;
                        }
                    } else {
                        // Replace newlines outside quotes with space
                        if !last_char_was_space {
                            result.push(' ');
                            last_char_was_space = true;
                        }
                    }
                }
                ' ' | '\t' => {
                    if in_single_quote || in_double_quote {
                        // Preserve spaces inside quoted strings
                        result.push(ch);
                        last_char_was_space = false;
                    } else {
                        // Collapse multiple spaces to single space
                        if !last_char_was_space {
                            result.push(' ');
                            last_char_was_space = true;
                        }
                    }
                }
                '-' if !in_single_quote && !in_double_quote => {
                    // Check for SQL comment (--)
                    if let Some(&'-') = chars.peek() {
                        // Skip rest of line comment
                        for c in chars.by_ref() {
                            if c == '\n' || c == '\r' {
                                break;
                            }
                        }
                        if !last_char_was_space {
                            result.push(' ');
                            last_char_was_space = true;
                        }
                    } else {
                        result.push(ch);
                        last_char_was_space = false;
                    }
                }
                _ => {
                    result.push(ch);
                    last_char_was_space = false;
                }
            }
        }

        result.trim().to_string()
    }

    /// Convert backslash-escaped quotes to SQL standard doubled quotes
    ///
    /// sqlparser doesn't support `\'` escape sequences, so we convert them to `''`
    /// which is the SQL standard way to escape quotes in string literals.
    fn convert_backslash_escaped_quotes(sql: &str) -> String {
        let mut result = String::with_capacity(sql.len());
        let mut chars = sql.chars().peekable();
        let mut in_single_quote = false;
        let mut in_double_quote = false;

        while let Some(ch) = chars.next() {
            match ch {
                '\\' if (in_single_quote || in_double_quote) => {
                    // Handle escape sequences inside quoted strings
                    if let Some(&next_ch) = chars.peek() {
                        match next_ch {
                            '\'' if in_single_quote => {
                                // Convert \' to '' (SQL standard escaped quote)
                                result.push_str("''");
                                chars.next(); // Consume the escaped quote
                            }
                            '"' if in_double_quote => {
                                // Convert \" to "" (SQL standard escaped quote)
                                result.push_str("\"\"");
                                chars.next(); // Consume the escaped quote
                            }
                            '\\' => {
                                // Convert \\ to \\ (keep as is, but we need to handle it)
                                result.push('\\');
                                result.push('\\');
                                chars.next(); // Consume the escaped backslash
                            }
                            _ => {
                                // Other escape sequences - preserve as is
                                result.push(ch);
                                result.push(next_ch);
                                chars.next();
                            }
                        }
                    } else {
                        // Backslash at end - just add it
                        result.push(ch);
                    }
                }
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                    result.push(ch);
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                    result.push(ch);
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        result
    }

    /// Replace variable references in STRUCT field types with STRING
    ///
    /// Handles patterns like STRUCT<field: :variable_type> -> STRUCT<field: STRING>
    fn replace_variables_in_struct_types(sql: &str) -> String {
        RE_VARIABLE_TYPE
            .replace_all(sql, |_caps: &regex::Captures| ": STRING".to_string())
            .to_string()
    }

    /// Replace variable references in ARRAY element types with STRING
    ///
    /// Handles patterns like ARRAY<:element_type> -> ARRAY<STRING>
    fn replace_variables_in_array_types(sql: &str) -> String {
        RE_ARRAY_VARIABLE
            .replace_all(sql, |_caps: &regex::Captures| "ARRAY<STRING>".to_string())
            .to_string()
    }

    /// Replace variable references in column definitions
    ///
    /// Handles patterns like `column_name :variable STRING` by removing the variable reference.
    /// Example: `id :id_var STRING` -> `id STRING`
    fn replace_variables_in_column_definitions(sql: &str) -> String {
        RE_FIELD_VARIABLE
            .replace_all(sql, |caps: &regex::Captures| {
                let col_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let type_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                format!("{} {}", col_name, type_name)
            })
            .to_string()
    }

    /// Replace nested variable references recursively
    ///
    /// Handles patterns like ARRAY<STRUCT<field: :type>> by applying both
    /// STRUCT and ARRAY replacements recursively.
    fn replace_nested_variables(sql: &str) -> String {
        let mut result = sql.to_string();
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10; // Prevent infinite loops

        // Keep applying replacements until no more changes occur
        while changed && iterations < MAX_ITERATIONS {
            let before = result.clone();

            // First replace variables in STRUCT types
            result = Self::replace_variables_in_struct_types(&result);

            // Then replace variables in ARRAY types
            result = Self::replace_variables_in_array_types(&result);

            // Check if anything changed
            changed = before != result;
            iterations += 1;
        }

        result
    }

    /// Extract STRUCT, ARRAY, and MAP column definitions and replace with placeholders
    ///
    /// sqlparser doesn't support these complex types, so we need to extract them manually
    /// and replace with a simple type that can be parsed, then restore the original
    /// type string after parsing.
    ///
    /// Assumes SQL is already normalized (single line, single spaces).
    fn extract_complex_type_columns(sql: &str) -> (String, Vec<(String, String)>) {
        let mut column_types = Vec::new();
        let mut result = sql.to_string();

        // Find all matches and extract the full type
        let mut matches_to_replace: Vec<(usize, usize, String, String)> = Vec::new();

        for cap in RE_COMPLEX_TYPE.captures_iter(sql) {
            let col_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let type_start = cap.get(0).map(|m| m.start()).unwrap_or(0);
            let struct_or_array = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Find the matching closing bracket
            // Start counting from the '<' in STRUCT<, ARRAY<, or MAP<
            let bracket_start = type_start + col_name.len() + 1 + struct_or_array.len() - 1; // After "column_name STRUCT<", "column_name ARRAY<", or "column_name MAP<"
            let mut bracket_count = 0;
            let mut type_end = bracket_start;

            for (idx, ch) in sql[bracket_start..].char_indices() {
                let pos = bracket_start + idx;
                if ch == '<' {
                    bracket_count += 1;
                } else if ch == '>' {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        type_end = pos + 1;
                        break;
                    }
                }
            }

            if bracket_count == 0 && type_end > type_start {
                // Extract the full type (STRUCT<...>, ARRAY<...>, or MAP<...>)
                // Start from after the column name and space
                let type_start_pos = type_start + col_name.len() + 1;
                let full_type = sql[type_start_pos..type_end].trim().to_string();
                matches_to_replace.push((
                    type_start_pos,
                    type_end,
                    col_name.to_string(),
                    full_type,
                ));
            }
        }

        // Replace matches in reverse order
        for (start, end, col_name, full_type) in matches_to_replace.iter().rev() {
            column_types.push((col_name.clone(), full_type.clone()));
            result.replace_range(*start..*end, "STRING");
        }

        (result, column_types)
    }

    /// Parse SQL and extract table definitions
    ///
    /// # Arguments
    ///
    /// * `sql` - SQL string containing CREATE TABLE, CREATE VIEW, or CREATE MATERIALIZED VIEW statements
    ///
    /// # Returns
    ///
    /// An `ImportResult` containing extracted tables/views and any parse errors.
    ///
    /// # Example - Standard SQL
    ///
    /// ```rust
    /// use data_modelling_sdk::import::sql::SQLImporter;
    ///
    /// let importer = SQLImporter::new("postgres");
    /// let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));";
    /// let result = importer.parse(sql).unwrap();
    /// assert_eq!(result.tables.len(), 1);
    /// ```
    ///
    /// # Example - Databricks SQL with IDENTIFIER()
    ///
    /// ```rust
    /// use data_modelling_sdk::import::sql::SQLImporter;
    ///
    /// let importer = SQLImporter::new("databricks");
    /// let sql = "CREATE TABLE IDENTIFIER(:catalog || '.schema.table') (id STRING, name STRING);";
    /// let result = importer.parse(sql).unwrap();
    /// assert_eq!(result.tables.len(), 1);
    /// assert_eq!(result.tables[0].name.as_deref(), Some("schema.table"));
    /// ```
    ///
    /// # Example - Databricks SQL with Views
    ///
    /// ```rust
    /// use data_modelling_sdk::import::sql::SQLImporter;
    ///
    /// let importer = SQLImporter::new("databricks");
    /// let sql = "CREATE VIEW example_view AS SELECT * FROM table1;";
    /// let result = importer.parse(sql).unwrap();
    /// assert_eq!(result.tables.len(), 1);
    /// ```
    pub fn parse(&self, sql: &str) -> Result<ImportResult> {
        // Minimal preprocessing: only handle variable replacement and unsupported clauses
        // For Databricks: let DatabricksDialect try to parse STRUCT/ARRAY first, then restore full type strings
        // For BigQuery/Hive: can parse STRUCT/ARRAY directly without extraction
        // For other dialects: extract STRUCT/ARRAY if present (they may not support them)
        let (preprocessed_sql, preprocessing_state, complex_types) = if self.dialect.to_lowercase()
            == "databricks"
        {
            let mut state = PreprocessingState::new();
            let mut preprocessed = sql.to_string();

            // Step 1: Replace IDENTIFIER() expressions (needed for variable table names)
            preprocessed = Self::preprocess_identifier_expressions(&preprocessed, &mut state);
            // Step 2: Replace variable references in column definitions (e.g., "id :var STRING" -> "id STRING")
            preprocessed = Self::replace_variables_in_column_definitions(&preprocessed);
            // Step 3: Replace variable references in type definitions (e.g., STRUCT<field: :type> -> STRUCT<field: STRING>)
            preprocessed = Self::replace_nested_variables(&preprocessed);
            // Step 4: Remove unsupported clauses that break parsing
            preprocessed = Self::preprocess_materialized_views(&preprocessed);
            preprocessed = Self::preprocess_table_comment(&preprocessed);
            preprocessed = Self::preprocess_tblproperties(&preprocessed);
            preprocessed = Self::preprocess_cluster_by(&preprocessed);
            preprocessed = Self::preprocess_using_format(&preprocessed);
            // Step 5: Normalize SQL (handle multiline) - needed for regex matching
            let normalized = Self::normalize_sql_preserving_quotes(&preprocessed);
            // Step 6: Convert backslash-escaped quotes (sqlparser doesn't support \' escape sequences)
            let normalized = Self::convert_backslash_escaped_quotes(&normalized);

            // Step 7: Try parsing with DatabricksDialect first (without extraction)
            let dialect = self.dialect_impl();
            let parse_result = Parser::parse_sql(dialect.as_ref(), &normalized);

            // If parsing fails, extract STRUCT/ARRAY types and try again
            // Otherwise, extract type strings for restoration (without modifying SQL)
            let (final_sql, complex_cols) = if parse_result.is_err() {
                // Parsing failed - extract STRUCT/ARRAY/MAP columns and replace with STRING
                let (simplified, cols) = Self::extract_complex_type_columns(&normalized);
                (simplified, cols)
            } else {
                // Parsing succeeded - extract type strings for restoration after parsing
                // We'll restore the full type strings in parse_create_table_with_preprocessing
                let (_, cols) = Self::extract_complex_type_columns(&normalized);
                (normalized, cols)
            };

            (final_sql, state, complex_cols)
        } else if matches!(self.dialect.to_lowercase().as_str(), "bigquery" | "hive") {
            // BigQuery/Hive: can parse STRUCT/ARRAY directly, just normalize
            let normalized = Self::normalize_sql_preserving_quotes(sql);
            let normalized = Self::convert_backslash_escaped_quotes(&normalized);
            (normalized, PreprocessingState::new(), Vec::new())
        } else {
            // Other dialects: extract STRUCT/ARRAY if present (they may not support them)
            let normalized = Self::normalize_sql_preserving_quotes(sql);
            let normalized = Self::convert_backslash_escaped_quotes(&normalized);
            let (simplified_sql, complex_cols) = Self::extract_complex_type_columns(&normalized);
            (simplified_sql, PreprocessingState::new(), complex_cols)
        };

        let dialect = self.dialect_impl();
        let statements = match Parser::parse_sql(dialect.as_ref(), &preprocessed_sql) {
            Ok(stmts) => stmts,
            Err(e) => {
                return Ok(ImportResult {
                    tables: Vec::new(),
                    tables_requiring_name: Vec::new(),
                    errors: vec![ImportError::ParseError(e.to_string())],
                    ai_suggestions: None,
                });
            }
        };

        let mut tables = Vec::new();
        let mut errors = Vec::new();
        let mut tables_requiring_name = Vec::new();

        for (idx, stmt) in statements.into_iter().enumerate() {
            match stmt {
                Statement::CreateTable(create) => {
                    match self.parse_create_table_with_preprocessing(
                        idx,
                        &create.name,
                        &create.columns,
                        &create.constraints,
                        &preprocessing_state,
                        &complex_types,
                    ) {
                        Ok((table, requires_name)) => {
                            if requires_name {
                                tables_requiring_name.push(super::TableRequiringName {
                                    table_index: idx,
                                    suggested_name: None,
                                });
                            }
                            tables.push(table);
                        }
                        Err(e) => errors.push(ImportError::ParseError(e)),
                    }
                }
                Statement::CreateView(create_view) => {
                    match self.parse_create_view(idx, &create_view.name, &preprocessing_state) {
                        Ok((table, requires_name)) => {
                            if requires_name {
                                tables_requiring_name.push(super::TableRequiringName {
                                    table_index: idx,
                                    suggested_name: None,
                                });
                            }
                            tables.push(table);
                        }
                        Err(e) => errors.push(ImportError::ParseError(e)),
                    }
                }
                _ => {
                    // Other statements (INSERT, UPDATE, DELETE, etc.) are ignored.
                }
            }
        }

        Ok(ImportResult {
            tables,
            tables_requiring_name,
            errors,
            ai_suggestions: None,
        })
    }

    /// Parse SQL with Liquibase format support
    ///
    /// Strips Liquibase directive comments (--liquibase formatted sql, --changeset, etc.)
    /// before parsing the SQL.
    ///
    /// # Arguments
    ///
    /// * `sql` - SQL string with optional Liquibase comments
    ///
    /// # Returns
    ///
    /// An `ImportResult` containing extracted tables.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::sql::SQLImporter;
    ///
    /// let importer = SQLImporter::new("postgres");
    /// let sql = r#"
    /// --liquibase formatted sql
    /// --changeset user:1
    /// CREATE TABLE users (id INT);
    /// "#;
    /// let result = importer.parse_liquibase(sql).unwrap();
    /// ```
    pub fn parse_liquibase(&self, sql: &str) -> Result<ImportResult> {
        // Liquibase "formatted SQL" is still SQL, but often includes directive comments like:
        // --liquibase formatted sql
        // --changeset user:id
        // We strip those comment lines, then parse the remaining SQL.
        let cleaned = sql
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                if !t.starts_with("--") {
                    return true;
                }
                // Keep regular SQL comments? For now, drop all -- lines to avoid parser issues.
                false
            })
            .collect::<Vec<_>>()
            .join("\n");

        self.parse(&cleaned)
    }

    fn dialect_impl(&self) -> Box<dyn Dialect + Send + Sync> {
        match self.dialect.to_lowercase().as_str() {
            "ansi" => Box::new(AnsiDialect {}),
            "bigquery" => Box::new(BigQueryDialect {}),
            "databricks" => Box::new(CustomDatabricksDialect::new()),
            "hive" => Box::new(HiveDialect {}),
            "mssql" | "sqlserver" => Box::new(MsSqlDialect {}),
            "mysql" => Box::new(MySqlDialect {}),
            "postgres" | "postgresql" => Box::new(PostgreSqlDialect {}),
            "sqlite" => Box::new(SQLiteDialect {}),
            _ => Box::new(GenericDialect {}),
        }
    }

    /// Strip quote characters from an identifier
    ///
    /// Handles various SQL quoting styles:
    /// - Double quotes: `"identifier"` -> `identifier`
    /// - Backticks: `` `identifier` `` -> `identifier`
    /// - Brackets: `[identifier]` -> `identifier`
    ///
    /// Also handles escaped quotes within identifiers:
    /// - `""` -> `"` (PostgreSQL style)
    /// - ``` `` ``` -> `` ` `` (MySQL style)
    /// - `]]` -> `]` (SQL Server style)
    fn unquote_identifier(identifier: &str) -> String {
        let trimmed = identifier.trim();

        // Check for double quotes
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            return inner.replace("\"\"", "\"");
        }

        // Check for backticks (MySQL)
        if trimmed.starts_with('`') && trimmed.ends_with('`') && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            return inner.replace("``", "`");
        }

        // Check for brackets (SQL Server)
        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            return inner.replace("]]", "]");
        }

        // No quoting detected, return as-is
        trimmed.to_string()
    }

    fn object_name_to_string(name: &ObjectName) -> String {
        // Use final identifier (supports schema-qualified names).
        // In sqlparser 0.60, ObjectNamePart might be a different structure
        // Fall back to to_string() which should work
        let raw_name = name
            .0
            .last()
            .map(|ident| {
                // Try to get the identifier value - structure may have changed in 0.60
                // Use to_string() as fallback
                ident.to_string()
            })
            .unwrap_or_else(|| name.to_string());

        // Strip any quote characters from the identifier
        Self::unquote_identifier(&raw_name)
    }

    fn parse_create_table_with_preprocessing(
        &self,
        table_index: usize,
        name: &ObjectName,
        columns: &[ColumnDef],
        constraints: &[TableConstraint],
        preprocessing_state: &PreprocessingState,
        complex_types: &[(String, String)],
    ) -> std::result::Result<(TableData, bool), String> {
        let mut table_name = Self::object_name_to_string(name);
        let mut requires_name = false;

        // Check if this is a placeholder table name from IDENTIFIER() preprocessing
        if table_name.starts_with("__databricks_table_")
            && let Some(original_expr) =
                preprocessing_state.identifier_replacements.get(&table_name)
        {
            // Try to extract table name from the original expression
            if let Some(extracted_name) = Self::extract_identifier_table_name(original_expr) {
                table_name = extracted_name;
            } else {
                // Expression contains only variables - mark as requiring name
                requires_name = true;
            }
        }

        // Validate table name (warnings are logged but don't fail import)
        if let Err(e) = validate_table_name(&table_name) {
            // Log warning but continue - imported SQL may have valid but unusual names
            tracing::warn!("Table name validation warning: {}", e);
        }

        // Collect PK columns from table-level constraints.
        let mut pk_cols = std::collections::HashSet::<String>::new();
        for c in constraints {
            if let TableConstraint::PrimaryKey(pk_constraint) = c {
                for col in &pk_constraint.columns {
                    // In sqlparser 0.60, IndexColumn structure may have changed
                    // Try to get the column name - might be a field or method
                    // Unquote the column name to match against column definitions
                    pk_cols.insert(Self::unquote_identifier(&col.to_string()));
                }
            }
        }

        let mut out_cols = Vec::new();
        for col in columns {
            let mut nullable = true;
            let mut is_pk = false;

            for opt_def in &col.options {
                match &opt_def.option {
                    ColumnOption::NotNull => nullable = false,
                    ColumnOption::Null => nullable = true,
                    ColumnOption::Unique(_) => {
                        // UNIQUE constraint (not primary key)
                    }
                    ColumnOption::PrimaryKey(_) => {
                        // In sqlparser 0.60, PRIMARY KEY is a separate variant
                        is_pk = true;
                    }
                    _ => {}
                }
            }

            let col_name = Self::unquote_identifier(&col.name.value);

            if pk_cols.contains(&col_name) {
                is_pk = true;
            }
            let mut data_type = col.data_type.to_string();
            let mut description = None;

            // Extract COMMENT clause from column options
            for opt_def in &col.options {
                if let ColumnOption::Comment(comment) = &opt_def.option {
                    description = Some(comment.clone());
                }
            }

            // Restore complex types (STRUCT/ARRAY/MAP) if this column was extracted
            // Keep the full type string - we'll simplify it when converting to Column model
            if let Some((_, original_type)) =
                complex_types.iter().find(|(name, _)| name == &col_name)
            {
                data_type = original_type.clone();
            }

            // Validate column name and data type (warnings are logged but don't fail import)
            if let Err(e) = validate_column_name(&col_name) {
                tracing::warn!("Column name validation warning for '{}': {}", col_name, e);
            }
            if let Err(e) = validate_data_type(&data_type) {
                tracing::warn!("Data type validation warning for '{}': {}", data_type, e);
            }

            out_cols.push(ColumnData {
                name: col_name,
                data_type,
                nullable,
                primary_key: is_pk,
                description,
                ..Default::default()
            });
        }

        Ok((
            TableData {
                table_index,
                name: Some(table_name),
                columns: out_cols,
            },
            requires_name,
        ))
    }

    /// Parse CREATE VIEW statement
    ///
    /// Extracts view name and creates a TableData entry for the view.
    /// Views are treated as table-like entities for data modeling purposes.
    fn parse_create_view(
        &self,
        view_index: usize,
        name: &ObjectName,
        preprocessing_state: &PreprocessingState,
    ) -> std::result::Result<(TableData, bool), String> {
        let mut view_name = Self::object_name_to_string(name);
        let mut requires_name = false;

        // Check if this is a placeholder view name from IDENTIFIER() preprocessing
        if view_name.starts_with("__databricks_table_")
            && let Some(original_expr) = preprocessing_state.identifier_replacements.get(&view_name)
        {
            // Try to extract view name from the original expression
            if let Some(extracted_name) = Self::extract_identifier_table_name(original_expr) {
                view_name = extracted_name;
            } else {
                // Expression contains only variables - mark as requiring name
                requires_name = true;
            }
        }

        // Validate view name
        if let Err(e) = validate_table_name(&view_name) {
            tracing::warn!("View name validation warning: {}", e);
        }

        // Views don't have explicit column definitions in CREATE VIEW statements
        // The columns are derived from the SELECT query, which we don't parse here
        // So we create a view with empty columns - the actual columns would need
        // to be extracted from the query if needed in the future
        Ok((
            TableData {
                table_index: view_index,
                name: Some(view_name),
                columns: Vec::new(), // Views don't have explicit column definitions
            },
            requires_name,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_importer_default() {
        let importer = SQLImporter::default();
        assert_eq!(importer.dialect, "generic");
    }

    #[test]
    fn test_sql_importer_parse_basic() {
        let importer = SQLImporter::new("postgres");
        let result = importer
            .parse("CREATE TABLE test (id INT PRIMARY KEY, name TEXT NOT NULL);")
            .unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        let t = &result.tables[0];
        assert_eq!(t.name.as_deref(), Some("test"));
        assert_eq!(t.columns.len(), 2);
        assert!(t.columns.iter().any(|c| c.name == "id" && c.primary_key));
        assert!(t.columns.iter().any(|c| c.name == "name" && !c.nullable));
    }

    #[test]
    fn test_sql_importer_parse_table_pk_constraint() {
        let importer = SQLImporter::new("postgres");
        let result = importer
            .parse("CREATE TABLE t (id INT, name TEXT, CONSTRAINT pk PRIMARY KEY (id));")
            .unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        let t = &result.tables[0];
        assert!(t.columns.iter().any(|c| c.name == "id" && c.primary_key));
    }

    #[test]
    fn test_sql_importer_parse_liquibase_formatted_sql() {
        let importer = SQLImporter::new("postgres");
        let result = importer
            .parse_liquibase(
                "--liquibase formatted sql\n--changeset user:1\nCREATE TABLE test (id INT);\n",
            )
            .unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn test_databricks_identifier_with_literal() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE IDENTIFIER('test_table') (id STRING);";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("test_table"));
    }

    #[test]
    fn test_databricks_identifier_with_variable() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE IDENTIFIER(:table_name) (id STRING);";
        let result = importer.parse(sql).unwrap();
        // Should create placeholder table name and add to tables_requiring_name
        assert_eq!(result.tables.len(), 1);
        assert!(
            result.tables[0]
                .name
                .as_deref()
                .unwrap()
                .starts_with("__databricks_table_")
        );
        assert_eq!(result.tables_requiring_name.len(), 1);
    }

    #[test]
    fn test_databricks_identifier_with_concatenation() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE IDENTIFIER(:catalog || '.schema.table') (id STRING);";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        // Should extract table name from concatenation
        assert_eq!(result.tables[0].name.as_deref(), Some("schema.table"));
    }

    #[test]
    fn test_databricks_variable_in_struct() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE example (metadata STRUCT<key: STRING, value: :variable_type, timestamp: TIMESTAMP>);";
        let result = importer.parse(sql).unwrap();
        if !result.errors.is_empty() {
            eprintln!("Parse errors: {:?}", result.errors);
        }
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        // Variable should be replaced with STRING
        assert!(
            result.tables[0].columns[0]
                .data_type
                .contains("value: STRING")
        );
    }

    #[test]
    fn test_databricks_variable_in_array() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE example (items ARRAY<:element_type>);";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        // Variable should be replaced with STRING
        assert_eq!(result.tables[0].columns[0].data_type, "ARRAY<STRING>");
    }

    #[test]
    fn test_databricks_nested_variables() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE example (events ARRAY<STRUCT<id: STRING, name: STRING, details: STRUCT<name: STRING, status: :variable_type, timestamp: TIMESTAMP>>>);";
        let result = importer.parse(sql).unwrap();
        if !result.errors.is_empty() {
            eprintln!("Parse errors: {:?}", result.errors);
        }
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        // Nested variable should be replaced with STRING
        assert!(
            result.tables[0].columns[0]
                .data_type
                .contains("status: STRING")
        );
    }

    #[test]
    fn test_databricks_comment_variable() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE example (id STRING) COMMENT ':comment_variable';";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn test_databricks_tblproperties_variable() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE TABLE example (id STRING) TBLPROPERTIES ('key1' = ':variable_value', 'key2' = 'static_value');";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn test_databricks_column_variable() {
        let importer = SQLImporter::new("databricks");
        // Test column definition with variable reference like "column_name :variable STRING"
        // This pattern may need preprocessing to remove the variable
        let sql = "CREATE TABLE example (id :id_var STRING, name :name_var STRING);";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_databricks_create_view() {
        let importer = SQLImporter::new("databricks");
        let sql = "CREATE VIEW example_view AS SELECT id, name FROM source_table;";
        let result = importer.parse(sql).unwrap();
        // Views should be imported as table-like entities
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("example_view"));
    }

    #[test]
    fn test_databricks_view_with_identifier() {
        let importer = SQLImporter::new("databricks");
        let sql =
            "CREATE VIEW IDENTIFIER(:catalog || '.schema.view_name') AS SELECT * FROM table1;";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        // Should extract view name from IDENTIFIER() expression
        assert_eq!(result.tables[0].name.as_deref(), Some("schema.view_name"));
    }

    #[test]
    fn test_databricks_create_materialized_view() {
        let importer = SQLImporter::new("databricks");
        // MATERIALIZED VIEW is preprocessed to CREATE VIEW for sqlparser compatibility
        let sql = "CREATE MATERIALIZED VIEW mv_example AS SELECT id, name FROM source_table;";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("mv_example"));
    }

    #[test]
    fn test_databricks_using_delta() {
        let importer = SQLImporter::new("databricks");
        // USING DELTA is preprocessed out for sqlparser compatibility
        let sql = "CREATE TABLE delta_table (id INT, name STRING) USING DELTA;";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("delta_table"));
        assert_eq!(result.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_databricks_using_parquet() {
        let importer = SQLImporter::new("databricks");
        // USING PARQUET is preprocessed out for sqlparser compatibility
        let sql = "CREATE TABLE parquet_table (id INT, data STRING) USING PARQUET;";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("parquet_table"));
    }

    #[test]
    fn test_databricks_using_delta_with_tblproperties() {
        let importer = SQLImporter::new("databricks");
        // Combined USING DELTA and TBLPROPERTIES - both should be removed
        let sql = r#"CREATE TABLE complex_table (
            id INT,
            name STRING
        ) USING DELTA
        TBLPROPERTIES ('delta.autoOptimize.optimizeWrite' = 'true');"#;
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("complex_table"));
        assert_eq!(result.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_databricks_materialized_view_using_delta() {
        let importer = SQLImporter::new("databricks");
        // Combined MATERIALIZED VIEW - should work
        let sql = "CREATE MATERIALIZED VIEW mv_delta AS SELECT id FROM source;";
        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("mv_delta"));
    }
}
