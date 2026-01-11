//! Validation functionality
//!
//! Provides validation logic for:
//! - Table validation (naming conflicts, pattern exclusivity)
//! - Relationship validation (circular dependencies)
//! - Input validation and sanitization (security)
//! - JSON Schema validation for various file formats (ODCS, ODCL, Decision, Knowledge, etc.)

pub mod input;
pub mod relationships;
pub mod schema;
pub mod tables;
pub mod xml;

pub use input::{
    ValidationError, sanitize_model_name, sanitize_path, sanitize_sql_identifier,
    validate_bpmn_dmn_file_size, validate_column_name, validate_data_type, validate_glob_pattern,
    validate_openapi_file_size, validate_path, validate_table_name, validate_url, validate_uuid,
};
pub use relationships::{RelationshipValidationError, RelationshipValidationResult};
pub use schema::{
    validate_avro_internal, validate_cads_internal, validate_decision_internal,
    validate_decisions_index_internal, validate_json_schema_internal,
    validate_knowledge_index_internal, validate_knowledge_internal, validate_odcl_internal,
    validate_odcs_internal, validate_odps_internal, validate_openapi_internal,
    validate_protobuf_internal, validate_relationships_internal, validate_sql_internal,
    validate_workspace_internal,
};
pub use tables::{TableValidationError, TableValidationResult};
pub use xml::{load_xsd_schema, validate_xml_against_xsd};
