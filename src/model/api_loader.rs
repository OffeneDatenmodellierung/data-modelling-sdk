//! API-based model loading
//! 
//! Specialized loader for API storage backend that loads models via HTTP endpoints.

#[cfg(feature = "api-backend")]
use crate::storage::api::ApiStorageBackend;
#[cfg(feature = "api-backend")]
use crate::storage::StorageError;
#[cfg(feature = "api-backend")]
use std::collections::HashMap;
#[cfg(feature = "api-backend")]
use tracing::info;
#[cfg(feature = "api-backend")]
use uuid::Uuid;

#[cfg(feature = "api-backend")]
use super::loader::{ModelLoadResult, RelationshipData, TableData};

/// API-based model loader
#[cfg(feature = "api-backend")]
pub struct ApiModelLoader {
    backend: ApiStorageBackend,
}

#[cfg(feature = "api-backend")]
impl ApiModelLoader {
    /// Create a new API model loader
    pub fn new(backend: ApiStorageBackend) -> Self {
        Self { backend }
    }

    /// Load model from API endpoints
    /// 
    /// Loads tables and relationships via HTTP API:
    /// - GET /tables
    /// - GET /relationships
    /// 
    /// Returns the loaded model data and a list of orphaned relationships.
    pub async fn load_model(&self) -> Result<ModelLoadResult, StorageError> {
        // Load tables and relationships via API
        let tables_json = self.backend.load_tables().await?;
        let relationships_json = self.backend.load_relationships().await?;
        
        // Convert JSON to TableData and RelationshipData
        let mut tables = Vec::new();
        let mut table_ids: HashMap<Uuid, String> = HashMap::new();
        
        for (_idx, table_json) in tables_json.iter().enumerate() {
            // Extract basic fields from JSON
            let id_str = table_json.get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| StorageError::SerializationError("Missing table id".to_string()))?;
            let id = Uuid::parse_str(id_str)
                .map_err(|e| StorageError::SerializationError(format!("Invalid table id: {}", e)))?;
            
            let name = table_json.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| StorageError::SerializationError("Missing table name".to_string()))?;
            
            // Serialize back to YAML for consistency with file-based storage
            let yaml_content = serde_yaml::to_string(table_json)
                .map_err(|e| StorageError::SerializationError(format!("Failed to serialize table: {}", e)))?;
            
            table_ids.insert(id, name.clone());
            tables.push(TableData {
                id,
                name,
                yaml_file_path: None, // API doesn't have file paths
                yaml_content,
            });
        }
        
        info!("Loaded {} tables from API", tables.len());
        
        // Convert relationships
        let mut relationships = Vec::new();
        let mut orphaned_relationships = Vec::new();
        
        for rel_json in relationships_json {
            let id_str = rel_json.get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| StorageError::SerializationError("Missing relationship id".to_string()))?;
            let id = Uuid::parse_str(id_str)
                .map_err(|e| StorageError::SerializationError(format!("Invalid relationship id: {}", e)))?;
            
            let source_id_str = rel_json.get("source_table_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| StorageError::SerializationError("Missing source_table_id".to_string()))?;
            let source_table_id = Uuid::parse_str(source_id_str)
                .map_err(|e| StorageError::SerializationError(format!("Invalid source_table_id: {}", e)))?;
            
            let target_id_str = rel_json.get("target_table_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| StorageError::SerializationError("Missing target_table_id".to_string()))?;
            let target_table_id = Uuid::parse_str(target_id_str)
                .map_err(|e| StorageError::SerializationError(format!("Invalid target_table_id: {}", e)))?;
            
            let source_exists = table_ids.contains_key(&source_table_id);
            let target_exists = table_ids.contains_key(&target_table_id);
            
            if source_exists && target_exists {
                relationships.push(RelationshipData {
                    id,
                    source_table_id,
                    target_table_id,
                });
            } else {
                orphaned_relationships.push(RelationshipData {
                    id,
                    source_table_id,
                    target_table_id,
                });
            }
        }
        
        info!(
            "Loaded {} relationships ({} orphaned) from API",
            relationships.len(),
            orphaned_relationships.len()
        );
        
        Ok(ModelLoadResult {
            tables,
            relationships,
            orphaned_relationships,
        })
    }
}
