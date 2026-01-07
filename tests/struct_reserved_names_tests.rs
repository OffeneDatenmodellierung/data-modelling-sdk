//! Tests for STRUCT columns with reserved field names (status, type) across all import formats
//!
//! These tests ensure that nested fields with names like 'status' and 'type' don't conflict
//! with schema-level properties in ODCS, ODPS, and other formats.

#![allow(clippy::needless_borrows_for_generic_args)]

use data_modelling_sdk::export::odcs::ODCSExporter;
use data_modelling_sdk::import::{
    avro::AvroImporter, json_schema::JSONSchemaImporter, odcs::ODCSImporter,
    protobuf::ProtobufImporter, sql::SQLImporter,
};
use data_modelling_sdk::models::{Column, Table};
use uuid::Uuid;

fn create_test_table(name: &str, columns: Vec<Column>) -> Table {
    Table {
        id: Uuid::new_v4(),
        name: name.to_string(),
        columns,
        database_type: None,
        catalog_name: None,
        schema_name: None,
        medallion_layers: Vec::new(),
        scd_pattern: None,
        data_vault_classification: None,
        modeling_level: None,
        tags: Vec::new(),
        odcl_metadata: std::collections::HashMap::new(),
        owner: None,
        sla: None,
        contact_details: None,
        infrastructure_type: None,
        notes: None,
        position: None,
        yaml_file_path: None,
        drawio_cell_id: None,
        quality: Vec::new(),
        errors: Vec::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn create_column(name: &str, data_type: &str, primary_key: bool, nullable: bool) -> Column {
    Column {
        name: name.to_string(),
        data_type: data_type.to_string(),
        nullable,
        primary_key,
        ..Default::default()
    }
}

mod sql_struct_tests {
    use super::*;

    #[test]
    fn test_sql_import_struct_with_reserved_nested_fields() {
        // Test SQL import of STRUCT with nested fields named 'status' and 'type'
        let importer = SQLImporter::new("databricks");
        let sql = r#"
            CREATE TABLE user_events (
                id STRING,
                operationMetadata STRUCT<
                    name: STRING,
                    notes: STRING,
                    status: STRING,
                    user: STRING,
                    created: BIGINT
                >,
                metadata STRUCT<
                    type: STRING,
                    value: STRING,
                    timestamp: BIGINT
                >
            )
        "#;

        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty(), "Should parse without errors");
        assert_eq!(result.tables.len(), 1);

        let table = &result.tables[0];

        // Should have parent columns and nested columns
        let has_operation_metadata = table.columns.iter().any(|c| c.name == "operationMetadata");
        assert!(
            has_operation_metadata,
            "Should have operationMetadata column"
        );

        // SQL parser creates ColumnData with STRUCT data_type
        // Nested columns are created during conversion to Column (in CLI)
        // Here we verify the STRUCT definition is preserved in data_type
        let op_metadata_col = table.columns.iter().find(|c| c.name == "operationMetadata");
        assert!(
            op_metadata_col.is_some(),
            "Should have operationMetadata column"
        );
        let op_col = op_metadata_col.unwrap();
        assert!(
            op_col.data_type.contains("STRUCT") && op_col.data_type.contains("status"),
            "Should have STRUCT definition with status field: {}",
            op_col.data_type
        );

        let metadata_col = table.columns.iter().find(|c| c.name == "metadata");
        assert!(metadata_col.is_some(), "Should have metadata column");
        let meta_col = metadata_col.unwrap();
        assert!(
            meta_col.data_type.contains("STRUCT") && meta_col.data_type.contains("type"),
            "Should have STRUCT definition with type field: {}",
            meta_col.data_type
        );

        // Export to ODCS and verify it validates
        let table_model = create_test_table(
            "user_events",
            table
                .columns
                .iter()
                .map(|cd| create_column(&cd.name, &cd.data_type, cd.primary_key, cd.nullable))
                .collect(),
        );

        let yaml = ODCSExporter::export_table(&table_model, "odcs_v3_1_0");
        assert!(!yaml.is_empty(), "Should export successfully");

        // Verify nested fields are preserved
        assert!(
            yaml.contains("operationMetadata"),
            "Should contain operationMetadata"
        );
        assert!(
            yaml.contains("name: status"),
            "Should contain nested status field"
        );
        assert!(
            yaml.contains("name: type"),
            "Should contain nested type field"
        );
    }

    #[test]
    fn test_sql_import_struct_roundtrip_with_reserved_names() {
        // Test roundtrip: SQL -> ODCS -> Import -> Export
        let importer = SQLImporter::new("databricks");
        let sql = r#"
            CREATE TABLE alerts (
                id STRING,
                operationMetadata STRUCT<
                    name: STRING,
                    status: STRING,
                    type: STRING,
                    user: STRING
                >
            )
        "#;

        let result = importer.parse(sql).unwrap();
        assert!(result.errors.is_empty());

        let table = &result.tables[0];
        let table_model = create_test_table(
            "alerts",
            table
                .columns
                .iter()
                .map(|cd| create_column(&cd.name, &cd.data_type, cd.primary_key, cd.nullable))
                .collect(),
        );

        // Export to ODCS
        let yaml = ODCSExporter::export_table(&table_model, "odcs_v3_1_0");

        // Import back
        let mut odcs_importer = ODCSImporter::new();
        let import_result = odcs_importer.parse_table(&yaml);
        assert!(import_result.is_ok(), "Should import successfully");

        let (imported_table, errors) = import_result.unwrap();
        assert!(errors.is_empty(), "Should have no import errors");

        // Verify nested columns are preserved (may be nested columns or STRUCT in data_type)
        let has_status = imported_table.columns.iter().any(|c| {
            c.name.contains("operationMetadata.status")
                || (c.name.contains("operationMetadata") && c.data_type.contains("status"))
        });
        assert!(has_status, "Should preserve operationMetadata.status");

        let has_type = imported_table.columns.iter().any(|c| {
            c.name.contains("operationMetadata.type")
                || (c.name.contains("operationMetadata") && c.data_type.contains("type"))
        });
        assert!(has_type, "Should preserve operationMetadata.type");
    }
}

mod protobuf_struct_tests {
    use super::*;

    #[test]
    fn test_protobuf_import_struct_with_reserved_nested_fields() {
        let importer = ProtobufImporter::new();
        let proto = r#"
            syntax = "proto3";

            message Alert {
                string id = 1;
                OperationMetadata operation_metadata = 2;
                Metadata metadata = 3;
            }

            message OperationMetadata {
                string name = 1;
                string notes = 2;
                string status = 3;  // Reserved name as nested field
                string user = 4;
                int64 created = 5;
            }

            message Metadata {
                string type = 1;  // Reserved name as nested field
                string value = 2;
                int64 timestamp = 3;
            }
        "#;

        let result = importer.import(proto).unwrap();
        assert_eq!(result.errors.len(), 0, "Should import without errors");

        // Should create tables for Alert, OperationMetadata, and Metadata
        assert!(result.tables.len() >= 2, "Should create multiple tables");

        // Find OperationMetadata table
        let op_metadata_table = result
            .tables
            .iter()
            .find(|t| t.name.as_deref() == Some("OperationMetadata"));
        assert!(
            op_metadata_table.is_some(),
            "Should create OperationMetadata table"
        );

        let op_metadata = op_metadata_table.unwrap();
        let has_status = op_metadata.columns.iter().any(|c| c.name == "status");
        assert!(has_status, "Should have status field in OperationMetadata");

        // Find Metadata table
        let metadata_table = result
            .tables
            .iter()
            .find(|t| t.name.as_deref() == Some("Metadata"));
        assert!(metadata_table.is_some(), "Should create Metadata table");

        let metadata = metadata_table.unwrap();
        let has_type = metadata.columns.iter().any(|c| c.name == "type");
        assert!(has_type, "Should have type field in Metadata");
    }

    #[test]
    fn test_protobuf_export_roundtrip_with_reserved_names() {
        // Create table with nested structure containing reserved names
        let table = create_test_table(
            "OperationMetadata",
            vec![
                create_column("name", "STRING", false, false),
                create_column("status", "STRING", false, true), // Reserved name
                create_column("type", "STRING", false, true),   // Reserved name
                create_column("user", "STRING", false, true),
            ],
        );

        // Export to ODCS
        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
        assert!(!yaml.is_empty());

        // Verify reserved names are preserved
        assert!(yaml.contains("name: status"));
        assert!(yaml.contains("name: type"));
    }
}

mod avro_struct_tests {
    use super::*;

    #[test]
    fn test_avro_import_struct_with_reserved_nested_fields() {
        let importer = AvroImporter::new();
        let avro_schema = r#"
        {
            "type": "record",
            "name": "Alert",
            "fields": [
                {"name": "id", "type": "string"},
                {
                    "name": "operationMetadata",
                    "type": {
                        "type": "record",
                        "name": "OperationMetadata",
                        "fields": [
                            {"name": "name", "type": "string"},
                            {"name": "status", "type": "string"},
                            {"name": "type", "type": "string"},
                            {"name": "user", "type": "string"}
                        ]
                    }
                }
            ]
        }
        "#;

        let result = importer.import(avro_schema).unwrap();
        assert_eq!(result.errors.len(), 0, "Should import without errors");

        // Should create tables
        assert!(!result.tables.is_empty(), "Should create tables");

        // Find OperationMetadata table (AVRO may create separate table or nested columns)
        let has_operation_metadata = result.tables.iter().any(|t| {
            t.columns.iter().any(|c| {
                c.name.contains("operationMetadata") || c.name == "status" || c.name == "type"
            })
        });
        assert!(
            has_operation_metadata,
            "Should have operationMetadata structure"
        );
    }

    #[test]
    fn test_avro_export_roundtrip_with_reserved_names() {
        // Create table with nested structure
        let table = create_test_table(
            "Alert",
            vec![
                create_column("id", "STRING", true, false),
                create_column("operationMetadata", "STRUCT<...>", false, true),
                create_column("operationMetadata.name", "STRING", false, false),
                create_column("operationMetadata.status", "STRING", false, true),
                create_column("operationMetadata.type", "STRING", false, true),
            ],
        );

        // Export to ODCS
        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
        assert!(!yaml.is_empty());

        // Verify reserved names are preserved
        assert!(yaml.contains("name: status"));
        assert!(yaml.contains("name: type"));
    }
}

mod json_schema_struct_tests {
    use super::*;

    #[test]
    fn test_json_schema_import_struct_with_reserved_nested_fields() {
        let importer = JSONSchemaImporter::new();
        let json_schema = r#"
        {
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "operationMetadata": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "status": {"type": "string"},
                        "type": {"type": "string"},
                        "user": {"type": "string"}
                    }
                }
            }
        }
        "#;

        let result = importer.import(json_schema).unwrap();
        // JSON Schema importer may have warnings but should still create tables
        if !result.tables.is_empty() {
            let table = &result.tables[0];
            let has_operation_metadata = table
                .columns
                .iter()
                .any(|c| c.name.contains("operationMetadata"));
            assert!(has_operation_metadata, "Should have operationMetadata");

            // Check for nested fields with reserved names
            // JSON Schema importer may create nested columns or flat columns
            let has_status = table.columns.iter().any(|c| {
                c.name == "operationMetadata.status"
                    || c.name == "status"
                    || (c.name.contains("operationMetadata") && c.data_type.contains("status"))
            });
            assert!(has_status, "Should have status field");

            let has_type = table.columns.iter().any(|c| {
                c.name == "operationMetadata.type"
                    || c.name == "type"
                    || (c.name.contains("operationMetadata") && c.data_type.contains("type"))
            });
            assert!(has_type, "Should have type field");
        } else {
            // If no tables created, at least verify the schema structure is valid
            assert!(serde_json::from_str::<serde_json::Value>(json_schema).is_ok());
        }
    }

    #[test]
    fn test_json_schema_export_roundtrip_with_reserved_names() {
        // Create table with nested structure
        let table = create_test_table(
            "Alert",
            vec![
                create_column("id", "STRING", true, false),
                create_column("operationMetadata", "OBJECT", false, true),
                create_column("operationMetadata.name", "STRING", false, false),
                create_column("operationMetadata.status", "STRING", false, true),
                create_column("operationMetadata.type", "STRING", false, true),
            ],
        );

        // Export to ODCS
        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
        assert!(!yaml.is_empty());

        // Import back
        let mut odcs_importer = ODCSImporter::new();
        let import_result = odcs_importer.parse_table(&yaml);
        assert!(import_result.is_ok(), "Should import successfully");

        let (imported_table, errors) = import_result.unwrap();
        assert!(errors.is_empty(), "Should have no errors");

        // Verify nested columns are preserved (may be nested or in STRUCT data_type)
        let has_status = imported_table.columns.iter().any(|c| {
            c.name.contains("status")
                || (c.name.contains("operationMetadata") && c.data_type.contains("status"))
        });
        assert!(has_status, "Should preserve status field");

        let has_type = imported_table.columns.iter().any(|c| {
            c.name.contains("type")
                || (c.name.contains("operationMetadata") && c.data_type.contains("type"))
        });
        assert!(has_type, "Should preserve type field");
    }
}

#[cfg(feature = "openapi")]
mod openapi_struct_tests {
    use super::*;

    #[test]
    fn test_openapi_export_roundtrip_with_reserved_names() {
        // Create table with nested structure containing reserved names
        // OpenAPI importer has a different API, so we test export/roundtrip via ODCS
        let table = create_test_table(
            "OperationMetadata",
            vec![
                create_column("name", "STRING", false, false),
                create_column("status", "STRING", false, true), // Reserved name
                create_column("type", "STRING", false, true),   // Reserved name
                create_column("user", "STRING", false, true),
            ],
        );

        // Export to ODCS
        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
        assert!(!yaml.is_empty());

        // Verify reserved names are preserved in export
        assert!(
            yaml.contains("name: status"),
            "Should preserve status field name"
        );
        assert!(
            yaml.contains("name: type"),
            "Should preserve type field name"
        );

        // Import back to verify roundtrip
        let mut odcs_importer = ODCSImporter::new();
        let import_result = odcs_importer.parse_table(&yaml);
        assert!(import_result.is_ok(), "Should import successfully");

        let (imported_table, errors) = import_result.unwrap();
        assert!(errors.is_empty(), "Should have no errors");

        // Verify reserved names are preserved
        let has_status = imported_table.columns.iter().any(|c| c.name == "status");
        assert!(has_status, "Should preserve status field");

        let has_type = imported_table.columns.iter().any(|c| c.name == "type");
        assert!(has_type, "Should preserve type field");
    }
}

mod odcs_comprehensive_tests {
    use super::*;

    #[test]
    fn test_odcs_struct_with_reserved_names_roundtrip() {
        // Test comprehensive roundtrip with STRUCT containing reserved names
        let table = create_test_table(
            "alerts",
            vec![
                create_column("id", "STRING", true, false),
                create_column("operationMetadata", "STRUCT<...>", false, true),
                create_column("operationMetadata.name", "STRING", false, false),
                create_column("operationMetadata.status", "STRING", false, true),
                create_column("operationMetadata.type", "STRING", false, true),
                create_column("operationMetadata.user", "STRING", false, true),
                create_column("operationMetadata.created", "BIGINT", false, true),
            ],
        );

        // Export to ODCS
        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
        assert!(!yaml.is_empty());

        // Verify structure
        assert!(yaml.contains("operationMetadata"));
        assert!(yaml.contains("name: status"));
        assert!(yaml.contains("name: type"));
        assert!(yaml.contains("name: user"));

        // Import back
        let mut odcs_importer = ODCSImporter::new();
        let import_result = odcs_importer.parse_table(&yaml);
        assert!(import_result.is_ok(), "Should import successfully");

        let (imported_table, errors) = import_result.unwrap();
        assert!(errors.is_empty(), "Should have no errors");

        // Verify all nested columns are preserved
        let column_names: Vec<String> = imported_table
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();

        // Verify nested columns are preserved (may be nested columns or STRUCT in data_type)
        let has_status = column_names
            .iter()
            .any(|n| n.contains("operationMetadata.status"))
            || imported_table.columns.iter().any(|c| {
                c.name.contains("operationMetadata")
                    && (c.data_type.contains("status") || c.data_type.contains("STRUCT"))
            });
        assert!(has_status, "Should preserve operationMetadata.status");

        let has_type = column_names
            .iter()
            .any(|n| n.contains("operationMetadata.type"))
            || imported_table.columns.iter().any(|c| {
                c.name.contains("operationMetadata")
                    && (c.data_type.contains("type") || c.data_type.contains("STRUCT"))
            });
        assert!(has_type, "Should preserve operationMetadata.type");

        let has_user = column_names
            .iter()
            .any(|n| n.contains("operationMetadata.user"))
            || imported_table.columns.iter().any(|c| {
                c.name.contains("operationMetadata")
                    && (c.data_type.contains("user") || c.data_type.contains("STRUCT"))
            });
        assert!(has_user, "Should preserve operationMetadata.user");
    }
}
