//! Validation functionality
//!
//! Provides validation logic for:
//! - Table validation (naming conflicts, pattern exclusivity)
//! - Relationship validation (circular dependencies)
//! - Input validation and sanitization (security)

pub mod input;
pub mod relationships;
pub mod tables;
pub mod xml;

pub use input::{
    ValidationError, sanitize_model_name, sanitize_sql_identifier, validate_bpmn_dmn_file_size,
    validate_column_name, validate_data_type, validate_openapi_file_size, validate_table_name,
    validate_uuid,
};
pub use relationships::{RelationshipValidationError, RelationshipValidationResult};
pub use tables::{TableValidationError, TableValidationResult};
pub use xml::{load_xsd_schema, validate_xml_against_xsd};
