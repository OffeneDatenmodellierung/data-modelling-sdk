//! Integration tests for CLI

#[cfg(feature = "cli")]
use data_modelling_sdk::cli::commands::import::{
    ImportArgs, ImportFormat, InputSource, handle_import_sql,
};
#[cfg(feature = "cli")]
use std::io::Write;
#[cfg(feature = "cli")]
use tempfile::NamedTempFile;

#[cfg(feature = "cli")]
#[test]
fn test_cli_sql_import_all_dialects() {
    let dialects = vec!["postgres", "mysql", "sqlite", "generic", "databricks"];

    for dialect in dialects {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "CREATE TABLE test (id INT);").unwrap();
        file.flush().unwrap();

        let args = ImportArgs {
            format: ImportFormat::Sql,
            input: InputSource::File(file.path().to_path_buf()),
            dialect: Some(dialect.to_string()),
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
        assert!(
            result.is_ok(),
            "SQL import should succeed for dialect: {}",
            dialect
        );
    }
}
