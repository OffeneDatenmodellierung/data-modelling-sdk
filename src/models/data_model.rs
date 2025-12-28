//! DataModel for the SDK

use super::relationship::Relationship;
use super::table::Table;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataModel {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub git_directory_path: String,
    #[serde(default)]
    pub tables: Vec<Table>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    pub control_file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_file_path: Option<String>,
    #[serde(default)]
    pub is_subfolder: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_git_directory: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DataModel {
    pub fn new(name: String, git_directory_path: String, control_file_path: String) -> Self {
        let now = Utc::now();
        // Use deterministic UUID v5 based on model name and git path
        // This avoids requiring random number generation (getrandom/wasm_js)
        let key = format!("{}:{}", git_directory_path, name);
        let id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, key.as_bytes());
        Self {
            id,
            name,
            description: None,
            git_directory_path,
            tables: Vec::new(),
            relationships: Vec::new(),
            control_file_path,
            diagram_file_path: None,
            is_subfolder: false,
            parent_git_directory: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn get_table_by_id(&self, table_id: Uuid) -> Option<&Table> {
        self.tables.iter().find(|t| t.id == table_id)
    }

    pub fn get_table_by_id_mut(&mut self, table_id: Uuid) -> Option<&mut Table> {
        self.tables.iter_mut().find(|t| t.id == table_id)
    }

    pub fn get_table_by_name(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn get_table_by_unique_key(
        &self,
        database_type: Option<&str>,
        name: &str,
        catalog_name: Option<&str>,
        schema_name: Option<&str>,
    ) -> Option<&Table> {
        let target_key = (
            database_type.map(|s| s.to_string()),
            name.to_string(),
            catalog_name.map(|s| s.to_string()),
            schema_name.map(|s| s.to_string()),
        );
        self.tables
            .iter()
            .find(|t| t.get_unique_key() == target_key)
    }

    pub fn get_relationships_for_table(&self, table_id: Uuid) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| r.source_table_id == table_id || r.target_table_id == table_id)
            .collect()
    }
}



