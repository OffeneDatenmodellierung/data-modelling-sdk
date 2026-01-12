//! Comprehensive tests for workspace module

use data_modelling_core::workspace::{
    CreateWorkspaceRequest, CreateWorkspaceResponse, ListProfilesResponse, LoadProfileRequest,
    ProfileInfo, WorkspaceInfo,
};

mod workspace_info_tests {
    use super::*;

    #[test]
    fn test_workspace_info_default() {
        let info = WorkspaceInfo::default();
        assert_eq!(info.model_id, "default");
        assert_eq!(info.name, "Default Workspace");
        assert!(info.git_directory_path.is_none());
        assert!(info.email.is_none());
        assert!(info.domain.is_none());
    }

    #[test]
    fn test_workspace_info_with_all_fields() {
        let info = WorkspaceInfo {
            model_id: "my-model".to_string(),
            name: "My Model".to_string(),
            git_directory_path: Some("/path/to/git".to_string()),
            email: Some("user@example.com".to_string()),
            domain: Some("finance".to_string()),
        };
        assert_eq!(info.model_id, "my-model");
        assert_eq!(info.name, "My Model");
        assert_eq!(info.git_directory_path, Some("/path/to/git".to_string()));
        assert_eq!(info.email, Some("user@example.com".to_string()));
        assert_eq!(info.domain, Some("finance".to_string()));
    }

    #[test]
    fn test_workspace_info_serialization() {
        let info = WorkspaceInfo {
            model_id: "test-model".to_string(),
            name: "Test Model".to_string(),
            git_directory_path: Some("/path".to_string()),
            email: Some("test@example.com".to_string()),
            domain: Some("test".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: WorkspaceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, parsed);
    }

    #[test]
    fn test_workspace_info_partial_eq() {
        let info1 = WorkspaceInfo {
            model_id: "test".to_string(),
            name: "Test".to_string(),
            git_directory_path: None,
            email: None,
            domain: None,
        };
        let info2 = WorkspaceInfo {
            model_id: "test".to_string(),
            name: "Test".to_string(),
            git_directory_path: None,
            email: None,
            domain: None,
        };
        assert_eq!(info1, info2);
    }

    #[test]
    fn test_workspace_info_without_optional_fields() {
        let info = WorkspaceInfo {
            model_id: "minimal".to_string(),
            name: "Minimal Workspace".to_string(),
            git_directory_path: None,
            email: None,
            domain: None,
        };
        assert_eq!(info.model_id, "minimal");
        assert!(info.git_directory_path.is_none());
    }
}

mod profile_info_tests {
    use super::*;

    #[test]
    fn test_profile_info_new() {
        let profile = ProfileInfo {
            email: "user@example.com".to_string(),
            domains: vec!["finance".to_string(), "risk".to_string()],
        };
        assert_eq!(profile.email, "user@example.com");
        assert_eq!(profile.domains.len(), 2);
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

    #[test]
    fn test_profile_info_with_empty_domains() {
        let profile = ProfileInfo {
            email: "user@example.com".to_string(),
            domains: vec![],
        };
        assert_eq!(profile.domains.len(), 0);
    }

    #[test]
    fn test_profile_info_with_multiple_domains() {
        let profile = ProfileInfo {
            email: "user@example.com".to_string(),
            domains: vec![
                "finance".to_string(),
                "risk".to_string(),
                "compliance".to_string(),
                "operations".to_string(),
            ],
        };
        assert_eq!(profile.domains.len(), 4);
    }

    #[test]
    fn test_profile_info_partial_eq() {
        let profile1 = ProfileInfo {
            email: "test@example.com".to_string(),
            domains: vec!["domain1".to_string()],
        };
        let profile2 = ProfileInfo {
            email: "test@example.com".to_string(),
            domains: vec!["domain1".to_string()],
        };
        assert_eq!(profile1, profile2);
    }
}

mod request_response_tests {
    use super::*;

    #[test]
    fn test_create_workspace_request() {
        let req = CreateWorkspaceRequest {
            email: "user@example.com".to_string(),
            domain: "finance".to_string(),
        };
        assert_eq!(req.email, "user@example.com");
        assert_eq!(req.domain, "finance");
    }

    #[test]
    fn test_create_workspace_request_serialization() {
        let req = CreateWorkspaceRequest {
            email: "test@example.com".to_string(),
            domain: "test".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: CreateWorkspaceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.email, parsed.email);
        assert_eq!(req.domain, parsed.domain);
    }

    #[test]
    fn test_create_workspace_response_success() {
        let workspace = WorkspaceInfo {
            model_id: "new-model".to_string(),
            name: "New Model".to_string(),
            git_directory_path: None,
            email: None,
            domain: None,
        };
        let resp = CreateWorkspaceResponse {
            success: true,
            workspace: Some(workspace.clone()),
            error: None,
        };
        assert!(resp.success);
        assert!(resp.workspace.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_create_workspace_response_error() {
        let resp = CreateWorkspaceResponse {
            success: false,
            workspace: None,
            error: Some("Domain already exists".to_string()),
        };
        assert!(!resp.success);
        assert!(resp.workspace.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_create_workspace_response_serialization() {
        let resp = CreateWorkspaceResponse {
            success: true,
            workspace: Some(WorkspaceInfo::default()),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: CreateWorkspaceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.success, parsed.success);
    }

    #[test]
    fn test_list_profiles_response() {
        let resp = ListProfilesResponse {
            profiles: vec![
                ProfileInfo {
                    email: "user1@example.com".to_string(),
                    domains: vec!["finance".to_string()],
                },
                ProfileInfo {
                    email: "user2@example.com".to_string(),
                    domains: vec!["risk".to_string()],
                },
            ],
        };
        assert_eq!(resp.profiles.len(), 2);
    }

    #[test]
    fn test_list_profiles_response_serialization() {
        let resp = ListProfilesResponse {
            profiles: vec![ProfileInfo {
                email: "test@example.com".to_string(),
                domains: vec!["test".to_string()],
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ListProfilesResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.profiles.len(), parsed.profiles.len());
    }

    #[test]
    fn test_load_profile_request() {
        let req = LoadProfileRequest {
            domain: "finance".to_string(),
            email: "user@example.com".to_string(),
        };
        assert_eq!(req.domain, "finance");
        assert_eq!(req.email, "user@example.com");
    }

    #[test]
    fn test_load_profile_request_serialization() {
        let req = LoadProfileRequest {
            domain: "test".to_string(),
            email: "test@example.com".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: LoadProfileRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.domain, parsed.domain);
        assert_eq!(req.email, parsed.email);
    }
}
