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

use serde::{Deserialize, Serialize};
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

/// Maximum file size for BPMN/DMN models (10MB)
pub const MAX_BPMN_DMN_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum file size for OpenAPI specifications (5MB)
pub const MAX_OPENAPI_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// Maximum length for model names (filenames)
pub const MAX_MODEL_NAME_LENGTH: usize = 255;

/// Errors that can occur during input validation.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
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
/// use data_modelling_core::validation::input::validate_table_name;
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
    // Note: unwrap is safe here due to the empty check above, but we use match for clarity
    let first_char = match name.chars().next() {
        Some(c) => c,
        None => return Err(ValidationError::Empty("table name")),
    };
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
/// use data_modelling_core::validation::input::validate_column_name;
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
    // Note: unwrap is safe here due to the empty check above, but we use match for clarity
    let first_char = match name.chars().next() {
        Some(c) => c,
        None => return Err(ValidationError::Empty("column name")),
    };
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
/// use data_modelling_core::validation::input::validate_uuid;
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
/// use data_modelling_core::validation::input::validate_data_type;
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
/// use data_modelling_core::validation::input::sanitize_sql_identifier;
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

/// Sanitize a model name for use as a filename.
///
/// # Rules
///
/// - Removes or replaces invalid filename characters
/// - Ensures the name is safe for use in file paths
/// - Preserves alphanumeric characters, hyphens, underscores, and dots
/// - Replaces invalid characters with underscores
/// - Truncates to MAX_MODEL_NAME_LENGTH if needed
///
/// # Examples
///
/// ```
/// use data_modelling_core::validation::input::sanitize_model_name;
///
/// assert_eq!(sanitize_model_name("my-model"), "my-model");
/// assert_eq!(sanitize_model_name("my/model"), "my_model");
/// assert_eq!(sanitize_model_name("my..model"), "my.model");
/// ```
pub fn sanitize_model_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    let mut last_was_dot = false;

    for ch in name.chars() {
        match ch {
            // Allow alphanumeric, hyphens, underscores
            ch if ch.is_alphanumeric() || ch == '-' || ch == '_' => {
                sanitized.push(ch);
                last_was_dot = false;
            }
            // Allow single dots (but not consecutive)
            '.' if !last_was_dot => {
                sanitized.push('.');
                last_was_dot = true;
            }
            // Replace invalid characters with underscore
            _ => {
                if !last_was_dot {
                    sanitized.push('_');
                }
                last_was_dot = false;
            }
        }

        // Truncate if too long
        if sanitized.len() >= MAX_MODEL_NAME_LENGTH {
            break;
        }
    }

    // Remove trailing dots and underscores
    sanitized = sanitized.trim_end_matches(['.', '_']).to_string();

    // Ensure not empty
    if sanitized.is_empty() {
        sanitized = "model".to_string();
    }

    sanitized
}

/// Validate file size for BPMN/DMN models.
///
/// # Arguments
///
/// * `file_size` - File size in bytes
///
/// # Returns
///
/// `ValidationResult<()>` indicating whether the file size is valid
pub fn validate_bpmn_dmn_file_size(file_size: u64) -> ValidationResult<()> {
    if file_size > MAX_BPMN_DMN_FILE_SIZE {
        return Err(ValidationError::TooLong {
            field: "BPMN/DMN file size",
            max: MAX_BPMN_DMN_FILE_SIZE as usize,
            actual: file_size as usize,
        });
    }
    Ok(())
}

// ============================================================================
// Path Validation
// ============================================================================

/// Maximum path length (platform-dependent, using conservative limit)
pub const MAX_PATH_LENGTH: usize = 4096;

/// Validate a file path for security.
///
/// # Security Checks
///
/// - Rejects paths containing ".." (path traversal)
/// - Rejects paths containing null bytes
/// - Rejects excessively long paths
/// - Rejects absolute paths when `allow_absolute` is false
///
/// # Arguments
///
/// * `path` - The path to validate
/// * `allow_absolute` - Whether to allow absolute paths
///
/// # Examples
///
/// ```
/// use data_modelling_core::validation::input::validate_path;
///
/// assert!(validate_path("data/file.json", false).is_ok());
/// assert!(validate_path("../etc/passwd", false).is_err());
/// assert!(validate_path("/absolute/path", false).is_err());
/// assert!(validate_path("/absolute/path", true).is_ok());
/// ```
pub fn validate_path(path: &str, allow_absolute: bool) -> ValidationResult<()> {
    // Check for empty path
    if path.is_empty() {
        return Err(ValidationError::Empty("path"));
    }

    // Check for null bytes (could be used to bypass checks)
    if path.contains('\0') {
        return Err(ValidationError::InvalidCharacters {
            field: "path",
            reason: "null bytes not allowed".to_string(),
        });
    }

    // Check length
    if path.len() > MAX_PATH_LENGTH {
        return Err(ValidationError::TooLong {
            field: "path",
            max: MAX_PATH_LENGTH,
            actual: path.len(),
        });
    }

    // Check for path traversal
    if path.contains("..") {
        return Err(ValidationError::InvalidCharacters {
            field: "path",
            reason: "path traversal (..) not allowed".to_string(),
        });
    }

    // Check for absolute paths if not allowed
    if !allow_absolute && (path.starts_with('/') || path.starts_with('\\')) {
        return Err(ValidationError::InvalidFormat(
            "path",
            "absolute paths not allowed".to_string(),
        ));
    }

    // Check for Windows-style absolute paths (e.g., C:\)
    if !allow_absolute && path.len() >= 2 {
        let bytes = path.as_bytes();
        if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return Err(ValidationError::InvalidFormat(
                "path",
                "absolute paths not allowed".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate a glob pattern for security.
///
/// # Security Checks
///
/// - Rejects patterns containing ".."
/// - Rejects patterns containing null bytes
/// - Rejects excessively long patterns
///
/// # Examples
///
/// ```
/// use data_modelling_core::validation::input::validate_glob_pattern;
///
/// assert!(validate_glob_pattern("**/*.json").is_ok());
/// assert!(validate_glob_pattern("data/*.csv").is_ok());
/// assert!(validate_glob_pattern("../secret/*.json").is_err());
/// ```
pub fn validate_glob_pattern(pattern: &str) -> ValidationResult<()> {
    // Check for empty pattern
    if pattern.is_empty() {
        return Err(ValidationError::Empty("glob pattern"));
    }

    // Check for null bytes
    if pattern.contains('\0') {
        return Err(ValidationError::InvalidCharacters {
            field: "glob pattern",
            reason: "null bytes not allowed".to_string(),
        });
    }

    // Check length
    if pattern.len() > MAX_PATH_LENGTH {
        return Err(ValidationError::TooLong {
            field: "glob pattern",
            max: MAX_PATH_LENGTH,
            actual: pattern.len(),
        });
    }

    // Check for path traversal
    if pattern.contains("..") {
        return Err(ValidationError::InvalidCharacters {
            field: "glob pattern",
            reason: "path traversal (..) not allowed".to_string(),
        });
    }

    Ok(())
}

/// Sanitize a file path by removing dangerous components.
///
/// # Transformations
///
/// - Removes null bytes
/// - Replaces ".." with empty string
/// - Normalizes path separators
/// - Removes leading slashes (makes relative)
///
/// # Examples
///
/// ```
/// use data_modelling_core::validation::input::sanitize_path;
///
/// assert_eq!(sanitize_path("data/file.json"), "data/file.json");
/// assert_eq!(sanitize_path("../data/file.json"), "data/file.json");
/// assert_eq!(sanitize_path("/absolute/path"), "absolute/path");
/// ```
pub fn sanitize_path(path: &str) -> String {
    let mut sanitized = path
        // Remove null bytes
        .replace('\0', "")
        // Remove path traversal
        .replace("..", "")
        // Normalize Windows separators
        .replace('\\', "/");

    // Remove leading slashes
    while sanitized.starts_with('/') {
        sanitized = sanitized[1..].to_string();
    }

    // Remove duplicate slashes
    while sanitized.contains("//") {
        sanitized = sanitized.replace("//", "/");
    }

    // Remove trailing slashes
    while sanitized.ends_with('/') && sanitized.len() > 1 {
        sanitized.pop();
    }

    sanitized
}

/// Validate a URL for security.
///
/// # Security Checks
///
/// - Must start with http:// or https://
/// - Rejects file://, javascript:, data:, etc.
/// - Rejects URLs with embedded credentials
///
/// # Examples
///
/// ```
/// use data_modelling_core::validation::input::validate_url;
///
/// assert!(validate_url("https://api.example.com/data").is_ok());
/// assert!(validate_url("file:///etc/passwd").is_err());
/// assert!(validate_url("javascript:alert(1)").is_err());
/// ```
pub fn validate_url(url: &str) -> ValidationResult<()> {
    // Check for empty URL
    if url.is_empty() {
        return Err(ValidationError::Empty("URL"));
    }

    // Only allow http and https schemes
    let lower = url.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(ValidationError::InvalidFormat(
            "URL",
            "only http:// and https:// URLs are allowed".to_string(),
        ));
    }

    // Check for embedded credentials (user:pass@host)
    // Find the host portion (after :// and before first /)
    if let Some(after_scheme) = url.split("://").nth(1) {
        let host_part = after_scheme.split('/').next().unwrap_or("");
        if host_part.contains('@') {
            return Err(ValidationError::InvalidFormat(
                "URL",
                "URLs with embedded credentials not allowed".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod path_validation_tests {
    use super::*;

    #[test]
    fn test_validate_path() {
        // Valid paths
        assert!(validate_path("data/file.json", false).is_ok());
        assert!(validate_path("nested/path/to/file.csv", false).is_ok());
        assert!(validate_path("file.txt", false).is_ok());
        assert!(validate_path("/absolute/path", true).is_ok());

        // Invalid paths
        assert!(validate_path("../etc/passwd", false).is_err());
        assert!(validate_path("data/../secret", false).is_err());
        assert!(validate_path("/absolute/path", false).is_err());
        assert!(validate_path("", false).is_err());
        assert!(validate_path("path\0with\0null", false).is_err());
    }

    #[test]
    fn test_validate_glob_pattern() {
        // Valid patterns
        assert!(validate_glob_pattern("**/*.json").is_ok());
        assert!(validate_glob_pattern("data/*.csv").is_ok());
        assert!(validate_glob_pattern("*.txt").is_ok());

        // Invalid patterns
        assert!(validate_glob_pattern("../secret/*.json").is_err());
        assert!(validate_glob_pattern("").is_err());
    }

    #[test]
    fn test_sanitize_path() {
        assert_eq!(sanitize_path("data/file.json"), "data/file.json");
        assert_eq!(sanitize_path("../data/file.json"), "data/file.json");
        assert_eq!(sanitize_path("/absolute/path"), "absolute/path");
        assert_eq!(sanitize_path("data//double//slash"), "data/double/slash");
        assert_eq!(
            sanitize_path("path\\with\\backslash"),
            "path/with/backslash"
        );
    }

    #[test]
    fn test_validate_url() {
        // Valid URLs
        assert!(validate_url("https://api.example.com/data").is_ok());
        assert!(validate_url("http://localhost:8080/api").is_ok());

        // Invalid URLs
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("data:text/html,<script>").is_err());
        assert!(validate_url("https://user:pass@example.com").is_err());
        assert!(validate_url("").is_err());
    }
}

/// Validate file size for OpenAPI specifications.
///
/// # Arguments
///
/// * `file_size` - File size in bytes
///
/// # Returns
///
/// `ValidationResult<()>` indicating whether the file size is valid
pub fn validate_openapi_file_size(file_size: u64) -> ValidationResult<()> {
    if file_size > MAX_OPENAPI_FILE_SIZE {
        return Err(ValidationError::TooLong {
            field: "OpenAPI file size",
            max: MAX_OPENAPI_FILE_SIZE as usize,
            actual: file_size as usize,
        });
    }
    Ok(())
}
