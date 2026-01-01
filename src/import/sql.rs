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
use sqlparser::ast::{ColumnDef, ColumnOption, ObjectName, Statement, TableConstraint};
use sqlparser::dialect::{Dialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;

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
    /// * `dialect` - SQL dialect name ("postgres", "mysql", "sqlite", "generic")
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::sql::SQLImporter;
    ///
    /// let importer = SQLImporter::new("postgres");
    /// ```
    pub fn new(dialect: &str) -> Self {
        Self {
            dialect: dialect.to_string(),
        }
    }

    /// Parse SQL and extract table definitions
    ///
    /// # Arguments
    ///
    /// * `sql` - SQL string containing CREATE TABLE statements
    ///
    /// # Returns
    ///
    /// An `ImportResult` containing extracted tables and any parse errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::sql::SQLImporter;
    ///
    /// let importer = SQLImporter::new("postgres");
    /// let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));";
    /// let result = importer.parse(sql).unwrap();
    /// assert_eq!(result.tables.len(), 1);
    /// ```
    pub fn parse(&self, sql: &str) -> Result<ImportResult> {
        let dialect = self.dialect_impl();
        let statements = match Parser::parse_sql(dialect.as_ref(), sql) {
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

        for (idx, stmt) in statements.into_iter().enumerate() {
            if let Statement::CreateTable(create) = stmt {
                match self.parse_create_table(
                    idx,
                    &create.name,
                    &create.columns,
                    &create.constraints,
                ) {
                    Ok(t) => tables.push(t),
                    Err(e) => errors.push(ImportError::ParseError(e)),
                }
            }
            // Only CREATE TABLE statements are relevant for data modeling.
            // Other statements (INSERT, UPDATE, DELETE, etc.) are ignored.
        }

        Ok(ImportResult {
            tables,
            tables_requiring_name: Vec::new(),
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
            "postgres" | "postgresql" => Box::new(PostgreSqlDialect {}),
            "mysql" => Box::new(MySqlDialect {}),
            "sqlite" => Box::new(SQLiteDialect {}),
            _ => Box::new(GenericDialect {}),
        }
    }

    fn object_name_to_string(name: &ObjectName) -> String {
        // Use final identifier (supports schema-qualified names).
        name.0
            .last()
            .map(|ident| ident.value.clone())
            .unwrap_or_else(|| name.to_string())
    }

    fn parse_create_table(
        &self,
        table_index: usize,
        name: &ObjectName,
        columns: &[ColumnDef],
        constraints: &[TableConstraint],
    ) -> std::result::Result<TableData, String> {
        let table_name = Self::object_name_to_string(name);

        // Validate table name (warnings are logged but don't fail import)
        if let Err(e) = validate_table_name(&table_name) {
            // Log warning but continue - imported SQL may have valid but unusual names
            tracing::warn!("Table name validation warning: {}", e);
        }

        // Collect PK columns from table-level constraints.
        let mut pk_cols = std::collections::HashSet::<String>::new();
        for c in constraints {
            if let TableConstraint::PrimaryKey { columns, .. } = c {
                for col in columns {
                    pk_cols.insert(col.value.clone());
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
                    ColumnOption::Unique { is_primary, .. } => {
                        if *is_primary {
                            is_pk = true;
                        }
                    }
                    _ => {}
                }
            }

            if pk_cols.contains(&col.name.value) {
                is_pk = true;
            }

            let col_name = col.name.value.clone();
            let data_type = col.data_type.to_string();

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
            });
        }

        Ok(TableData {
            table_index,
            name: Some(table_name),
            columns: out_cols,
        })
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
}
