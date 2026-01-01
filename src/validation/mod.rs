//! Validation functionality
//!
//! Provides validation logic for:
//! - Table validation (naming conflicts, pattern exclusivity)
//! - Relationship validation (circular dependencies)
//! - Input validation and sanitization (security)

pub mod input;
pub mod relationships;
pub mod tables;

pub use input::{
    ValidationError, sanitize_sql_identifier, validate_column_name, validate_data_type,
    validate_table_name, validate_uuid,
};
pub use relationships::{RelationshipValidationError, RelationshipValidationResult};
pub use tables::{TableValidationError, TableValidationResult};
