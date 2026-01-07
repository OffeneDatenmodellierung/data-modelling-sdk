//! Validate command implementation

use crate::cli::error::CliError;
use crate::validation::schema::{
    validate_avro_internal, validate_cads_internal, validate_decision_internal,
    validate_decisions_index_internal, validate_json_schema_internal,
    validate_knowledge_index_internal, validate_knowledge_internal, validate_odcl_internal,
    validate_odcs_internal, validate_odps_internal, validate_openapi_internal,
    validate_protobuf_internal, validate_sql_internal,
};
use std::io::Read;
use std::path::PathBuf;

/// Load input content from file or stdin
fn load_input(input: &str) -> Result<String, CliError> {
    if input == "-" {
        // Read from stdin
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|e| CliError::InvalidArgument(format!("Failed to read stdin: {}", e)))?;
        Ok(content)
    } else {
        // Read from file
        let path = PathBuf::from(input);
        std::fs::read_to_string(&path).map_err(|e| CliError::FileReadError(path, e.to_string()))
    }
}

/// Handle the validate command
pub fn handle_validate(format: &str, input: &str) -> Result<(), CliError> {
    let content = load_input(input)?;

    let result = match format {
        "odcs" => validate_odcs_internal(&content),
        "odcl" => validate_odcl_internal(&content),
        "odps" => validate_odps_internal(&content),
        "cads" => validate_cads_internal(&content),
        "openapi" => validate_openapi_internal(&content),
        "protobuf" => validate_protobuf_internal(&content),
        "avro" => validate_avro_internal(&content),
        "json-schema" => validate_json_schema_internal(&content),
        "sql" => validate_sql_internal(&content),
        "decision" => validate_decision_internal(&content),
        "knowledge" => validate_knowledge_internal(&content),
        "decisions-index" => validate_decisions_index_internal(&content),
        "knowledge-index" => validate_knowledge_index_internal(&content),
        _ => {
            return Err(CliError::InvalidArgument(format!(
                "Unknown format: {}",
                format
            )));
        }
    };

    result.map_err(CliError::ValidationError)?;

    println!("Validation successful");
    Ok(())
}
