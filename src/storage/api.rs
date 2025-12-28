//! API storage backend
//! 
//! Implements StorageBackend for HTTP API operations.
//! Used for online mode (default).

use super::{StorageBackend, StorageError};
use async_trait::async_trait;
use serde_json;

/// API storage backend that communicates with HTTP API
pub struct ApiStorageBackend {
    base_url: String,
    session_id: Option<String>,
    client: reqwest::Client,
}

impl ApiStorageBackend {
    /// Create a new API storage backend
    pub fn new(base_url: impl Into<String>, session_id: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            session_id,
            client: reqwest::Client::new(),
        }
    }

    /// Build a request with authentication headers
    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, &url);
        
        if let Some(ref session_id) = self.session_id {
            request = request.header("x-session-id", session_id);
        }
        
        request
    }

    /// Get workspace info to check if workspace exists
    pub async fn get_workspace_info(&self) -> Result<WorkspaceInfo, StorageError> {
        let response = self
            .build_request(reqwest::Method::GET, "/workspace/info")
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to get workspace info: {}", e)))?;

        if !response.status().is_success() {
            return Err(StorageError::BackendError(format!(
                "Workspace info request failed: {}",
                response.status()
            )));
        }

        let info: WorkspaceInfo = response
            .json()
            .await
            .map_err(|e| StorageError::SerializationError(format!("Failed to parse workspace info: {}", e)))?;

        Ok(info)
    }

    /// Load tables from API
    pub async fn load_tables(&self) -> Result<Vec<serde_json::Value>, StorageError> {
        let response = self
            .build_request(reqwest::Method::GET, "/tables")
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to load tables: {}", e)))?;

        if !response.status().is_success() {
            return Err(StorageError::BackendError(format!(
                "Load tables request failed: {}",
                response.status()
            )));
        }

        let tables: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| StorageError::SerializationError(format!("Failed to parse tables: {}", e)))?;

        Ok(tables)
    }

    /// Load relationships from API
    pub async fn load_relationships(&self) -> Result<Vec<serde_json::Value>, StorageError> {
        let response = self
            .build_request(reqwest::Method::GET, "/relationships")
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to load relationships: {}", e)))?;

        if !response.status().is_success() {
            return Err(StorageError::BackendError(format!(
                "Load relationships request failed: {}",
                response.status()
            )));
        }

        let relationships: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| StorageError::SerializationError(format!("Failed to parse relationships: {}", e)))?;

        Ok(relationships)
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
            "Direct file reading not supported in API backend. Use load_model() instead.".to_string(),
        ))
    }

    async fn write_file(&self, _path: &str, _content: &[u8]) -> Result<(), StorageError> {
        // For API backend, file writing is done through model endpoints
        // Direct file writing not supported - use save_table() or save_relationships() instead
        Err(StorageError::BackendError(
            "Direct file writing not supported in API backend. Use save_table() or save_relationships() instead.".to_string(),
        ))
    }

    async fn list_files(&self, _dir: &str) -> Result<Vec<String>, StorageError> {
        // For API backend, file listing might be done through git/subfolders endpoint
        // This could be implemented if needed
        Err(StorageError::BackendError(
            "File listing not yet implemented in API backend".to_string(),
        ))
    }

    async fn file_exists(&self, _path: &str) -> Result<bool, StorageError> {
        // Check file existence via API
        // This could be implemented by checking workspace info or model endpoints
        Ok(false)
    }

    async fn delete_file(&self, _path: &str) -> Result<(), StorageError> {
        // Delete file via API
        // This could be implemented via DELETE endpoints
        Err(StorageError::BackendError(
            "File deletion not yet implemented in API backend".to_string(),
        ))
    }

    async fn create_dir(&self, _path: &str) -> Result<(), StorageError> {
        // Create directory via API
        // Workspace creation is handled separately via workspace endpoints
        Err(StorageError::BackendError(
            "Directory creation not supported in API backend. Use workspace creation endpoints instead.".to_string(),
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
                .map_err(|e| StorageError::NetworkError(format!("Failed to check directory: {}", e)))?;
            
            Ok(response.status().is_success())
        }
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, API backend shouldn't be used - return error
            Err(StorageError::BackendError(
                "API backend not supported in WASM. Use browser storage backend instead.".to_string(),
            ))
        }
    }
}
