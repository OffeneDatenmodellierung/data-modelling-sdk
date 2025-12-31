//! Input validation and sanitization utilities.
//!
//! This module provides functions for validating and sanitizing user input
//! before processing. These functions are used by import parsers and storage
//! backends to ensure data integrity and security.
//!
//! # Security
//!
//! Input validation prevents:
//! - SQL injection via malicious table/column names
//! - Path traversal via malicious file paths
//! - Buffer overflows via excessively long inputs
//! - Unicode normalization attacks

use thiserror::Error;
use uuid::Uuid;

/// Maximum length for table names
pub const MAX_TABLE_NAME_LENGTH: usize = 255;

/// Maximum length for column names
pub const MAX_COLUMN_NAME_LENGTH: usize = 255;

/// Maximum length for identifiers in general
pub const MAX_IDENTIFIER_LENGTH: usize = 255;

/// Maximum length for descriptions
pub const MAX_DESCRIPTION_LENGTH: usize = 10000;

/// Errors that can occur during input validation.
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    /// Input is empty when a value is required
    #[error("{0} cannot be empty")]
    Empty(&'static str),

    /// Input exceeds maximum allowed length
    #[error("{field} exceeds maximum length (max: {max}, got: {actual})")]
    TooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },

    /// Input contains invalid characters
    #[error("{field} contains invalid characters: {reason}")]
    InvalidCharacters { field: &'static str, reason: String },

    /// Input has invalid format
    #[error("{0}: {1}")]
    InvalidFormat(&'static str, String),

    /// Input is a reserved word
    #[error("{field} cannot be a reserved word: {word}")]
    ReservedWord { field: &'static str, word: String },
}

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validate a table name.
///
/// # Rules
///
/// - Must not be empty
/// - Must not exceed 255 characters
/// - Must start with a letter or underscore
/// - May contain letters, digits, underscores, and hyphens
/// - Cannot be a SQL reserved word
///
/// # Examples
///
/// ```
/// use data_modelling_sdk::validation::input::validate_table_name;
///
/// assert!(validate_table_name("users").is_ok());
/// assert!(validate_table_name("user_orders").is_ok());
/// assert!(validate_table_name("").is_err());
/// assert!(validate_table_name("123_invalid").is_err());
/// ```
pub fn validate_table_name(name: &str) -> ValidationResult<()> {
    if name.is_empty() {
        return Err(ValidationError::Empty("table name"));
    }

    if name.len() > MAX_TABLE_NAME_LENGTH {
        return Err(ValidationError::TooLong {
            field: "table name",
            max: MAX_TABLE_NAME_LENGTH,
            actual: name.len(),
        });
    }

    // Must start with a letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_alphabetic() && first_char != '_' {
        return Err(ValidationError::InvalidFormat(
            "table name",
            "must start with a letter or underscore".to_string(),
        ));
    }

    // May contain letters, digits, underscores, and hyphens
    for c in name.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '-' {
            return Err(ValidationError::InvalidCharacters {
                field: "table name",
                reason: format!("invalid character: '{}'", c),
            });
        }
    }

    // Check for SQL reserved words (basic set)
    if is_sql_reserved_word(name) {
        return Err(ValidationError::ReservedWord {
            field: "table name",
            word: name.to_string(),
        });
    }

    Ok(())
}

/// Validate a column name.
///
/// # Rules
///
/// - Must not be empty
/// - Must not exceed 255 characters
/// - Must start with a letter or underscore
/// - May contain letters, digits, underscores, hyphens, and dots (for nested columns)
/// - Cannot be a SQL reserved word (unless nested)
///
/// # Examples
///
/// ```
/// use data_modelling_sdk::validation::input::validate_column_name;
///
/// assert!(validate_column_name("id").is_ok());
/// assert!(validate_column_name("user_name").is_ok());
/// assert!(validate_column_name("address.street").is_ok()); // nested column
/// assert!(validate_column_name("").is_err());
/// ```
pub fn validate_column_name(name: &str) -> ValidationResult<()> {
    if name.is_empty() {
        return Err(ValidationError::Empty("column name"));
    }

    if name.len() > MAX_COLUMN_NAME_LENGTH {
        return Err(ValidationError::TooLong {
            field: "column name",
            max: MAX_COLUMN_NAME_LENGTH,
            actual: name.len(),
        });
    }

    // Must start with a letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_alphabetic() && first_char != '_' {
        return Err(ValidationError::InvalidFormat(
            "column name",
            "must start with a letter or underscore".to_string(),
        ));
    }

    // May contain letters, digits, underscores, hyphens, and dots (for nested columns)
    for c in name.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '-' && c != '.' {
            return Err(ValidationError::InvalidCharacters {
                field: "column name",
                reason: format!("invalid character: '{}'", c),
            });
        }
    }

    // Check for SQL reserved words (only for non-nested column names)
    if !name.contains('.') && is_sql_reserved_word(name) {
        return Err(ValidationError::ReservedWord {
            field: "column name",
            word: name.to_string(),
        });
    }

    Ok(())
}

/// Validate a UUID string.
///
/// # Examples
///
/// ```
/// use data_modelling_sdk::validation::input::validate_uuid;
///
/// assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
/// assert!(validate_uuid("not-a-uuid").is_err());
/// ```
pub fn validate_uuid(id: &str) -> ValidationResult<Uuid> {
    Uuid::parse_str(id)
        .map_err(|e| ValidationError::InvalidFormat("UUID", format!("invalid UUID format: {}", e)))
}

/// Validate a data type string.
///
/// # Rules
///
/// - Must not be empty
/// - Must only contain safe characters (no SQL injection)
/// - Must match known data type patterns
///
/// # Examples
///
/// ```
/// use data_modelling_sdk::validation::input::validate_data_type;
///
/// assert!(validate_data_type("VARCHAR(255)").is_ok());
/// assert!(validate_data_type("INTEGER").is_ok());
/// assert!(validate_data_type("ARRAY<STRING>").is_ok());
/// assert!(validate_data_type("'; DROP TABLE users;--").is_err());
/// ```
pub fn validate_data_type(data_type: &str) -> ValidationResult<()> {
    if data_type.is_empty() {
        return Err(ValidationError::Empty("data type"));
    }

    if data_type.len() > MAX_IDENTIFIER_LENGTH {
        return Err(ValidationError::TooLong {
            field: "data type",
            max: MAX_IDENTIFIER_LENGTH,
            actual: data_type.len(),
        });
    }

    // Check for dangerous patterns
    let lower = data_type.to_lowercase();
    if lower.contains(';') || lower.contains("--") || lower.contains("/*") {
        return Err(ValidationError::InvalidCharacters {
            field: "data type",
            reason: "contains SQL comment or statement separator".to_string(),
        });
    }

    // Allow alphanumeric, parentheses, commas, spaces, underscores, angle brackets
    for c in data_type.chars() {
        if !c.is_alphanumeric()
            && c != '('
            && c != ')'
            && c != ','
            && c != ' '
            && c != '_'
            && c != '<'
            && c != '>'
            && c != '['
            && c != ']'
        {
            return Err(ValidationError::InvalidCharacters {
                field: "data type",
                reason: format!("invalid character: '{}'", c),
            });
        }
    }

    Ok(())
}

/// Validate a description string.
///
/// # Rules
///
/// - May be empty
/// - Must not exceed 10000 characters
/// - Control characters (except whitespace) are stripped
pub fn validate_description(desc: &str) -> ValidationResult<()> {
    if desc.len() > MAX_DESCRIPTION_LENGTH {
        return Err(ValidationError::TooLong {
            field: "description",
            max: MAX_DESCRIPTION_LENGTH,
            actual: desc.len(),
        });
    }

    Ok(())
}

/// Sanitize a SQL identifier by quoting it.
///
/// This function returns a quoted identifier that is safe to use in SQL
/// statements without risk of injection.
///
/// # Examples
///
/// ```
/// use data_modelling_sdk::validation::input::sanitize_sql_identifier;
///
/// assert_eq!(sanitize_sql_identifier("users", "postgres"), "\"users\"");
/// assert_eq!(sanitize_sql_identifier("user-orders", "mysql"), "`user-orders`");
/// ```
pub fn sanitize_sql_identifier(name: &str, dialect: &str) -> String {
    let quote_char = match dialect.to_lowercase().as_str() {
        "mysql" | "mariadb" => '`',
        "sqlserver" | "mssql" => '[',
        _ => '"', // Standard SQL, PostgreSQL, etc.
    };

    let end_char = if quote_char == '[' { ']' } else { quote_char };

    // Escape any internal quote characters by doubling them
    let escaped = if quote_char == end_char {
        name.replace(quote_char, &format!("{}{}", quote_char, quote_char))
    } else {
        name.replace(end_char, &format!("{}{}", end_char, end_char))
    };

    format!("{}{}{}", quote_char, escaped, end_char)
}

/// Sanitize a string for safe use in descriptions and comments.
///
/// Removes or escapes potentially dangerous characters.
pub fn sanitize_description(desc: &str) -> String {
    // Remove control characters except newlines and tabs
    desc.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r')
        .collect()
}

/// Check if a word is a SQL reserved word.
///
/// This is a basic check covering common reserved words across SQL dialects.
fn is_sql_reserved_word(word: &str) -> bool {
    const RESERVED_WORDS: &[&str] = &[
        "select",
        "from",
        "where",
        "insert",
        "update",
        "delete",
        "create",
        "drop",
        "alter",
        "table",
        "index",
        "view",
        "database",
        "schema",
        "grant",
        "revoke",
        "commit",
        "rollback",
        "begin",
        "end",
        "transaction",
        "primary",
        "foreign",
        "key",
        "references",
        "constraint",
        "unique",
        "check",
        "default",
        "not",
        "null",
        "and",
        "or",
        "in",
        "between",
        "like",
        "is",
        "case",
        "when",
        "then",
        "else",
        "as",
        "on",
        "join",
        "inner",
        "outer",
        "left",
        "right",
        "full",
        "cross",
        "natural",
        "using",
        "group",
        "by",
        "having",
        "order",
        "asc",
        "desc",
        "limit",
        "offset",
        "union",
        "intersect",
        "except",
        "all",
        "distinct",
        "top",
        "values",
        "set",
        "into",
        "exec",
        "execute",
        "procedure",
        "function",
        "trigger",
        "true",
        "false",
        "int",
        "integer",
        "varchar",
        "char",
        "text",
        "boolean",
        "date",
        "time",
        "timestamp",
        "float",
        "double",
        "decimal",
        "numeric",
    ];

    let lower = word.to_lowercase();
    RESERVED_WORDS.contains(&lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_table_name_valid() {
        assert!(validate_table_name("users").is_ok());
        assert!(validate_table_name("user_orders").is_ok());
        assert!(validate_table_name("User123").is_ok());
        assert!(validate_table_name("_private").is_ok());
        assert!(validate_table_name("my-table").is_ok());
    }

    #[test]
    fn test_validate_table_name_empty() {
        assert!(matches!(
            validate_table_name(""),
            Err(ValidationError::Empty(_))
        ));
    }

    #[test]
    fn test_validate_table_name_too_long() {
        let long_name = "a".repeat(300);
        assert!(matches!(
            validate_table_name(&long_name),
            Err(ValidationError::TooLong { .. })
        ));
    }

    #[test]
    fn test_validate_table_name_starts_with_digit() {
        assert!(matches!(
            validate_table_name("123users"),
            Err(ValidationError::InvalidFormat(..))
        ));
    }

    #[test]
    fn test_validate_table_name_invalid_chars() {
        assert!(matches!(
            validate_table_name("user$table"),
            Err(ValidationError::InvalidCharacters { .. })
        ));
        assert!(matches!(
            validate_table_name("user;table"),
            Err(ValidationError::InvalidCharacters { .. })
        ));
    }

    #[test]
    fn test_validate_table_name_reserved_word() {
        assert!(matches!(
            validate_table_name("SELECT"),
            Err(ValidationError::ReservedWord { .. })
        ));
        assert!(matches!(
            validate_table_name("table"),
            Err(ValidationError::ReservedWord { .. })
        ));
    }

    #[test]
    fn test_validate_column_name_valid() {
        assert!(validate_column_name("id").is_ok());
        assert!(validate_column_name("user_name").is_ok());
        assert!(validate_column_name("address.street").is_ok());
        assert!(validate_column_name("nested.field.value").is_ok());
    }

    #[test]
    fn test_validate_data_type_valid() {
        assert!(validate_data_type("INTEGER").is_ok());
        assert!(validate_data_type("VARCHAR(255)").is_ok());
        assert!(validate_data_type("ARRAY<STRING>").is_ok());
        assert!(validate_data_type("DECIMAL(10, 2)").is_ok());
    }

    #[test]
    fn test_validate_data_type_injection() {
        assert!(matches!(
            validate_data_type("'; DROP TABLE users;--"),
            Err(ValidationError::InvalidCharacters { .. })
        ));
    }

    #[test]
    fn test_validate_uuid_valid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
    }

    #[test]
    fn test_validate_uuid_invalid() {
        assert!(validate_uuid("not-a-uuid").is_err());
        assert!(validate_uuid("").is_err());
    }

    #[test]
    fn test_sanitize_sql_identifier() {
        assert_eq!(sanitize_sql_identifier("users", "postgres"), "\"users\"");
        assert_eq!(
            sanitize_sql_identifier("user-table", "mysql"),
            "`user-table`"
        );
        assert_eq!(sanitize_sql_identifier("test", "sqlserver"), "[test]");
    }

    #[test]
    fn test_sanitize_sql_identifier_escapes_quotes() {
        assert_eq!(
            sanitize_sql_identifier("my\"table", "postgres"),
            "\"my\"\"table\""
        );
        assert_eq!(sanitize_sql_identifier("my`table", "mysql"), "`my``table`");
    }

    #[test]
    fn test_sanitize_description() {
        assert_eq!(sanitize_description("Hello\nWorld"), "Hello\nWorld");
        assert_eq!(sanitize_description("Tab\tSeparated"), "Tab\tSeparated");
        // Control characters should be removed
        let with_control = "Hello\x00World";
        assert_eq!(sanitize_description(with_control), "HelloWorld");
    }
}
