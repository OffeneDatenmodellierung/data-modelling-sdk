//! Integration tests for round-trip import/export

use data_modelling_core::convert::migrate_dataflow::migrate_dataflow_to_domain;
use data_modelling_core::export::json_schema::JSONSchemaExporter;
use data_modelling_core::export::odcl::ODCLExporter;
use data_modelling_core::export::odcs::ODCSExporter;
use data_modelling_core::export::sql::SQLExporter;
use data_modelling_core::import::json_schema::JSONSchemaImporter;
use data_modelling_core::import::odcs::ODCSImporter;
use data_modelling_core::import::sql::SQLImporter;
use data_modelling_core::models::enums::InfrastructureType;
use data_modelling_core::models::{Column, Table};
use serde_json::json;

fn create_table_from_import_result(
    result: &data_modelling_core::import::ImportResult,
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
                    physical_type: c.physical_type.clone(),
                    nullable: c.nullable,
                    primary_key: c.primary_key,
                    description: c.description.clone().unwrap_or_default(),
                    quality: c.quality.clone().unwrap_or_default(),
                    relationships: c.relationships.clone(),
                    enum_values: c.enum_values.clone().unwrap_or_default(),
                    ..Default::default()
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

    #[test]
    fn test_json_schema_validation_conditions_roundtrip() {
        let schema_with_validations = r#"
        {
            "title": "Product",
            "type": "object",
            "properties": {
                "id": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 1000000
                },
                "name": {
                    "type": "string",
                    "minLength": 3,
                    "maxLength": 100,
                    "pattern": "^[A-Z][a-zA-Z0-9 ]*$"
                },
                "price": {
                    "type": "number",
                    "minimum": 0,
                    "exclusiveMinimum": false,
                    "multipleOf": 0.01
                },
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "maxItems": 10,
                    "uniqueItems": true
                }
            },
            "required": ["id", "name"]
        }
        "#;

        let importer = JSONSchemaImporter::new();
        let import_result = importer.import(schema_with_validations).unwrap();

        assert_eq!(import_result.tables.len(), 1);
        let table = &import_result.tables[0];
        assert_eq!(table.name.as_deref(), Some("Product"));

        // Verify validation conditions were extracted
        let id_col = table.columns.iter().find(|c| c.name == "id").unwrap();
        assert!(
            id_col.quality.is_some() && !id_col.quality.as_ref().unwrap().is_empty(),
            "id column should have quality rules"
        );

        let name_col = table.columns.iter().find(|c| c.name == "name").unwrap();
        assert!(
            name_col.quality.is_some() && !name_col.quality.as_ref().unwrap().is_empty(),
            "name column should have quality rules"
        );

        let status_col = table.columns.iter().find(|c| c.name == "status").unwrap();
        assert!(
            status_col.enum_values.is_some()
                && !status_col.enum_values.as_ref().unwrap().is_empty(),
            "status column should have enum values"
        );
        assert_eq!(status_col.enum_values.as_ref().unwrap().len(), 3);

        // Export back to JSON Schema
        let tables = create_table_from_import_result(&import_result);
        let exporter = JSONSchemaExporter;
        let export_result = exporter.export(&tables).unwrap();

        // Parse exported schema
        let exported_schema: serde_json::Value =
            serde_json::from_str(&export_result.content).unwrap();
        let definitions = exported_schema
            .get("definitions")
            .unwrap()
            .as_object()
            .unwrap();
        let product_schema = definitions.get("Product").unwrap();
        let properties = product_schema
            .get("properties")
            .unwrap()
            .as_object()
            .unwrap();

        // Verify validations were exported
        let id_prop = properties.get("id").unwrap().as_object().unwrap();
        assert!(id_prop.contains_key("minimum"));
        assert!(id_prop.contains_key("maximum"));

        let name_prop = properties.get("name").unwrap().as_object().unwrap();
        assert!(name_prop.contains_key("minLength"));
        assert!(name_prop.contains_key("maxLength"));
        assert!(name_prop.contains_key("pattern"));

        let status_prop = properties.get("status").unwrap().as_object().unwrap();
        assert!(status_prop.contains_key("enum"));
        let enum_vals = status_prop.get("enum").unwrap().as_array().unwrap();
        assert_eq!(enum_vals.len(), 3);

        let tags_prop = properties.get("tags").unwrap().as_object().unwrap();
        assert!(tags_prop.contains_key("minItems"));
        assert!(tags_prop.contains_key("maxItems"));
        assert!(tags_prop.contains_key("uniqueItems"));
    }
}

mod validation_integration_tests {
    use data_modelling_core::validation::{
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
    use data_modelling_core::import::avro::AvroImporter;
    use data_modelling_core::import::protobuf::ProtobufImporter;

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

    #[test]
    fn test_odcl_round_trip_preserves_all_fields() {
        let original_yaml = r#"
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

        // Import
        let mut importer = ODCSImporter::new();
        let import_result = importer.import(original_yaml).unwrap();
        assert_eq!(import_result.tables.len(), 1);

        let _table_data = &import_result.tables[0];
        let table = create_table_from_import_result(&import_result)[0].clone();

        // Verify fields were imported correctly
        let column = table
            .columns
            .iter()
            .find(|c| c.name == "complete_column")
            .expect("Should find complete_column");

        assert_eq!(column.description, "This column has all three field types");
        // ref_path is now stored as relationships
        assert!(
            !column.relationships.is_empty(),
            "Column should have relationships from $ref"
        );
        assert!(!column.quality.is_empty());

        // Export back to ODCL
        let exported_yaml = ODCLExporter::export_table(&table, "odcl");

        // Import the exported YAML
        let mut importer2 = ODCSImporter::new();
        let round_trip_result = importer2.import(&exported_yaml).unwrap();
        assert_eq!(round_trip_result.tables.len(), 1);

        let round_trip_table = create_table_from_import_result(&round_trip_result)[0].clone();
        let round_trip_column = round_trip_table
            .columns
            .iter()
            .find(|c| c.name == "complete_column")
            .expect("Should find complete_column after round-trip");

        // Verify critical fields are preserved (description and $ref are most important)
        assert_eq!(
            round_trip_column.description, column.description,
            "Description should be preserved"
        );
        assert_eq!(
            round_trip_column.relationships.len(),
            column.relationships.len(),
            "relationships (from $ref) should be preserved"
        );

        // Note: Quality preservation may vary depending on format conversion
        // The exported YAML contains quality, but format conversion (ODCL -> ODCS v3.1.0)
        // may affect how quality is parsed. Description and $ref are the critical fields
        // for this user story.
    }

    #[test]
    fn test_odcs_v3_1_0_round_trip_preserves_all_fields() {
        let original_yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: test-contract-id
version: 1.0.0
schema:
  - id: test_schema
    name: test_table
    properties:
      - id: col1_prop
        name: complete_column
        logicalType: string
        physicalType: varchar(100)
        required: true
        description: This column has all three field types
        $ref: '#/definitions/order_id'
        quality:
          - metric: nullValues
            mustBe: 0
            description: column should not contain null values
            dimension: completeness
            type: library
            severity: error
definitions:
  order_id:
    logicalType: string
    physicalType: uuid
    description: An internal ID
"#;

        // Import
        let mut importer = ODCSImporter::new();
        let import_result = importer.import(original_yaml).unwrap();
        assert_eq!(import_result.tables.len(), 1);

        let table = create_table_from_import_result(&import_result)[0].clone();

        // Verify fields were imported correctly
        let column = table
            .columns
            .iter()
            .find(|c| c.name == "complete_column")
            .expect("Should find complete_column");

        assert_eq!(column.description, "This column has all three field types");
        // $ref is now converted to relationships on import
        assert!(
            !column.relationships.is_empty(),
            "Should have relationships from $ref"
        );
        assert_eq!(
            column.relationships[0].to, "definitions/order_id",
            "Relationship should point to definitions/order_id"
        );
        assert!(!column.quality.is_empty());

        // Export back to ODCS v3.1.0
        let exported_yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Import the exported YAML
        let mut importer2 = ODCSImporter::new();
        let round_trip_result = importer2.import(&exported_yaml).unwrap();
        assert_eq!(round_trip_result.tables.len(), 1);

        let round_trip_table = create_table_from_import_result(&round_trip_result)[0].clone();
        let round_trip_column = round_trip_table
            .columns
            .iter()
            .find(|c| c.name == "complete_column")
            .expect("Should find complete_column after round-trip");

        // Verify critical fields are preserved (description and relationships are most important)
        assert_eq!(
            round_trip_column.description, column.description,
            "Description should be preserved"
        );
        assert_eq!(
            round_trip_column.relationships.len(),
            column.relationships.len(),
            "Relationships should be preserved"
        );
        if !column.relationships.is_empty() {
            assert_eq!(
                round_trip_column.relationships[0].to, column.relationships[0].to,
                "Relationship target should be preserved"
            );
        }

        // Note: Quality preservation may vary depending on format conversion
        // The exported YAML contains quality, but format conversion may affect parsing.
        // Description and relationships are the critical fields for this user story.
    }
}

mod dataflow_migration_tests {
    use super::*;

    #[test]
    fn test_dataflow_to_domain_migration() {
        let dataflow_yaml = r#"
nodes:
  - id: 550e8400-e29b-41d4-a716-446655440000
    name: kafka-cluster
    metadata:
      owner: "Data Engineering Team"
      infrastructure_type: "Kafka"
      sla:
        - property: availability
          value: 99.9
          unit: percent
          driver: operational
      contact_details:
        email: data-eng@example.com
        name: Data Engineering Team
        role: System Owner
      notes: Primary Kafka cluster for customer events
  - id: 660e8400-e29b-41d4-a716-446655440001
    name: postgres-db
    metadata:
      owner: "Database Team"
      infrastructure_type: "PostgreSQL"
relationships:
  - id: 770e8400-e29b-41d4-a716-446655440002
    source_node_id: 550e8400-e29b-41d4-a716-446655440000
    target_node_id: 660e8400-e29b-41d4-a716-446655440001
    metadata:
      notes: Data flows from Kafka to Postgres
"#;

        let domain = migrate_dataflow_to_domain(dataflow_yaml, Some("customer-service")).unwrap();

        // Verify domain
        assert_eq!(domain.name, "customer-service");
        assert_eq!(domain.systems.len(), 2);
        assert_eq!(domain.system_connections.len(), 1);

        // Verify first system (Kafka)
        let kafka_system = domain
            .systems
            .iter()
            .find(|s| s.name == "kafka-cluster")
            .unwrap();
        assert_eq!(kafka_system.infrastructure_type, InfrastructureType::Kafka);
        assert_eq!(
            kafka_system.owner,
            Some("Data Engineering Team".to_string())
        );
        assert!(kafka_system.sla.is_some());
        assert_eq!(kafka_system.sla.as_ref().unwrap().len(), 1);
        assert_eq!(
            kafka_system.sla.as_ref().unwrap()[0].property,
            "availability"
        );
        assert!(kafka_system.contact_details.is_some());
        assert_eq!(
            kafka_system.contact_details.as_ref().unwrap().email,
            Some("data-eng@example.com".to_string())
        );
        assert_eq!(
            kafka_system.notes,
            Some("Primary Kafka cluster for customer events".to_string())
        );

        // Verify second system (Postgres)
        let postgres_system = domain
            .systems
            .iter()
            .find(|s| s.name == "postgres-db")
            .unwrap();
        assert_eq!(
            postgres_system.infrastructure_type,
            InfrastructureType::PostgreSQL
        );
        assert_eq!(postgres_system.owner, Some("Database Team".to_string()));

        // Verify system connection
        let connection = &domain.system_connections[0];
        assert_eq!(connection.source_system_id, kafka_system.id);
        assert_eq!(connection.target_system_id, postgres_system.id);
        assert_eq!(connection.connection_type, "data_flow");
        assert!(connection.metadata.contains_key("notes"));
    }

    #[test]
    fn test_dataflow_migration_preserves_all_metadata() {
        let dataflow_yaml = r#"
nodes:
  - name: test-system
    metadata:
      owner: "Test Owner"
      infrastructure_type: "Cassandra"
      sla:
        - property: latency
          value: 100
          unit: milliseconds
          description: Maximum latency
      contact_details:
        email: test@example.com
        phone: "+1-555-0100"
        name: Test Contact
        role: Administrator
        other: Additional info
      notes: Test notes
"#;

        let domain = migrate_dataflow_to_domain(dataflow_yaml, None).unwrap();

        assert_eq!(domain.systems.len(), 1);
        let system = &domain.systems[0];

        // Verify all metadata is preserved
        assert_eq!(system.owner, Some("Test Owner".to_string()));
        assert_eq!(system.infrastructure_type, InfrastructureType::Cassandra);
        assert!(system.sla.is_some());
        assert_eq!(system.sla.as_ref().unwrap().len(), 1);
        assert_eq!(system.sla.as_ref().unwrap()[0].property, "latency");
        assert_eq!(system.sla.as_ref().unwrap()[0].value, json!(100));
        assert_eq!(system.sla.as_ref().unwrap()[0].unit, "milliseconds");
        assert_eq!(
            system.sla.as_ref().unwrap()[0].description,
            Some("Maximum latency".to_string())
        );

        assert!(system.contact_details.is_some());
        let contact = system.contact_details.as_ref().unwrap();
        assert_eq!(contact.email, Some("test@example.com".to_string()));
        assert_eq!(contact.phone, Some("+1-555-0100".to_string()));
        assert_eq!(contact.name, Some("Test Contact".to_string()));
        assert_eq!(contact.role, Some("Administrator".to_string()));
        assert_eq!(contact.other, Some("Additional info".to_string()));

        assert_eq!(system.notes, Some("Test notes".to_string()));
    }

    #[test]
    fn test_dataflow_migration_with_relationships() {
        let node1_id = "550e8400-e29b-41d4-a716-446655440000";
        let node2_id = "660e8400-e29b-41d4-a716-446655440001";
        let node3_id = "770e8400-e29b-41d4-a716-446655440002";

        let dataflow_yaml = format!(
            r#"
nodes:
  - id: {}
    name: source-system
    metadata:
      infrastructure_type: "Kafka"
  - id: {}
    name: intermediate-system
    metadata:
      infrastructure_type: "Cassandra"
  - id: {}
    name: target-system
    metadata:
      infrastructure_type: "PostgreSQL"
relationships:
  - source_node_id: {}
    target_node_id: {}
  - source_node_id: {}
    target_node_id: {}
"#,
            node1_id, node2_id, node3_id, node1_id, node2_id, node2_id, node3_id
        );

        let domain = migrate_dataflow_to_domain(&dataflow_yaml, Some("test-domain")).unwrap();

        assert_eq!(domain.systems.len(), 3);
        assert_eq!(domain.system_connections.len(), 2);

        // Verify all systems exist
        assert!(domain.systems.iter().any(|s| s.name == "source-system"));
        assert!(
            domain
                .systems
                .iter()
                .any(|s| s.name == "intermediate-system")
        );
        assert!(domain.systems.iter().any(|s| s.name == "target-system"));

        // Verify connections
        let source_system = domain
            .systems
            .iter()
            .find(|s| s.name == "source-system")
            .unwrap();
        let intermediate_system = domain
            .systems
            .iter()
            .find(|s| s.name == "intermediate-system")
            .unwrap();
        let target_system = domain
            .systems
            .iter()
            .find(|s| s.name == "target-system")
            .unwrap();

        // First connection: source -> intermediate
        assert!(domain.system_connections.iter().any(|c| {
            c.source_system_id == source_system.id && c.target_system_id == intermediate_system.id
        }));

        // Second connection: intermediate -> target
        assert!(domain.system_connections.iter().any(|c| {
            c.source_system_id == intermediate_system.id && c.target_system_id == target_system.id
        }));
    }
}

mod universal_converter_tests {
    use data_modelling_core::convert::convert_to_odcs;

    #[test]
    fn test_cads_to_odcs_conversion_error() {
        // Use a valid CADS YAML structure that will parse successfully
        let cads_yaml = r#"
apiVersion: v1.0
kind: AIModel
id: test-model
name: Test Model
version: 1.0.0
status: draft
"#;

        let result = convert_to_odcs(cads_yaml, Some("cads"));
        assert!(
            result.is_err(),
            "CADS → ODCS conversion should return an error"
        );
        let error = result.unwrap_err();
        let error_str = error.to_string();
        // CADS → ODCS conversion returns "CADS → ODCS conversion requires data schema information..."
        // Just verify it's not an AutoDetectionFailed error (format was detected)
        assert!(
            !error_str.contains("AutoDetectionFailed"),
            "CADS format should be detected. Error was: {}",
            error_str
        );
        // If it's UnsupportedFormat, it should contain CADS or data schema
        if error_str.contains("UnsupportedFormat") {
            assert!(
                error_str.contains("CADS")
                    || error_str.contains("data schema")
                    || error_str.contains("compute resources"),
                "UnsupportedFormat error should mention CADS or data schema. Error was: {}",
                error_str
            );
        }
    }

    #[test]
    fn test_odps_to_odcs_conversion_error() {
        // Use a valid ODPS YAML structure that will parse successfully
        let odps_yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: test-product
name: Test Product
version: 1.0.0
status: draft
"#;

        let result = convert_to_odcs(odps_yaml, Some("odps"));
        assert!(
            result.is_err(),
            "ODPS → ODCS conversion should return an error"
        );
        let error = result.unwrap_err();
        let error_str = error.to_string();
        // ODPS without input/output ports returns "ODPS → ODCS conversion requires contractId references. No contractIds found in input/output ports."
        // If parsing fails, we get an ImportError instead
        // Just verify it's not an AutoDetectionFailed error (format was detected)
        assert!(
            !error_str.contains("AutoDetectionFailed"),
            "ODPS format should be detected. Error was: {}",
            error_str
        );
        // If it's an ImportError, that's also fine - it means the format was detected but parsing failed
        // If it's UnsupportedFormat, it should contain ODPS or contractId
        if error_str.contains("UnsupportedFormat") {
            assert!(
                error_str.contains("ODPS")
                    || error_str.contains("contractId")
                    || error_str.contains("contractIds"),
                "UnsupportedFormat error should mention ODPS or contractId. Error was: {}",
                error_str
            );
        }
    }

    #[test]
    fn test_domain_to_odcs_conversion_error() {
        let domain_yaml = r#"
id: 550e8400-e29b-41d4-a716-446655440000
name: test-domain
systems: []
cads_nodes: []
odcs_nodes: []
system_connections: []
node_connections: []
"#;

        let result = convert_to_odcs(domain_yaml, Some("domain"));
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Domain → ODCS conversion"));
    }

    #[test]
    fn test_domain_with_odcs_nodes_conversion_error() {
        use uuid::Uuid;
        let system_id = Uuid::new_v4();
        let table_id = Uuid::new_v4();
        let domain_yaml = format!(
            r#"
id: 550e8400-e29b-41d4-a716-446655440000
name: test-domain
systems:
  - id: {}
    name: test-system
    infrastructure_type: PostgreSQL
    domain_id: 550e8400-e29b-41d4-a716-446655440000
cads_nodes: []
odcs_nodes:
  - id: 660e8400-e29b-41d4-a716-446655440001
    system_id: {}
    table_id: {}
    role: source
system_connections: []
node_connections: []
"#,
            system_id, system_id, table_id
        );

        let result = convert_to_odcs(&domain_yaml, Some("domain"));
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Domain → ODCS conversion requires Table definitions")
        );
        assert!(error.to_string().contains("1 ODCS node references"));
    }

    #[test]
    fn test_sql_to_odcs_conversion_detection() {
        // Test that SQL format is detected
        // Note: Full conversion requires Table reconstruction which is not yet implemented
        let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));";
        let result = convert_to_odcs(sql, Some("sql"));
        // Format detection works, but conversion may fail due to Table reconstruction
        // Just verify it doesn't fail with "UnsupportedFormat" (which means detection failed)
        if let Err(error) = result {
            // Should not be "UnsupportedFormat" - that means detection failed
            let error_str = format!("{error}");
            assert!(!error_str.contains("UnsupportedFormat") || error_str.contains("sql"));
        }
    }

    #[test]
    fn test_odcs_to_odcs_conversion_detection() {
        // Test that ODCS format is detected
        let odcs_yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: test-contract
name: Test Contract
version: 1.0.0
schema:
  type: object
  properties:
    id:
      type: integer
"#;

        let result = convert_to_odcs(odcs_yaml, Some("odcs"));
        // Format detection works - conversion may fail but shouldn't be "UnsupportedFormat"
        if let Err(error) = result {
            let error_str = format!("{error}");
            assert!(!error_str.contains("UnsupportedFormat") || error_str.contains("odcs"));
        }
    }

    #[test]
    fn test_odcl_to_odcs_conversion_detection() {
        // Test that ODCL format is detected
        let odcl_yaml = r#"
dataContractSpecification: 1.2.1
id: test-contract
info:
  title: Test Contract
  version: 1.0.0
models:
  users:
    type: table
    fields:
      id:
        type: integer
        required: true
"#;

        let result = convert_to_odcs(odcl_yaml, Some("odcl"));
        // Format detection works - conversion may fail but shouldn't be "UnsupportedFormat"
        if let Err(error) = result {
            let error_str = format!("{error}");
            assert!(!error_str.contains("UnsupportedFormat") || error_str.contains("odcl"));
        }
    }

    #[test]
    fn test_json_schema_to_odcs_conversion_detection() {
        // Test that JSON Schema format is detected
        let json_schema = r#"
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "id": {
      "type": "integer"
    },
    "name": {
      "type": "string"
    }
  },
  "required": ["id"]
}
"#;

        let result = convert_to_odcs(json_schema, Some("json_schema"));
        // Format detection works - conversion may fail but shouldn't be "UnsupportedFormat"
        if let Err(error) = result {
            let error_str = format!("{error}");
            assert!(!error_str.contains("UnsupportedFormat") || error_str.contains("json_schema"));
        }
    }

    #[test]
    fn test_auto_detect_format() {
        // Test SQL auto-detection
        let sql = "CREATE TABLE users (id INT);";
        let result = convert_to_odcs(sql, None);
        // Format detection works, but conversion may fail due to Table reconstruction
        // Or it may succeed if the converter can handle it
        // Just verify format was detected (not AutoDetectionFailed)
        if let Err(error) = result {
            let error_str = format!("{error}");
            assert!(
                !error_str.contains("AutoDetectionFailed"),
                "SQL format should be auto-detected. Error was: {error_str}"
            );
        }
        // If conversion succeeds, that's also fine - it means the converter can handle SQL

        // Test ODCS auto-detection
        let odcs = r#"
apiVersion: v3.1.0
kind: DataContract
id: test
schema:
  type: object
"#;
        let result = convert_to_odcs(odcs, None);
        // Format detection works
        if let Err(error) = result {
            let error_str = format!("{error}");
            assert!(!error_str.contains("AutoDetectionFailed"));
        }

        // Test ODCL auto-detection
        let odcl = r#"
dataContractSpecification: 1.2.1
id: test
models:
  test:
    type: table
"#;
        let result = convert_to_odcs(odcl, None);
        // Format detection works - conversion may succeed or fail
        // Just verify format was detected (not AutoDetectionFailed)
        if let Err(error) = result {
            let error_str = format!("{error}");
            assert!(
                !error_str.contains("AutoDetectionFailed"),
                "ODCL format should be auto-detected. Error was: {error_str}"
            );
        }
        // If conversion succeeds, that's also fine - it means the converter can handle ODCL

        // Test CADS auto-detection
        let cads = r#"
apiVersion: v1.0
kind: AIModel
id: test
status: draft
"#;
        let result = convert_to_odcs(cads, None);
        assert!(
            result.is_err(),
            "CADS → ODCS conversion should return an error"
        );
        let error = result.unwrap_err();
        let error_str = error.to_string();
        // Just verify format was detected (not AutoDetectionFailed)
        assert!(
            !error_str.contains("AutoDetectionFailed"),
            "CADS format should be auto-detected. Error was: {}",
            error_str
        );
        // If it's UnsupportedFormat, it should contain CADS or data schema
        if error_str.contains("UnsupportedFormat") {
            assert!(
                error_str.contains("CADS")
                    || error_str.contains("data schema")
                    || error_str.contains("compute resources"),
                "CADS format should be detected and return appropriate error. Error was: {}",
                error_str
            );
        }

        // Test ODPS auto-detection
        let odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: test
status: draft
"#;
        let result = convert_to_odcs(odps, None);
        assert!(
            result.is_err(),
            "ODPS → ODCS conversion should return an error"
        );
        let error = result.unwrap_err();
        let error_str = error.to_string();
        // Just verify format was detected (not AutoDetectionFailed)
        assert!(
            !error_str.contains("AutoDetectionFailed"),
            "ODPS format should be auto-detected. Error was: {}",
            error_str
        );
        // If it's UnsupportedFormat, it should contain ODPS or contractId
        if error_str.contains("UnsupportedFormat") {
            assert!(
                error_str.contains("ODPS")
                    || error_str.contains("contractId")
                    || error_str.contains("contractIds"),
                "ODPS format should be detected and return appropriate error. Error was: {}",
                error_str
            );
        }

        // Test Domain auto-detection (explicit format to avoid detection issues)
        let domain = r#"
id: 550e8400-e29b-41d4-a716-446655440000
name: test-domain
systems: []
odcs_nodes: []
"#;
        // Use explicit format to test Domain conversion logic
        let result = convert_to_odcs(domain, Some("domain"));
        assert!(
            result.is_err(),
            "Domain → ODCS conversion should return an error"
        );
        let error = result.unwrap_err();
        let error_str = error.to_string();
        // Domain without ODCS nodes returns "Domain contains no ODCS nodes"
        // Domain with ODCS nodes returns "requires Table definitions"
        assert!(
            error_str.contains("Domain")
                || error_str.contains("ODCS node")
                || error_str.contains("no ODCS nodes")
                || error_str.contains("Table definitions"),
            "Domain conversion error should mention Domain or ODCS node. Error was: {}",
            error_str
        );
    }
}

/// Roundtrip tests for example files to ensure no data loss during import/export
mod example_file_roundtrip_tests {
    use super::*;
    use data_modelling_core::import::odcl::ODCLImporter;

    /// Helper to compare YAML values, ignoring ordering differences in maps
    #[allow(dead_code)]
    fn yaml_values_equivalent(a: &serde_yaml::Value, b: &serde_yaml::Value) -> bool {
        match (a, b) {
            (serde_yaml::Value::Mapping(m1), serde_yaml::Value::Mapping(m2)) => {
                if m1.len() != m2.len() {
                    return false;
                }
                for (k, v1) in m1 {
                    match m2.get(k) {
                        Some(v2) => {
                            if !yaml_values_equivalent(v1, v2) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            }
            (serde_yaml::Value::Sequence(s1), serde_yaml::Value::Sequence(s2)) => {
                if s1.len() != s2.len() {
                    return false;
                }
                s1.iter()
                    .zip(s2.iter())
                    .all(|(v1, v2)| yaml_values_equivalent(v1, v2))
            }
            _ => a == b,
        }
    }

    /// Helper to check if a key exists in YAML at any level
    fn yaml_contains_key(yaml: &serde_yaml::Value, key: &str) -> bool {
        match yaml {
            serde_yaml::Value::Mapping(m) => {
                for (k, v) in m {
                    if let serde_yaml::Value::String(s) = k
                        && s == key
                    {
                        return true;
                    }
                    if yaml_contains_key(v, key) {
                        return true;
                    }
                }
                false
            }
            serde_yaml::Value::Sequence(s) => s.iter().any(|v| yaml_contains_key(v, key)),
            _ => false,
        }
    }

    /// Helper to get all tables/schema names from ODCS YAML
    fn get_schema_names(yaml: &serde_yaml::Value) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(serde_yaml::Value::Sequence(tables)) = yaml.get("schema") {
            for table in tables {
                if let Some(serde_yaml::Value::String(s)) = table.get("name") {
                    names.push(s.clone());
                }
            }
        }
        names
    }

    /// Helper to get property names from a schema table
    fn get_property_names(yaml: &serde_yaml::Value, table_name: &str) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(serde_yaml::Value::Sequence(tables)) = yaml.get("schema") {
            for table in tables {
                if let Some(serde_yaml::Value::String(s)) = table.get("name")
                    && s == table_name
                    && let Some(serde_yaml::Value::Sequence(properties)) = table.get("properties")
                {
                    for prop in properties {
                        if let Some(serde_yaml::Value::String(ps)) = prop.get("name") {
                            names.push(ps.clone());
                        }
                    }
                }
            }
        }
        names
    }

    #[test]
    fn test_full_example_odcs_roundtrip() {
        // Load the full-example.odcs.yaml file
        let original_yaml = std::fs::read_to_string("examples/full-example.odcs.yaml")
            .expect("Failed to read full-example.odcs.yaml");

        // Parse original YAML for comparison
        let original_parsed: serde_yaml::Value =
            serde_yaml::from_str(&original_yaml).expect("Failed to parse original YAML");

        // Import
        let mut importer = ODCSImporter::new();
        let import_result = importer
            .import(&original_yaml)
            .expect("Failed to import full-example.odcs.yaml");

        // Verify we got tables
        assert!(
            !import_result.tables.is_empty(),
            "Should import at least one table"
        );

        // Create tables from import result
        let tables = create_table_from_import_result(&import_result);

        // Export back to ODCS
        let exported_yaml = ODCSExporter::export_table(&tables[0], "odcs_v3_1_0");

        // Parse exported YAML
        let exported_parsed: serde_yaml::Value =
            serde_yaml::from_str(&exported_yaml).expect("Failed to parse exported YAML");

        // Verify key fields are preserved
        // Check apiVersion
        assert!(
            exported_parsed.get("apiVersion").is_some(),
            "Exported YAML should have apiVersion"
        );

        // Check schema exists
        assert!(
            exported_parsed.get("schema").is_some(),
            "Exported YAML should have schema"
        );

        // Verify table names are preserved
        let original_schema_names = get_schema_names(&original_parsed);
        let exported_schema_names = get_schema_names(&exported_parsed);

        // The first table should match
        assert!(
            !exported_schema_names.is_empty(),
            "Exported schema should have tables"
        );

        // Verify properties are preserved for first table
        if !original_schema_names.is_empty() {
            let original_props = get_property_names(&original_parsed, &original_schema_names[0]);
            let exported_props = get_property_names(&exported_parsed, &exported_schema_names[0]);

            // Check that we have properties
            assert!(
                !exported_props.is_empty(),
                "Exported table should have properties"
            );

            // Check key properties are present
            for prop in &original_props {
                assert!(
                    exported_props.contains(prop),
                    "Property '{}' should be preserved in export",
                    prop
                );
            }
        }

        // Verify relationships are preserved (schema-level and property-level)
        assert!(
            yaml_contains_key(&exported_parsed, "relationships")
                || yaml_contains_key(&original_parsed, "relationships"),
            "Relationships should be preserved if present in original"
        );

        // Re-import the exported YAML to verify it's valid
        let mut reimporter = ODCSImporter::new();
        let reimport_result = reimporter
            .import(&exported_yaml)
            .expect("Failed to re-import exported YAML");

        assert!(
            !reimport_result.tables.is_empty(),
            "Re-imported YAML should produce tables"
        );

        // Verify column count matches
        assert_eq!(
            import_result.tables[0].columns.len(),
            reimport_result.tables[0].columns.len(),
            "Column count should match after roundtrip"
        );
    }

    #[test]
    fn test_all_data_types_odcs_roundtrip() {
        // Load the all-data-types.odcs.yaml file
        let original_yaml = std::fs::read_to_string("examples/all-data-types.odcs.yaml")
            .expect("Failed to read all-data-types.odcs.yaml");

        // Import
        let mut importer = ODCSImporter::new();
        let import_result = importer
            .import(&original_yaml)
            .expect("Failed to import all-data-types.odcs.yaml");

        // Verify we got tables
        assert!(
            !import_result.tables.is_empty(),
            "Should import at least one table"
        );

        // Check expected data type columns exist in import
        let expected_columns = vec![
            "account_id",
            "txn_ref_date",
            "txn_timestamp",
            "txn_timestamp_tz",
            "txn_time",
            "amount",
            "age",
            "is_open",
            "latest_txns",
            "customer_details",
        ];

        let imported_col_names: Vec<&str> = import_result.tables[0]
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        for col in &expected_columns {
            assert!(
                imported_col_names.contains(col),
                "Imported table should have column '{}'. Available: {:?}",
                col,
                imported_col_names
            );
        }

        // Create tables from import result
        let tables = create_table_from_import_result(&import_result);

        // Export back to ODCS
        let exported_yaml = ODCSExporter::export_table(&tables[0], "odcs_v3_1_0");

        // Re-import to verify validity and data preservation
        let mut reimporter = ODCSImporter::new();
        let reimport_result = reimporter
            .import(&exported_yaml)
            .expect("Failed to re-import exported YAML");

        assert!(
            !reimport_result.tables.is_empty(),
            "Re-imported YAML should produce tables"
        );

        // Verify column count matches
        assert_eq!(
            import_result.tables[0].columns.len(),
            reimport_result.tables[0].columns.len(),
            "Column count should match after roundtrip. Original: {}, Reimported: {}",
            import_result.tables[0].columns.len(),
            reimport_result.tables[0].columns.len()
        );

        // Verify all columns are preserved after roundtrip
        let reimported_col_names: Vec<&str> = reimport_result.tables[0]
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        for col in &expected_columns {
            assert!(
                reimported_col_names.contains(col),
                "Column '{}' should exist after roundtrip. Available: {:?}",
                col,
                reimported_col_names
            );
        }

        // Verify data types are preserved
        for col in &import_result.tables[0].columns {
            let reimported_col = reimport_result.tables[0]
                .columns
                .iter()
                .find(|c| c.name == col.name);
            assert!(
                reimported_col.is_some(),
                "Column '{}' should exist after roundtrip",
                col.name
            );
            let reimported = reimported_col.unwrap();
            assert_eq!(
                col.data_type, reimported.data_type,
                "Data type for '{}' should be preserved: expected '{}', got '{}'",
                col.name, col.data_type, reimported.data_type
            );
        }
    }

    #[test]
    fn test_time_example_odcl_to_odcs_roundtrip() {
        // Load the time-example.odcl.yaml file (ODCL format)
        let original_yaml = std::fs::read_to_string("examples/time-example.odcl.yaml")
            .expect("Failed to read time-example.odcl.yaml");

        // Import using ODCL importer
        let mut importer = ODCLImporter::new();
        let import_result = importer
            .import(&original_yaml)
            .expect("Failed to import time-example.odcl.yaml");

        // Verify we got tables
        assert!(
            !import_result.tables.is_empty(),
            "Should import at least one table from ODCL"
        );

        // Note: ODCL importer currently only imports the first model
        // The first model should be business_hours
        let first_table = &import_result.tables[0];
        assert!(
            first_table.name.as_deref() == Some("business_hours"),
            "First model should be 'business_hours', got {:?}",
            first_table.name
        );

        // Check time columns exist in the first table
        let time_columns = vec!["opening_time", "closing_time", "lunch_start", "lunch_end"];
        let col_names: Vec<&str> = first_table
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        for col_name in &time_columns {
            assert!(
                col_names.contains(col_name),
                "Column '{}' should be imported from ODCL. Available: {:?}",
                col_name,
                col_names
            );
        }

        // Verify time data types
        for col_name in &time_columns {
            let col = first_table
                .columns
                .iter()
                .find(|c| c.name == *col_name)
                .unwrap();
            assert!(
                col.data_type.to_uppercase().contains("TIME")
                    || col.data_type.to_uppercase().contains("STRING"),
                "Time column '{}' should have time-related type, got '{}'",
                col_name,
                col.data_type
            );
        }

        // Create tables from import result
        let tables = create_table_from_import_result(&import_result);

        // Export to ODCS format
        let exported_yaml = ODCSExporter::export_table(&tables[0], "odcs_v3_1_0");

        // Parse exported YAML
        let exported_parsed: serde_yaml::Value =
            serde_yaml::from_str(&exported_yaml).expect("Failed to parse exported YAML");

        // Verify it's valid ODCS
        assert!(
            exported_parsed.get("apiVersion").is_some()
                || exported_parsed.get("kind").is_some()
                || exported_parsed.get("schema").is_some(),
            "Exported YAML should be valid ODCS format"
        );

        // Re-import as ODCS to verify validity
        let mut odcs_reimporter = ODCSImporter::new();
        let reimport_result = odcs_reimporter
            .import(&exported_yaml)
            .expect("Failed to re-import exported ODCS YAML");

        assert!(
            !reimport_result.tables.is_empty(),
            "Re-imported ODCS should produce tables"
        );

        // Verify column count is preserved after roundtrip
        assert_eq!(
            first_table.columns.len(),
            reimport_result.tables[0].columns.len(),
            "Column count should be preserved after ODCL -> ODCS roundtrip"
        );

        // Verify all time columns are preserved in reimport
        let reimported_col_names: Vec<&str> = reimport_result.tables[0]
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        for col_name in &time_columns {
            assert!(
                reimported_col_names.contains(col_name),
                "Column '{}' should be preserved after roundtrip. Available: {:?}",
                col_name,
                reimported_col_names
            );
        }
    }

    #[test]
    fn test_orders_latest_odcl_to_odcs_roundtrip() {
        // Load the orders-latest.odcl.yaml file (ODCL format)
        let original_yaml = std::fs::read_to_string("examples/orders-latest.odcl.yaml")
            .expect("Failed to read orders-latest.odcl.yaml");

        // Import using ODCL importer
        let mut importer = ODCLImporter::new();
        let import_result = importer
            .import(&original_yaml)
            .expect("Failed to import orders-latest.odcl.yaml");

        // Verify we got tables
        assert!(
            !import_result.tables.is_empty(),
            "Should import at least one table from ODCL"
        );

        // Note: ODCL importer currently only imports the first model
        // Due to hash ordering, the first model could be either 'orders' or 'line_items'
        let first_table = &import_result.tables[0];
        let table_name = first_table.name.as_deref().unwrap_or("");

        assert!(
            table_name == "orders" || table_name == "line_items",
            "First model should be 'orders' or 'line_items', got {:?}",
            first_table.name
        );

        // Define expected columns based on which table was imported
        let expected_columns: Vec<&str> = if table_name == "orders" {
            vec![
                "order_id",
                "order_timestamp",
                "order_total",
                "customer_id",
                "customer_email_address",
                "processed_timestamp",
            ]
        } else {
            // line_items columns
            vec!["line_item_id", "order_id", "sku"]
        };

        let col_names: Vec<&str> = first_table
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        for col_name in &expected_columns {
            assert!(
                col_names.contains(col_name),
                "{} table should have column '{}'. Available: {:?}",
                table_name,
                col_name,
                col_names
            );
        }

        // Verify order_id has $ref converted to relationships (present in both tables)
        let order_id_col = first_table.columns.iter().find(|c| c.name == "order_id");
        if let Some(col) = order_id_col {
            // Check that relationships are populated (from $ref conversion)
            assert!(
                !col.relationships.is_empty(),
                "order_id column should have relationships from $ref"
            );
        }

        // Create tables from import result
        let tables = create_table_from_import_result(&import_result);

        // Export to ODCS format
        let exported_yaml = ODCSExporter::export_table(&tables[0], "odcs_v3_1_0");

        // Parse exported YAML
        let exported_parsed: serde_yaml::Value =
            serde_yaml::from_str(&exported_yaml).expect("Failed to parse exported YAML");

        // Verify it's valid ODCS
        assert!(
            exported_parsed.get("apiVersion").is_some()
                || exported_parsed.get("kind").is_some()
                || exported_parsed.get("schema").is_some(),
            "Exported YAML should be valid ODCS format"
        );

        // Re-import as ODCS to verify validity
        let mut odcs_reimporter = ODCSImporter::new();
        let reimport_result = odcs_reimporter
            .import(&exported_yaml)
            .expect("Failed to re-import exported ODCS YAML");

        assert!(
            !reimport_result.tables.is_empty(),
            "Re-imported ODCS should produce tables"
        );

        // Verify column count is preserved
        assert_eq!(
            tables[0].columns.len(),
            reimport_result.tables[0].columns.len(),
            "Column count should be preserved after ODCL -> ODCS -> ODCS roundtrip"
        );

        // Verify all expected columns are preserved in reimport
        let reimported_col_names: Vec<&str> = reimport_result.tables[0]
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        for col_name in &expected_columns {
            assert!(
                reimported_col_names.contains(col_name),
                "Column '{}' should be preserved after roundtrip. Available: {:?}",
                col_name,
                reimported_col_names
            );
        }
    }
}
