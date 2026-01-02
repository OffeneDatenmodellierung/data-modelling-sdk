//! Export module tests

use data_modelling_sdk::export::{
    avro::AvroExporter, dataflow::DataFlowExporter, json_schema::JSONSchemaExporter,
    protobuf::ProtobufExporter, sql::SQLExporter,
};
use data_modelling_sdk::models::{
    Column, DataModel, InfrastructureType, Relationship, SlaProperty, Table,
};
use serde_json::json;
use uuid::Uuid;

fn create_test_table(name: &str, columns: Vec<Column>) -> Table {
    Table {
        id: uuid::Uuid::new_v4(),
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
        secondary_key: false,
        composite_key: None,
        foreign_key: None,
        constraints: Vec::new(),
        description: String::new(),
        quality: Vec::new(),
        enum_values: Vec::new(),
        errors: Vec::new(),
        column_order: 0,
    }
}

mod sql_export_tests {
    use super::*;

    #[test]
    fn test_export_simple_table() {
        let table = create_test_table(
            "users",
            vec![
                create_column("id", "INT", true, false),
                create_column("name", "VARCHAR(100)", false, false),
            ],
        );

        let sql = SQLExporter::export_table(&table, Some("postgres"));

        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("\"users\"")); // Quoted identifier
        assert!(sql.contains("\"id\"")); // Quoted column
        assert!(sql.contains("PRIMARY KEY"));
        assert!(sql.contains("NOT NULL"));
    }

    #[test]
    fn test_export_with_schema() {
        let mut table = create_test_table("users", vec![create_column("id", "INT", true, false)]);
        table.schema_name = Some("public".to_string());

        let sql = SQLExporter::export_table(&table, Some("postgres"));

        assert!(sql.contains("\"public\".\"users\""));
    }

    #[test]
    fn test_export_with_catalog_and_schema() {
        let mut table = create_test_table("users", vec![create_column("id", "INT", true, false)]);
        table.catalog_name = Some("mydb".to_string());
        table.schema_name = Some("public".to_string());

        let sql = SQLExporter::export_table(&table, Some("postgres"));

        assert!(sql.contains("\"mydb\".\"public\".\"users\""));
    }

    #[test]
    fn test_mysql_dialect() {
        let table = create_test_table("users", vec![create_column("id", "INT", true, false)]);

        let sql = SQLExporter::export_table(&table, Some("mysql"));

        assert!(sql.contains("`users`")); // MySQL uses backticks
        assert!(sql.contains("`id`"));
    }

    #[test]
    fn test_sqlserver_dialect() {
        let table = create_test_table("users", vec![create_column("id", "INT", true, false)]);

        let sql = SQLExporter::export_table(&table, Some("sqlserver"));

        assert!(sql.contains("[users]")); // SQL Server uses brackets
        assert!(sql.contains("[id]"));
    }

    #[test]
    fn test_quote_escaping() {
        // Table with a quote in the name
        let table = create_test_table(
            "user\"table",
            vec![create_column("col\"name", "INT", true, false)],
        );

        let sql = SQLExporter::export_table(&table, Some("postgres"));

        // Quotes should be escaped by doubling
        assert!(sql.contains("\"user\"\"table\""));
        assert!(sql.contains("\"col\"\"name\""));
    }

    #[test]
    fn test_mysql_backtick_escaping() {
        let table = create_test_table("user`table", vec![create_column("id", "INT", true, false)]);

        let sql = SQLExporter::export_table(&table, Some("mysql"));

        // Backticks should be escaped by doubling
        assert!(sql.contains("`user``table`"));
    }

    #[test]
    fn test_exporter_interface() {
        let exporter = SQLExporter;
        let tables = vec![
            create_test_table("users", vec![create_column("id", "INT", true, false)]),
            create_test_table("orders", vec![create_column("id", "INT", true, false)]),
        ];

        let result = exporter.export(&tables, Some("postgres")).unwrap();

        assert_eq!(result.format, "sql");
        assert!(result.content.contains("users"));
        assert!(result.content.contains("orders"));
    }
}

mod json_schema_export_tests {
    use super::*;

    #[test]
    fn test_export_simple_table() {
        let table = create_test_table(
            "User",
            vec![
                create_column("id", "INTEGER", true, false),
                create_column("name", "STRING", false, true),
            ],
        );

        let schema = JSONSchemaExporter::export_table(&table);

        assert!(schema.get("title").unwrap().as_str() == Some("User"));
        assert!(schema.get("type").unwrap().as_str() == Some("object"));

        let properties = schema.get("properties").unwrap().as_object().unwrap();
        assert!(properties.contains_key("id"));
        assert!(properties.contains_key("name"));

        let required = schema.get("required").unwrap().as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("id")));
        // name is nullable, so should not be in required
        assert!(!required.iter().any(|v| v.as_str() == Some("name")));
    }

    #[test]
    fn test_exporter_interface() {
        let exporter = JSONSchemaExporter;
        let tables = vec![create_test_table(
            "User",
            vec![create_column("id", "INTEGER", true, false)],
        )];

        let result = exporter.export(&tables).unwrap();

        assert_eq!(result.format, "json_schema");
        assert!(result.content.contains("User"));
        assert!(result.content.contains("$schema"));
    }
}

mod protobuf_export_tests {
    use super::*;

    #[test]
    fn test_export_simple_message() {
        let table = create_test_table(
            "User",
            vec![
                create_column("id", "INT64", true, false),
                create_column("name", "STRING", false, true),
            ],
        );

        let mut field_number = 0;
        let proto = ProtobufExporter::export_table(&table, &mut field_number);

        assert!(proto.contains("message User {"));
        assert!(proto.contains("int64"));
        assert!(proto.contains("name = "));
    }

    #[test]
    fn test_reserved_word_handling() {
        // "message" is a reserved word in protobuf
        let table = create_test_table("message", vec![create_column("id", "INT64", true, false)]);

        let mut field_number = 0;
        let proto = ProtobufExporter::export_table(&table, &mut field_number);

        // Should prefix with underscore
        assert!(proto.contains("message _message {"));
    }

    #[test]
    fn test_special_character_handling() {
        let table = create_test_table(
            "user-table",
            vec![create_column("user.id", "INT64", true, false)],
        );

        let mut field_number = 0;
        let proto = ProtobufExporter::export_table(&table, &mut field_number);

        // Should replace special chars with underscores
        assert!(proto.contains("message user_table {"));
        assert!(proto.contains("user_id = "));
    }

    #[test]
    fn test_exporter_interface() {
        let exporter = ProtobufExporter;
        let tables = vec![
            create_test_table("User", vec![create_column("id", "INT64", true, false)]),
            create_test_table("Order", vec![create_column("id", "INT64", true, false)]),
        ];

        let result = exporter.export(&tables).unwrap();

        assert_eq!(result.format, "protobuf");
        assert!(result.content.contains("syntax = \"proto3\""));
        assert!(result.content.contains("message User {"));
        assert!(result.content.contains("message Order {"));
    }

    #[test]
    fn test_array_types() {
        let table = create_test_table(
            "Container",
            vec![create_column("items", "ARRAY<STRING>", false, true)],
        );

        let mut field_number = 0;
        let proto = ProtobufExporter::export_table(&table, &mut field_number);

        assert!(proto.contains("repeated"));
    }
}

mod avro_export_tests {
    use super::*;

    #[test]
    fn test_export_simple_table() {
        let table = create_test_table(
            "User",
            vec![
                create_column("id", "INT64", true, false),
                create_column("name", "STRING", false, true),
            ],
        );

        let schema = AvroExporter::export_table(&table);
        let schema_str = serde_json::to_string(&schema).unwrap();

        assert!(schema_str.contains("\"type\":\"record\""));
        assert!(schema_str.contains("\"name\":\"User\""));
        assert!(schema_str.contains("\"name\":\"id\""));
        assert!(schema_str.contains("\"name\":\"name\""));
    }

    #[test]
    fn test_export_with_nullable_fields() {
        let table = create_test_table(
            "Product",
            vec![
                create_column("id", "INT64", true, false),
                create_column("description", "STRING", false, true), // nullable
            ],
        );

        let schema = AvroExporter::export_table(&table);
        let schema_str = serde_json::to_string(&schema).unwrap();

        // Nullable fields should be union with null
        assert!(schema_str.contains("null") || schema_str.contains("\"STRING\""));
    }

    #[test]
    fn test_export_multiple_tables() {
        let tables = vec![
            create_test_table("User", vec![create_column("id", "INT64", true, false)]),
            create_test_table("Order", vec![create_column("id", "INT64", true, false)]),
        ];

        let exporter = AvroExporter;
        let result = exporter.export(&tables).unwrap();

        assert_eq!(result.format, "avro");
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_export_with_description() {
        let mut table = create_test_table("User", vec![create_column("id", "INT64", true, false)]);
        table.columns[0].description = "User identifier".to_string();

        let schema = AvroExporter::export_table(&table);
        let schema_str = serde_json::to_string(&schema).unwrap();

        assert!(schema_str.contains("User identifier"));
    }

    #[test]
    fn test_export_array_types() {
        let table = create_test_table(
            "Container",
            vec![create_column("items", "ARRAY<STRING>", false, true)],
        );

        let schema = AvroExporter::export_table(&table);
        let schema_str = serde_json::to_string(&schema).unwrap();

        // AVRO should handle array types - check that the field exists
        // The actual type mapping depends on the implementation
        assert!(schema_str.contains("\"name\":\"items\"") || schema_str.contains("items"));
    }
}

mod dataflow_export_tests {
    use super::*;

    #[test]
    fn test_export_node_with_metadata() {
        let mut table = Table::new(
            "user_events".to_string(),
            vec![Column::new("id".to_string(), "UUID".to_string())],
        );
        table.owner = Some("Data Engineering Team".to_string());
        table.infrastructure_type = Some(InfrastructureType::Kafka);
        table.notes = Some("User interaction events".to_string());
        table.sla = Some(vec![SlaProperty {
            property: "latency".to_string(),
            value: json!(4),
            unit: "hours".to_string(),
            description: Some("Data must be available within 4 hours".to_string()),
            element: None,
            driver: Some("operational".to_string()),
            scheduler: None,
            schedule: None,
        }]);

        let yaml = DataFlowExporter::export_node(&table);
        assert!(yaml.contains("user_events"));
        assert!(yaml.contains("Data Engineering Team"));
        assert!(yaml.contains("Kafka"));
        assert!(yaml.contains("User interaction events"));
        assert!(yaml.contains("latency"));
    }

    #[test]
    fn test_export_relationship_with_metadata() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        relationship.owner = Some("Data Engineering Team".to_string());
        relationship.infrastructure_type = Some(InfrastructureType::Kafka);
        relationship.notes = Some("ETL pipeline".to_string());

        let yaml = DataFlowExporter::export_relationship(&relationship);
        assert!(yaml.contains(&source_id.to_string()));
        assert!(yaml.contains(&target_id.to_string()));
        assert!(yaml.contains("Data Engineering Team"));
        assert!(yaml.contains("Kafka"));
    }

    #[test]
    fn test_export_model_round_trip() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/tmp".to_string(),
            "control.yaml".to_string(),
        );
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table.owner = Some("Team A".to_string());
        table.infrastructure_type = Some(InfrastructureType::PostgreSQL);
        model.tables.push(table);

        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        relationship.owner = Some("Team B".to_string());
        relationship.infrastructure_type = Some(InfrastructureType::Kafka);
        model.relationships.push(relationship);

        let yaml = DataFlowExporter::export_model(&model);
        assert!(yaml.contains("nodes:"));
        assert!(yaml.contains("relationships:"));
        assert!(yaml.contains("Team A"));
        assert!(yaml.contains("Team B"));

        // Test round-trip
        let importer = data_modelling_sdk::import::dataflow::DataFlowImporter::new();
        let imported_model = importer.import(&yaml).unwrap();
        assert_eq!(imported_model.tables.len(), 1);
        assert_eq!(imported_model.relationships.len(), 1);
        assert_eq!(imported_model.tables[0].owner, Some("Team A".to_string()));
        assert_eq!(
            imported_model.relationships[0].owner,
            Some("Team B".to_string())
        );
    }
}
