//! Comprehensive tests for models module

use data_modelling_sdk::models::cross_domain::{
    CrossDomainConfig, CrossDomainRelationshipRef, CrossDomainTableRef,
};
use data_modelling_sdk::models::enums::{
    Cardinality, DataVaultClassification, DatabaseType, InfrastructureType, MedallionLayer,
    ModelingLevel, RelationshipType, SCDPattern,
};
use data_modelling_sdk::models::relationship::{
    ConnectionPoint, ETLJobMetadata, ForeignKeyDetails, VisualMetadata,
};
use data_modelling_sdk::models::{
    Column, ContactDetails, DataModel, ForeignKey, Position, Relationship, SlaProperty, Table, Tag,
};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

mod column_tests {
    use super::*;

    #[test]
    fn test_column_new() {
        let col = Column::new("id".to_string(), "INT".to_string());
        assert_eq!(col.name, "id");
        assert_eq!(col.data_type, "INT");
        assert!(col.nullable);
        assert!(!col.primary_key);
        assert!(!col.secondary_key);
        assert_eq!(col.column_order, 0);
    }

    #[test]
    fn test_column_with_foreign_key() {
        let mut col = Column::new("user_id".to_string(), "BIGINT".to_string());
        col.foreign_key = Some(ForeignKey {
            table_id: Uuid::new_v4().to_string(),
            column_name: "id".to_string(),
        });
        assert!(col.foreign_key.is_some());
    }

    #[test]
    fn test_column_with_composite_key() {
        let mut col = Column::new("key1".to_string(), "INT".to_string());
        col.composite_key = Some("composite_pk".to_string());
        assert_eq!(col.composite_key, Some("composite_pk".to_string()));
    }

    #[test]
    fn test_column_with_constraints() {
        let mut col = Column::new("email".to_string(), "VARCHAR(255)".to_string());
        col.constraints.push("UNIQUE".to_string());
        col.constraints.push("CHECK (email LIKE '%@%')".to_string());
        assert_eq!(col.constraints.len(), 2);
    }

    #[test]
    fn test_column_with_enum_values() {
        let mut col = Column::new("status".to_string(), "ENUM".to_string());
        col.enum_values = vec![
            "active".to_string(),
            "inactive".to_string(),
            "pending".to_string(),
        ];
        assert_eq!(col.enum_values.len(), 3);
    }

    #[test]
    fn test_column_serialization() {
        let col = Column::new("name".to_string(), "STRING".to_string());
        let json = serde_json::to_string(&col).unwrap();
        let parsed: Column = serde_json::from_str(&json).unwrap();
        assert_eq!(col.name, parsed.name);
        assert_eq!(col.data_type, parsed.data_type);
    }

    #[test]
    fn test_column_data_type_normalization() {
        let col1 = Column::new("id".to_string(), "int".to_string());
        assert_eq!(col1.data_type, "INT");

        let col2 = Column::new("name".to_string(), "varchar(100)".to_string());
        assert_eq!(col2.data_type, "VARCHAR(100)");

        let col3 = Column::new("data".to_string(), "struct<field:string>".to_string());
        assert_eq!(col3.data_type, "STRUCT<field:string>");

        let col4 = Column::new("tags".to_string(), "array<string>".to_string());
        assert_eq!(col4.data_type, "ARRAY<string>");
    }

    #[test]
    fn test_column_with_quality_and_errors() {
        let mut col = Column::new("score".to_string(), "INT".to_string());
        let mut quality = HashMap::new();
        quality.insert("min".to_string(), json!(0));
        quality.insert("max".to_string(), json!(100));
        col.quality.push(quality);

        let mut error = HashMap::new();
        error.insert("type".to_string(), json!("validation_error"));
        error.insert("message".to_string(), json!("Value out of range"));
        col.errors.push(error);

        assert_eq!(col.quality.len(), 1);
        assert_eq!(col.errors.len(), 1);
    }
}

mod table_tests {
    use super::*;

    #[test]
    fn test_table_new() {
        let table = Table::new("users".to_string(), vec![]);
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 0);
        assert!(!table.id.is_nil());
    }

    #[test]
    fn test_table_with_columns() {
        let columns = vec![
            Column::new("id".to_string(), "INT".to_string()),
            Column::new("name".to_string(), "VARCHAR(100)".to_string()),
        ];
        let table = Table::new("users".to_string(), columns);
        assert_eq!(table.columns.len(), 2);
    }

    #[test]
    fn test_table_get_unique_key() {
        let mut table = Table::new("users".to_string(), vec![]);
        table.database_type = Some(DatabaseType::Postgres);
        table.schema_name = Some("public".to_string());

        let key = table.get_unique_key();
        assert_eq!(key.0, Some("Postgres".to_string()));
        assert_eq!(key.1, "users".to_string());
        assert_eq!(key.2, None);
        assert_eq!(key.3, Some("public".to_string()));
    }

    #[test]
    fn test_table_with_medallion_layers() {
        let mut table = Table::new("bronze_table".to_string(), vec![]);
        table.medallion_layers.push(MedallionLayer::Bronze);
        table.medallion_layers.push(MedallionLayer::Silver);
        assert_eq!(table.medallion_layers.len(), 2);
    }

    #[test]
    fn test_table_with_scd_pattern() {
        let mut table = Table::new("customer_history".to_string(), vec![]);
        table.scd_pattern = Some(SCDPattern::Type2);
        assert_eq!(table.scd_pattern, Some(SCDPattern::Type2));
    }

    #[test]
    fn test_table_with_data_vault() {
        let mut table = Table::new("customer_hub".to_string(), vec![]);
        table.data_vault_classification = Some(DataVaultClassification::Hub);
        assert_eq!(
            table.data_vault_classification,
            Some(DataVaultClassification::Hub)
        );
    }

    #[test]
    fn test_table_with_position() {
        let mut table = Table::new("users".to_string(), vec![]);
        table.position = Some(Position { x: 100.0, y: 200.0 });
        let pos = table.position.as_ref().unwrap();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 200.0);
    }

    #[test]
    fn test_table_with_tags() {
        let mut table = Table::new("users".to_string(), vec![]);
        table.tags.push(Tag::Simple("pii".to_string()));
        table.tags.push(Tag::Simple("customer_data".to_string()));
        assert_eq!(table.tags.len(), 2);
    }

    #[test]
    fn test_table_serialization() {
        let table = Table::new(
            "users".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        let json = serde_json::to_string(&table).unwrap();
        let parsed: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(table.name, parsed.name);
        assert_eq!(table.columns.len(), parsed.columns.len());
    }

    #[test]
    fn test_table_generate_id() {
        let id1 = Table::generate_id("test", None, None, None);
        let id2 = Table::generate_id("test", None, None, None);
        // UUIDv4 should be different each time
        assert_ne!(id1, id2);
        assert!(!id1.is_nil());
        assert!(!id2.is_nil());
    }

    #[test]
    fn test_table_with_modeling_level() {
        let mut table = Table::new("conceptual_model".to_string(), vec![]);
        table.modeling_level = Some(ModelingLevel::Conceptual);
        assert_eq!(table.modeling_level, Some(ModelingLevel::Conceptual));
    }

    #[test]
    fn test_table_with_odcl_metadata() {
        let mut table = Table::new("legacy_table".to_string(), vec![]);
        table
            .odcl_metadata
            .insert("version".to_string(), json!("1.0"));
        table
            .odcl_metadata
            .insert("author".to_string(), json!("test"));
        assert_eq!(table.odcl_metadata.len(), 2);
    }
}

mod relationship_tests {
    use super::*;

    #[test]
    fn test_relationship_new() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let rel = Relationship::new(source_id, target_id);
        assert_eq!(rel.source_table_id, source_id);
        assert_eq!(rel.target_table_id, target_id);
        assert!(!rel.id.is_nil());
    }

    #[test]
    fn test_relationship_with_cardinality() {
        let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel.cardinality = Some(Cardinality::OneToMany);
        assert_eq!(rel.cardinality, Some(Cardinality::OneToMany));
    }

    #[test]
    fn test_relationship_with_foreign_key_details() {
        let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel.foreign_key_details = Some(ForeignKeyDetails {
            source_column: "user_id".to_string(),
            target_column: "id".to_string(),
        });
        assert!(rel.foreign_key_details.is_some());
        assert_eq!(
            rel.foreign_key_details.as_ref().unwrap().source_column,
            "user_id"
        );
    }

    #[test]
    fn test_relationship_with_etl_job_metadata() {
        let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel.etl_job_metadata = Some(ETLJobMetadata {
            job_name: "daily_sync".to_string(),
            notes: Some("Syncs customer data daily".to_string()),
            frequency: Some("daily".to_string()),
        });
        assert!(rel.etl_job_metadata.is_some());
        assert_eq!(
            rel.etl_job_metadata.as_ref().unwrap().job_name,
            "daily_sync"
        );
    }

    #[test]
    fn test_relationship_with_visual_metadata() {
        let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel.visual_metadata = Some(VisualMetadata {
            source_connection_point: Some("top".to_string()),
            target_connection_point: Some("bottom".to_string()),
            routing_waypoints: vec![
                ConnectionPoint { x: 50.0, y: 100.0 },
                ConnectionPoint { x: 150.0, y: 100.0 },
            ],
            label_position: Some(ConnectionPoint { x: 100.0, y: 100.0 }),
        });
        assert!(rel.visual_metadata.is_some());
        assert_eq!(
            rel.visual_metadata
                .as_ref()
                .unwrap()
                .routing_waypoints
                .len(),
            2
        );
    }

    #[test]
    fn test_relationship_with_relationship_type() {
        let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel.relationship_type = Some(RelationshipType::DataFlow);
        assert_eq!(rel.relationship_type, Some(RelationshipType::DataFlow));
    }

    #[test]
    fn test_relationship_serialization() {
        let rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        let json = serde_json::to_string(&rel).unwrap();
        let parsed: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(rel.source_table_id, parsed.source_table_id);
        assert_eq!(rel.target_table_id, parsed.target_table_id);
    }

    #[test]
    fn test_relationship_generate_id() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let id1 = Relationship::generate_id(source_id, target_id);
        let id2 = Relationship::generate_id(source_id, target_id);
        // UUIDv4 should be different each time
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_relationship_with_optional_flags() {
        let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel.source_optional = Some(true);
        rel.target_optional = Some(false);
        assert_eq!(rel.source_optional, Some(true));
        assert_eq!(rel.target_optional, Some(false));
    }

    #[test]
    fn test_relationship_with_notes() {
        let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel.notes = Some("This relationship represents customer orders".to_string());
        assert!(rel.notes.is_some());
    }
}

mod data_model_tests {
    use super::*;

    #[test]
    fn test_data_model_new() {
        let model = DataModel::new(
            "MyModel".to_string(),
            "/path/to/git".to_string(),
            "control.yaml".to_string(),
        );
        assert_eq!(model.name, "MyModel");
        assert_eq!(model.git_directory_path, "/path/to/git");
        assert_eq!(model.control_file_path, "control.yaml");
        assert_eq!(model.tables.len(), 0);
        assert_eq!(model.relationships.len(), 0);
    }

    #[test]
    fn test_data_model_get_table_by_id() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let table = Table::new("users".to_string(), vec![]);
        let table_id = table.id;
        model.tables.push(table);

        let found = model.get_table_by_id(table_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "users");
    }

    #[test]
    fn test_data_model_get_table_by_id_not_found() {
        let model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let not_found = model.get_table_by_id(Uuid::new_v4());
        assert!(not_found.is_none());
    }

    #[test]
    fn test_data_model_get_table_by_id_mut() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let table = Table::new("users".to_string(), vec![]);
        let table_id = table.id;
        model.tables.push(table);

        let found = model.get_table_by_id_mut(table_id);
        assert!(found.is_some());
        found.unwrap().name = "updated_users".to_string();
        assert_eq!(model.tables[0].name, "updated_users");
    }

    #[test]
    fn test_data_model_get_table_by_name() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        model.tables.push(Table::new("users".to_string(), vec![]));
        model.tables.push(Table::new("orders".to_string(), vec![]));

        let found = model.get_table_by_name("users");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "users");
    }

    #[test]
    fn test_data_model_get_table_by_unique_key() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let mut table = Table::new("users".to_string(), vec![]);
        table.database_type = Some(DatabaseType::Postgres);
        table.schema_name = Some("public".to_string());
        model.tables.push(table);

        let found = model.get_table_by_unique_key(Some("Postgres"), "users", None, Some("public"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "users");
    }

    #[test]
    fn test_data_model_get_relationships_for_table() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let table_a = Table::new("users".to_string(), vec![]);
        let table_b = Table::new("orders".to_string(), vec![]);
        let table_c = Table::new("products".to_string(), vec![]);
        let table_a_id = table_a.id;
        let table_b_id = table_b.id;
        let table_c_id = table_c.id;

        model.tables.push(table_a);
        model.tables.push(table_b);
        model.tables.push(table_c);

        model
            .relationships
            .push(Relationship::new(table_a_id, table_b_id));
        model
            .relationships
            .push(Relationship::new(table_b_id, table_c_id));
        model
            .relationships
            .push(Relationship::new(table_c_id, table_a_id));

        let rels = model.get_relationships_for_table(table_a_id);
        assert_eq!(rels.len(), 2); // a->b and c->a
    }

    #[test]
    fn test_data_model_serialization() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        model.tables.push(Table::new("users".to_string(), vec![]));
        model.description = Some("Test model".to_string());

        let json = serde_json::to_string(&model).unwrap();
        let parsed: DataModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model.name, parsed.name);
        assert_eq!(model.tables.len(), parsed.tables.len());
    }

    #[test]
    fn test_data_model_with_subfolder() {
        let mut model = DataModel::new(
            "submodel".to_string(),
            "/path/sub".to_string(),
            "control.yaml".to_string(),
        );
        model.is_subfolder = true;
        model.parent_git_directory = Some("/path".to_string());
        assert!(model.is_subfolder);
        assert_eq!(model.parent_git_directory, Some("/path".to_string()));
    }

    #[test]
    fn test_data_model_id_deterministic() {
        let model1 = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let model2 = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        // UUIDv5 should be deterministic
        assert_eq!(model1.id, model2.id);
    }
}

mod enums_tests {
    use super::*;

    #[test]
    fn test_database_type_serialization() {
        let db_type = DatabaseType::Postgres;
        let json = serde_json::to_string(&db_type).unwrap();
        assert_eq!(json, "\"POSTGRES\"");
        let parsed: DatabaseType = serde_json::from_str(&json).unwrap();
        assert_eq!(db_type, parsed);
    }

    #[test]
    fn test_medallion_layer_serialization() {
        let layer = MedallionLayer::Bronze;
        let json = serde_json::to_string(&layer).unwrap();
        assert_eq!(json, "\"bronze\"");
        let parsed: MedallionLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(layer, parsed);
    }

    #[test]
    fn test_scd_pattern_serialization() {
        let pattern = SCDPattern::Type2;
        let json = serde_json::to_string(&pattern).unwrap();
        assert_eq!(json, "\"TYPE2\"");
        let parsed: SCDPattern = serde_json::from_str(&json).unwrap();
        assert_eq!(pattern, parsed);
    }

    #[test]
    fn test_data_vault_classification_serialization() {
        let classification = DataVaultClassification::Hub;
        let json = serde_json::to_string(&classification).unwrap();
        let parsed: DataVaultClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(classification, parsed);
    }

    #[test]
    fn test_modeling_level_serialization() {
        let level = ModelingLevel::Physical;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"physical\"");
        let parsed: ModelingLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed);
    }

    #[test]
    fn test_cardinality_serialization() {
        let card = Cardinality::OneToMany;
        let json = serde_json::to_string(&card).unwrap();
        assert_eq!(json, "\"oneToMany\"");
        let parsed: Cardinality = serde_json::from_str(&json).unwrap();
        assert_eq!(card, parsed);
    }

    #[test]
    fn test_relationship_type_serialization() {
        let rel_type = RelationshipType::DataFlow;
        let json = serde_json::to_string(&rel_type).unwrap();
        assert_eq!(json, "\"dataFlow\"");
        let parsed: RelationshipType = serde_json::from_str(&json).unwrap();
        assert_eq!(rel_type, parsed);
    }

    #[test]
    fn test_all_database_types() {
        let types = vec![
            DatabaseType::DatabricksDelta,
            DatabaseType::DatabricksIceberg,
            DatabaseType::AwsGlue,
            DatabaseType::DatabricksLakebase,
            DatabaseType::Postgres,
            DatabaseType::Mysql,
            DatabaseType::SqlServer,
            DatabaseType::Dynamodb,
            DatabaseType::Cassandra,
            DatabaseType::Kafka,
            DatabaseType::Pulsar,
        ];
        for db_type in types {
            let json = serde_json::to_string(&db_type).unwrap();
            let parsed: DatabaseType = serde_json::from_str(&json).unwrap();
            assert_eq!(db_type, parsed);
        }
    }

    #[test]
    fn test_all_medallion_layers() {
        let layers = vec![
            MedallionLayer::Bronze,
            MedallionLayer::Silver,
            MedallionLayer::Gold,
            MedallionLayer::Operational,
        ];
        for layer in layers {
            let json = serde_json::to_string(&layer).unwrap();
            let parsed: MedallionLayer = serde_json::from_str(&json).unwrap();
            assert_eq!(layer, parsed);
        }
    }
}

mod cross_domain_tests {
    use super::*;

    #[test]
    fn test_cross_domain_table_ref_new() {
        let table_id = Uuid::new_v4();
        let ref_entry = CrossDomainTableRef::new("finance".to_string(), table_id);
        assert_eq!(ref_entry.source_domain, "finance");
        assert_eq!(ref_entry.table_id, table_id);
        assert!(!ref_entry.id.is_nil());
    }

    #[test]
    fn test_cross_domain_relationship_ref_new() {
        let rel_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let ref_entry =
            CrossDomainRelationshipRef::new("finance".to_string(), rel_id, source_id, target_id);
        assert_eq!(ref_entry.source_domain, "finance");
        assert_eq!(ref_entry.relationship_id, rel_id);
        assert_eq!(ref_entry.source_table_id, source_id);
        assert_eq!(ref_entry.target_table_id, target_id);
    }

    #[test]
    fn test_cross_domain_config_new() {
        let config = CrossDomainConfig::new();
        // Default::default() doesn't call default_schema_version(), so it's empty
        // The default_schema_version() is only used during deserialization
        assert_eq!(config.schema_version, "");
        assert_eq!(config.imported_tables.len(), 0);
        assert_eq!(config.imported_relationships.len(), 0);
    }

    #[test]
    fn test_cross_domain_config_add_table_ref() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        let idx = config.add_table_ref("finance".to_string(), table_id);
        assert_eq!(idx, 0);
        assert_eq!(config.imported_tables.len(), 1);
    }

    #[test]
    fn test_cross_domain_config_get_table_ref() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        config.add_table_ref("finance".to_string(), table_id);
        let ref_entry = config.get_table_ref(0);
        assert!(ref_entry.is_some());
        assert_eq!(ref_entry.unwrap().table_id, table_id);
    }

    #[test]
    fn test_cross_domain_config_remove_table_ref() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        config.add_table_ref("finance".to_string(), table_id);
        assert_eq!(config.imported_tables.len(), 1);
        let removed = config.remove_table_ref(table_id);
        assert!(removed);
        assert_eq!(config.imported_tables.len(), 0);
    }

    #[test]
    fn test_cross_domain_config_add_relationship_ref() {
        let mut config = CrossDomainConfig::new();
        let rel_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let idx = config.add_relationship_ref("finance".to_string(), rel_id, source_id, target_id);
        assert_eq!(idx, 0);
        assert_eq!(config.imported_relationships.len(), 1);
    }

    #[test]
    fn test_cross_domain_config_get_relationship_ref() {
        let mut config = CrossDomainConfig::new();
        let rel_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        config.add_relationship_ref("finance".to_string(), rel_id, source_id, target_id);
        let ref_entry = config.get_relationship_ref(0);
        assert!(ref_entry.is_some());
        assert_eq!(ref_entry.unwrap().relationship_id, rel_id);
    }

    #[test]
    fn test_cross_domain_config_remove_relationship_ref() {
        let mut config = CrossDomainConfig::new();
        let rel_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        config.add_relationship_ref("finance".to_string(), rel_id, source_id, target_id);
        assert_eq!(config.imported_relationships.len(), 1);
        let removed = config.remove_relationship_ref(rel_id);
        assert!(removed);
        assert_eq!(config.imported_relationships.len(), 0);
    }

    #[test]
    fn test_cross_domain_config_get_tables_from_domain() {
        let mut config = CrossDomainConfig::new();
        let table1 = Uuid::new_v4();
        let table2 = Uuid::new_v4();
        config.add_table_ref("finance".to_string(), table1);
        config.add_table_ref("risk".to_string(), table2);
        config.add_table_ref("finance".to_string(), Uuid::new_v4());

        let finance_tables = config.get_tables_from_domain("finance");
        assert_eq!(finance_tables.len(), 2);
        assert!(finance_tables.contains(&table1));
    }

    #[test]
    fn test_cross_domain_config_is_table_imported() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        config.add_table_ref("finance".to_string(), table_id);
        assert!(config.is_table_imported(table_id));
        assert!(!config.is_table_imported(Uuid::new_v4()));
    }

    #[test]
    fn test_cross_domain_config_get_table_source_domain() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        config.add_table_ref("finance".to_string(), table_id);
        assert_eq!(config.get_table_source_domain(table_id), Some("finance"));
        assert_eq!(config.get_table_source_domain(Uuid::new_v4()), None);
    }

    #[test]
    fn test_cross_domain_config_remove_table_removes_relationships() {
        let mut config = CrossDomainConfig::new();
        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();
        let rel_id = Uuid::new_v4();
        config.add_table_ref("finance".to_string(), table_a);
        config.add_table_ref("finance".to_string(), table_b);
        config.add_relationship_ref("finance".to_string(), rel_id, table_a, table_b);
        assert_eq!(config.imported_relationships.len(), 1);
        config.remove_table_ref(table_a);
        assert_eq!(config.imported_relationships.len(), 0);
    }

    #[test]
    fn test_cross_domain_table_ref_with_alias() {
        let mut ref_entry = CrossDomainTableRef::new("finance".to_string(), Uuid::new_v4());
        ref_entry.display_alias = Some("FinanceUsers".to_string());
        assert_eq!(ref_entry.display_alias, Some("FinanceUsers".to_string()));
    }

    #[test]
    fn test_cross_domain_table_ref_with_position() {
        let mut ref_entry = CrossDomainTableRef::new("finance".to_string(), Uuid::new_v4());
        ref_entry.position = Some(Position { x: 50.0, y: 100.0 });
        assert_eq!(ref_entry.position.unwrap().x, 50.0);
    }

    #[test]
    fn test_cross_domain_config_duplicate_table_ref() {
        let mut config = CrossDomainConfig::new();
        let table_id = Uuid::new_v4();
        let idx1 = config.add_table_ref("finance".to_string(), table_id);
        let idx2 = config.add_table_ref("finance".to_string(), table_id);
        assert_eq!(idx1, idx2);
        assert_eq!(config.imported_tables.len(), 1);
    }

    #[test]
    fn test_cross_domain_config_duplicate_relationship_ref() {
        let mut config = CrossDomainConfig::new();
        let rel_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let idx1 = config.add_relationship_ref("finance".to_string(), rel_id, source_id, target_id);
        let idx2 = config.add_relationship_ref("finance".to_string(), rel_id, source_id, target_id);
        assert_eq!(idx1, idx2);
        assert_eq!(config.imported_relationships.len(), 1);
    }
}

mod relationship_structs_tests {
    use super::*;

    #[test]
    fn test_foreign_key_details() {
        let fk = ForeignKeyDetails {
            source_column: "user_id".to_string(),
            target_column: "id".to_string(),
        };
        assert_eq!(fk.source_column, "user_id");
        assert_eq!(fk.target_column, "id");
    }

    #[test]
    fn test_etl_job_metadata() {
        let etl = ETLJobMetadata {
            job_name: "daily_sync".to_string(),
            notes: Some("Syncs data daily".to_string()),
            frequency: Some("daily".to_string()),
        };
        assert_eq!(etl.job_name, "daily_sync");
        assert!(etl.notes.is_some());
        assert!(etl.frequency.is_some());
    }

    #[test]
    fn test_connection_point() {
        let point = ConnectionPoint { x: 10.5, y: 20.5 };
        assert_eq!(point.x, 10.5);
        assert_eq!(point.y, 20.5);
    }

    #[test]
    fn test_visual_metadata() {
        let visual = VisualMetadata {
            source_connection_point: Some("top".to_string()),
            target_connection_point: Some("bottom".to_string()),
            routing_waypoints: vec![ConnectionPoint { x: 0.0, y: 0.0 }],
            label_position: Some(ConnectionPoint { x: 50.0, y: 50.0 }),
        };
        assert!(visual.source_connection_point.is_some());
        assert_eq!(visual.routing_waypoints.len(), 1);
    }

    #[test]
    fn test_relationship_structs_serialization() {
        let fk = ForeignKeyDetails {
            source_column: "user_id".to_string(),
            target_column: "id".to_string(),
        };
        let json = serde_json::to_string(&fk).unwrap();
        let parsed: ForeignKeyDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(fk, parsed);
    }
}

mod metadata_tests {
    use super::*;

    #[test]
    fn test_table_with_owner_metadata() {
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table.owner = Some("Data Engineering Team".to_string());
        assert_eq!(table.owner, Some("Data Engineering Team".to_string()));
    }

    #[test]
    fn test_table_with_sla_metadata() {
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        let sla = vec![SlaProperty {
            property: "latency".to_string(),
            value: json!(4),
            unit: "hours".to_string(),
            description: Some("Data must be available within 4 hours".to_string()),
            element: None,
            driver: Some("operational".to_string()),
            scheduler: None,
            schedule: None,
        }];
        table.sla = Some(sla.clone());
        assert_eq!(table.sla, Some(sla));
    }

    #[test]
    fn test_table_with_contact_details_metadata() {
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        let contact = ContactDetails {
            email: Some("team@example.com".to_string()),
            phone: Some("+1-555-0123".to_string()),
            name: Some("Data Team".to_string()),
            role: Some("Data Owner".to_string()),
            other: None,
        };
        table.contact_details = Some(contact.clone());
        assert_eq!(table.contact_details, Some(contact));
    }

    #[test]
    fn test_table_with_infrastructure_type_metadata() {
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table.infrastructure_type = Some(InfrastructureType::Kafka);
        assert_eq!(table.infrastructure_type, Some(InfrastructureType::Kafka));
    }

    #[test]
    fn test_table_with_notes_metadata() {
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table.notes = Some("This is a test table".to_string());
        assert_eq!(table.notes, Some("This is a test table".to_string()));
    }

    #[test]
    fn test_relationship_with_owner_metadata() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        relationship.owner = Some("Data Engineering Team".to_string());
        assert_eq!(
            relationship.owner,
            Some("Data Engineering Team".to_string())
        );
    }

    #[test]
    fn test_relationship_with_sla_metadata() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        let sla = vec![SlaProperty {
            property: "latency".to_string(),
            value: json!(2),
            unit: "hours".to_string(),
            description: Some("Data flow must complete within 2 hours".to_string()),
            element: None,
            driver: Some("operational".to_string()),
            scheduler: None,
            schedule: None,
        }];
        relationship.sla = Some(sla.clone());
        assert_eq!(relationship.sla, Some(sla));
    }

    #[test]
    fn test_relationship_with_contact_details_metadata() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        let contact = ContactDetails {
            email: Some("team@example.com".to_string()),
            phone: None,
            name: Some("Data Team".to_string()),
            role: Some("Data Owner".to_string()),
            other: None,
        };
        relationship.contact_details = Some(contact.clone());
        assert_eq!(relationship.contact_details, Some(contact));
    }

    #[test]
    fn test_relationship_with_infrastructure_type_metadata() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        relationship.infrastructure_type = Some(InfrastructureType::Kafka);
        assert_eq!(
            relationship.infrastructure_type,
            Some(InfrastructureType::Kafka)
        );
    }

    #[test]
    fn test_table_metadata_serialization() {
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table.owner = Some("Team".to_string());
        table.infrastructure_type = Some(InfrastructureType::PostgreSQL);
        table.notes = Some("Test notes".to_string());

        let json = serde_json::to_string(&table).unwrap();
        let parsed: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(table.owner, parsed.owner);
        assert_eq!(table.infrastructure_type, parsed.infrastructure_type);
        assert_eq!(table.notes, parsed.notes);
    }

    #[test]
    fn test_relationship_metadata_serialization() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        relationship.owner = Some("Team".to_string());
        relationship.infrastructure_type = Some(InfrastructureType::Kafka);
        relationship.notes = Some("Test notes".to_string());

        let json = serde_json::to_string(&relationship).unwrap();
        let parsed: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(relationship.owner, parsed.owner);
        assert_eq!(relationship.infrastructure_type, parsed.infrastructure_type);
        assert_eq!(relationship.notes, parsed.notes);
    }

    #[test]
    fn test_table_metadata_update() {
        let mut table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table.owner = Some("Team A".to_string());
        assert_eq!(table.owner, Some("Team A".to_string()));

        // Update owner
        table.owner = Some("Team B".to_string());
        assert_eq!(table.owner, Some("Team B".to_string()));
    }

    #[test]
    fn test_relationship_metadata_update() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut relationship = Relationship::new(source_id, target_id);
        relationship.owner = Some("Team A".to_string());
        assert_eq!(relationship.owner, Some("Team A".to_string()));

        // Update owner
        relationship.owner = Some("Team B".to_string());
        assert_eq!(relationship.owner, Some("Team B".to_string()));
    }

    #[test]
    fn test_backward_compatibility_table_without_metadata() {
        // Test that tables without metadata deserialize correctly
        let table = Table::new(
            "test_table".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        let json = serde_json::to_string(&table).unwrap();
        let parsed: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(table.owner, parsed.owner);
        assert_eq!(table.sla, parsed.sla);
        assert_eq!(table.contact_details, parsed.contact_details);
        assert_eq!(table.infrastructure_type, parsed.infrastructure_type);
        assert_eq!(table.notes, parsed.notes);
    }

    #[test]
    fn test_backward_compatibility_relationship_without_metadata() {
        // Test that relationships without metadata deserialize correctly
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let relationship = Relationship::new(source_id, target_id);
        let json = serde_json::to_string(&relationship).unwrap();
        let parsed: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(relationship.owner, parsed.owner);
        assert_eq!(relationship.sla, parsed.sla);
        assert_eq!(relationship.contact_details, parsed.contact_details);
        assert_eq!(relationship.infrastructure_type, parsed.infrastructure_type);
    }
}

mod filter_tests {
    use super::*;

    #[test]
    fn test_filter_nodes_by_owner() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let mut table1 = Table::new(
            "table1".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table1.owner = Some("Team A".to_string());
        let mut table2 = Table::new(
            "table2".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table2.owner = Some("Team B".to_string());
        model.tables.push(table1);
        model.tables.push(table2);

        let owned = model.filter_nodes_by_owner("Team A");
        assert_eq!(owned.len(), 1);
        assert_eq!(owned[0].name, "table1");
    }

    #[test]
    fn test_filter_relationships_by_owner() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut rel1 = Relationship::new(source_id, target_id);
        rel1.owner = Some("Team A".to_string());
        let mut rel2 = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel2.owner = Some("Team B".to_string());
        model.relationships.push(rel1);
        model.relationships.push(rel2);

        let owned = model.filter_relationships_by_owner("Team A");
        assert_eq!(owned.len(), 1);
    }

    #[test]
    fn test_filter_nodes_by_infrastructure_type() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let mut table1 = Table::new(
            "table1".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table1.infrastructure_type = Some(InfrastructureType::Kafka);
        let mut table2 = Table::new(
            "table2".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table2.infrastructure_type = Some(InfrastructureType::PostgreSQL);
        model.tables.push(table1);
        model.tables.push(table2);

        let kafka_nodes = model.filter_nodes_by_infrastructure_type(InfrastructureType::Kafka);
        assert_eq!(kafka_nodes.len(), 1);
        assert_eq!(kafka_nodes[0].name, "table1");
    }

    #[test]
    fn test_filter_relationships_by_infrastructure_type() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut rel1 = Relationship::new(source_id, target_id);
        rel1.infrastructure_type = Some(InfrastructureType::Kafka);
        let mut rel2 = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
        rel2.infrastructure_type = Some(InfrastructureType::PostgreSQL);
        model.relationships.push(rel1);
        model.relationships.push(rel2);

        let kafka_rels =
            model.filter_relationships_by_infrastructure_type(InfrastructureType::Kafka);
        assert_eq!(kafka_rels.len(), 1);
    }

    #[test]
    fn test_filter_by_tags() {
        let mut model = DataModel::new(
            "test".to_string(),
            "/path".to_string(),
            "control.yaml".to_string(),
        );
        let mut table1 = Table::new(
            "table1".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table1.tags.push(Tag::Simple("production".to_string()));
        let mut table2 = Table::new(
            "table2".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );
        table2.tags.push(Tag::Simple("staging".to_string()));
        model.tables.push(table1);
        model.tables.push(table2);

        let (tagged_nodes, _tagged_relationships) = model.filter_by_tags("production");
        assert_eq!(tagged_nodes.len(), 1);
        assert_eq!(tagged_nodes[0].name, "table1");
    }
}
