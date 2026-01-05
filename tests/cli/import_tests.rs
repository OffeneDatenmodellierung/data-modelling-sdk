//! Import command tests

#[cfg(feature = "cli")]
use data_modelling_sdk::cli::commands::import::{
    ImportArgs, ImportFormat, InputSource, handle_import_avro, handle_import_json_schema,
    handle_import_odcs, handle_import_protobuf, handle_import_sql,
};
#[cfg(feature = "cli")]
use data_modelling_sdk::cli::error::CliError;
#[cfg(feature = "cli")]
use std::io::Write;
#[cfg(feature = "cli")]
use std::path::PathBuf;
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
