//! Export command tests

#[cfg(feature = "cli")]
use data_modelling_sdk::cli::commands::export::{
    ExportArgs, ExportFormat, handle_export_avro, handle_export_json_schema, handle_export_odcs,
    handle_export_odps, handle_export_protobuf,
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
        logo_url: None,
        header: None,
        footer: None,
        brand_color: None,
        company_name: None,
        include_toc: false,
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
        logo_url: None,
        header: None,
        footer: None,
        brand_color: None,
        company_name: None,
        include_toc: false,
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
        logo_url: None,
        header: None,
        footer: None,
        brand_color: None,
        company_name: None,
        include_toc: false,
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
        logo_url: None,
        header: None,
        footer: None,
        brand_color: None,
        company_name: None,
        include_toc: false,
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
        logo_url: None,
        header: None,
        footer: None,
        brand_color: None,
        company_name: None,
        include_toc: false,
    };

    let result = handle_export_odcs(&args);
    assert!(
        result.is_err(),
        "Should error when file exists and force is false"
    );
}

#[cfg(all(feature = "cli", feature = "odps-validation"))]
mod odps_export_tests {
    use super::*;

    #[test]
    fn test_cli_export_odps_rejects_odcs_input() {
        let input_file = create_test_odcs_file();
        let output_file = NamedTempFile::new().unwrap();

        let args = ExportArgs {
            format: ExportFormat::Odps,
            input: input_file.path().to_path_buf(),
            output: output_file.path().to_path_buf(),
            force: true,
            protoc_path: None,
            protobuf_version: None,
            logo_url: None,
            header: None,
            footer: None,
            brand_color: None,
            company_name: None,
            include_toc: false,
        };

        let result = handle_export_odps(&args);
        assert!(
            result.is_err(),
            "ODPS export should reject ODCS input files"
        );

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("not ODPS format") || error_msg.contains("ODCS"),
                "Error should indicate ODCS input is not accepted: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_cli_export_odps_rejects_odcs_empty_input() {
        use std::io::Write;
        let mut input_file = NamedTempFile::new().unwrap();
        writeln!(input_file, "apiVersion: v3.1.0\nkind: DataContract\nid: 550e8400-e29b-41d4-a716-446655440000\nname: empty\nversion: 1.0.0\nschema:\n  fields: []").unwrap();
        input_file.flush().unwrap();

        let output_file = NamedTempFile::new().unwrap();

        let args = ExportArgs {
            format: ExportFormat::Odps,
            input: input_file.path().to_path_buf(),
            output: output_file.path().to_path_buf(),
            force: true,
            protoc_path: None,
            protobuf_version: None,
            logo_url: None,
            header: None,
            footer: None,
            brand_color: None,
            company_name: None,
            include_toc: false,
        };

        // ODPS export should reject ODCS input
        let result = handle_export_odps(&args);
        assert!(
            result.is_err(),
            "ODPS export should reject ODCS input files"
        );
    }

    #[test]
    fn test_cli_export_odps_from_odps_input() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("input.odps.yaml");
        let output_file = temp_dir.path().join("output.odps.yaml");

        // Create a valid ODPS input file
        let odps_content = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: test-product
version: 1.0.0
status: active
outputPorts:
  - name: output1
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440001
"#;
        fs::write(&input_file, odps_content).unwrap();

        let args = ExportArgs {
            format: ExportFormat::Odps,
            input: input_file.clone(),
            output: output_file.clone(),
            force: false,
            protoc_path: None,
            protobuf_version: None,
            logo_url: None,
            header: None,
            footer: None,
            brand_color: None,
            company_name: None,
            include_toc: false,
        };

        let result = handle_export_odps(&args);
        assert!(result.is_ok(), "ODPS export from ODPS input should succeed");

        // Verify output file exists and contains ODPS content
        assert!(output_file.exists(), "Output file should be created");
        let output_content = fs::read_to_string(&output_file).unwrap();
        assert!(
            output_content.contains("apiVersion"),
            "Output should contain apiVersion"
        );
        assert!(
            output_content.contains("kind: DataProduct"),
            "Output should contain DataProduct"
        );
        assert!(
            output_content.contains("test-product"),
            "Output should contain product name"
        );
    }
}
