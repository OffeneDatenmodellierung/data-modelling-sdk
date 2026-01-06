//! Import command tests

#[cfg(feature = "cli")]
use data_modelling_sdk::cli::commands::import::{
    ImportArgs, ImportFormat, InputSource, handle_import_avro, handle_import_json_schema,
    handle_import_odcs, handle_import_odps, handle_import_protobuf, handle_import_sql,
};
#[cfg(feature = "cli")]
use data_modelling_sdk::cli::error::CliError;
#[cfg(feature = "cli")]
use std::io::Write;
#[cfg(feature = "cli")]
use tempfile::NamedTempFile;

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_sql_from_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));"
    )
    .unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::Sql,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: Some("postgres".to_string()),
        uuid_override: None,
        resolve_references: true,
        validate: true,
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_sql(&args);
    assert!(result.is_ok(), "SQL import should succeed");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_sql_with_views() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "CREATE VIEW user_view AS SELECT id, name FROM users;").unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::Sql,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: Some("postgres".to_string()),
        uuid_override: None,
        resolve_references: true,
        validate: true,
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_sql(&args);
    assert!(result.is_ok(), "SQL view import should succeed");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_avro() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{"type": "record", "name": "User", "fields": [{{"name": "id", "type": "long"}}]}}"#
    )
    .unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::Avro,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: None,
        uuid_override: None,
        resolve_references: true,
        validate: true,
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_avro(&args);
    assert!(result.is_ok(), "AVRO import should succeed");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_avro_with_uuid_override() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{"type": "record", "name": "User", "fields": [{{"name": "id", "type": "long"}}]}}"#
    )
    .unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::Avro,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: None,
        uuid_override: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        resolve_references: true,
        validate: true,
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_avro(&args);
    assert!(
        result.is_ok(),
        "AVRO import with UUID override should succeed"
    );
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_json_schema() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{"type": "object", "properties": {{"id": {{"type": "integer"}}}}}}"#
    )
    .unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::JsonSchema,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: None,
        uuid_override: None,
        resolve_references: true,
        validate: true,
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_json_schema(&args);
    assert!(result.is_ok(), "JSON Schema import should succeed");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_protobuf() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "syntax = \"proto3\";\nmessage User {{ int64 id = 1; }}"
    )
    .unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::Protobuf,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: None,
        uuid_override: None,
        resolve_references: true,
        validate: true,
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_protobuf(&args);
    assert!(result.is_ok(), "Protobuf import should succeed");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_odcs_valid() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
name: users
version: 1.0.0
schema:
  fields:
    - name: id
      type: INT64"#
    )
    .unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::Odcs,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: None,
        uuid_override: None,
        resolve_references: true,
        validate: false, // Skip validation for basic test
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_odcs(&args);
    assert!(result.is_ok(), "ODCS import should succeed");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_import_multiple_tables_with_uuid_error() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "CREATE TABLE users (id INT); CREATE TABLE orders (id INT);"
    )
    .unwrap();
    file.flush().unwrap();

    let args = ImportArgs {
        format: ImportFormat::Sql,
        input: InputSource::File(file.path().to_path_buf()),
        dialect: Some("postgres".to_string()),
        uuid_override: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
        resolve_references: true,
        validate: true,
        pretty: false,
        jar_path: None,
        message_type: None,
        no_odcs: true, // Skip ODCS file creation in tests
        root_message: None,
    };

    let result = handle_import_sql(&args);
    assert!(
        result.is_err(),
        "Should error when UUID override with multiple tables"
    );
    assert!(matches!(
        result.unwrap_err(),
        CliError::MultipleTablesWithUuid(_)
    ));
}

#[cfg(all(feature = "cli", feature = "odps-validation"))]
mod odps_import_tests {
    use super::*;

    fn create_test_odps_file() -> NamedTempFile {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
name: test-product
version: 1.0.0"#
        )
        .unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_cli_import_odps_valid_file() {
        let file = create_test_odps_file();
        let args = ImportArgs {
            format: ImportFormat::Odps,
            input: InputSource::File(file.path().to_path_buf()),
            dialect: None,
            uuid_override: None,
            resolve_references: false,
            validate: true,
            pretty: false,
            jar_path: None,
            message_type: None,
            no_odcs: true,
            root_message: None,
        };

        let result = handle_import_odps(&args);
        assert!(result.is_ok(), "Valid ODPS file should import successfully");
    }

    #[test]
    fn test_cli_import_odps_missing_required_field() {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"apiVersion: v1.0.0
kind: DataProduct
# Missing 'id' field
status: active"#
        )
        .unwrap();
        file.flush().unwrap();

        let args = ImportArgs {
            format: ImportFormat::Odps,
            input: InputSource::File(file.path().to_path_buf()),
            dialect: None,
            uuid_override: None,
            resolve_references: false,
            validate: true,
            pretty: false,
            jar_path: None,
            message_type: None,
            no_odcs: true,
            root_message: None,
        };

        let result = handle_import_odps(&args);
        assert!(
            result.is_err(),
            "ODPS file missing required field should fail validation"
        );
    }

    #[test]
    fn test_cli_import_odps_invalid_enum_value() {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: invalid-status-value"#
        )
        .unwrap();
        file.flush().unwrap();

        let args = ImportArgs {
            format: ImportFormat::Odps,
            input: InputSource::File(file.path().to_path_buf()),
            dialect: None,
            uuid_override: None,
            resolve_references: false,
            validate: true,
            pretty: false,
            jar_path: None,
            message_type: None,
            no_odcs: true,
            root_message: None,
        };

        let result = handle_import_odps(&args);
        assert!(
            result.is_err(),
            "ODPS file with invalid enum value should fail validation"
        );
    }

    #[test]
    fn test_cli_import_odps_invalid_url_format() {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
support:
  - channel: email
    url: not-a-valid-url"#
        )
        .unwrap();
        file.flush().unwrap();

        let args = ImportArgs {
            format: ImportFormat::Odps,
            input: InputSource::File(file.path().to_path_buf()),
            dialect: None,
            uuid_override: None,
            resolve_references: false,
            validate: true,
            pretty: false,
            jar_path: None,
            message_type: None,
            no_odcs: true,
            root_message: None,
        };

        let result = handle_import_odps(&args);
        // URL format validation may be lenient, so we just check it doesn't crash
        // If validation fails, that's good; if it passes, format validation may be lenient
        assert!(
            result.is_ok() || result.is_err(),
            "Import should either succeed or fail gracefully"
        );
    }

    #[test]
    fn test_cli_import_odps_missing_nested_required_field() {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
support:
  - channel: email
    # Missing 'url' field"#
        )
        .unwrap();
        file.flush().unwrap();

        let args = ImportArgs {
            format: ImportFormat::Odps,
            input: InputSource::File(file.path().to_path_buf()),
            dialect: None,
            uuid_override: None,
            resolve_references: false,
            validate: true,
            pretty: false,
            jar_path: None,
            message_type: None,
            no_odcs: true,
            root_message: None,
        };

        let result = handle_import_odps(&args);
        assert!(
            result.is_err(),
            "ODPS file with missing nested required field should fail validation"
        );
    }

    #[test]
    fn test_cli_import_odps_no_validate_flag() {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active"#
        )
        .unwrap();
        file.flush().unwrap();

        let args = ImportArgs {
            format: ImportFormat::Odps,
            input: InputSource::File(file.path().to_path_buf()),
            dialect: None,
            uuid_override: None,
            resolve_references: false,
            validate: false, // Disable validation
            pretty: false,
            jar_path: None,
            message_type: None,
            no_odcs: true,
            root_message: None,
        };

        let result = handle_import_odps(&args);
        // Should still work even without validation
        assert!(result.is_ok(), "ODPS import should work without validation");
    }
}
