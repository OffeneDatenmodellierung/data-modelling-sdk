//! Integration tests for round-trip import/export

use data_modelling_sdk::export::json_schema::JSONSchemaExporter;
use data_modelling_sdk::export::sql::SQLExporter;
use data_modelling_sdk::import::json_schema::JSONSchemaImporter;
use data_modelling_sdk::import::sql::SQLImporter;
use data_modelling_sdk::models::{Column, Table};

fn create_table_from_import_result(
    result: &data_modelling_sdk::import::ImportResult,
) -> Vec<Table> {
    result
        .tables
        .iter()
        .map(|t| Table {
            id: uuid::Uuid::new_v4(),
            name: t.name.clone().unwrap_or_default(),
            columns: t
                .columns
                .iter()
                .map(|c| Column {
                    name: c.name.clone(),
                    data_type: c.data_type.clone(),
                    nullable: c.nullable,
                    primary_key: c.primary_key,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description: String::new(),
                    quality: Vec::new(),
                    enum_values: Vec::new(),
                    errors: Vec::new(),
                    column_order: 0,
                })
                .collect(),
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
        })
        .collect()
}

mod sql_roundtrip_tests {
    use super::*;

    #[test]
    fn test_sql_import_export_roundtrip() {
        // Import SQL
        let original_sql =
            "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL, email TEXT);";
        let importer = SQLImporter::new("postgres");
        let import_result = importer.parse(original_sql).unwrap();

        assert_eq!(import_result.tables.len(), 1);
        let original_table = &import_result.tables[0];
        assert_eq!(original_table.columns.len(), 3);

        // Convert to Table objects
        let tables = create_table_from_import_result(&import_result);

        // Export back to SQL
        let exporter = SQLExporter;
        let export_result = exporter.export(&tables, Some("postgres")).unwrap();

        // Re-import the exported SQL
        let reimport_result = importer.parse(&export_result.content).unwrap();

        // Verify structure is preserved
        assert_eq!(reimport_result.tables.len(), 1);
        let reimported_table = &reimport_result.tables[0];

        assert_eq!(original_table.name, reimported_table.name);
        assert_eq!(original_table.columns.len(), reimported_table.columns.len());

        // Verify each column
        for (orig, reimp) in original_table
            .columns
            .iter()
            .zip(reimported_table.columns.iter())
        {
            assert_eq!(orig.name, reimp.name);
            assert_eq!(orig.nullable, reimp.nullable);
            assert_eq!(orig.primary_key, reimp.primary_key);
        }
    }

    #[test]
    fn test_multiple_tables_roundtrip() {
        let original_sql = r#"
            CREATE TABLE users (id INT PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, total DECIMAL);
        "#;
        let importer = SQLImporter::new("postgres");
        let import_result = importer.parse(original_sql).unwrap();

        assert_eq!(import_result.tables.len(), 2);

        let tables = create_table_from_import_result(&import_result);
        let exporter = SQLExporter;
        let export_result = exporter.export(&tables, Some("postgres")).unwrap();

        let reimport_result = importer.parse(&export_result.content).unwrap();

        assert_eq!(reimport_result.tables.len(), 2);
        assert_eq!(import_result.tables[0].name, reimport_result.tables[0].name);
        assert_eq!(import_result.tables[1].name, reimport_result.tables[1].name);
    }
}

mod json_schema_roundtrip_tests {
    use super::*;

    #[test]
    fn test_json_schema_import_export_roundtrip() {
        let original_schema = r#"
        {
            "title": "User",
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "name": { "type": "string" },
                "active": { "type": "boolean" }
            },
            "required": ["id", "name"]
        }
        "#;

        let importer = JSONSchemaImporter::new();
        let import_result = importer.import(original_schema).unwrap();

        assert_eq!(import_result.tables.len(), 1);
        assert_eq!(import_result.tables[0].columns.len(), 3);

        let tables = create_table_from_import_result(&import_result);

        let exporter = JSONSchemaExporter;
        let export_result = exporter.export(&tables).unwrap();

        // Parse the exported JSON Schema
        let exported_schema: serde_json::Value =
            serde_json::from_str(&export_result.content).unwrap();

        // Verify structure
        let definitions = exported_schema
            .get("definitions")
            .unwrap()
            .as_object()
            .unwrap();
        assert!(definitions.contains_key("User"));

        let user_schema = definitions.get("User").unwrap();
        let properties = user_schema.get("properties").unwrap().as_object().unwrap();
        assert!(properties.contains_key("id"));
        assert!(properties.contains_key("name"));
        assert!(properties.contains_key("active"));
    }
}

mod validation_integration_tests {
    use data_modelling_sdk::validation::{
        validate_column_name, validate_data_type, validate_table_name,
    };

    #[test]
    fn test_validation_with_imported_data() {
        use super::*;

        let sql = "CREATE TABLE valid_table (id INT PRIMARY KEY, user_name TEXT);";
        let importer = SQLImporter::new("postgres");
        let result = importer.parse(sql).unwrap();

        let table = &result.tables[0];

        // Table name should be valid
        assert!(validate_table_name(table.name.as_deref().unwrap()).is_ok());

        // Column names should be valid
        for col in &table.columns {
            assert!(validate_column_name(&col.name).is_ok());
            assert!(validate_data_type(&col.data_type).is_ok());
        }
    }

    #[test]
    fn test_validation_catches_issues() {
        // Empty table name
        assert!(validate_table_name("").is_err());

        // Table name with invalid character
        assert!(validate_table_name("user;table").is_err());

        // Reserved word
        assert!(validate_table_name("SELECT").is_err());

        // Column starting with number
        assert!(validate_column_name("123column").is_err());

        // SQL injection in data type
        assert!(validate_data_type("INT; DROP TABLE users;--").is_err());
    }
}

mod cross_format_tests {
    use super::*;
    use data_modelling_sdk::import::avro::AvroImporter;
    use data_modelling_sdk::import::protobuf::ProtobufImporter;

    #[test]
    fn test_sql_to_json_schema() {
        // Import from SQL
        let sql =
            "CREATE TABLE users (id BIGINT PRIMARY KEY, name VARCHAR(100) NOT NULL, age INT);";
        let sql_importer = SQLImporter::new("postgres");
        let import_result = sql_importer.parse(sql).unwrap();

        let tables = create_table_from_import_result(&import_result);

        // Export to JSON Schema
        let json_exporter = JSONSchemaExporter;
        let export_result = json_exporter.export(&tables).unwrap();

        let schema: serde_json::Value = serde_json::from_str(&export_result.content).unwrap();
        let definitions = schema.get("definitions").unwrap().as_object().unwrap();

        let users = definitions.get("users").unwrap();
        let properties = users.get("properties").unwrap().as_object().unwrap();

        assert!(properties.contains_key("id"));
        assert!(properties.contains_key("name"));
        assert!(properties.contains_key("age"));
    }

    #[test]
    fn test_avro_to_sql() {
        let avro = r#"
        {
            "type": "record",
            "name": "Event",
            "fields": [
                { "name": "id", "type": "long" },
                { "name": "timestamp", "type": "long" },
                { "name": "payload", "type": "string" }
            ]
        }
        "#;

        let avro_importer = AvroImporter::new();
        let import_result = avro_importer.import(avro).unwrap();

        let tables = create_table_from_import_result(&import_result);

        let sql_exporter = SQLExporter;
        let export_result = sql_exporter.export(&tables, Some("postgres")).unwrap();

        assert!(export_result.content.contains("\"Event\""));
        assert!(export_result.content.contains("\"id\""));
        assert!(export_result.content.contains("\"timestamp\""));
        assert!(export_result.content.contains("\"payload\""));
    }

    #[test]
    fn test_protobuf_to_sql() {
        let proto = r#"
            syntax = "proto3";

            message Product {
                int64 id = 1;
                string name = 2;
                double price = 3;
            }
        "#;

        let proto_importer = ProtobufImporter::new();
        let import_result = proto_importer.import(proto).unwrap();

        let tables = create_table_from_import_result(&import_result);

        let sql_exporter = SQLExporter;
        let export_result = sql_exporter.export(&tables, Some("postgres")).unwrap();

        assert!(export_result.content.contains("\"Product\""));
        assert!(export_result.content.contains("\"id\""));
        assert!(export_result.content.contains("\"name\""));
        assert!(export_result.content.contains("\"price\""));
    }
}
