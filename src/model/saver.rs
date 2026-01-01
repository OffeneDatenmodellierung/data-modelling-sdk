//! Model saving functionality
//!
//! Saves models to storage backends, handling YAML serialization.

use crate::storage::{StorageBackend, StorageError};
use anyhow::Result;
use serde_yaml;
use tracing::info;
use uuid::Uuid;

/// Model saver that uses a storage backend
pub struct ModelSaver<B: StorageBackend> {
    storage: B,
}

impl<B: StorageBackend> ModelSaver<B> {
    /// Create a new model saver with the given storage backend
    pub fn new(storage: B) -> Self {
        Self { storage }
    }

    /// Save a table to storage
    ///
    /// Saves the table as a YAML file in the workspace's `tables/` directory.
    /// The filename will be based on the table name if yaml_file_path is not provided.
    pub async fn save_table(
        &self,
        workspace_path: &str,
        table: &TableData,
    ) -> Result<(), StorageError> {
        let tables_dir = format!("{}/tables", workspace_path);

        // Ensure tables directory exists
        if !self.storage.dir_exists(&tables_dir).await? {
            self.storage.create_dir(&tables_dir).await?;
        }

        // Determine file path
        let file_path = if let Some(ref yaml_path) = table.yaml_file_path {
            format!(
                "{}/{}",
                workspace_path,
                yaml_path.strip_prefix('/').unwrap_or(yaml_path)
            )
        } else {
            // Generate filename from table name
            let sanitized_name = sanitize_filename(&table.name);
            format!("{}/tables/{}.yaml", workspace_path, sanitized_name)
        };

        // Serialize table to YAML
        let yaml_content = serde_yaml::to_string(&table.yaml_value).map_err(|e| {
            StorageError::SerializationError(format!("Failed to serialize table: {}", e))
        })?;

        // Write to storage
        self.storage
            .write_file(&file_path, yaml_content.as_bytes())
            .await?;

        info!("Saved table '{}' to {}", table.name, file_path);
        Ok(())
    }

    /// Save relationships to storage
    ///
    /// Saves relationships to `relationships.yaml` in the workspace directory.
    pub async fn save_relationships(
        &self,
        workspace_path: &str,
        relationships: &[RelationshipData],
    ) -> Result<(), StorageError> {
        let file_path = format!("{}/relationships.yaml", workspace_path);

        // Serialize relationships to YAML
        let mut yaml_map = serde_yaml::Mapping::new();
        let mut rels_array = serde_yaml::Sequence::new();
        for rel in relationships {
            rels_array.push(rel.yaml_value.clone());
        }
        yaml_map.insert(
            serde_yaml::Value::String("relationships".to_string()),
            serde_yaml::Value::Sequence(rels_array),
        );
        let yaml_value = serde_yaml::Value::Mapping(yaml_map);

        let yaml_content = serde_yaml::to_string(&yaml_value).map_err(|e| {
            StorageError::SerializationError(format!("Failed to write YAML: {}", e))
        })?;

        // Write to storage
        self.storage
            .write_file(&file_path, yaml_content.as_bytes())
            .await?;

        info!(
            "Saved {} relationships to {}",
            relationships.len(),
            file_path
        );
        Ok(())
    }
}

/// Table data to save
#[derive(Debug, Clone)]
pub struct TableData {
    pub id: Uuid,
    pub name: String,
    pub yaml_file_path: Option<String>,
    pub yaml_value: serde_yaml::Value,
}

/// Relationship data to save
#[derive(Debug, Clone)]
pub struct RelationshipData {
    pub id: Uuid,
    pub source_table_id: Uuid,
    pub target_table_id: Uuid,
    pub yaml_value: serde_yaml::Value,
}

/// Sanitize a filename by removing invalid characters
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
