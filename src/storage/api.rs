//! API storage backend
//!
//! Implements StorageBackend for HTTP API operations.
//! Used for online mode (default).
//!
//! ## Security
//!
//! All domain parameters are validated to prevent injection attacks.
//! Only alphanumeric characters, hyphens, and underscores are allowed.

use super::{StorageBackend, StorageError};
use async_trait::async_trait;
use serde_json;

/// Maximum allowed length for domain slugs
const MAX_DOMAIN_LENGTH: usize = 100;

/// Validate a domain slug for safe use in API paths.
///
/// # Security
///
/// This function ensures domain names cannot contain:
/// - Path traversal sequences
/// - URL injection characters
/// - Excessively long values
///
/// Only alphanumeric characters, hyphens, and underscores are allowed.
fn validate_domain_slug(domain: &str) -> Result<(), StorageError> {
    if domain.is_empty() {
        return Err(StorageError::BackendError(
            "Domain name cannot be empty".to_string(),
        ));
    }

    if domain.len() > MAX_DOMAIN_LENGTH {
        return Err(StorageError::BackendError(format!(
            "Domain name too long (max {} characters)",
            MAX_DOMAIN_LENGTH
        )));
    }

    // Only allow safe characters: alphanumeric, hyphens, underscores
    if !domain
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(StorageError::BackendError(
            "Domain contains invalid characters. Only alphanumeric, hyphens, and underscores are allowed.".to_string()
        ));
    }

    // Prevent reserved patterns
    if domain == "." || domain == ".." || domain.starts_with('.') {
        return Err(StorageError::BackendError(
            "Domain name cannot start with a period".to_string(),
        ));
    }

    Ok(())
}

/// API storage backend that communicates with HTTP API
pub struct ApiStorageBackend {
    base_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
}

impl ApiStorageBackend {
    /// Create a new API storage backend
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of the API server (e.g., "https://api.example.com/api/v1")
    /// * `auth_token` - Optional bearer token for authentication
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::storage::api::ApiStorageBackend;
    ///
    /// let backend = ApiStorageBackend::new(
    ///     "https://api.example.com/api/v1",
    ///     Some("bearer_token_here".to_string()),
    /// );
    /// ```
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            auth_token,
            client: reqwest::Client::new(),
        }
    }

    /// Build a request with authentication headers
    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, &url);

        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        request
    }

    /// Get workspace info to check if workspace exists
    ///
    /// # Returns
    ///
    /// `WorkspaceInfo` if the workspace exists, or an error if not found or network error occurs.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_modelling_sdk::storage::api::ApiStorageBackend;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let backend = ApiStorageBackend::new("http://localhost:8080/api/v1", None);
    /// # let info = backend.get_workspace_info().await?;
    /// # println!("Workspace: {}", info.workspace_path);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_workspace_info(&self) -> Result<WorkspaceInfo, StorageError> {
        let response = self
            .build_request(reqwest::Method::GET, "/workspace/info")
            .send()
            .await
            .map_err(|e| {
                StorageError::NetworkError(format!("Failed to get workspace info: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(StorageError::BackendError(format!(
                "Workspace info request failed: {}",
                response.status()
            )));
        }

        let info: WorkspaceInfo = response.json().await.map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse workspace info: {}", e))
        })?;

        Ok(info)
    }

    /// Load tables from API
    ///
    /// # Security
    ///
    /// The domain parameter is validated to prevent injection attacks.
    pub async fn load_tables(&self, domain: &str) -> Result<Vec<serde_json::Value>, StorageError> {
        // Validate domain slug for security
        validate_domain_slug(domain)?;

        let encoded_domain = urlencoding::encode(domain);
        let response = self
            .build_request(
                reqwest::Method::GET,
                &format!("/workspace/domains/{}/tables", encoded_domain),
            )
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to load tables: {}", e)))?;

        if !response.status().is_success() {
            return Err(StorageError::BackendError(format!(
                "Load tables request failed: {}",
                response.status()
            )));
        }

        let tables: Vec<serde_json::Value> = response.json().await.map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse tables: {}", e))
        })?;

        Ok(tables)
    }

    /// Load relationships from API
    ///
    /// # Security
    ///
    /// The domain parameter is validated to prevent injection attacks.
    pub async fn load_relationships(
        &self,
        domain: &str,
    ) -> Result<Vec<serde_json::Value>, StorageError> {
        // Validate domain slug for security
        validate_domain_slug(domain)?;

        let encoded_domain = urlencoding::encode(domain);
        let response = self
            .build_request(
                reqwest::Method::GET,
                &format!("/workspace/domains/{}/relationships", encoded_domain),
            )
            .send()
            .await
            .map_err(|e| {
                StorageError::NetworkError(format!("Failed to load relationships: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(StorageError::BackendError(format!(
                "Load relationships request failed: {}",
                response.status()
            )));
        }

        let relationships: Vec<serde_json::Value> = response.json().await.map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse relationships: {}", e))
        })?;

        Ok(relationships)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain_slug_valid() {
        assert!(validate_domain_slug("my-domain").is_ok());
        assert!(validate_domain_slug("my_domain").is_ok());
        assert!(validate_domain_slug("domain123").is_ok());
        assert!(validate_domain_slug("MyDomain").is_ok());
    }

    #[test]
    fn test_validate_domain_slug_empty() {
        let result = validate_domain_slug("");
        assert!(matches!(result, Err(StorageError::BackendError(_))));
    }

    #[test]
    fn test_validate_domain_slug_too_long() {
        let long_domain = "a".repeat(101);
        let result = validate_domain_slug(&long_domain);
        assert!(matches!(result, Err(StorageError::BackendError(_))));
    }

    #[test]
    fn test_validate_domain_slug_invalid_chars() {
        // Path traversal
        assert!(validate_domain_slug("../etc").is_err());
        // Special characters
        assert!(validate_domain_slug("domain/path").is_err());
        assert!(validate_domain_slug("domain?query").is_err());
        assert!(validate_domain_slug("domain#hash").is_err());
        assert!(validate_domain_slug("domain with spaces").is_err());
    }

    #[test]
    fn test_validate_domain_slug_dot_patterns() {
        assert!(validate_domain_slug(".").is_err());
        assert!(validate_domain_slug("..").is_err());
        assert!(validate_domain_slug(".hidden").is_err());
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct WorkspaceInfo {
    pub workspace_path: String,
    pub email: String,
}

#[async_trait(?Send)]
impl StorageBackend for ApiStorageBackend {
    async fn read_file(&self, _path: &str) -> Result<Vec<u8>, StorageError> {
        // For API backend, file reading is done through model endpoints
        // Direct file reading not supported - use load_model() instead
        Err(StorageError::BackendError(
            "Direct file reading not supported in API backend. Use load_model() instead."
                .to_string(),
        ))
    }

    async fn write_file(&self, _path: &str, _content: &[u8]) -> Result<(), StorageError> {
        // For API backend, file writing is done through model endpoints
        // Direct file writing not supported - use save_table() or save_relationships() instead
        Err(StorageError::BackendError(
            "Direct file writing not supported in API backend. Use save_table() or save_relationships() instead.".to_string(),
        ))
    }

    /// List files in a directory.
    ///
    /// # Note
    ///
    /// This method is intentionally not supported in the API backend.
    /// The API uses a model-based approach where tables and relationships
    /// are accessed via dedicated endpoints rather than as files.
    ///
    /// Use `load_tables()` and `load_relationships()` instead.
    async fn list_files(&self, _dir: &str) -> Result<Vec<String>, StorageError> {
        Err(StorageError::BackendError(
            "File listing not supported in API backend. Use load_tables() or load_relationships() instead.".to_string(),
        ))
    }

    /// Check if a file exists.
    ///
    /// # Note
    ///
    /// File existence checks are not meaningful in the API backend.
    /// The API uses model endpoints - use `load_tables()` to check for tables.
    async fn file_exists(&self, _path: &str) -> Result<bool, StorageError> {
        // For API backend, we cannot check individual file existence
        // Return false to indicate the concept doesn't apply
        Ok(false)
    }

    /// Delete a file.
    ///
    /// # Note
    ///
    /// This method is intentionally not supported in the API backend.
    /// Use the API's table/relationship DELETE endpoints directly.
    async fn delete_file(&self, _path: &str) -> Result<(), StorageError> {
        Err(StorageError::BackendError(
            "File deletion not supported in API backend. Use dedicated table/relationship DELETE endpoints.".to_string(),
        ))
    }

    /// Create a directory.
    ///
    /// # Note
    ///
    /// Directory creation is not supported in the API backend.
    /// Workspaces and domains are created via dedicated API endpoints.
    async fn create_dir(&self, _path: &str) -> Result<(), StorageError> {
        Err(StorageError::BackendError(
            "Directory creation not supported in API backend. Use workspace/domain creation endpoints.".to_string(),
        ))
    }

    async fn dir_exists(&self, _path: &str) -> Result<bool, StorageError> {
        // Check directory existence via API
        // For API backend, directories are virtual - assume they exist if workspace is accessible
        // Use a simple HEAD request to check instead of get_workspace_info to avoid Send issues
        // Note: reqwest in WASM is not Send, but this is only used in non-WASM builds
        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = self
                .build_request(reqwest::Method::HEAD, "/workspace/info")
                .send()
                .await
                .map_err(|e| {
                    StorageError::NetworkError(format!("Failed to check directory: {}", e))
                })?;

            Ok(response.status().is_success())
        }
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, API backend shouldn't be used - return error
            Err(StorageError::BackendError(
                "API backend not supported in WASM. Use browser storage backend instead."
                    .to_string(),
            ))
        }
    }
}
