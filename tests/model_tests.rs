//! Model loading and saving tests

#[cfg(feature = "native-fs")]
mod model_loader_tests {
    use data_modelling_sdk::model::loader::ModelLoader;
    use data_modelling_sdk::storage::{StorageBackend, filesystem::FileSystemStorageBackend};
    use tempfile::TempDir;
    use tokio::runtime::Runtime;
    use uuid::Uuid;

    fn runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn test_load_model_empty_workspace() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let loader = ModelLoader::new(backend);

            let result = loader.load_model("workspace").await.unwrap();
            assert_eq!(result.tables.len(), 0);
            assert_eq!(result.relationships.len(), 0);
            assert_eq!(result.orphaned_relationships.len(), 0);
        });
    }

    #[test]
    fn test_load_model_with_tables() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let backend_setup = FileSystemStorageBackend::new(temp.path());
            let loader = ModelLoader::new(backend);

            // Create workspace structure
            backend_setup.create_dir("workspace/tables").await.unwrap();

            // Create a table YAML file
            let table_yaml = r#"
name: users
id: 550e8400-e29b-41d4-a716-446655440000
columns:
  - name: id
    data_type: INT
    primary_key: true
    nullable: false
"#;
            backend_setup
                .write_file("workspace/tables/users.yaml", table_yaml.as_bytes())
                .await
                .unwrap();

            let result = loader.load_model("workspace").await.unwrap();
            assert_eq!(result.tables.len(), 1);
            assert_eq!(result.tables[0].name, "users");
            assert_eq!(
                result.tables[0].id,
                Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
            );
        });
    }

    #[test]
    fn test_load_model_with_relationships() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let backend_setup = FileSystemStorageBackend::new(temp.path());
            let loader = ModelLoader::new(backend);

            // Create workspace structure
            backend_setup.create_dir("workspace/tables").await.unwrap();

            // Create tables
            let users_table = r#"
name: users
id: 550e8400-e29b-41d4-a716-446655440000
columns:
  - name: id
    type: string
"#;
            backend_setup
                .write_file("workspace/tables/users.yaml", users_table.as_bytes())
                .await
                .unwrap();

            let orders_table = r#"
name: orders
id: 550e8400-e29b-41d4-a716-446655440001
columns:
  - name: id
    type: string
"#;
            backend_setup
                .write_file("workspace/tables/orders.yaml", orders_table.as_bytes())
                .await
                .unwrap();

            // Create relationships file
            let relationships_yaml = r#"
relationships:
  - id: 660e8400-e29b-41d4-a716-446655440000
    source_table_id: 550e8400-e29b-41d4-a716-446655440000
    target_table_id: 550e8400-e29b-41d4-a716-446655440001
"#;
            backend_setup
                .write_file(
                    "workspace/relationships.yaml",
                    relationships_yaml.as_bytes(),
                )
                .await
                .unwrap();

            let result = loader.load_model("workspace").await.unwrap();
            assert_eq!(result.tables.len(), 2);
            assert_eq!(result.relationships.len(), 1);
            assert_eq!(result.orphaned_relationships.len(), 0);
        });
    }

    #[test]
    fn test_load_model_orphaned_relationships() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let backend_setup = FileSystemStorageBackend::new(temp.path());
            let loader = ModelLoader::new(backend);

            // Create workspace structure
            backend_setup.create_dir("workspace/tables").await.unwrap();

            // Create only one table
            let users_table = r#"
name: users
id: 550e8400-e29b-41d4-a716-446655440000
columns:
  - name: id
    type: string
"#;
            backend_setup
                .write_file("workspace/tables/users.yaml", users_table.as_bytes())
                .await
                .unwrap();

            // Create relationships file with orphaned relationship
            let relationships_yaml = r#"
relationships:
  - id: 660e8400-e29b-41d4-a716-446655440000
    source_table_id: 550e8400-e29b-41d4-a716-446655440000
    target_table_id: 550e8400-e29b-41d4-a716-446655440001
"#;
            backend_setup
                .write_file(
                    "workspace/relationships.yaml",
                    relationships_yaml.as_bytes(),
                )
                .await
                .unwrap();

            let result = loader.load_model("workspace").await.unwrap();
            assert_eq!(result.tables.len(), 1);
            assert_eq!(result.relationships.len(), 0);
            assert_eq!(result.orphaned_relationships.len(), 1);
        });
    }

    #[test]
    fn test_load_model_invalid_yaml() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let backend_setup = FileSystemStorageBackend::new(temp.path());
            let loader = ModelLoader::new(backend);

            // Create workspace structure
            backend_setup.create_dir("workspace/tables").await.unwrap();

            // Create invalid YAML file
            let invalid_yaml = "invalid: yaml: content: [";
            backend_setup
                .write_file("workspace/tables/invalid.yaml", invalid_yaml.as_bytes())
                .await
                .unwrap();

            // Should not panic, but skip invalid files
            let result = loader.load_model("workspace").await.unwrap();
            assert_eq!(result.tables.len(), 0);
        });
    }

    #[test]
    fn test_load_model_missing_name_field() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let backend_setup = FileSystemStorageBackend::new(temp.path());
            let loader = ModelLoader::new(backend);

            // Create workspace structure
            backend_setup.create_dir("workspace/tables").await.unwrap();

            // Create table YAML without name field
            let table_yaml = r#"
id: 550e8400-e29b-41d4-a716-446655440000
columns: []
"#;
            backend_setup
                .write_file("workspace/tables/table.yaml", table_yaml.as_bytes())
                .await
                .unwrap();

            // Should skip files with missing required fields
            let result = loader.load_model("workspace").await.unwrap();
            assert_eq!(result.tables.len(), 0);
        });
    }
}

#[cfg(feature = "native-fs")]
mod model_saver_tests {
    use data_modelling_sdk::model::saver::{ModelSaver, RelationshipData, TableData};
    use data_modelling_sdk::storage::{StorageBackend, filesystem::FileSystemStorageBackend};
    use tempfile::TempDir;
    use tokio::runtime::Runtime;
    use uuid::Uuid;

    fn runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn test_save_table() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let saver = ModelSaver::new(backend);

            let mut yaml_map = serde_yaml::Mapping::new();
            yaml_map.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String("users".to_string()),
            );
            yaml_map.insert(
                serde_yaml::Value::String("id".to_string()),
                serde_yaml::Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
            );
            yaml_map.insert(
                serde_yaml::Value::String("columns".to_string()),
                serde_yaml::Value::Sequence(vec![]),
            );
            let table = TableData {
                id: Uuid::new_v4(),
                name: "users".to_string(),
                yaml_file_path: None,
                yaml_value: serde_yaml::Value::Mapping(yaml_map),
            };

            let backend_clone = FileSystemStorageBackend::new(temp.path());
            saver.save_table("workspace", &table).await.unwrap();

            // Verify file was created
            assert!(
                backend_clone
                    .file_exists("workspace/tables/users.yaml")
                    .await
                    .unwrap()
            );
        });
    }

    #[test]
    fn test_save_table_with_custom_path() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let saver = ModelSaver::new(backend);

            let mut yaml_map = serde_yaml::Mapping::new();
            yaml_map.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String("users".to_string()),
            );
            yaml_map.insert(
                serde_yaml::Value::String("id".to_string()),
                serde_yaml::Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
            );
            let table = TableData {
                id: Uuid::new_v4(),
                name: "users".to_string(),
                yaml_file_path: Some("tables/custom_users.yaml".to_string()),
                yaml_value: serde_yaml::Value::Mapping(yaml_map),
            };

            let backend_clone = FileSystemStorageBackend::new(temp.path());
            saver.save_table("workspace", &table).await.unwrap();

            // Verify file was created at custom path
            assert!(
                backend_clone
                    .file_exists("workspace/tables/custom_users.yaml")
                    .await
                    .unwrap()
            );
        });
    }

    #[test]
    fn test_save_relationships() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let saver = ModelSaver::new(backend);

            let mut rel_map = serde_yaml::Mapping::new();
            rel_map.insert(
                serde_yaml::Value::String("id".to_string()),
                serde_yaml::Value::String("660e8400-e29b-41d4-a716-446655440000".to_string()),
            );
            rel_map.insert(
                serde_yaml::Value::String("source_table_id".to_string()),
                serde_yaml::Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
            );
            rel_map.insert(
                serde_yaml::Value::String("target_table_id".to_string()),
                serde_yaml::Value::String("550e8400-e29b-41d4-a716-446655440001".to_string()),
            );
            let relationships = vec![RelationshipData {
                id: Uuid::new_v4(),
                source_table_id: Uuid::new_v4(),
                target_table_id: Uuid::new_v4(),
                yaml_value: serde_yaml::Value::Mapping(rel_map),
            }];

            let backend_clone = FileSystemStorageBackend::new(temp.path());
            saver
                .save_relationships("workspace", &relationships)
                .await
                .unwrap();

            // Verify file was created
            assert!(
                backend_clone
                    .file_exists("workspace/relationships.yaml")
                    .await
                    .unwrap()
            );

            // Verify content
            let content = backend_clone
                .read_file("workspace/relationships.yaml")
                .await
                .unwrap();
            let yaml_str = String::from_utf8(content).unwrap();
            assert!(yaml_str.contains("relationships"));
            assert!(yaml_str.contains("source_table_id"));
        });
    }

    #[test]
    fn test_save_table_sanitizes_filename() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());
            let saver = ModelSaver::new(backend);

            let mut yaml_map = serde_yaml::Mapping::new();
            yaml_map.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String("users/table".to_string()),
            );
            yaml_map.insert(
                serde_yaml::Value::String("id".to_string()),
                serde_yaml::Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
            );
            let table = TableData {
                id: Uuid::new_v4(),
                name: "users/table".to_string(), // Contains invalid filename character
                yaml_file_path: None,
                yaml_value: serde_yaml::Value::Mapping(yaml_map),
            };

            let backend_clone = FileSystemStorageBackend::new(temp.path());
            saver.save_table("workspace", &table).await.unwrap();

            // Verify file was created with sanitized name
            assert!(
                backend_clone
                    .file_exists("workspace/tables/users_table.yaml")
                    .await
                    .unwrap()
            );
        });
    }
}

#[cfg(feature = "api-backend")]
mod api_model_loader_tests {

    // Note: These tests would require a mock HTTP server
    // For now, we'll document what tests should be added:
    // - test_load_model_from_api_success
    // - test_load_model_from_api_orphaned_relationships
    // - test_load_model_from_api_invalid_json
    // - test_load_model_from_api_missing_fields
    // - test_load_model_from_api_network_error
}
