//! Workspace types shared across all platforms
//!
//! These types are used for:
//! - Workspace management (profiles, domains)
//! - Data organization ({email}/{domain}/ structure)

use serde::{Deserialize, Serialize};

/// Workspace information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub model_id: String,
    pub name: String,
    pub git_directory_path: Option<String>,
    pub email: Option<String>,
    pub domain: Option<String>,
}

impl Default for WorkspaceInfo {
    fn default() -> Self {
        Self {
            model_id: "default".to_string(),
            name: "Default Workspace".to_string(),
            git_directory_path: None,
            email: None,
            domain: None,
        }
    }
}

/// Profile information (user profile with domains)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileInfo {
    pub email: String,
    pub domains: Vec<String>,
}

/// Request to create a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub email: String,
    pub domain: String,
}

/// Response after creating a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceResponse {
    pub success: bool,
    pub workspace: Option<WorkspaceInfo>,
    pub error: Option<String>,
}

/// List profiles response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProfilesResponse {
    pub profiles: Vec<ProfileInfo>,
}

/// Load profile request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadProfileRequest {
    pub domain: String,
    pub email: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_info_default() {
        let info = WorkspaceInfo::default();
        assert_eq!(info.model_id, "default");
        assert_eq!(info.name, "Default Workspace");
    }

    #[test]
    fn test_profile_info_serialization() {
        let profile = ProfileInfo {
            email: "test@example.com".to_string(),
            domains: vec!["Risk".to_string(), "Finance".to_string()],
        };

        let json = serde_json::to_string(&profile).unwrap();
        let parsed: ProfileInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, parsed);
    }
}

