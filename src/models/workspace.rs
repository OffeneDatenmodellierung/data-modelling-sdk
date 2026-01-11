//! Workspace model
//!
//! Defines the Workspace entity for the data modelling application.
//! Workspaces are top-level containers that organize domains and their associated assets.
//!
//! ## File Naming Convention
//!
//! All files use a flat naming pattern:
//! - `workspace.yaml` - workspace metadata with references to all assets and relationships
//! - `{workspace}_{domain}_{system}_{resource}.odcs.yaml` - ODCS table files
//! - `{workspace}_{domain}_{system}_{resource}.odps.yaml` - ODPS product files
//! - `{workspace}_{domain}_{system}_{resource}.cads.yaml` - CADS asset files
//!
//! Where `{system}` is optional if the resource is at the domain level.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::Relationship;
use super::domain_config::ViewPosition;

/// Asset reference within a workspace
///
/// Contains information about an asset file and its location in the domain/system hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetReference {
    /// Asset identifier (UUID)
    pub id: Uuid,
    /// Asset name
    pub name: String,
    /// Domain name this asset belongs to
    pub domain: String,
    /// Optional system name (if asset is within a system)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Asset type (odcs, odps, cads)
    pub asset_type: AssetType,
    /// File path relative to workspace (generated from naming convention)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Type of asset or file in the workspace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    /// Workspace configuration file
    Workspace,
    /// Relationships file
    Relationships,
    /// ODCS table definition
    Odcs,
    /// ODPS data product
    Odps,
    /// CADS compute asset
    Cads,
    /// BPMN process model
    Bpmn,
    /// DMN decision model
    Dmn,
    /// OpenAPI specification
    Openapi,
    /// MADR decision record
    Decision,
    /// Knowledge base article
    Knowledge,
    /// Decision log index file
    DecisionIndex,
    /// Knowledge base index file
    KnowledgeIndex,
}

impl AssetType {
    /// Get file extension for this asset type
    pub fn extension(&self) -> &'static str {
        match self {
            AssetType::Workspace => "yaml",
            AssetType::Relationships => "yaml",
            AssetType::Odcs => "odcs.yaml",
            AssetType::Odps => "odps.yaml",
            AssetType::Cads => "cads.yaml",
            AssetType::Bpmn => "bpmn.xml",
            AssetType::Dmn => "dmn.xml",
            AssetType::Openapi => "openapi.yaml",
            AssetType::Decision => "madr.yaml",
            AssetType::Knowledge => "kb.yaml",
            AssetType::DecisionIndex => "yaml",
            AssetType::KnowledgeIndex => "yaml",
        }
    }

    /// Get the full filename for workspace-level files
    pub fn filename(&self) -> Option<&'static str> {
        match self {
            AssetType::Workspace => Some("workspace.yaml"),
            AssetType::Relationships => Some("relationships.yaml"),
            AssetType::DecisionIndex => Some("decisions.yaml"),
            AssetType::KnowledgeIndex => Some("knowledge.yaml"),
            _ => None,
        }
    }

    /// Check if this is a workspace-level file (not a domain/system asset)
    pub fn is_workspace_level(&self) -> bool {
        matches!(
            self,
            AssetType::Workspace
                | AssetType::Relationships
                | AssetType::DecisionIndex
                | AssetType::KnowledgeIndex
        )
    }

    /// Detect asset type from filename
    pub fn from_filename(filename: &str) -> Option<Self> {
        if filename == "workspace.yaml" {
            Some(AssetType::Workspace)
        } else if filename == "relationships.yaml" {
            Some(AssetType::Relationships)
        } else if filename == "decisions.yaml" {
            Some(AssetType::DecisionIndex)
        } else if filename == "knowledge.yaml" {
            Some(AssetType::KnowledgeIndex)
        } else if filename.ends_with(".odcs.yaml") {
            Some(AssetType::Odcs)
        } else if filename.ends_with(".odps.yaml") {
            Some(AssetType::Odps)
        } else if filename.ends_with(".cads.yaml") {
            Some(AssetType::Cads)
        } else if filename.ends_with(".madr.yaml") {
            Some(AssetType::Decision)
        } else if filename.ends_with(".kb.yaml") {
            Some(AssetType::Knowledge)
        } else if filename.ends_with(".bpmn.xml") {
            Some(AssetType::Bpmn)
        } else if filename.ends_with(".dmn.xml") {
            Some(AssetType::Dmn)
        } else if filename.ends_with(".openapi.yaml") || filename.ends_with(".openapi.json") {
            Some(AssetType::Openapi)
        } else {
            None
        }
    }

    /// Get all supported file extensions
    pub fn supported_extensions() -> &'static [&'static str] {
        &[
            "workspace.yaml",
            "relationships.yaml",
            "decisions.yaml",
            "knowledge.yaml",
            ".odcs.yaml",
            ".odps.yaml",
            ".cads.yaml",
            ".madr.yaml",
            ".kb.yaml",
            ".bpmn.xml",
            ".dmn.xml",
            ".openapi.yaml",
            ".openapi.json",
        ]
    }

    /// Check if a filename is a supported asset type
    pub fn is_supported_file(filename: &str) -> bool {
        Self::from_filename(filename).is_some()
    }
}

/// Domain reference within a workspace
///
/// Contains information about a domain and its systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DomainReference {
    /// Domain identifier
    pub id: Uuid,
    /// Domain name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Systems within this domain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systems: Vec<SystemReference>,
    /// View positions for different view modes (operational, analytical, process, systems)
    /// Key: view mode name, Value: Map of entity ID to position
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub view_positions: HashMap<String, HashMap<String, ViewPosition>>,
}

/// System reference within a domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemReference {
    /// System identifier
    pub id: Uuid,
    /// System name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional array of table UUIDs that belong to this system.
    /// When present, provides explicit table-to-system mapping without requiring parsing of individual ODCS files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_ids: Vec<Uuid>,
    /// Optional array of compute asset (CADS) UUIDs that belong to this system.
    /// When present, provides explicit asset-to-system mapping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asset_ids: Vec<Uuid>,
}

/// Workspace - Top-level container for domains, assets, and relationships
///
/// Workspaces organize domains, systems, and their associated assets.
/// All files use a flat naming convention: `{workspace}_{domain}_{system}_{resource}.xxx.yaml`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    /// Unique identifier for the workspace
    pub id: Uuid,
    /// Workspace name (used in file naming)
    pub name: String,
    /// Owner/creator user identifier
    pub owner_id: Uuid,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub last_modified_at: DateTime<Utc>,
    /// Domain references with their systems
    #[serde(default)]
    pub domains: Vec<DomainReference>,
    /// All asset references in this workspace
    #[serde(default)]
    pub assets: Vec<AssetReference>,
    /// Relationships between assets in this workspace
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<Relationship>,
}

impl Workspace {
    /// Create a new Workspace
    pub fn new(name: String, owner_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            owner_id,
            created_at: now,
            last_modified_at: now,
            domains: Vec::new(),
            assets: Vec::new(),
            relationships: Vec::new(),
        }
    }

    /// Create a workspace with a specific ID
    pub fn with_id(id: Uuid, name: String, owner_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            owner_id,
            created_at: now,
            last_modified_at: now,
            domains: Vec::new(),
            assets: Vec::new(),
            relationships: Vec::new(),
        }
    }

    /// Add a relationship to the workspace
    pub fn add_relationship(&mut self, relationship: Relationship) {
        // Check if relationship already exists
        if self.relationships.iter().any(|r| r.id == relationship.id) {
            return;
        }
        self.relationships.push(relationship);
        self.last_modified_at = Utc::now();
    }

    /// Remove a relationship by ID
    pub fn remove_relationship(&mut self, relationship_id: Uuid) -> bool {
        let initial_len = self.relationships.len();
        self.relationships.retain(|r| r.id != relationship_id);
        let removed = self.relationships.len() < initial_len;
        if removed {
            self.last_modified_at = Utc::now();
        }
        removed
    }

    /// Get relationships by source table ID
    pub fn get_relationships_for_source(&self, source_table_id: Uuid) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| r.source_table_id == source_table_id)
            .collect()
    }

    /// Get relationships by target table ID
    pub fn get_relationships_for_target(&self, target_table_id: Uuid) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| r.target_table_id == target_table_id)
            .collect()
    }

    /// Add a domain reference to the workspace
    pub fn add_domain(&mut self, domain_id: Uuid, domain_name: String) {
        // Check if domain already exists
        if self.domains.iter().any(|d| d.id == domain_id) {
            return;
        }
        self.domains.push(DomainReference {
            id: domain_id,
            name: domain_name,
            description: None,
            systems: Vec::new(),
            view_positions: HashMap::new(),
        });
        self.last_modified_at = Utc::now();
    }

    /// Add a domain with description
    pub fn add_domain_with_description(
        &mut self,
        domain_id: Uuid,
        domain_name: String,
        description: Option<String>,
    ) {
        if self.domains.iter().any(|d| d.id == domain_id) {
            return;
        }
        self.domains.push(DomainReference {
            id: domain_id,
            name: domain_name,
            description,
            systems: Vec::new(),
            view_positions: HashMap::new(),
        });
        self.last_modified_at = Utc::now();
    }

    /// Add a system to a domain
    pub fn add_system_to_domain(
        &mut self,
        domain_name: &str,
        system_id: Uuid,
        system_name: String,
        description: Option<String>,
    ) -> bool {
        if let Some(domain) = self.domains.iter_mut().find(|d| d.name == domain_name)
            && !domain.systems.iter().any(|s| s.id == system_id)
        {
            domain.systems.push(SystemReference {
                id: system_id,
                name: system_name,
                description,
                table_ids: Vec::new(),
                asset_ids: Vec::new(),
            });
            self.last_modified_at = Utc::now();
            return true;
        }
        false
    }

    /// Remove a domain reference by ID
    pub fn remove_domain(&mut self, domain_id: Uuid) -> bool {
        let initial_len = self.domains.len();
        self.domains.retain(|d| d.id != domain_id);
        // Also remove assets belonging to this domain
        if let Some(domain) = self.domains.iter().find(|d| d.id == domain_id) {
            let domain_name = domain.name.clone();
            self.assets.retain(|a| a.domain != domain_name);
        }
        if self.domains.len() != initial_len {
            self.last_modified_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Get a domain reference by ID
    pub fn get_domain(&self, domain_id: Uuid) -> Option<&DomainReference> {
        self.domains.iter().find(|d| d.id == domain_id)
    }

    /// Get a domain reference by name
    pub fn get_domain_by_name(&self, name: &str) -> Option<&DomainReference> {
        self.domains.iter().find(|d| d.name == name)
    }

    /// Add an asset reference
    pub fn add_asset(&mut self, asset: AssetReference) {
        // Check if asset already exists
        if self.assets.iter().any(|a| a.id == asset.id) {
            return;
        }
        self.assets.push(asset);
        self.last_modified_at = Utc::now();
    }

    /// Remove an asset by ID
    pub fn remove_asset(&mut self, asset_id: Uuid) -> bool {
        let initial_len = self.assets.len();
        self.assets.retain(|a| a.id != asset_id);
        if self.assets.len() != initial_len {
            self.last_modified_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Get an asset by ID
    pub fn get_asset(&self, asset_id: Uuid) -> Option<&AssetReference> {
        self.assets.iter().find(|a| a.id == asset_id)
    }

    /// Get assets by domain
    pub fn get_assets_by_domain(&self, domain_name: &str) -> Vec<&AssetReference> {
        self.assets
            .iter()
            .filter(|a| a.domain == domain_name)
            .collect()
    }

    /// Get assets by type
    pub fn get_assets_by_type(&self, asset_type: &AssetType) -> Vec<&AssetReference> {
        self.assets
            .iter()
            .filter(|a| &a.asset_type == asset_type)
            .collect()
    }

    /// Generate filename for an asset using the naming convention
    /// Format: {workspace}_{domain}_{system}_{resource}.{extension}
    pub fn generate_asset_filename(&self, asset: &AssetReference) -> String {
        let mut parts = vec![sanitize_name(&self.name), sanitize_name(&asset.domain)];

        if let Some(ref system) = asset.system {
            parts.push(sanitize_name(system));
        }

        parts.push(sanitize_name(&asset.name));

        format!("{}.{}", parts.join("_"), asset.asset_type.extension())
    }

    /// Parse a filename to extract workspace, domain, system, and resource names
    /// Returns (domain, system, resource_name) or None if parsing fails
    pub fn parse_asset_filename(
        filename: &str,
    ) -> Option<(String, Option<String>, String, AssetType)> {
        // Determine asset type from extension
        let (base, asset_type) = if filename.ends_with(".odcs.yaml") {
            (filename.strip_suffix(".odcs.yaml")?, AssetType::Odcs)
        } else if filename.ends_with(".odps.yaml") {
            (filename.strip_suffix(".odps.yaml")?, AssetType::Odps)
        } else if filename.ends_with(".cads.yaml") {
            (filename.strip_suffix(".cads.yaml")?, AssetType::Cads)
        } else if filename.ends_with(".bpmn.xml") {
            (filename.strip_suffix(".bpmn.xml")?, AssetType::Bpmn)
        } else if filename.ends_with(".dmn.xml") {
            (filename.strip_suffix(".dmn.xml")?, AssetType::Dmn)
        } else if filename.ends_with(".openapi.yaml") {
            (filename.strip_suffix(".openapi.yaml")?, AssetType::Openapi)
        } else {
            return None;
        };

        let parts: Vec<&str> = base.split('_').collect();

        match parts.len() {
            // workspace_domain_resource (no system)
            3 => Some((parts[1].to_string(), None, parts[2].to_string(), asset_type)),
            // workspace_domain_system_resource
            4 => Some((
                parts[1].to_string(),
                Some(parts[2].to_string()),
                parts[3].to_string(),
                asset_type,
            )),
            // More than 4 parts - treat remaining as resource name with underscores
            n if n > 4 => Some((
                parts[1].to_string(),
                Some(parts[2].to_string()),
                parts[3..].join("_"),
                asset_type,
            )),
            _ => None,
        }
    }

    /// Import workspace from YAML
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }

    /// Export workspace to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Import workspace from JSON
    pub fn from_json(json_content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_content)
    }

    /// Export workspace to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Export workspace to pretty JSON
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Sanitize a name for use in filenames (replace spaces/special chars with hyphens)
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            ' ' | '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new("Default Workspace".to_string(), Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_new() {
        let owner_id = Uuid::new_v4();
        let workspace = Workspace::new("Test Workspace".to_string(), owner_id);

        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.owner_id, owner_id);
        assert!(workspace.domains.is_empty());
        assert!(workspace.assets.is_empty());
    }

    #[test]
    fn test_workspace_add_domain() {
        let mut workspace = Workspace::new("Test".to_string(), Uuid::new_v4());
        let domain_id = Uuid::new_v4();

        workspace.add_domain(domain_id, "customer-management".to_string());

        assert_eq!(workspace.domains.len(), 1);
        assert_eq!(workspace.domains[0].id, domain_id);
        assert_eq!(workspace.domains[0].name, "customer-management");

        // Adding same domain again should not duplicate
        workspace.add_domain(domain_id, "customer-management".to_string());
        assert_eq!(workspace.domains.len(), 1);
    }

    #[test]
    fn test_workspace_add_system_to_domain() {
        let mut workspace = Workspace::new("Test".to_string(), Uuid::new_v4());
        let domain_id = Uuid::new_v4();
        let system_id = Uuid::new_v4();

        workspace.add_domain(domain_id, "sales".to_string());
        let result = workspace.add_system_to_domain(
            "sales",
            system_id,
            "kafka".to_string(),
            Some("Kafka streaming".to_string()),
        );

        assert!(result);
        assert_eq!(workspace.domains[0].systems.len(), 1);
        assert_eq!(workspace.domains[0].systems[0].name, "kafka");
    }

    #[test]
    fn test_workspace_remove_domain() {
        let mut workspace = Workspace::new("Test".to_string(), Uuid::new_v4());
        let domain_id = Uuid::new_v4();
        workspace.add_domain(domain_id, "test-domain".to_string());

        assert!(workspace.remove_domain(domain_id));
        assert!(workspace.domains.is_empty());
        assert!(!workspace.remove_domain(domain_id)); // Already removed
    }

    #[test]
    fn test_workspace_add_asset() {
        let mut workspace = Workspace::new("enterprise".to_string(), Uuid::new_v4());
        let asset_id = Uuid::new_v4();

        let asset = AssetReference {
            id: asset_id,
            name: "orders".to_string(),
            domain: "sales".to_string(),
            system: Some("kafka".to_string()),
            asset_type: AssetType::Odcs,
            file_path: None,
        };

        workspace.add_asset(asset);
        assert_eq!(workspace.assets.len(), 1);
        assert_eq!(workspace.assets[0].name, "orders");
    }

    #[test]
    fn test_workspace_generate_asset_filename() {
        let workspace = Workspace::new("enterprise".to_string(), Uuid::new_v4());

        // With system
        let asset_with_system = AssetReference {
            id: Uuid::new_v4(),
            name: "orders".to_string(),
            domain: "sales".to_string(),
            system: Some("kafka".to_string()),
            asset_type: AssetType::Odcs,
            file_path: None,
        };
        assert_eq!(
            workspace.generate_asset_filename(&asset_with_system),
            "enterprise_sales_kafka_orders.odcs.yaml"
        );

        // Without system
        let asset_no_system = AssetReference {
            id: Uuid::new_v4(),
            name: "customers".to_string(),
            domain: "crm".to_string(),
            system: None,
            asset_type: AssetType::Odcs,
            file_path: None,
        };
        assert_eq!(
            workspace.generate_asset_filename(&asset_no_system),
            "enterprise_crm_customers.odcs.yaml"
        );

        // ODPS product
        let odps_asset = AssetReference {
            id: Uuid::new_v4(),
            name: "analytics".to_string(),
            domain: "finance".to_string(),
            system: None,
            asset_type: AssetType::Odps,
            file_path: None,
        };
        assert_eq!(
            workspace.generate_asset_filename(&odps_asset),
            "enterprise_finance_analytics.odps.yaml"
        );
    }

    #[test]
    fn test_workspace_parse_asset_filename() {
        // With system
        let result = Workspace::parse_asset_filename("enterprise_sales_kafka_orders.odcs.yaml");
        assert!(result.is_some());
        let (domain, system, name, asset_type) = result.unwrap();
        assert_eq!(domain, "sales");
        assert_eq!(system, Some("kafka".to_string()));
        assert_eq!(name, "orders");
        assert_eq!(asset_type, AssetType::Odcs);

        // Without system (3 parts)
        let result = Workspace::parse_asset_filename("enterprise_crm_customers.odcs.yaml");
        assert!(result.is_some());
        let (domain, system, name, asset_type) = result.unwrap();
        assert_eq!(domain, "crm");
        assert_eq!(system, None);
        assert_eq!(name, "customers");
        assert_eq!(asset_type, AssetType::Odcs);

        // ODPS type
        let result = Workspace::parse_asset_filename("workspace_domain_product.odps.yaml");
        assert!(result.is_some());
        let (_, _, _, asset_type) = result.unwrap();
        assert_eq!(asset_type, AssetType::Odps);
    }

    #[test]
    fn test_workspace_yaml_roundtrip() {
        let mut workspace = Workspace::new("Enterprise Models".to_string(), Uuid::new_v4());
        workspace.add_domain(Uuid::new_v4(), "finance".to_string());
        workspace.add_domain(Uuid::new_v4(), "risk".to_string());
        workspace.add_asset(AssetReference {
            id: Uuid::new_v4(),
            name: "accounts".to_string(),
            domain: "finance".to_string(),
            system: None,
            asset_type: AssetType::Odcs,
            file_path: None,
        });

        let yaml = workspace.to_yaml().unwrap();
        let parsed = Workspace::from_yaml(&yaml).unwrap();

        assert_eq!(workspace.id, parsed.id);
        assert_eq!(workspace.name, parsed.name);
        assert_eq!(workspace.domains.len(), parsed.domains.len());
        assert_eq!(workspace.assets.len(), parsed.assets.len());
    }

    #[test]
    fn test_workspace_json_roundtrip() {
        let workspace = Workspace::new("Test".to_string(), Uuid::new_v4());

        let json = workspace.to_json().unwrap();
        let parsed = Workspace::from_json(&json).unwrap();

        assert_eq!(workspace.id, parsed.id);
        assert_eq!(workspace.name, parsed.name);
    }

    #[test]
    fn test_asset_type_extension() {
        assert_eq!(AssetType::Workspace.extension(), "yaml");
        assert_eq!(AssetType::Relationships.extension(), "yaml");
        assert_eq!(AssetType::Odcs.extension(), "odcs.yaml");
        assert_eq!(AssetType::Odps.extension(), "odps.yaml");
        assert_eq!(AssetType::Cads.extension(), "cads.yaml");
        assert_eq!(AssetType::Bpmn.extension(), "bpmn.xml");
        assert_eq!(AssetType::Dmn.extension(), "dmn.xml");
        assert_eq!(AssetType::Openapi.extension(), "openapi.yaml");
    }

    #[test]
    fn test_asset_type_filename() {
        assert_eq!(AssetType::Workspace.filename(), Some("workspace.yaml"));
        assert_eq!(
            AssetType::Relationships.filename(),
            Some("relationships.yaml")
        );
        assert_eq!(AssetType::Odcs.filename(), None);
    }

    #[test]
    fn test_asset_type_from_filename() {
        assert_eq!(
            AssetType::from_filename("workspace.yaml"),
            Some(AssetType::Workspace)
        );
        assert_eq!(
            AssetType::from_filename("relationships.yaml"),
            Some(AssetType::Relationships)
        );
        assert_eq!(
            AssetType::from_filename("test.odcs.yaml"),
            Some(AssetType::Odcs)
        );
        assert_eq!(
            AssetType::from_filename("test.odps.yaml"),
            Some(AssetType::Odps)
        );
        assert_eq!(
            AssetType::from_filename("test.cads.yaml"),
            Some(AssetType::Cads)
        );
        assert_eq!(
            AssetType::from_filename("test.bpmn.xml"),
            Some(AssetType::Bpmn)
        );
        assert_eq!(
            AssetType::from_filename("test.dmn.xml"),
            Some(AssetType::Dmn)
        );
        assert_eq!(
            AssetType::from_filename("test.openapi.yaml"),
            Some(AssetType::Openapi)
        );
        assert_eq!(
            AssetType::from_filename("test.openapi.json"),
            Some(AssetType::Openapi)
        );
        assert_eq!(AssetType::from_filename("random.txt"), None);
        assert_eq!(AssetType::from_filename("test.yaml"), None);
    }

    #[test]
    fn test_asset_type_is_supported_file() {
        assert!(AssetType::is_supported_file("workspace.yaml"));
        assert!(AssetType::is_supported_file("relationships.yaml"));
        assert!(AssetType::is_supported_file(
            "enterprise_sales_orders.odcs.yaml"
        ));
        assert!(!AssetType::is_supported_file("readme.md"));
        assert!(!AssetType::is_supported_file("config.json"));
    }

    #[test]
    fn test_asset_type_is_workspace_level() {
        assert!(AssetType::Workspace.is_workspace_level());
        assert!(AssetType::Relationships.is_workspace_level());
        assert!(!AssetType::Odcs.is_workspace_level());
        assert!(!AssetType::Odps.is_workspace_level());
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("Hello World"), "hello-world");
        assert_eq!(sanitize_name("Test/Path"), "test-path");
        assert_eq!(sanitize_name("Normal"), "normal");
    }
}
