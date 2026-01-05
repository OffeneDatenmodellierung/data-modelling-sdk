//! Tests for schema validation functions

#[cfg(all(feature = "cli", feature = "odps-validation"))]
mod odps_validation_tests {
    use data_modelling_sdk::cli::validation::validate_odps;

    #[test]
    fn test_validate_odps_valid_file() {
        let valid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
name: test-product
version: 1.0.0
"#;

        let result = validate_odps(valid_odps);
        assert!(result.is_ok(), "Valid ODPS file should pass validation");
    }

    #[test]
    fn test_validate_odps_missing_required_field() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
# Missing 'id' field
status: active
"#;

        let result = validate_odps(invalid_odps);
        assert!(result.is_err(), "ODPS file missing required field should fail validation");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("ODPS validation failed"), "Error message should indicate validation failure");
        assert!(error_msg.contains("id") || error_msg.contains("missing"), "Error should mention missing id field");
    }

    #[test]
    fn test_validate_odps_invalid_enum_value() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: invalid-status
"#;

        let result = validate_odps(invalid_odps);
        assert!(result.is_err(), "ODPS file with invalid enum value should fail validation");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("ODPS validation failed"), "Error message should indicate validation failure");
    }

    #[test]
    fn test_validate_odps_invalid_url_format() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
support:
  - channel: email
    url: not-a-valid-url
"#;

        let result = validate_odps(invalid_odps);
        assert!(result.is_err(), "ODPS file with invalid URL format should fail validation");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("ODPS validation failed"), "Error message should indicate validation failure");
    }

    #[test]
    fn test_validate_odps_missing_nested_required_field() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
support:
  - channel: email
    # Missing 'url' field
"#;

        let result = validate_odps(invalid_odps);
        assert!(result.is_err(), "ODPS file with missing nested required field should fail validation");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("ODPS validation failed"), "Error message should indicate validation failure");
    }
}
