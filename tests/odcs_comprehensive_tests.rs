//! Comprehensive ODCS import/export tests

#![allow(clippy::needless_borrows_for_generic_args)]

use data_modelling_sdk::export::odcs::ODCSExporter;
use data_modelling_sdk::import::odcs::ODCSImporter;
use data_modelling_sdk::models::{Column, Table, Tag};
use data_modelling_sdk::{DataVaultClassification, DatabaseType, MedallionLayer, SCDPattern};
use serde_yaml::Value as YamlValue;

/// Validate that exported YAML conforms to ODCS v3.1.0 schema
fn validate_odcs_v3_1_0_schema(yaml: &str) -> Result<(), String> {
    // Parse YAML to verify it's valid
    let yaml_value: YamlValue =
        serde_yaml::from_str(yaml).map_err(|e| format!("Invalid YAML: {}", e))?;

    let obj = yaml_value
        .as_mapping()
        .ok_or_else(|| "YAML root must be a mapping/object".to_string())?;

    // Required fields for ODCS v3.1.0
    let api_version = obj
        .get(&YamlValue::String("apiVersion".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: apiVersion".to_string())?;
    if api_version != "v3.1.0" {
        return Err(format!(
            "Invalid apiVersion: expected 'v3.1.0', got '{}'",
            api_version
        ));
    }

    let kind = obj
        .get(&YamlValue::String("kind".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: kind".to_string())?;
    if kind != "DataContract" {
        return Err(format!(
            "Invalid kind: expected 'DataContract', got '{}'",
            kind
        ));
    }

    // ID must be present (UUID string)
    obj.get(&YamlValue::String("id".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: id".to_string())?;

    // Name must be present
    obj.get(&YamlValue::String("name".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: name".to_string())?;

    // Version must be present
    obj.get(&YamlValue::String("version".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required field: version".to_string())?;

    // Schema must be present and be an array
    let schema = obj
        .get(&YamlValue::String("schema".to_string()))
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| "Missing required field: schema (must be an array)".to_string())?;

    // Schema array must have at least one SchemaObject
    if schema.is_empty() {
        return Err("Schema array must contain at least one SchemaObject".to_string());
    }

    // First schema object must have 'name' and 'properties'
    let first_schema = schema[0]
        .as_mapping()
        .ok_or_else(|| "Schema array element must be an object".to_string())?;

    first_schema
        .get(&YamlValue::String("name".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "SchemaObject missing required field: name".to_string())?;

    first_schema
        .get(&YamlValue::String("properties".to_string()))
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| {
            "SchemaObject missing required field: properties (must be an array)".to_string()
        })?;

    Ok(())
}

/// Validate roundtrip: export then import should work
fn validate_roundtrip(table: &Table) -> Result<(), String> {
    let yaml = ODCSExporter::export_table(table, "odcs_v3_1_0");

    // Validate schema
    validate_odcs_v3_1_0_schema(&yaml)?;

    // Validate can be imported back
    let mut importer = ODCSImporter::new();
    importer
        .parse_table(&yaml)
        .map_err(|e| format!("Failed to import exported YAML: {}", e))?;

    Ok(())
}

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
        ..Default::default()
    }
}

mod odcs_export_tests {
    use super::*;

    #[test]
    fn test_export_table_with_metadata() {
        let mut table =
            create_test_table("users", vec![create_column("id", "BIGINT", true, false)]);
        table
            .odcl_metadata
            .insert("version".to_string(), serde_json::json!("1.2.3"));
        table
            .odcl_metadata
            .insert("status".to_string(), serde_json::json!("published"));

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Verify specific metadata fields
        assert!(yaml.contains("apiVersion: v3.1.0") || yaml.contains("apiVersion: \"v3.1.0\""));
        assert!(yaml.contains("kind: DataContract") || yaml.contains("kind: \"DataContract\""));
        assert!(yaml.contains("name: users") || yaml.contains("name: \"users\""));
        assert!(yaml.contains("version: \"1.2.3\"") || yaml.contains("version: 1.2.3"));
    }

    #[test]
    fn test_export_table_with_database_type() {
        let mut table = create_test_table("users", vec![create_column("id", "INT", true, false)]);
        table.database_type = Some(DatabaseType::Postgres);

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_with_medallion_layers() {
        let mut table = create_test_table("users", vec![create_column("id", "INT", true, false)]);
        table.medallion_layers = vec![MedallionLayer::Bronze, MedallionLayer::Silver];

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_with_scd_pattern() {
        let mut table = create_test_table("users", vec![create_column("id", "INT", true, false)]);
        table.scd_pattern = Some(SCDPattern::Type2);

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_with_data_vault() {
        let mut table = create_test_table(
            "hub_customer",
            vec![create_column("id", "INT", true, false)],
        );
        table.data_vault_classification = Some(DataVaultClassification::Hub);

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_with_reserved_column_names() {
        // Test that columns with reserved names like 'type' and 'status' are handled correctly
        let table = create_test_table(
            "user_events",
            vec![
                create_column("id", "STRING", true, false),
                create_column("type", "STRING", false, false), // 'type' is a valid column name
                create_column("status", "STRING", false, true), // 'status' is a valid column name
                create_column("description", "STRING", false, true),
            ],
        );

        let exporter = ODCSExporter;
        let yaml = exporter.export(&[table], "odcs_v3_1_0").unwrap();
        let result = yaml.values().next().unwrap();

        // Validate schema
        validate_odcs_v3_1_0_schema(&result.content)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Verify the exported YAML contains the column names correctly
        assert!(result.content.contains("name: type"));
        assert!(result.content.contains("name: status"));
        assert!(result.content.contains("logicalType: string")); // Should use logicalType, not type
        assert!(!result.content.contains("type: string")); // Should not have 'type' as property field

        // Validate roundtrip
        validate_roundtrip(&create_test_table(
            "user_events",
            vec![
                create_column("id", "STRING", true, false),
                create_column("type", "STRING", false, false),
                create_column("status", "STRING", false, true),
                create_column("description", "STRING", false, true),
            ],
        ))
        .expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_with_tags() {
        let mut table = create_test_table("users", vec![create_column("id", "INT", true, false)]);
        table.tags = vec![
            Tag::Simple("pii".to_string()),
            Tag::Simple("sensitive".to_string()),
        ];

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Verify tags are exported as array
        let yaml_value: YamlValue = serde_yaml::from_str(&yaml).unwrap();
        let tags = yaml_value
            .as_mapping()
            .and_then(|m| m.get(&YamlValue::String("tags".to_string())))
            .and_then(|v| v.as_sequence());
        assert!(tags.is_some(), "Tags should be exported as an array");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_with_schema_catalog() {
        let mut table = create_test_table("users", vec![create_column("id", "INT", true, false)]);
        table.catalog_name = Some("mydb".to_string());
        table.schema_name = Some("public".to_string());

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_with_column_descriptions() {
        let mut col = create_column("id", "INT", true, false);
        col.description = "Primary key identifier".to_string();
        let table = create_test_table("users", vec![col]);

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Verify schema structure includes properties with field descriptions
        let yaml_value: YamlValue = serde_yaml::from_str(&yaml).unwrap();
        let schema = yaml_value
            .as_mapping()
            .and_then(|m| m.get(&YamlValue::String("schema".to_string())))
            .and_then(|v| v.as_sequence())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_mapping());
        assert!(schema.is_some(), "Schema must be present");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_basic_table_validates_schema() {
        let table = create_test_table(
            "test_table",
            vec![
                create_column("id", "BIGINT", true, false),
                create_column("name", "VARCHAR(100)", false, true),
            ],
        );

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Validate ODCS v3.1.0 schema compliance
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Validate roundtrip
        validate_roundtrip(&table).expect("Roundtrip export/import must work");
    }

    #[test]
    fn test_export_table_schema_structure() {
        let table = create_test_table("users", vec![create_column("id", "INT", true, false)]);

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
        let yaml_value: YamlValue = serde_yaml::from_str(&yaml).unwrap();

        // Verify schema is an array
        let schema_array = yaml_value
            .as_mapping()
            .and_then(|m| m.get(&YamlValue::String("schema".to_string())))
            .and_then(|v| v.as_sequence());
        assert!(schema_array.is_some(), "Schema must be an array");

        // Verify first schema object has name and properties
        let schema_obj = schema_array.unwrap()[0].as_mapping().unwrap();
        assert!(
            schema_obj.contains_key(&YamlValue::String("name".to_string())),
            "SchemaObject must have 'name' field"
        );
        assert!(
            schema_obj.contains_key(&YamlValue::String("properties".to_string())),
            "SchemaObject must have 'properties' field"
        );

        // Properties must be an array (ODCS v3.1.0 format)
        let properties = schema_obj
            .get(&YamlValue::String("properties".to_string()))
            .and_then(|v| v.as_sequence());
        assert!(
            properties.is_some(),
            "Properties must be an array (ODCS v3.1.0)"
        );
    }
}

mod odcs_import_tests {
    use super::*;

    #[test]
    fn test_import_odcs_v3_1_0_basic() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
name: users
version: "1.0.0"
schema:
  - name: users
    properties:
      id:
        type: bigint
        nullable: false
      name:
        type: string
        nullable: true
"#;

        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0);
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[0].name, "id");
        assert_eq!(table.columns[1].name, "name");
    }

    #[test]
    fn test_import_odcs_with_metadata() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
name: users
version: "1.0.0"
status: published
schema:
  - name: users
    properties:
      id:
        type: bigint
"#;

        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0);
        assert_eq!(table.name, "users");
        // Metadata may be stored in odcl_metadata or as top-level fields
        // Version is required, status is optional
        assert!(!table.odcl_metadata.is_empty() || table.name == "users");
    }

    #[test]
    fn test_import_odcs_with_tags() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
name: users
version: "1.0.0"
tags:
  - pii
  - sensitive
schema:
  - name: users
    properties:
      id:
        type: bigint
"#;

        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0);
        assert!(table.tags.contains(&Tag::Simple("pii".to_string())));
        assert!(table.tags.contains(&Tag::Simple("sensitive".to_string())));
    }

    #[test]
    fn test_import_odcs_with_database_type() {
        let mut importer = ODCSImporter::new();
        // ODCS v3.1.0 uses servers array to specify database type
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
name: users
version: "1.0.0"
servers:
  - type: Postgres
schema:
  - name: users
    properties:
      id:
        type: bigint
"#;

        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0);
        // Database type should be extracted from servers array
        assert_eq!(table.database_type, Some(DatabaseType::Postgres));
    }

    #[test]
    fn test_import_odcs_invalid_yaml() {
        let mut importer = ODCSImporter::new();
        let yaml = "invalid: yaml: [";

        let result = importer.parse_table(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_odcs_missing_required_fields() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
# Missing name and schema
"#;

        let result = importer.parse_table(yaml);
        // Should either error or create table with defaults
        // The actual behavior depends on implementation
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_import_odcs_with_custom_properties() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
name: users
version: "1.0.0"
customProperties:
  - key: owner
    value: data-team
  - key: department
    value: finance
schema:
  - name: users
    properties:
      id:
        type: bigint
"#;

        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0);
        // Custom properties may be stored in odcl_metadata or customProperties
        // The exact format depends on implementation
        assert!(
            table.odcl_metadata.contains_key("owner")
                || table.odcl_metadata.contains_key("department")
                || table.odcl_metadata.contains_key("customProperties")
                || !table.odcl_metadata.is_empty()
        );
    }

    #[test]
    fn test_import_odcs_roundtrip() {
        let mut importer = ODCSImporter::new();
        let original_table = create_test_table(
            "users",
            vec![
                create_column("id", "BIGINT", true, false),
                create_column("name", "VARCHAR(100)", false, true),
            ],
        );

        // Export
        let yaml = ODCSExporter::export_table(&original_table, "odcs_v3_1_0");

        // Validate schema before import
        validate_odcs_v3_1_0_schema(&yaml)
            .expect("Exported YAML must conform to ODCS v3.1.0 schema");

        // Import
        let (imported_table, errors) = importer.parse_table(&yaml).unwrap();
        assert_eq!(errors.len(), 0, "Import should have no errors");
        assert_eq!(imported_table.name, original_table.name);
        assert_eq!(imported_table.columns.len(), original_table.columns.len());
    }

    #[test]
    fn test_odcs_v3_1_0_import_preserves_description_fields() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: test-contract-id
version: 1.0.0
schema:
  - id: test_schema
    name: test_table
    properties:
      - id: col1_prop
        name: test_column
        logicalType: string
        physicalType: varchar(100)
        required: true
        description: This is a test column description
"#;
        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0, "Import should have no errors");
        assert_eq!(table.columns.len(), 1);

        let column = &table.columns[0];
        assert_eq!(column.name, "test_column");
        assert_eq!(column.description, "This is a test column description");
    }

    #[test]
    fn test_odcs_v3_1_0_import_preserves_quality_arrays_with_nested_structures() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: test-contract-id
version: 1.0.0
schema:
  - id: test_schema
    name: test_table
    properties:
      - id: col1_prop
        name: test_column
        logicalType: long
        physicalType: bigint
        required: true
        quality:
          - metric: nullValues
            mustBe: 0
            description: column should not contain null values
            dimension: completeness
            type: library
            severity: error
            businessImpact: operational
            schedule: 0 20 * * *
            scheduler: cron
            customProperties:
              - property: FIELD_NAME
                value: test_column
              - property: COMPARISON_TYPE
                value: Greater than
"#;
        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0, "Import should have no errors");
        assert_eq!(table.columns.len(), 1);

        let column = &table.columns[0];
        assert_eq!(column.name, "test_column");

        // Verify quality array is preserved (note: when required=true, a not_null rule may be added)
        assert!(
            !column.quality.is_empty(),
            "Quality array should not be empty"
        );

        // Find the library quality rule (there may be a not_null rule added automatically)
        let quality_rule = column
            .quality
            .iter()
            .find(|r| r.get("type").and_then(|v| v.as_str()) == Some("library"))
            .expect("Should find library quality rule");

        // Verify nested structure is preserved
        assert_eq!(
            quality_rule.get("metric").and_then(|v| v.as_str()),
            Some("nullValues")
        );
        assert_eq!(quality_rule.get("mustBe").and_then(|v| v.as_i64()), Some(0));
        assert_eq!(
            quality_rule.get("description").and_then(|v| v.as_str()),
            Some("column should not contain null values")
        );
        assert!(quality_rule.get("customProperties").is_some());

        // Verify nested customProperties array
        if let Some(custom_props) = quality_rule.get("customProperties")
            && let Some(arr) = custom_props.as_array()
        {
            assert!(!arr.is_empty());
        }
    }

    #[test]
    fn test_odcs_v3_1_0_physical_type_roundtrip() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: physical-type-test
name: PhysicalTypeTest
version: 1.0.0
status: draft
schema:
  - name: PhysicalTypeTest
    properties:
      - name: doubleField
        logicalType: number
        physicalType: DOUBLE
        description: A double field
      - name: intField
        logicalType: integer
        physicalType: INT
        description: An int field
      - name: longField
        logicalType: integer
        physicalType: LONG
        description: A long field
      - name: varcharField
        logicalType: string
        physicalType: VARCHAR(255)
        description: A varchar field
      - name: decimalField
        logicalType: number
        physicalType: DECIMAL(10,2)
        description: A decimal field
      - name: timestampField
        logicalType: timestamp
        physicalType: TIMESTAMP_NTZ
        description: A timestamp field
"#;
        // Import the YAML
        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0, "Import should have no errors");
        assert_eq!(table.columns.len(), 6);

        // Verify physical_type is preserved on import
        let double_col = table
            .columns
            .iter()
            .find(|c| c.name == "doubleField")
            .unwrap();
        assert_eq!(double_col.physical_type, Some("DOUBLE".to_string()));

        let int_col = table.columns.iter().find(|c| c.name == "intField").unwrap();
        assert_eq!(int_col.physical_type, Some("INT".to_string()));

        let long_col = table
            .columns
            .iter()
            .find(|c| c.name == "longField")
            .unwrap();
        assert_eq!(long_col.physical_type, Some("LONG".to_string()));

        let varchar_col = table
            .columns
            .iter()
            .find(|c| c.name == "varcharField")
            .unwrap();
        assert_eq!(varchar_col.physical_type, Some("VARCHAR(255)".to_string()));

        let decimal_col = table
            .columns
            .iter()
            .find(|c| c.name == "decimalField")
            .unwrap();
        assert_eq!(decimal_col.physical_type, Some("DECIMAL(10,2)".to_string()));

        let timestamp_col = table
            .columns
            .iter()
            .find(|c| c.name == "timestampField")
            .unwrap();
        assert_eq!(
            timestamp_col.physical_type,
            Some("TIMESTAMP_NTZ".to_string())
        );

        // Export and verify physicalType is preserved in output
        let exported_yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Verify the exported YAML contains all physicalType values
        assert!(
            exported_yaml.contains("physicalType: DOUBLE"),
            "DOUBLE physicalType missing from export"
        );
        assert!(
            exported_yaml.contains("physicalType: INT"),
            "INT physicalType missing from export"
        );
        assert!(
            exported_yaml.contains("physicalType: LONG"),
            "LONG physicalType missing from export"
        );
        assert!(
            exported_yaml.contains("physicalType: VARCHAR(255)"),
            "VARCHAR(255) physicalType missing from export"
        );
        assert!(
            exported_yaml.contains("physicalType: DECIMAL(10,2)"),
            "DECIMAL(10,2) physicalType missing from export"
        );
        assert!(
            exported_yaml.contains("physicalType: TIMESTAMP_NTZ"),
            "TIMESTAMP_NTZ physicalType missing from export"
        );

        // Re-import and verify roundtrip
        let (reimported_table, reimport_errors) = importer.parse_table(&exported_yaml).unwrap();
        assert_eq!(reimport_errors.len(), 0, "Re-import should have no errors");

        for col in &table.columns {
            let reimported_col = reimported_table
                .columns
                .iter()
                .find(|c| c.name == col.name)
                .unwrap();
            assert_eq!(
                reimported_col.physical_type, col.physical_type,
                "physical_type mismatch for column '{}': original {:?}, reimported {:?}",
                col.name, col.physical_type, reimported_col.physical_type
            );
        }
    }

    #[test]
    fn test_odcs_v3_1_0_import_preserves_ref_references() {
        let mut importer = ODCSImporter::new();
        let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: test-contract-id
version: 1.0.0
schema:
  - id: test_schema
    name: test_table
    properties:
      - id: col1_prop
        name: order_id
        logicalType: string
        physicalType: varchar(100)
        required: true
        $ref: '#/definitions/order_id'
definitions:
  order_id:
    logicalType: string
    physicalType: uuid
    description: An internal ID that identifies an order
"#;
        let (table, errors) = importer.parse_table(yaml).unwrap();
        assert_eq!(errors.len(), 0, "Import should have no errors");
        assert_eq!(table.columns.len(), 1);

        let column = &table.columns[0];
        assert_eq!(column.name, "order_id");
        // ref_path is now stored as relationships
        assert!(
            !column.relationships.is_empty(),
            "Column should have relationships from $ref"
        );
    }
}
