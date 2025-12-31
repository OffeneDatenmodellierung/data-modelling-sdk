//! Comprehensive tests for auth module

use data_modelling_sdk::auth::{
    AuthMode, AuthState, GitHubEmail, InitiateOAuthRequest, InitiateOAuthResponse,
    SelectEmailRequest,
};

mod auth_mode_tests {
    use super::*;

    #[test]
    fn test_auth_mode_default() {
        let mode = AuthMode::default();
        assert_eq!(mode, AuthMode::None);
    }

    #[test]
    fn test_auth_mode_web() {
        let mode = AuthMode::Web;
        assert_eq!(mode, AuthMode::Web);
    }

    #[test]
    fn test_auth_mode_local() {
        let mode = AuthMode::Local;
        assert_eq!(mode, AuthMode::Local);
    }

    #[test]
    fn test_auth_mode_online() {
        let mode = AuthMode::Online {
            api_url: "https://api.example.com".to_string(),
        };
        match mode {
            AuthMode::Online { api_url } => assert_eq!(api_url, "https://api.example.com"),
            _ => panic!("Expected Online mode"),
        }
    }

    #[test]
    fn test_auth_mode_serialization() {
        let modes = vec![
            AuthMode::None,
            AuthMode::Web,
            AuthMode::Local,
            AuthMode::Online {
                api_url: "http://localhost:8080".to_string(),
            },
        ];
        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: AuthMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, parsed);
        }
    }

    #[test]
    fn test_auth_mode_partial_eq() {
        let mode1 = AuthMode::Web;
        let mode2 = AuthMode::Web;
        assert_eq!(mode1, mode2);

        let mode3 = AuthMode::Online {
            api_url: "http://localhost".to_string(),
        };
        let mode4 = AuthMode::Online {
            api_url: "http://localhost".to_string(),
        };
        assert_eq!(mode3, mode4);
    }
}

mod auth_state_tests {
    use super::*;

    #[test]
    fn test_auth_state_default() {
        let state = AuthState::default();
        assert!(!state.authenticated);
        assert_eq!(state.mode, AuthMode::None);
        assert_eq!(state.auth_source, "web");
        assert!(state.email.is_none());
        assert!(state.available_emails.is_none());
        assert!(state.github_token.is_none());
        assert!(state.api_url.is_none());
    }

    #[test]
    fn test_auth_state_web_mode() {
        let state = AuthState {
            mode: AuthMode::Web,
            authenticated: true,
            email: Some("user@example.com".to_string()),
            available_emails: None,
            github_token: Some("token123".to_string()),
            api_url: None,
            auth_source: "web".to_string(),
        };
        assert!(state.authenticated);
        assert_eq!(state.mode, AuthMode::Web);
        assert_eq!(state.email, Some("user@example.com".to_string()));
    }

    #[test]
    fn test_auth_state_online_mode() {
        let state = AuthState {
            mode: AuthMode::Online {
                api_url: "http://localhost:8080".to_string(),
            },
            authenticated: true,
            email: Some("user@example.com".to_string()),
            available_emails: None,
            github_token: None,
            api_url: Some("http://localhost:8080".to_string()),
            auth_source: "desktop".to_string(),
        };
        assert!(state.authenticated);
        match state.mode {
            AuthMode::Online { api_url } => assert_eq!(api_url, "http://localhost:8080"),
            _ => panic!("Expected Online mode"),
        }
    }

    #[test]
    fn test_auth_state_serialization() {
        let state = AuthState {
            mode: AuthMode::Online {
                api_url: "http://localhost:8080".to_string(),
            },
            authenticated: true,
            email: Some("test@example.com".to_string()),
            available_emails: Some(vec![GitHubEmail {
                email: "test@example.com".to_string(),
                verified: true,
                primary: true,
            }]),
            github_token: None,
            api_url: Some("http://localhost:8080".to_string()),
            auth_source: "desktop".to_string(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: AuthState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, parsed);
    }

    #[test]
    fn test_auth_state_with_github_emails() {
        let state = AuthState {
            mode: AuthMode::Web,
            authenticated: false,
            email: None,
            available_emails: Some(vec![
                GitHubEmail {
                    email: "user@example.com".to_string(),
                    verified: true,
                    primary: true,
                },
                GitHubEmail {
                    email: "user+work@example.com".to_string(),
                    verified: true,
                    primary: false,
                },
            ]),
            github_token: Some("token123".to_string()),
            api_url: None,
            auth_source: "web".to_string(),
        };
        assert_eq!(state.available_emails.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_auth_state_partial_eq() {
        let state1 = AuthState {
            mode: AuthMode::Web,
            authenticated: true,
            email: Some("test@example.com".to_string()),
            available_emails: None,
            github_token: None,
            api_url: None,
            auth_source: "web".to_string(),
        };
        let state2 = AuthState {
            mode: AuthMode::Web,
            authenticated: true,
            email: Some("test@example.com".to_string()),
            available_emails: None,
            github_token: None,
            api_url: None,
            auth_source: "web".to_string(),
        };
        assert_eq!(state1, state2);
    }
}

mod github_email_tests {
    use super::*;

    #[test]
    fn test_github_email_new() {
        let email = GitHubEmail {
            email: "user@example.com".to_string(),
            verified: true,
            primary: true,
        };
        assert_eq!(email.email, "user@example.com");
        assert!(email.verified);
        assert!(email.primary);
    }

    #[test]
    fn test_github_email_serialization() {
        let email = GitHubEmail {
            email: "test@example.com".to_string(),
            verified: true,
            primary: false,
        };
        let json = serde_json::to_string(&email).unwrap();
        let parsed: GitHubEmail = serde_json::from_str(&json).unwrap();
        assert_eq!(email, parsed);
    }

    #[test]
    fn test_github_email_partial_eq() {
        let email1 = GitHubEmail {
            email: "test@example.com".to_string(),
            verified: true,
            primary: true,
        };
        let email2 = GitHubEmail {
            email: "test@example.com".to_string(),
            verified: true,
            primary: true,
        };
        assert_eq!(email1, email2);
    }
}

mod oauth_tests {
    use super::*;

    #[test]
    fn test_initiate_oauth_request() {
        let req = InitiateOAuthRequest {
            redirect_uri: "http://localhost:3000/callback".to_string(),
            source: "web".to_string(),
        };
        assert_eq!(req.redirect_uri, "http://localhost:3000/callback");
        assert_eq!(req.source, "web");
    }

    #[test]
    fn test_initiate_oauth_request_serialization() {
        let req = InitiateOAuthRequest {
            redirect_uri: "http://localhost/callback".to_string(),
            source: "desktop".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: InitiateOAuthRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.redirect_uri, parsed.redirect_uri);
        assert_eq!(req.source, parsed.source);
    }

    #[test]
    fn test_initiate_oauth_response() {
        let resp = InitiateOAuthResponse {
            oauth_url: "https://github.com/login/oauth/authorize?client_id=123".to_string(),
            state: "random_state_string".to_string(),
        };
        assert!(resp.oauth_url.contains("github.com"));
        assert_eq!(resp.state, "random_state_string");
    }

    #[test]
    fn test_initiate_oauth_response_serialization() {
        let resp = InitiateOAuthResponse {
            oauth_url: "https://github.com/oauth".to_string(),
            state: "state123".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: InitiateOAuthResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.oauth_url, parsed.oauth_url);
        assert_eq!(resp.state, parsed.state);
    }

    #[test]
    fn test_select_email_request() {
        let req = SelectEmailRequest {
            email: "user@example.com".to_string(),
        };
        assert_eq!(req.email, "user@example.com");
    }

    #[test]
    fn test_select_email_request_serialization() {
        let req = SelectEmailRequest {
            email: "test@example.com".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: SelectEmailRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.email, parsed.email);
    }
}
