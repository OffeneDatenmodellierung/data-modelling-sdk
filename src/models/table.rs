//! Table model for the SDK

use super::column::Column;
use super::enums::{
    DataVaultClassification, DatabaseType, MedallionLayer, ModelingLevel, SCDPattern,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table {
    pub id: Uuid,
    pub name: String,
    pub columns: Vec<Column>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_type: Option<DatabaseType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    #[serde(default)]
    pub medallion_layers: Vec<MedallionLayer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scd_pattern: Option<SCDPattern>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_vault_classification: Option<DataVaultClassification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modeling_level: Option<ModelingLevel>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub odcl_metadata: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yaml_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawio_cell_id: Option<String>,
    #[serde(default)]
    pub quality: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub errors: Vec<HashMap<String, serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Table {
    pub fn new(name: String, columns: Vec<Column>) -> Self {
        let now = Utc::now();
        // Use deterministic UUID v5 based on table name (no randomness needed)
        // This avoids requiring getrandom/wasm_js for WASM builds
        let id = Self::generate_id(&name, None, None, None);
        Self {
            id,
            name,
            columns,
            database_type: None,
            catalog_name: None,
            schema_name: None,
            medallion_layers: Vec::new(),
            scd_pattern: None,
            data_vault_classification: None,
            modeling_level: None,
            tags: Vec::new(),
            odcl_metadata: HashMap::new(),
            position: None,
            yaml_file_path: None,
            drawio_cell_id: None,
            quality: Vec::new(),
            errors: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn get_unique_key(&self) -> (Option<String>, String, Option<String>, Option<String>) {
        (
            self.database_type.as_ref().map(|dt| format!("{:?}", dt)),
            self.name.clone(),
            self.catalog_name.clone(),
            self.schema_name.clone(),
        )
    }

    /// Generate a deterministic UUID v5 for a table based on its unique key
    /// This avoids requiring random number generation (getrandom/wasm_js)
    pub fn generate_id(
        name: &str,
        database_type: Option<&DatabaseType>,
        catalog_name: Option<&str>,
        schema_name: Option<&str>,
    ) -> Uuid {
        // Create a deterministic string from the unique key components
        let key = format!(
            "{}:{}:{}:{}",
            database_type.map(|dt| format!("{:?}", dt)).unwrap_or_default(),
            name,
            catalog_name.unwrap_or(""),
            schema_name.unwrap_or("")
        );
        // Use UUID v5 (deterministic) with a namespace UUID for tables
        // This generates the same UUID for the same table name/key
        Uuid::new_v5(&Uuid::NAMESPACE_DNS, key.as_bytes())
    }
}



