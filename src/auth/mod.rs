//! Authentication types shared across all platforms
//!
//! These types are used by:
//! - Web app (WASM) - via server functions
//! - Desktop app - via local state and remote API client
//! - API server - for session management

use serde::{Deserialize, Serialize};

/// Authentication mode
///
/// Determines how the application authenticates and where data is stored.
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::auth::AuthMode;
///
/// // Web mode (GitHub SSO)
/// let web_mode = AuthMode::Web;
///
/// // Online mode (remote API)
/// let online_mode = AuthMode::Online {
///     api_url: "https://api.example.com".to_string(),
/// };
///
/// // Local mode (offline)
/// let local_mode = AuthMode::Local;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMode {
    /// Not yet selected (desktop/mobile only)
    None,
    /// Web platform - GitHub SSO required
    Web,
    /// Local mode - works offline with local files
    Local,
    /// Online mode - connects to remote API server
    Online { api_url: String },
}

impl Default for AuthMode {
    fn default() -> Self {
        Self::None
    }
}

/// GitHub email information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubEmail {
    pub email: String,
    pub verified: bool,
    pub primary: bool,
}

/// Current authentication state
///
/// Tracks the current authentication status and available credentials.
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::auth::{AuthState, AuthMode};
///
/// let state = AuthState {
///     mode: AuthMode::Web,
///     authenticated: true,
///     email: Some("user@example.com".to_string()),
///     available_emails: None,
///     github_token: Some("token123".to_string()),
///     api_url: None,
///     auth_source: "web".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthState {
    /// Current authentication mode
    pub mode: AuthMode,
    /// Whether the user is currently authenticated
    pub authenticated: bool,
    /// Selected email address (if authenticated)
    pub email: Option<String>,
    /// Available GitHub emails (for selection during OAuth)
    pub available_emails: Option<Vec<GitHubEmail>>,
    /// GitHub OAuth token (if authenticated via GitHub)
    pub github_token: Option<String>,
    /// API URL (for online mode)
    pub api_url: Option<String>,
    /// Source of auth flow: "web", "desktop", or "mobile"
    #[serde(default = "default_auth_source")]
    pub auth_source: String,
}

fn default_auth_source() -> String {
    "web".to_string()
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            mode: AuthMode::None,
            authenticated: false,
            email: None,
            available_emails: None,
            github_token: None,
            api_url: None,
            auth_source: "web".to_string(),
        }
    }
}

/// OAuth initiation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateOAuthRequest {
    pub redirect_uri: String,
    pub source: String,
}

/// OAuth initiation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateOAuthResponse {
    pub oauth_url: String,
    pub state: String,
}

/// Email selection request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectEmailRequest {
    pub email: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_mode_default() {
        let mode = AuthMode::default();
        assert_eq!(mode, AuthMode::None);
    }

    #[test]
    fn test_auth_state_default() {
        let state = AuthState::default();
        assert!(!state.authenticated);
        assert_eq!(state.mode, AuthMode::None);
        assert_eq!(state.auth_source, "web");
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
}
