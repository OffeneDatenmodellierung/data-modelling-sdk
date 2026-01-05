//! Domain configuration model
//!
//! Defines the DomainConfig entity for the data modelling application.
//! This represents the domain.yaml configuration file that stores domain metadata
//! and asset references.
//!
//! Note: This is separate from the `Domain` struct in domain.rs which represents
//! the visual domain schema with systems, nodes, and connections.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Owner information for a domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DomainOwner {
    /// Owner name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Owner email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Team name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    /// Role (e.g., "Data Owner", "Data Steward")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Position on canvas for view rendering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewPosition {
    /// X coordinate on canvas
    pub x: f64,
    /// Y coordinate on canvas
    pub y: f64,
}

/// DomainConfig - Configuration file for a domain (domain.yaml)
///
/// This represents the domain.yaml file that stores:
/// - Domain metadata (id, name, description, timestamps)
/// - Owner information
/// - References to assets (tables, products, assets, processes, decisions)
/// - View positions for canvas rendering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainConfig {
    /// Unique identifier for the domain
    pub id: Uuid,
    /// Parent workspace identifier
    pub workspace_id: Uuid,
    /// Domain name (unique within workspace, max 255 chars)
    pub name: String,
    /// Optional description of the domain's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub last_modified_at: DateTime<Utc>,
    /// Owner information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<DomainOwner>,
    /// Array of system IDs that belong to this domain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systems: Vec<Uuid>,
    /// Array of ODCS table IDs that belong to this domain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<Uuid>,
    /// Array of ODPS product IDs that belong to this domain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub products: Vec<Uuid>,
    /// Array of CADS compute asset IDs that belong to this domain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<Uuid>,
    /// Array of BPMN process IDs that belong to this domain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<Uuid>,
    /// Array of DMN decision IDs that belong to this domain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Uuid>,
    /// View positions for different view modes
    /// Key: view mode name (e.g., "systems", "process", "operational", "analytical", "products")
    /// Value: Map of entity ID to position
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub view_positions: HashMap<String, HashMap<String, ViewPosition>>,
    /// Path to domain folder (for offline mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_path: Option<String>,
    /// Path to workspace root folder (for offline mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
}

impl DomainConfig {
    /// Create a new DomainConfig
    pub fn new(name: String, workspace_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name,
            description: None,
            created_at: now,
            last_modified_at: now,
            owner: None,
            systems: Vec::new(),
            tables: Vec::new(),
            products: Vec::new(),
            assets: Vec::new(),
            processes: Vec::new(),
            decisions: Vec::new(),
            view_positions: HashMap::new(),
            folder_path: None,
            workspace_path: None,
        }
    }

    /// Create a DomainConfig with a specific ID
    pub fn with_id(id: Uuid, name: String, workspace_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id,
            workspace_id,
            name,
            description: None,
            created_at: now,
            last_modified_at: now,
            owner: None,
            systems: Vec::new(),
            tables: Vec::new(),
            products: Vec::new(),
            assets: Vec::new(),
            processes: Vec::new(),
            decisions: Vec::new(),
            view_positions: HashMap::new(),
            folder_path: None,
            workspace_path: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set owner
    pub fn with_owner(mut self, owner: DomainOwner) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Add a table ID
    pub fn add_table(&mut self, table_id: Uuid) {
        if !self.tables.contains(&table_id) {
            self.tables.push(table_id);
            self.last_modified_at = Utc::now();
        }
    }

    /// Remove a table ID
    pub fn remove_table(&mut self, table_id: Uuid) -> bool {
        let initial_len = self.tables.len();
        self.tables.retain(|&id| id != table_id);
        if self.tables.len() != initial_len {
            self.last_modified_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Add a product ID
    pub fn add_product(&mut self, product_id: Uuid) {
        if !self.products.contains(&product_id) {
            self.products.push(product_id);
            self.last_modified_at = Utc::now();
        }
    }

    /// Add an asset ID
    pub fn add_asset(&mut self, asset_id: Uuid) {
        if !self.assets.contains(&asset_id) {
            self.assets.push(asset_id);
            self.last_modified_at = Utc::now();
        }
    }

    /// Add a process ID
    pub fn add_process(&mut self, process_id: Uuid) {
        if !self.processes.contains(&process_id) {
            self.processes.push(process_id);
            self.last_modified_at = Utc::now();
        }
    }

    /// Add a decision ID
    pub fn add_decision(&mut self, decision_id: Uuid) {
        if !self.decisions.contains(&decision_id) {
            self.decisions.push(decision_id);
            self.last_modified_at = Utc::now();
        }
    }

    /// Add a system ID
    pub fn add_system(&mut self, system_id: Uuid) {
        if !self.systems.contains(&system_id) {
            self.systems.push(system_id);
            self.last_modified_at = Utc::now();
        }
    }

    /// Set view position for an entity in a view mode
    pub fn set_view_position(&mut self, view_mode: &str, entity_id: &str, x: f64, y: f64) {
        let positions = self
            .view_positions
            .entry(view_mode.to_string())
            .or_default();
        positions.insert(entity_id.to_string(), ViewPosition { x, y });
        self.last_modified_at = Utc::now();
    }

    /// Get view position for an entity in a view mode
    pub fn get_view_position(&self, view_mode: &str, entity_id: &str) -> Option<&ViewPosition> {
        self.view_positions
            .get(view_mode)
            .and_then(|positions| positions.get(entity_id))
    }

    /// Import from YAML
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }

    /// Export to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Import from JSON
    pub fn from_json(json_content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_content)
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Export to pretty JSON
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_config_new() {
        let workspace_id = Uuid::new_v4();
        let config = DomainConfig::new("Customer Management".to_string(), workspace_id);

        assert_eq!(config.name, "Customer Management");
        assert_eq!(config.workspace_id, workspace_id);
        assert!(config.tables.is_empty());
        assert!(config.products.is_empty());
    }

    #[test]
    fn test_domain_config_add_table() {
        let mut config = DomainConfig::new("Test".to_string(), Uuid::new_v4());
        let table_id = Uuid::new_v4();

        config.add_table(table_id);
        assert_eq!(config.tables.len(), 1);
        assert_eq!(config.tables[0], table_id);

        // Adding same ID should not duplicate
        config.add_table(table_id);
        assert_eq!(config.tables.len(), 1);
    }

    #[test]
    fn test_domain_config_view_positions() {
        let mut config = DomainConfig::new("Test".to_string(), Uuid::new_v4());
        let entity_id = Uuid::new_v4().to_string();

        config.set_view_position("systems", &entity_id, 100.0, 200.0);

        let pos = config.get_view_position("systems", &entity_id);
        assert!(pos.is_some());
        let pos = pos.unwrap();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 200.0);
    }

    #[test]
    fn test_domain_config_yaml_roundtrip() {
        let workspace_id = Uuid::new_v4();
        let mut config = DomainConfig::new("Finance".to_string(), workspace_id);
        config.description = Some("Financial data domain".to_string());
        config.owner = Some(DomainOwner {
            name: Some("Jane Doe".to_string()),
            email: Some("jane@example.com".to_string()),
            team: Some("Data Team".to_string()),
            role: Some("Data Owner".to_string()),
        });
        config.add_table(Uuid::new_v4());
        config.add_product(Uuid::new_v4());

        let yaml = config.to_yaml().unwrap();
        let parsed = DomainConfig::from_yaml(&yaml).unwrap();

        assert_eq!(config.id, parsed.id);
        assert_eq!(config.name, parsed.name);
        assert_eq!(config.description, parsed.description);
        assert_eq!(config.tables.len(), parsed.tables.len());
    }

    #[test]
    fn test_domain_config_json_roundtrip() {
        let config = DomainConfig::new("Test".to_string(), Uuid::new_v4());

        let json = config.to_json().unwrap();
        let parsed = DomainConfig::from_json(&json).unwrap();

        assert_eq!(config.id, parsed.id);
        assert_eq!(config.name, parsed.name);
    }
}
