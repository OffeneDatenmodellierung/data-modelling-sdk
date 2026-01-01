//! Model loading functionality
//!
//! Loads models from storage backends, handling YAML parsing and validation.
//!
//! Supports both file-based loading (FileSystemStorageBackend, BrowserStorageBackend)
//! and API-based loading (ApiStorageBackend).

use crate::storage::{StorageBackend, StorageError};
use anyhow::Result;
use serde_yaml;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;
// Serialize/Deserialize not currently used

/// Model loader that uses a storage backend
pub struct ModelLoader<B: StorageBackend> {
    storage: B,
}

impl<B: StorageBackend> ModelLoader<B> {
    /// Create a new model loader with the given storage backend
    pub fn new(storage: B) -> Self {
        Self { storage }
    }

    /// Load a model from storage
    ///
    /// For file-based backends (FileSystemStorageBackend, BrowserStorageBackend):
    /// - Loads from `tables/` subdirectory with YAML files
    /// - Loads from `relationships.yaml` file
    ///
    /// For API backend (ApiStorageBackend), use `load_model_from_api()` instead.
    ///
    /// Returns the loaded model data and a list of orphaned relationships
    /// (relationships that reference non-existent tables).
    pub async fn load_model(&self, workspace_path: &str) -> Result<ModelLoadResult, StorageError> {
        // File-based loading implementation
        self.load_model_from_files(workspace_path).await
    }

    /// Load model from file-based storage
    async fn load_model_from_files(
        &self,
        workspace_path: &str,
    ) -> Result<ModelLoadResult, StorageError> {
        let tables_dir = format!("{}/tables", workspace_path);

        // Ensure tables directory exists
        if !self.storage.dir_exists(&tables_dir).await? {
            self.storage.create_dir(&tables_dir).await?;
        }

        // Load tables from individual YAML files
        let mut tables = Vec::new();
        let mut table_ids: HashMap<Uuid, String> = HashMap::new();

        let files = self.storage.list_files(&tables_dir).await?;
        for file_name in files {
            if file_name.ends_with(".yaml") || file_name.ends_with(".yml") {
                let file_path = format!("{}/{}", tables_dir, file_name);
                match self.load_table_from_yaml(&file_path, workspace_path).await {
                    Ok(table_data) => {
                        table_ids.insert(table_data.id, table_data.name.clone());
                        tables.push(table_data);
                    }
                    Err(e) => {
                        warn!("Failed to load table from {}: {}", file_path, e);
                    }
                }
            }
        }

        info!(
            "Loaded {} tables from workspace {}",
            tables.len(),
            workspace_path
        );

        // Load relationships from control file
        let relationships_file = format!("{}/relationships.yaml", workspace_path);
        let mut relationships = Vec::new();
        let mut orphaned_relationships = Vec::new();

        if self.storage.file_exists(&relationships_file).await? {
            match self.load_relationships_from_yaml(&relationships_file).await {
                Ok(loaded_rels) => {
                    // Separate valid and orphaned relationships
                    for rel in loaded_rels {
                        let source_exists = table_ids.contains_key(&rel.source_table_id);
                        let target_exists = table_ids.contains_key(&rel.target_table_id);

                        if source_exists && target_exists {
                            relationships.push(rel.clone());
                        } else {
                            orphaned_relationships.push(rel.clone());
                            warn!(
                                "Orphaned relationship {}: source={} (exists: {}), target={} (exists: {})",
                                rel.id,
                                rel.source_table_id,
                                source_exists,
                                rel.target_table_id,
                                target_exists
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to load relationships from {}: {}",
                        relationships_file, e
                    );
                }
            }
        }

        info!(
            "Loaded {} relationships ({} orphaned) from workspace {}",
            relationships.len(),
            orphaned_relationships.len(),
            workspace_path
        );

        Ok(ModelLoadResult {
            tables,
            relationships,
            orphaned_relationships,
        })
    }

    /// Load a table from a YAML file
    ///
    /// Uses ODCSImporter to fully parse the table structure, including all columns,
    /// metadata, and nested properties. This ensures complete table data is loaded.
    async fn load_table_from_yaml(
        &self,
        yaml_path: &str,
        workspace_path: &str,
    ) -> Result<TableData, StorageError> {
        let content = self.storage.read_file(yaml_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        // Use ODCSImporter to fully parse the table structure
        let mut importer = crate::import::odcs::ODCSImporter::new();
        let (table, parse_errors) = importer.parse_table(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse ODCS YAML: {}", e))
        })?;

        // Log any parse warnings/errors but don't fail if table was successfully parsed
        if !parse_errors.is_empty() {
            warn!(
                "Table '{}' parsed with {} warnings/errors",
                table.name,
                parse_errors.len()
            );
        }

        // Calculate relative path
        let relative_path = yaml_path
            .strip_prefix(workspace_path)
            .map(|s| s.strip_prefix('/').unwrap_or(s).to_string())
            .unwrap_or_else(|| yaml_path.to_string());

        Ok(TableData {
            id: table.id,
            name: table.name,
            yaml_file_path: Some(relative_path),
            yaml_content,
        })
    }

    /// Load relationships from YAML file
    async fn load_relationships_from_yaml(
        &self,
        yaml_path: &str,
    ) -> Result<Vec<RelationshipData>, StorageError> {
        let content = self.storage.read_file(yaml_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        let data: serde_yaml::Value = serde_yaml::from_str(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse YAML: {}", e))
        })?;

        let mut relationships = Vec::new();

        // Handle both formats: direct array or object with "relationships" key
        let rels_array = data
            .get("relationships")
            .and_then(|v| v.as_sequence())
            .or_else(|| data.as_sequence());

        if let Some(rels_array) = rels_array {
            for rel_data in rels_array {
                match self.parse_relationship(rel_data) {
                    Ok(rel) => relationships.push(rel),
                    Err(e) => {
                        warn!("Failed to parse relationship: {}", e);
                    }
                }
            }
        }

        Ok(relationships)
    }

    /// Parse a relationship from YAML value
    fn parse_relationship(
        &self,
        data: &serde_yaml::Value,
    ) -> Result<RelationshipData, StorageError> {
        let source_table_id = data
            .get("source_table_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                StorageError::SerializationError("Missing source_table_id".to_string())
            })?;

        let target_table_id = data
            .get("target_table_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                StorageError::SerializationError("Missing target_table_id".to_string())
            })?;

        // Parse existing UUID or generate deterministic one based on source and target table IDs
        let id = data
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(|| {
                crate::models::relationship::Relationship::generate_id(
                    source_table_id,
                    target_table_id,
                )
            });

        Ok(RelationshipData {
            id,
            source_table_id,
            target_table_id,
        })
    }
}

/// Result of loading a model
#[derive(Debug)]
pub struct ModelLoadResult {
    pub tables: Vec<TableData>,
    pub relationships: Vec<RelationshipData>,
    pub orphaned_relationships: Vec<RelationshipData>,
}

/// Table data loaded from storage
#[derive(Debug, Clone)]
pub struct TableData {
    pub id: Uuid,
    pub name: String,
    pub yaml_file_path: Option<String>,
    pub yaml_content: String,
}

/// Relationship data loaded from storage
#[derive(Debug, Clone)]
pub struct RelationshipData {
    pub id: Uuid,
    pub source_table_id: Uuid,
    pub target_table_id: Uuid,
}
