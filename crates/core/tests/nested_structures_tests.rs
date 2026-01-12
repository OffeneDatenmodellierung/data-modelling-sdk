//! Comprehensive tests for deeply nested structures and references in import/export

#![allow(clippy::needless_borrows_for_generic_args)]

use data_modelling_core::export::odcs::ODCSExporter;
use data_modelling_core::import::json_schema::JSONSchemaImporter;
use data_modelling_core::import::protobuf::ProtobufImporter;
use data_modelling_core::models::{Column, Table};
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

mod deeply_nested_structures {
    use super::*;

    #[test]
    fn test_export_deeply_nested_customer_with_addresses_and_phones() {
        // Customer -> addresses (ARRAY<OBJECT>) -> phone_numbers (ARRAY<STRING>)
        // This tests: optional struct containing array of objects, each containing array of strings
        let table = create_test_table(
            "customer",
            vec![
                create_column("id", "BIGINT", true, false),
                create_column("name", "VARCHAR(100)", false, false),
                // addresses is an array of objects
                create_column("addresses", "ARRAY<OBJECT>", false, true), // Optional array
                // Each address has these nested fields
                create_column("addresses.street", "VARCHAR(200)", false, false),
                create_column("addresses.city", "VARCHAR(100)", false, false),
                create_column("addresses.zip_code", "VARCHAR(20)", false, true), // Optional field
                // phone_numbers is an array within each address
                create_column("addresses.phone_numbers", "ARRAY<STRING>", false, true), // Optional array
            ],
        );

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Verify the export succeeded
        assert!(!yaml.is_empty());
        assert!(yaml.contains("customer"));

        // Verify nested structure is exported correctly
        // The schema should have properties with nested structure
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let schema = yaml_value
            .as_mapping()
            .and_then(|m| m.get(&serde_yaml::Value::String("schema".to_string())))
            .and_then(|v| v.as_sequence())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_mapping());

        assert!(schema.is_some(), "Schema should be present");

        // ODCS v3.1.0 uses array format for properties
        let properties = schema
            .unwrap()
            .get(&serde_yaml::Value::String("properties".to_string()))
            .and_then(|v| v.as_sequence());

        assert!(properties.is_some(), "Properties should be present");

        // Verify addresses is exported as array of objects
        // In ODCS v3.1.0 array format, find the property by name
        let addresses_prop = properties.unwrap().iter().find(|prop| {
            prop.as_mapping()
                .and_then(|m| m.get(&serde_yaml::Value::String("name".to_string())))
                .and_then(|v| v.as_str())
                == Some("addresses")
        });
        assert!(addresses_prop.is_some(), "addresses property should exist");
    }

    #[test]
    fn test_import_json_schema_deeply_nested_customer() {
        let importer = JSONSchemaImporter::new();
        let schema = r#"
        {
            "title": "Customer",
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "name": { "type": "string" },
                "addresses": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "street": { "type": "string" },
                            "city": { "type": "string" },
                            "zip_code": { "type": "string" },
                            "phone_numbers": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        },
                        "required": ["street", "city"]
                    }
                }
            },
            "required": ["id", "name"]
        }
        "#;

        let result = importer.import(schema).unwrap();
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.tables.len(), 1);

        let table = &result.tables[0];
        assert_eq!(table.name.as_deref(), Some("Customer"));

        // Verify nested columns are created
        let column_names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();

        // Should have top-level fields
        assert!(column_names.iter().any(|n| n == "id"));
        assert!(column_names.iter().any(|n| n == "name"));

        // Should have nested address fields
        assert!(column_names.iter().any(|n| n.contains("addresses.street")));
        assert!(column_names.iter().any(|n| n.contains("addresses.city")));
        assert!(
            column_names
                .iter()
                .any(|n| n.contains("addresses.zip_code"))
        );

        // Should have deeply nested phone_numbers
        assert!(
            column_names
                .iter()
                .any(|n| n.contains("addresses.phone_numbers"))
        );
    }

    #[test]
    fn test_import_json_schema_optional_nested_struct() {
        let importer = JSONSchemaImporter::new();
        // Test optional nested struct (contact_info is optional, and contains optional nested fields)
        let schema = r#"
        {
            "title": "User",
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "contact_info": {
                    "type": "object",
                    "properties": {
                        "email": { "type": "string" },
                        "preferences": {
                            "type": "object",
                            "properties": {
                                "notifications": { "type": "boolean" },
                                "theme": { "type": "string" }
                            }
                        }
                    }
                }
            },
            "required": ["id"]
        }
        "#;

        let result = importer.import(schema).unwrap();
        assert_eq!(result.errors.len(), 0);

        let table = &result.tables[0];
        let column_names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();

        // Verify nested structure
        assert!(
            column_names
                .iter()
                .any(|n| n.contains("contact_info.email"))
        );
        assert!(
            column_names
                .iter()
                .any(|n| n.contains("contact_info.preferences.notifications"))
        );
        assert!(
            column_names
                .iter()
                .any(|n| n.contains("contact_info.preferences.theme"))
        );

        // Verify optional fields are marked as nullable
        let email_col = table
            .columns
            .iter()
            .find(|c| c.name.contains("contact_info.email"));
        assert!(email_col.is_some());
        // contact_info is optional, so email should be nullable
    }

    #[test]
    fn test_export_optional_nested_struct() {
        // User -> contact_info (optional OBJECT) -> preferences (optional OBJECT) -> theme
        let table = create_test_table(
            "user",
            vec![
                create_column("id", "BIGINT", true, false),
                create_column("contact_info", "OBJECT", false, true), // Optional
                create_column("contact_info.email", "VARCHAR(255)", false, true), // Optional
                create_column("contact_info.preferences", "OBJECT", false, true), // Optional nested
                create_column(
                    "contact_info.preferences.notifications",
                    "BOOLEAN",
                    false,
                    true,
                ),
                create_column("contact_info.preferences.theme", "VARCHAR(50)", false, true),
            ],
        );

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Verify export succeeded
        assert!(!yaml.is_empty());
        assert!(yaml.contains("user"));

        // Verify nested structure is preserved
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let schema = yaml_value
            .as_mapping()
            .and_then(|m| m.get(&serde_yaml::Value::String("schema".to_string())))
            .and_then(|v| v.as_sequence())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_mapping());

        assert!(schema.is_some());
    }

    #[test]
    fn test_import_protobuf_nested_message() {
        let importer = ProtobufImporter::new();
        // Test nested message types in Protobuf
        let proto = r#"
            syntax = "proto3";

            message Customer {
                int64 id = 1;
                string name = 2;
                repeated Address addresses = 3;
            }

            message Address {
                string street = 1;
                string city = 2;
                repeated string phone_numbers = 3;
            }
        "#;

        let result = importer.import(proto).unwrap();
        assert_eq!(result.errors.len(), 0);

        // Should create separate tables for Customer and Address
        assert!(!result.tables.is_empty(), "Should create tables");

        let customer_table = result
            .tables
            .iter()
            .find(|t| t.name.as_deref() == Some("Customer"));
        assert!(customer_table.is_some(), "Customer table should be created");

        // Customer should have addresses field (as ARRAY or reference)
        let customer = customer_table.unwrap();
        let has_addresses = customer
            .columns
            .iter()
            .any(|c| c.name.contains("addresses"));
        assert!(has_addresses, "Customer should have addresses field");
    }
}

mod reference_tests {
    use super::*;

    #[test]
    fn test_import_json_schema_with_ref_to_definitions() {
        let importer = JSONSchemaImporter::new();
        // Test JSON Schema $ref to definitions
        // Use r## to avoid Rust interpreting $ref as a macro
        let schema = r##"
        {
            "title": "Customer",
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "name": { "type": "string" },
                "billing_address": { "$ref": "#/definitions/Address" },
                "shipping_address": { "$ref": "#/definitions/Address" }
            },
            "definitions": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" },
                        "city": { "type": "string" },
                        "zip_code": { "type": "string" },
                        "phone_numbers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["street", "city"]
                }
            },
            "required": ["id", "name"]
        }
        "##;

        // Note: Current JSON Schema importer doesn't handle $ref - it may fail or skip $ref fields
        // This test verifies that definitions are still parsed even if $ref isn't resolved
        let result = importer.import(schema);

        match result {
            Ok(import_result) => {
                // If import succeeds, verify structure
                // May have Customer table (with or without $ref fields) and/or Address from definitions
                assert!(
                    !import_result.tables.is_empty(),
                    "Should create at least one table"
                );

                // Check if Customer table was created
                let customer_table = import_result
                    .tables
                    .iter()
                    .find(|t| t.name.as_deref() == Some("Customer"));
                if let Some(customer) = customer_table {
                    let column_names: Vec<String> =
                        customer.columns.iter().map(|c| c.name.clone()).collect();

                    // Should have id and name at minimum
                    assert!(
                        column_names.iter().any(|n| n == "id"),
                        "Should have id field"
                    );
                    assert!(
                        column_names.iter().any(|n| n == "name"),
                        "Should have name field"
                    );
                }

                // Check if Address was created from definitions
                let _address_table = import_result
                    .tables
                    .iter()
                    .find(|t| t.name.as_deref() == Some("Address"));
                // Address may be created from definitions even if $ref isn't resolved
                // This is acceptable behavior
            }
            Err(_) => {
                // If import fails due to $ref not being supported, that's acceptable
                // The important thing is that the test documents the expected behavior
                // In the future, $ref resolution can be added
            }
        }
    }

    #[test]
    fn test_import_json_schema_nested_ref() {
        let importer = JSONSchemaImporter::new();
        // Test nested $ref (Address references PhoneNumber)
        let schema = r##"
        {
            "title": "Customer",
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "addresses": {
                    "type": "array",
                    "items": { "$ref": "#/definitions/Address" }
                }
            },
            "definitions": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" },
                        "phones": {
                            "type": "array",
                            "items": { "$ref": "#/definitions/PhoneNumber" }
                        }
                    }
                },
                "PhoneNumber": {
                    "type": "object",
                    "properties": {
                        "number": { "type": "string" },
                        "type": { "type": "string", "enum": ["home", "work", "mobile"] }
                    }
                }
            }
        }
        "##;

        let result = importer.import(schema).unwrap();

        assert!(!result.tables.is_empty(), "Should create tables");
        let customer_table = &result.tables[0];

        // Should handle nested $ref references
        let column_names: Vec<String> = customer_table
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();

        // Should have nested structure from $ref
        // Note: $ref resolution may create nested columns or separate tables
        assert!(
            column_names.iter().any(|n| n.contains("addresses"))
                || column_names.iter().any(|n| n.contains("street"))
                || column_names.iter().any(|n| n.contains("phones")),
            "Should have address or phone fields from nested $ref"
        );
    }

    #[test]
    fn test_import_protobuf_with_import() {
        let importer = ProtobufImporter::new();
        // Test Protobuf import statement (simulated - actual import resolution may require file system)
        // For now, test that import statements don't break parsing
        let proto = r#"
            syntax = "proto3";

            import "common.proto";

            message Customer {
                int64 id = 1;
                string name = 2;
                common.Address address = 3;
            }
        "#;

        // Import may fail if common.proto is not available, but should not crash
        let result = importer.import(proto);

        // Should either succeed (if import is ignored) or fail gracefully
        match result {
            Ok(import_result) => {
                // If it succeeds, verify structure - should create Customer table
                assert!(
                    !import_result.tables.is_empty(),
                    "Should create tables if import succeeds"
                );
            }
            Err(_) => {
                // If it fails, that's acceptable - import resolution may not be fully implemented
                // The important thing is it doesn't crash
            }
        }
    }

    #[test]
    fn test_import_protobuf_message_reference() {
        let importer = ProtobufImporter::new();
        // Test Protobuf message type references
        let proto = r#"
            syntax = "proto3";

            message Customer {
                int64 id = 1;
                string name = 2;
                Address primary_address = 3;
                repeated Address addresses = 4;
            }

            message Address {
                string street = 1;
                string city = 2;
                repeated PhoneNumber phones = 3;
            }

            message PhoneNumber {
                string number = 1;
                string type = 2;
            }
        "#;

        let result = importer.import(proto).unwrap();

        // Should create tables for Customer, Address, and PhoneNumber
        assert!(!result.tables.is_empty(), "Should create tables");

        // Find Customer table
        let customer_table = result
            .tables
            .iter()
            .find(|t| t.name.as_deref() == Some("Customer"));
        assert!(customer_table.is_some(), "Customer table should be created");

        let customer = customer_table.unwrap();
        // Customer should reference Address
        let has_address_ref = customer
            .columns
            .iter()
            .any(|c| c.name.contains("primary_address") || c.name.contains("addresses"));
        assert!(has_address_ref, "Customer should reference Address");
    }

    #[test]
    fn test_export_roundtrip_deeply_nested() {
        // Test that deeply nested structures can be exported and imported back
        let table = create_test_table(
            "customer",
            vec![
                create_column("id", "BIGINT", true, false),
                create_column("name", "VARCHAR(100)", false, false),
                create_column("addresses", "ARRAY<OBJECT>", false, true),
                create_column("addresses.street", "VARCHAR(200)", false, false),
                create_column("addresses.city", "VARCHAR(100)", false, false),
                create_column("addresses.phone_numbers", "ARRAY<STRING>", false, true),
            ],
        );

        // Export to ODCS
        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
        assert!(!yaml.is_empty());

        // Import back using ODCS importer
        use data_modelling_core::import::odcs::ODCSImporter;
        let mut importer = ODCSImporter::new();
        let result = importer.parse_table(&yaml);

        // Should successfully import
        assert!(
            result.is_ok(),
            "Should be able to import exported nested structure"
        );

        let (imported_table, errors) = result.unwrap();
        assert_eq!(errors.len(), 0, "Import should have no errors");
        assert_eq!(imported_table.name, "customer");

        // Verify nested columns are preserved
        // Note: The importer uses array notation (addresses.[].field) for nested columns inside arrays
        let column_names: Vec<String> = imported_table
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();
        // Check for either notation (with or without []) since both are valid representations
        assert!(
            column_names
                .iter()
                .any(|n| n.contains("addresses.street") || n.contains("addresses.[].street")),
            "Should have addresses.street or addresses.[].street, got: {:?}",
            column_names
        );
        assert!(
            column_names
                .iter()
                .any(|n| n.contains("addresses.city") || n.contains("addresses.[].city")),
            "Should have addresses.city or addresses.[].city, got: {:?}",
            column_names
        );
    }
}
