//! Export command tests

#[cfg(feature = "cli")]
use data_modelling_sdk::cli::commands::export::{
    ExportArgs, ExportFormat, handle_export_avro, handle_export_json_schema, handle_export_odcs,
    handle_export_protobuf,
};
#[cfg(feature = "cli")]
use tempfile::NamedTempFile;

#[cfg(feature = "cli")]
fn create_test_odcs_file() -> NamedTempFile {
    use std::io::Write;
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
    file
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_export_odcs_single_table() {
    let input_file = create_test_odcs_file();
    let output_file = NamedTempFile::new().unwrap();

    let args = ExportArgs {
        format: ExportFormat::Odcs,
        input: input_file.path().to_path_buf(),
        output: output_file.path().to_path_buf(),
        force: true,
        protoc_path: None,
        protobuf_version: None,
    };

    let result = handle_export_odcs(&args);
    assert!(result.is_ok(), "ODCS export should succeed");
    assert!(output_file.path().exists(), "Output file should be created");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_export_avro() {
    let input_file = create_test_odcs_file();
    let output_file = NamedTempFile::new().unwrap();

    let args = ExportArgs {
        format: ExportFormat::Avro,
        input: input_file.path().to_path_buf(),
        output: output_file.path().to_path_buf(),
        force: true,
        protoc_path: None,
        protobuf_version: None,
    };

    let result = handle_export_avro(&args);
    assert!(result.is_ok(), "AVRO export should succeed");
    assert!(output_file.path().exists(), "Output file should be created");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_export_json_schema() {
    let input_file = create_test_odcs_file();
    let output_file = NamedTempFile::new().unwrap();

    let args = ExportArgs {
        format: ExportFormat::JsonSchema,
        input: input_file.path().to_path_buf(),
        output: output_file.path().to_path_buf(),
        force: true,
        protoc_path: None,
        protobuf_version: None,
    };

    let result = handle_export_json_schema(&args);
    assert!(result.is_ok(), "JSON Schema export should succeed");
    assert!(output_file.path().exists(), "Output file should be created");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_export_protobuf() {
    let input_file = create_test_odcs_file();
    let output_file = NamedTempFile::new().unwrap();

    let args = ExportArgs {
        format: ExportFormat::Protobuf,
        input: input_file.path().to_path_buf(),
        output: output_file.path().to_path_buf(),
        force: true,
        protoc_path: None,
        protobuf_version: None,
    };

    let result = handle_export_protobuf(&args);
    assert!(result.is_ok(), "Protobuf export should succeed");
    assert!(output_file.path().exists(), "Output file should be created");
}

#[cfg(feature = "cli")]
#[test]
fn test_cli_export_odcs_file_overwrite_prompt() {
    let input_file = create_test_odcs_file();
    let output_file = NamedTempFile::new().unwrap();
    std::fs::write(output_file.path(), "existing content").unwrap();

    let args = ExportArgs {
        format: ExportFormat::Odcs,
        input: input_file.path().to_path_buf(),
        output: output_file.path().to_path_buf(),
        force: false, // Don't force overwrite
        protoc_path: None,
        protobuf_version: None,
    };

    let result = handle_export_odcs(&args);
    assert!(
        result.is_err(),
        "Should error when file exists and force is false"
    );
}
