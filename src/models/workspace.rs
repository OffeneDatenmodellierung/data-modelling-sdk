//! Workspace model
//!
//! Defines the Workspace entity for the data modelling application.
//! Workspaces are top-level containers that organize domains and their associated assets.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domain reference within a workspace
///
/// Contains minimal information about a domain to avoid regenerating UUIDs on each load.
/// Full domain details are stored in separate domain.yaml files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainReference {
    /// Domain identifier
    pub id: Uuid,
    /// Domain name (must match folder name in offline mode)
    pub name: String,
}

/// Workspace - Top-level container for domains
///
/// Workspaces organize domains and their associated assets.
/// In offline mode, each workspace corresponds to a directory containing domain folders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    /// Unique identifier for the workspace
    pub id: Uuid,
    /// Workspace name
    pub name: String,
    /// Owner/creator user identifier
    pub owner_id: Uuid,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub last_modified_at: DateTime<Utc>,
    /// Domain references
    #[serde(default)]
    pub domains: Vec<DomainReference>,
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
        }
    }

    /// Add a domain reference to the workspace
    pub fn add_domain(&mut self, domain_id: Uuid, domain_name: String) {
        self.domains.push(DomainReference {
            id: domain_id,
            name: domain_name,
        });
        self.last_modified_at = Utc::now();
    }

    /// Remove a domain reference by ID
    pub fn remove_domain(&mut self, domain_id: Uuid) -> bool {
        let initial_len = self.domains.len();
        self.domains.retain(|d| d.id != domain_id);
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
    }

    #[test]
    fn test_workspace_add_domain() {
        let mut workspace = Workspace::new("Test".to_string(), Uuid::new_v4());
        let domain_id = Uuid::new_v4();

        workspace.add_domain(domain_id, "customer-management".to_string());

        assert_eq!(workspace.domains.len(), 1);
        assert_eq!(workspace.domains[0].id, domain_id);
        assert_eq!(workspace.domains[0].name, "customer-management");
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
    fn test_workspace_yaml_roundtrip() {
        let mut workspace = Workspace::new("Enterprise Models".to_string(), Uuid::new_v4());
        workspace.add_domain(Uuid::new_v4(), "finance".to_string());
        workspace.add_domain(Uuid::new_v4(), "risk".to_string());

        let yaml = workspace.to_yaml().unwrap();
        let parsed = Workspace::from_yaml(&yaml).unwrap();

        assert_eq!(workspace.id, parsed.id);
        assert_eq!(workspace.name, parsed.name);
        assert_eq!(workspace.domains.len(), parsed.domains.len());
    }

    #[test]
    fn test_workspace_json_roundtrip() {
        let workspace = Workspace::new("Test".to_string(), Uuid::new_v4());

        let json = workspace.to_json().unwrap();
        let parsed = Workspace::from_json(&json).unwrap();

        assert_eq!(workspace.id, parsed.id);
        assert_eq!(workspace.name, parsed.name);
    }
}
