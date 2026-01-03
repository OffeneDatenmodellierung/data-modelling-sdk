//! Import module tests

use data_modelling_sdk::import::{
    avro::AvroImporter, json_schema::JSONSchemaImporter, odcs::ODCSImporter,
    protobuf::ProtobufImporter, sql::SQLImporter,
};

mod sql_import_tests {
    use super::*;

    #[test]
    fn test_parse_simple_table() {
        let importer = SQLImporter::new("postgres");
        let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL);";
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);

        let table = &result.tables[0];
        assert_eq!(table.name.as_deref(), Some("users"));
        assert_eq!(table.columns.len(), 2);

        let id_col = &table.columns[0];
        assert_eq!(id_col.name, "id");
        assert!(id_col.primary_key);

        let name_col = &table.columns[1];
        assert_eq!(name_col.name, "name");
        assert!(!name_col.nullable);
    }

    #[test]
    fn test_parse_multiple_tables() {
        let importer = SQLImporter::new("postgres");
        let sql = r#"
            CREATE TABLE users (id INT PRIMARY KEY, name TEXT);
            CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, total DECIMAL);
        "#;
        let result = importer.parse(sql).unwrap();

        assert_eq!(result.tables.len(), 2);
        assert_eq!(result.tables[0].name.as_deref(), Some("users"));
        assert_eq!(result.tables[1].name.as_deref(), Some("orders"));
    }

    #[test]
    fn test_parse_with_schema_qualified_name() {
        let importer = SQLImporter::new("postgres");
        let sql = "CREATE TABLE public.users (id INT PRIMARY KEY);";
        let result = importer.parse(sql).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("users"));
    }

    #[test]
    fn test_parse_table_level_pk_constraint() {
        let importer = SQLImporter::new("postgres");
        let sql = "CREATE TABLE t (id INT, name TEXT, CONSTRAINT pk PRIMARY KEY (id));";
        let result = importer.parse(sql).unwrap();

        assert_eq!(result.tables.len(), 1);
        let id_col = &result.tables[0].columns[0];
        assert!(id_col.primary_key);
    }

    #[test]
    fn test_parse_mysql_dialect() {
        let importer = SQLImporter::new("mysql");
        let sql =
            "CREATE TABLE `users` (`id` INT AUTO_INCREMENT PRIMARY KEY, `name` VARCHAR(100));";
        let result = importer.parse(sql).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("users"));
    }

    #[test]
    fn test_parse_liquibase_formatted_sql() {
        let importer = SQLImporter::new("postgres");
        let sql = r#"
            --liquibase formatted sql
            --changeset user:1
            CREATE TABLE test (id INT PRIMARY KEY);
        "#;
        let result = importer.parse_liquibase(sql).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("test"));
    }

    #[test]
    fn test_parse_invalid_sql() {
        let importer = SQLImporter::new("postgres");
        let sql = "CREATE TABL users (id INT);"; // Typo: TABL instead of TABLE
        let result = importer.parse(sql).unwrap();

        // Should return errors rather than panic
        assert!(!result.errors.is_empty() || result.tables.is_empty());
    }
}

mod json_schema_import_tests {
    use super::*;

    #[test]
    fn test_parse_simple_schema() {
        let importer = JSONSchemaImporter::new();
        let schema = r#"
        {
            "title": "User",
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "name": { "type": "string" }
            },
            "required": ["id"]
        }
        "#;
        let result = importer.import(schema).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("User"));
        assert_eq!(result.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_parse_schema_with_definitions() {
        let importer = JSONSchemaImporter::new();
        let schema = r#"
        {
            "definitions": {
                "User": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    }
                },
                "Order": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    }
                }
            }
        }
        "#;
        let result = importer.import(schema).unwrap();

        assert_eq!(result.tables.len(), 2);
    }

    #[test]
    fn test_parse_nested_object() {
        let importer = JSONSchemaImporter::new();
        let schema = r#"
        {
            "title": "Person",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "address": {
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" },
                        "city": { "type": "string" }
                    }
                }
            }
        }
        "#;
        let result = importer.import(schema).unwrap();

        let table = &result.tables[0];
        // Should have name, address.street, address.city
        assert!(table.columns.len() >= 3);
        assert!(table.columns.iter().any(|c| c.name == "name"));
        assert!(table.columns.iter().any(|c| c.name.contains("address")));
    }

    #[test]
    fn test_parse_array_type() {
        let importer = JSONSchemaImporter::new();
        let schema = r#"
        {
            "title": "Container",
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        }
        "#;
        let result = importer.import(schema).unwrap();

        let table = &result.tables[0];
        let items_col = table.columns.iter().find(|c| c.name == "items").unwrap();
        assert!(items_col.data_type.contains("ARRAY"));
    }
}

mod avro_import_tests {
    use super::*;

    #[test]
    fn test_parse_simple_record() {
        let importer = AvroImporter::new();
        let schema = r#"
        {
            "type": "record",
            "name": "User",
            "fields": [
                { "name": "id", "type": "long" },
                { "name": "name", "type": "string" }
            ]
        }
        "#;
        let result = importer.import(schema).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("User"));
        assert_eq!(result.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_parse_nullable_field() {
        let importer = AvroImporter::new();
        let schema = r#"
        {
            "type": "record",
            "name": "User",
            "fields": [
                { "name": "nickname", "type": ["null", "string"] }
            ]
        }
        "#;
        let result = importer.import(schema).unwrap();

        let nickname_col = &result.tables[0].columns[0];
        assert!(nickname_col.nullable);
    }

    #[test]
    fn test_parse_multiple_records() {
        let importer = AvroImporter::new();
        let schema = r#"
        [
            {
                "type": "record",
                "name": "User",
                "fields": [{ "name": "id", "type": "long" }]
            },
            {
                "type": "record",
                "name": "Order",
                "fields": [{ "name": "id", "type": "long" }]
            }
        ]
        "#;
        let result = importer.import(schema).unwrap();

        assert_eq!(result.tables.len(), 2);
    }

    #[test]
    fn test_parse_with_namespace() {
        let importer = AvroImporter::new();
        let schema = r#"
        {
            "type": "record",
            "namespace": "com.example",
            "name": "User",
            "fields": [{ "name": "id", "type": "long" }]
        }
        "#;
        let result = importer.import(schema).unwrap();

        assert_eq!(result.tables[0].name.as_deref(), Some("User"));
    }
}

mod protobuf_import_tests {
    use super::*;

    #[test]
    fn test_parse_simple_message() {
        let importer = ProtobufImporter::new();
        let proto = r#"
            syntax = "proto3";

            message User {
                int64 id = 1;
                string name = 2;
            }
        "#;
        let result = importer.import(proto).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].name.as_deref(), Some("User"));
        assert_eq!(result.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_parse_multiple_messages() {
        let importer = ProtobufImporter::new();
        let proto = r#"
            syntax = "proto3";

            message User {
                int64 id = 1;
            }

            message Order {
                int64 id = 1;
            }
        "#;
        let result = importer.import(proto).unwrap();

        assert_eq!(result.tables.len(), 2);
    }

    #[test]
    fn test_parse_optional_fields() {
        let importer = ProtobufImporter::new();
        let proto = r#"
            syntax = "proto3";

            message User {
                optional string nickname = 1;
            }
        "#;
        let result = importer.import(proto).unwrap();

        let nickname_col = &result.tables[0].columns[0];
        assert!(nickname_col.nullable);
    }

    #[test]
    fn test_parse_repeated_fields() {
        let importer = ProtobufImporter::new();
        let proto = r#"
            syntax = "proto3";

            message Container {
                repeated string items = 1;
            }
        "#;
        let result = importer.import(proto).unwrap();

        let items_col = &result.tables[0].columns[0];
        // Repeated fields should be marked as nullable
        assert!(items_col.nullable);
    }
}

// DataFlow import tests removed - DataFlow format has been migrated to Domain schema
// Use migrate_dataflow_to_domain() for DataFlow → Domain migration

mod odcl_field_preservation_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn get_test_fixture_path(filename: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("specs");
        path.push("003-odcs-field-preservation");
        path.push("test-fixtures");
        path.push(filename);
        path
    }

    #[test]
    fn test_odcl_import_preserves_description_field() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
dataContractSpecification: 1.2.1
id: test-contract
info:
  title: Test Contract
  version: 1.0.0
models:
  test_table:
    type: table
    fields:
      test_column:
        description: This is a test column description
        type: text
        required: true
"#;
        let result = importer.import(yaml).unwrap();

        assert_eq!(result.tables.len(), 1);
        let table = &result.tables[0];
        assert_eq!(table.columns.len(), 1);

        let column = &table.columns[0];
        assert_eq!(column.name, "test_column");
        assert_eq!(
            column.description,
            Some("This is a test column description".to_string())
        );
    }

    #[test]
    fn test_odcl_import_preserves_quality_array_with_nested_structures() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
dataContractSpecification: 1.2.1
id: test-contract
info:
  title: Test Contract
  version: 1.0.0
models:
  test_table:
    type: table
    fields:
      test_column:
        type: long
        required: true
        quality:
          - type: sql
            description: 95% of all values are expected to be between 10 and 499
            query: |
              SELECT quantile_cont(test_column, 0.95) AS percentile_95
              FROM test_table
            mustBeBetween: [10, 499]
"#;
        let result = importer.import(yaml).unwrap();

        assert_eq!(result.tables.len(), 1);
        let table = &result.tables[0];

        // Find the test_column (there might be additional columns created from quality rules)
        let column = table
            .columns
            .iter()
            .find(|c| c.name == "test_column")
            .expect("Should find test_column");

        // Verify quality array is preserved
        // Note: When required=true, a not_null quality rule may be added automatically
        assert!(column.quality.is_some());
        let quality = column.quality.as_ref().unwrap();
        assert!(
            !quality.is_empty(),
            "Quality array should have at least 1 rule"
        );

        // Find the SQL quality rule (there may be a not_null rule added automatically)
        let quality_rule = quality
            .iter()
            .find(|r| r.get("type").and_then(|v| v.as_str()) == Some("sql"))
            .expect("Should find SQL quality rule");
        assert_eq!(
            quality_rule.get("type").and_then(|v| v.as_str()),
            Some("sql")
        );
        assert_eq!(
            quality_rule.get("description").and_then(|v| v.as_str()),
            Some("95% of all values are expected to be between 10 and 499")
        );
        assert!(quality_rule.get("query").is_some());
        assert!(quality_rule.get("mustBeBetween").is_some());

        // Verify nested array structure
        if let Some(must_be_between) = quality_rule.get("mustBeBetween") {
            if let Some(arr) = must_be_between.as_array() {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0].as_i64(), Some(10));
                assert_eq!(arr[1].as_i64(), Some(499));
            } else {
                panic!("mustBeBetween should be an array");
            }
        } else {
            panic!("mustBeBetween should be present");
        }
    }

    #[test]
    fn test_odcl_import_preserves_ref_references() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
dataContractSpecification: 1.2.1
id: test-contract
info:
  title: Test Contract
  version: 1.0.0
models:
  test_table:
    type: table
    fields:
      order_id:
        $ref: '#/definitions/order_id'
        type: text
        required: true
definitions:
  order_id:
    type: text
    format: uuid
    description: An internal ID that identifies an order
"#;
        let result = importer.import(yaml).unwrap();

        assert_eq!(result.tables.len(), 1);
        let table = &result.tables[0];
        assert_eq!(table.columns.len(), 1);

        let column = &table.columns[0];
        assert_eq!(column.name, "order_id");
        assert_eq!(column.ref_path, Some("#/definitions/order_id".to_string()));
    }

    #[test]
    fn test_odcl_import_preserves_all_three_field_types_together() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
dataContractSpecification: 1.2.1
id: test-contract
info:
  title: Test Contract
  version: 1.0.0
models:
  test_table:
    type: table
    fields:
      complete_column:
        $ref: '#/definitions/order_id'
        description: This column has all three field types
        type: text
        required: true
        quality:
          - type: sql
            description: Validation rule
            query: SELECT COUNT(*) FROM test_table
            mustBeGreaterThan: 0
definitions:
  order_id:
    type: text
    format: uuid
    description: An internal ID
"#;
        let result = importer.import(yaml).unwrap();

        assert_eq!(result.tables.len(), 1);
        let table = &result.tables[0];

        // Find the complete_column (there might be additional columns created from quality rules)
        let column = table
            .columns
            .iter()
            .find(|c| c.name == "complete_column")
            .expect("Should find complete_column");

        // Verify description is preserved
        assert_eq!(
            column.description,
            Some("This column has all three field types".to_string())
        );

        // Verify $ref is preserved
        assert_eq!(column.ref_path, Some("#/definitions/order_id".to_string()));

        // Verify quality array is preserved with nested structures
        // Note: When required=true, a not_null quality rule may be added automatically
        assert!(column.quality.is_some());
        let quality = column.quality.as_ref().unwrap();
        assert!(
            !quality.is_empty(),
            "Quality array should have at least 1 rule"
        );

        // Find the SQL quality rule (there may be a not_null rule added automatically)
        let quality_rule = quality
            .iter()
            .find(|r| r.get("type").and_then(|v| v.as_str()) == Some("sql"))
            .expect("Should find SQL quality rule");
        assert_eq!(
            quality_rule.get("type").and_then(|v| v.as_str()),
            Some("sql")
        );
        assert_eq!(
            quality_rule.get("description").and_then(|v| v.as_str()),
            Some("Validation rule")
        );
        assert!(quality_rule.get("query").is_some());
        assert!(quality_rule.get("mustBeGreaterThan").is_some());
    }

    #[test]
    fn test_odcl_import_from_fixture_file() {
        let fixture_path = get_test_fixture_path("example.odcl.yaml");
        let yaml_content = fs::read_to_string(&fixture_path)
            .unwrap_or_else(|_| panic!("Failed to read fixture file: {:?}", fixture_path));

        let mut importer = ODCSImporter::new();
        let result = importer.import(&yaml_content).unwrap();

        // Verify we got tables
        assert!(!result.tables.is_empty());

        // The ODCL parser only parses the first model from the models section
        // Let's verify that the parsed table has fields with description, quality, and $ref preserved
        let test_table = &result.tables[0];

        // Verify description is preserved (find a column with description)
        let desc_column = test_table
            .columns
            .iter()
            .find(|c| c.description.is_some() && !c.description.as_ref().unwrap().is_empty())
            .expect("Should find column with description");
        assert!(desc_column.description.is_some());

        // Verify quality array is preserved (find a column with quality rules)
        let quality_column = test_table
            .columns
            .iter()
            .find(|c| c.quality.is_some() && !c.quality.as_ref().unwrap().is_empty())
            .expect("Should find column with quality");
        assert!(quality_column.quality.is_some());
        let quality = quality_column.quality.as_ref().unwrap();
        assert!(!quality.is_empty());

        // Verify $ref is preserved (find a column with $ref)
        let ref_column = test_table
            .columns
            .iter()
            .find(|c| c.ref_path.is_some())
            .expect("Should find column with $ref");
        assert!(ref_column.ref_path.is_some());
        assert!(
            ref_column
                .ref_path
                .as_ref()
                .unwrap()
                .starts_with("#/definitions/")
        );
    }
}
