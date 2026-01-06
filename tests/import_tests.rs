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
        // ref_path is now stored as relationships
        assert!(
            !column.relationships.is_empty(),
            "Column should have relationships from $ref"
        );
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

        // Verify $ref is preserved (now as relationships)
        assert!(
            !column.relationships.is_empty(),
            "Column should have relationships from $ref"
        );

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

        // The fixture has multiple models (orders, line_items). The ODCL importer currently
        // only returns the first model alphabetically. We search across all returned tables.

        // Verify description is preserved (find any column with description across all tables)
        let desc_column = result
            .tables
            .iter()
            .flat_map(|t| t.columns.iter())
            .find(|c| c.description.is_some() && !c.description.as_ref().unwrap().is_empty())
            .expect("Should find column with description");
        assert!(desc_column.description.is_some());

        // Verify quality array is preserved (find any column with quality rules across all tables)
        let quality_column = result
            .tables
            .iter()
            .flat_map(|t| t.columns.iter())
            .find(|c| c.quality.is_some() && !c.quality.as_ref().unwrap().is_empty())
            .expect("Should find column with quality");
        assert!(quality_column.quality.is_some());
        let quality = quality_column.quality.as_ref().unwrap();
        assert!(!quality.is_empty());

        // Verify $ref is preserved as relationships (find any column with relationships across all tables)
        let ref_column = result
            .tables
            .iter()
            .flat_map(|t| t.columns.iter())
            .find(|c| !c.relationships.is_empty())
            .expect("Should find column with relationships from $ref");
        assert!(!ref_column.relationships.is_empty());
        assert!(
            ref_column.relationships[0].to.starts_with("definitions/"),
            "Relationship 'to' should reference definitions"
        );
    }
}

mod databricks_sql_tests {
    use super::*;

    #[test]
    fn test_databricks_identifier_basic() {
        let importer = SQLImporter::new("databricks");
        // Note: USING DELTA is not supported by sqlparser, so we test without it
        // The IDENTIFIER() preprocessing is what we're testing here
        let sql = "CREATE TABLE IDENTIFIER(:catalog || '.schema.example_table') (id STRING COMMENT 'Unique identifier', name STRING COMMENT 'Name of the record');";
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(
            result.tables[0].name.as_deref(),
            Some("schema.example_table")
        );
        assert_eq!(result.tables[0].columns.len(), 2);
    }

    #[test]
    fn test_databricks_variables_in_types() {
        let importer = SQLImporter::new("databricks");
        let sql = r#"
            CREATE TABLE example (
                id STRING,
                metadata STRUCT<key: STRING, value: :value_type, timestamp: TIMESTAMP>,
                items ARRAY<:element_type>,
                nested ARRAY<STRUCT<field1: :nested_type, field2: STRING>>
            );
        "#;
        let result = importer.parse(sql).unwrap();

        if !result.errors.is_empty() {
            eprintln!("Parse errors: {:?}", result.errors);
        }
        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].columns.len(), 4);

        // Check that variables were replaced with STRING
        assert!(
            result.tables[0].columns[1]
                .data_type
                .contains("value: STRING")
        );
        // ARRAY<:element_type> becomes ARRAY<STRING> after variable replacement
        assert!(result.tables[0].columns[2].data_type.contains("ARRAY"));
        // Nested ARRAY<STRUCT<...>> should have variables replaced
        assert!(
            result.tables[0].columns[3]
                .data_type
                .contains("field1: STRING")
        );
    }

    #[test]
    fn test_databricks_metadata_variables() {
        let importer = SQLImporter::new("databricks");
        // Test COMMENT with variable - sqlparser supports this
        let sql =
            "CREATE TABLE example (id STRING, name STRING) COMMENT ':table_comment_variable';";
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].columns.len(), 2);

        // Note: TBLPROPERTIES is not supported by sqlparser, so we test COMMENT separately
        // TBLPROPERTIES would require preprocessing to remove before parsing
    }

    #[test]
    fn test_databricks_column_variables() {
        let importer = SQLImporter::new("databricks");
        let sql =
            "CREATE TABLE example (id :id_var STRING, name :name_var STRING, age :age_var INT);";
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].columns.len(), 3);
        assert_eq!(result.tables[0].columns[0].name, "id");
        assert_eq!(result.tables[0].columns[0].data_type, "STRING");
        assert_eq!(result.tables[0].columns[1].name, "name");
        assert_eq!(result.tables[0].columns[1].data_type, "STRING");
        assert_eq!(result.tables[0].columns[2].name, "age");
        assert_eq!(result.tables[0].columns[2].data_type, "INT");
    }

    #[test]
    fn test_databricks_views_and_tables() {
        let importer = SQLImporter::new("databricks");
        let sql = r#"
            CREATE TABLE table1 (id STRING, name STRING);
            CREATE VIEW view1 AS SELECT id, name FROM table1;
            CREATE TABLE table2 (value INT);
        "#;
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        // Should import both tables and views
        assert_eq!(result.tables.len(), 3);
        assert_eq!(result.tables[0].name.as_deref(), Some("table1"));
        assert_eq!(result.tables[1].name.as_deref(), Some("view1"));
        assert_eq!(result.tables[2].name.as_deref(), Some("table2"));
    }

    #[test]
    fn test_databricks_backward_compatibility() {
        // Verify existing dialects still work
        let postgres_importer = SQLImporter::new("postgres");
        let mysql_importer = SQLImporter::new("mysql");
        let sqlite_importer = SQLImporter::new("sqlite");
        let generic_importer = SQLImporter::new("generic");

        let sql = "CREATE TABLE test (id INT PRIMARY KEY, name VARCHAR(100));";

        assert!(postgres_importer.parse(sql).unwrap().errors.is_empty());
        assert!(mysql_importer.parse(sql).unwrap().errors.is_empty());
        assert!(sqlite_importer.parse(sql).unwrap().errors.is_empty());
        assert!(generic_importer.parse(sql).unwrap().errors.is_empty());
    }

    #[test]
    fn test_databricks_full_example() {
        // Full example from GitHub issue #13
        let importer = SQLImporter::new("databricks");
        let sql = r#"
            CREATE TABLE IF NOT EXISTS IDENTIFIER(:catalog_name || '.schema.example_table') (
                id STRING COMMENT 'Unique identifier for each record.',
                name STRING COMMENT 'Name of the record.',
                events ARRAY<STRUCT<
                    id: STRING,
                    name: STRING,
                    details: STRUCT<
                        name: STRING,
                        status: :variable_type,
                        timestamp: TIMESTAMP
                    >
                >>,
                metadata STRUCT<
                    key: STRING,
                    value: :value_type,
                    timestamp: TIMESTAMP
                >,
                items ARRAY<:element_type>
            );
        "#;
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(
            result.tables[0].name.as_deref(),
            Some("schema.example_table")
        );
        assert!(result.tables[0].columns.len() >= 5);
    }

    #[test]
    fn test_databricks_mixed_sql() {
        // Test Databricks SQL mixed with standard SQL
        let importer = SQLImporter::new("databricks");
        let sql = r#"
            CREATE TABLE standard_table (id INT, name VARCHAR(100));
            CREATE TABLE IDENTIFIER(:catalog || '.schema.databricks_table') (id STRING, metadata STRUCT<key: STRING, value: :value_type>);
            CREATE VIEW standard_view AS SELECT * FROM standard_table;
        "#;
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 3);
    }

    #[test]
    fn test_databricks_multiline_comment_clauses() {
        // Test case based on issue #15 - multiline COMMENT clauses
        // Representative SQL without "bets" or "flutter" references
        let importer = SQLImporter::new("databricks");
        let sql = r#"
CREATE TABLE IF NOT EXISTS IDENTIFIER(:catalog_name || '.analytics.user_events') (
  event_id STRING COMMENT 'Unique identifier for each event.',
  event_type STRING COMMENT 'The type of event that occurred. This is a finite list which can be found at the bottom of this contract, under the quality section.',
  event_metadata ARRAY<STRUCT<
    id: STRING,
    name: STRING,
    priority: INT,
    category: STRING,
    source: STRING,
    event_details: STRUCT<
      name: STRING,
      field: STRING,
      timestamp: TIMESTAMP
    >
  >> COMMENT 'details associated with the events that have been triggered.',
  highest_priority INT COMMENT 'If there are multiple events that are completed at once, this value highlights the highest priority from the group of events.',
  created_at TIMESTAMP,
  updated_at TIMESTAMP
)
COMMENT 'User events table for analytics processing'
TBLPROPERTIES (
  'delta.autoOptimize.optimizeWrite' = 'true',
  'delta.autoOptimize.autoCompact' = 'true'
);
        "#;
        let result = importer.parse(sql);

        // Should parse successfully despite multiline COMMENT clauses
        // TBLPROPERTIES is removed during preprocessing, so parsing should succeed
        assert!(
            result.is_ok(),
            "Failed to parse SQL with multiline COMMENT clauses: {:?}",
            result.err()
        );
        let result = result.unwrap();
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
        assert_eq!(result.tables.len(), 1);
        assert_eq!(
            result.tables[0].name.as_deref(),
            Some("analytics.user_events")
        );
        assert!(result.tables[0].columns.len() >= 5);

        // Verify COMMENT clauses are extracted
        let event_id_col = result.tables[0]
            .columns
            .iter()
            .find(|c| c.name == "event_id")
            .expect("event_id column should exist");
        assert_eq!(
            event_id_col.description.as_deref(),
            Some("Unique identifier for each event.")
        );

        let highest_priority_col = result.tables[0]
            .columns
            .iter()
            .find(|c| c.name == "highest_priority")
            .expect("highest_priority column should exist");
        assert!(
            highest_priority_col
                .description
                .as_deref()
                .unwrap()
                .contains("highest priority")
        );
    }

    #[test]
    fn test_databricks_escaped_quotes_in_comments() {
        // Test escaped quotes in COMMENT clauses (e.g., customer\'s, aren\'t)
        let importer = SQLImporter::new("databricks");
        let sql = r#"
CREATE TABLE test (
  id STRING COMMENT 'Unique identifier.',
  name STRING COMMENT 'Annotations are values that users link to customer IDs to provide any additional information about the customer\'s profile.',
  description STRING COMMENT 'The time at which the record was updated by the UI, by either completing the task, dismissing the item or by the item hitting the system\'s expiry time.',
  metadata STRING COMMENT 'Extended metadata gives additional information, and will be populated whenever there\'s an event based off of a transaction or group of transactions. These fields aren\'t'
);
        "#;
        let result = importer.parse(sql).unwrap();

        assert!(result.errors.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].columns.len(), 4);

        // Verify escaped quotes are handled correctly
        let name_col = result.tables[0]
            .columns
            .iter()
            .find(|c| c.name == "name")
            .expect("name column should exist");
        assert!(
            name_col
                .description
                .as_deref()
                .unwrap()
                .contains("customer's")
        );

        let desc_col = result.tables[0]
            .columns
            .iter()
            .find(|c| c.name == "description")
            .expect("description column should exist");
        assert!(
            desc_col
                .description
                .as_deref()
                .unwrap()
                .contains("system's")
        );

        let meta_col = result.tables[0]
            .columns
            .iter()
            .find(|c| c.name == "metadata")
            .expect("metadata column should exist");
        assert!(meta_col.description.as_deref().unwrap().contains("there's"));
        assert!(meta_col.description.as_deref().unwrap().contains("aren't"));
    }
}
