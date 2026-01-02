//! Import module tests

use data_modelling_sdk::import::{
    avro::AvroImporter, dataflow::DataFlowImporter, json_schema::JSONSchemaImporter,
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

mod dataflow_import_tests {
    use super::*;
    use data_modelling_sdk::models::InfrastructureType;

    #[test]
    fn test_import_node_with_metadata() {
        let importer = DataFlowImporter::new();
        let yaml = r#"
nodes:
  - name: user_events
    metadata:
      owner: "Data Engineering Team"
      infrastructure_type: "Kafka"
      notes: "User interaction events"
      sla:
        - property: latency
          value: 4
          unit: hours
          description: "Data must be available within 4 hours"
"#;
        let model = importer.import(yaml).unwrap();
        assert_eq!(model.tables.len(), 1);
        let table = &model.tables[0];
        assert_eq!(table.name, "user_events");
        assert_eq!(table.owner, Some("Data Engineering Team".to_string()));
        assert_eq!(table.infrastructure_type, Some(InfrastructureType::Kafka));
        assert_eq!(table.notes, Some("User interaction events".to_string()));
        assert!(table.sla.is_some());
    }

    #[test]
    fn test_import_relationship_with_metadata() {
        let importer = DataFlowImporter::new();
        let source_id = uuid::Uuid::new_v4();
        let target_id = uuid::Uuid::new_v4();
        let yaml = format!(
            r#"
relationships:
  - source_node_id: "{}"
    target_node_id: "{}"
    metadata:
      owner: "Data Engineering Team"
      infrastructure_type: "Kafka"
      notes: "ETL pipeline"
"#,
            source_id, target_id
        );
        let model = importer.import(&yaml).unwrap();
        assert_eq!(model.relationships.len(), 1);
        let rel = &model.relationships[0];
        assert_eq!(rel.owner, Some("Data Engineering Team".to_string()));
        assert_eq!(rel.infrastructure_type, Some(InfrastructureType::Kafka));
        assert_eq!(rel.notes, Some("ETL pipeline".to_string()));
    }

    #[test]
    fn test_import_without_metadata() {
        let importer = DataFlowImporter::new();
        let yaml = r#"
nodes:
  - name: test_table
"#;
        let model = importer.import(yaml).unwrap();
        assert_eq!(model.tables.len(), 1);
        let table = &model.tables[0];
        assert_eq!(table.name, "test_table");
        assert_eq!(table.owner, None);
        assert_eq!(table.infrastructure_type, None);
    }
}
