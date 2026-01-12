//! Databricks Unity Catalog Volumes ingestion support
//!
//! This module provides file discovery and ingestion from Unity Catalog Volumes.

use std::path::PathBuf;

use super::error::IngestError;
use super::ingest::DiscoveredFile;

/// Unity Catalog Volume source configuration
#[derive(Debug, Clone)]
pub struct UnityVolumeSource {
    /// Databricks workspace URL (e.g., https://xxx.cloud.databricks.com)
    pub workspace_url: String,
    /// Catalog name
    pub catalog: String,
    /// Schema name
    pub schema: String,
    /// Volume name
    pub volume: String,
    /// Path within the volume
    pub path: String,
    /// Authentication token
    token: Option<String>,
}

impl UnityVolumeSource {
    /// Create a new Unity Catalog Volume source
    pub fn new(
        workspace_url: impl Into<String>,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        volume: impl Into<String>,
    ) -> Self {
        Self {
            workspace_url: workspace_url.into(),
            catalog: catalog.into(),
            schema: schema.into(),
            volume: volume.into(),
            path: String::new(),
            token: None,
        }
    }

    /// Set the path within the volume
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set the authentication token
    ///
    /// The token is stored securely and never logged.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Get the display string for this source (without sensitive data)
    pub fn display(&self) -> String {
        format!(
            "dbfs:/Volumes/{}/{}/{}/{}",
            self.catalog, self.schema, self.volume, self.path
        )
    }

    /// Get the full volume path
    pub fn volume_path(&self) -> String {
        format!("/Volumes/{}/{}/{}", self.catalog, self.schema, self.volume)
    }

    /// Get the API endpoint for file operations
    pub fn files_api_endpoint(&self) -> String {
        format!(
            "{}/api/2.0/fs/files",
            self.workspace_url.trim_end_matches('/')
        )
    }

    /// Check if token is set
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }
}

// Implement Display to prevent accidental token logging
impl std::fmt::Display for UnityVolumeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Unity Catalog Volume ingester
pub struct UnityVolumeIngester {
    source: UnityVolumeSource,
    client: reqwest::Client,
}

impl UnityVolumeIngester {
    /// Create a new Unity Volume ingester
    pub fn new(source: UnityVolumeSource) -> Result<Self, IngestError> {
        if source.token.is_none() {
            return Err(IngestError::SourceNotAccessible {
                path: source.display(),
                reason: "Authentication token is required".to_string(),
            });
        }

        let client = reqwest::Client::new();

        Ok(Self { source, client })
    }

    /// Create from environment variables
    ///
    /// Reads DATABRICKS_HOST and DATABRICKS_TOKEN from environment.
    pub fn from_env(
        catalog: impl Into<String>,
        schema: impl Into<String>,
        volume: impl Into<String>,
    ) -> Result<Self, IngestError> {
        let host =
            std::env::var("DATABRICKS_HOST").map_err(|_| IngestError::SourceNotAccessible {
                path: "Unity Catalog".to_string(),
                reason: "DATABRICKS_HOST environment variable not set".to_string(),
            })?;

        let token =
            std::env::var("DATABRICKS_TOKEN").map_err(|_| IngestError::SourceNotAccessible {
                path: "Unity Catalog".to_string(),
                reason: "DATABRICKS_TOKEN environment variable not set".to_string(),
            })?;

        let source = UnityVolumeSource::new(host, catalog, schema, volume).with_token(token);

        Self::new(source)
    }

    /// Discover files matching a pattern in the volume
    ///
    /// # Arguments
    /// * `pattern` - Glob pattern to match (e.g., "*.json", "data/*.jsonl")
    pub async fn discover_files(&self, pattern: &str) -> Result<Vec<DiscoveredFile>, IngestError> {
        let mut files = Vec::new();

        // Compile the glob pattern
        let glob_pattern = glob::Pattern::new(pattern)
            .map_err(|e| IngestError::InvalidPattern(format!("{}: {}", pattern, e)))?;

        // List files in the volume path
        let list_path = format!(
            "{}/{}",
            self.source.volume_path(),
            self.source.path.trim_start_matches('/')
        );

        let response = self.list_directory(&list_path).await?;

        // Filter files by pattern
        for entry in response {
            if !entry.is_directory {
                let filename = entry.name.clone();
                if glob_pattern.matches(&filename) {
                    files.push(DiscoveredFile::new(
                        PathBuf::from(&entry.path),
                        entry.file_size as u64,
                    ));
                }
            }
        }

        // Sort by path for consistent ordering
        files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(files)
    }

    /// List files in a directory
    async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>, IngestError> {
        let url = format!(
            "{}/api/2.0/fs/directories{}",
            self.source.workspace_url.trim_end_matches('/'),
            path
        );

        let token = self
            .source
            .token
            .as_ref()
            .ok_or_else(|| IngestError::SourceNotAccessible {
                path: self.source.display(),
                reason: "No authentication token".to_string(),
            })?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| IngestError::SourceNotAccessible {
                path: self.source.display(),
                reason: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(IngestError::SourceNotAccessible {
                path: self.source.display(),
                reason: format!("HTTP {}: {}", status, body),
            });
        }

        let result: DirectoryListResponse =
            response
                .json()
                .await
                .map_err(|e| IngestError::SourceNotAccessible {
                    path: self.source.display(),
                    reason: format!("Failed to parse response: {}", e),
                })?;

        Ok(result.contents.unwrap_or_default())
    }

    /// Download a file from the volume
    pub async fn download_file(&self, path: &str) -> Result<Vec<u8>, IngestError> {
        let url = format!(
            "{}/api/2.0/fs/files{}",
            self.source.workspace_url.trim_end_matches('/'),
            path
        );

        let token = self
            .source
            .token
            .as_ref()
            .ok_or_else(|| IngestError::SourceNotAccessible {
                path: self.source.display(),
                reason: "No authentication token".to_string(),
            })?;

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| IngestError::SourceNotAccessible {
                path: path.to_string(),
                reason: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(IngestError::SourceNotAccessible {
                path: path.to_string(),
                reason: format!("HTTP {}: {}", status, body),
            });
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| IngestError::Io(std::io::Error::other(e.to_string())))?;

        Ok(bytes.to_vec())
    }

    /// Get the source configuration
    pub fn source(&self) -> &UnityVolumeSource {
        &self.source
    }
}

/// Directory listing response from Databricks API
#[derive(Debug, serde::Deserialize)]
struct DirectoryListResponse {
    contents: Option<Vec<FileInfo>>,
}

/// File info from Databricks API
#[derive(Debug, serde::Deserialize)]
struct FileInfo {
    path: String,
    name: String,
    is_directory: bool,
    file_size: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unity_source_display() {
        let source = UnityVolumeSource::new(
            "https://workspace.cloud.databricks.com",
            "main",
            "default",
            "raw_data",
        )
        .with_path("json/2024");

        assert_eq!(
            source.display(),
            "dbfs:/Volumes/main/default/raw_data/json/2024"
        );
    }

    #[test]
    fn test_unity_source_volume_path() {
        let source = UnityVolumeSource::new(
            "https://workspace.cloud.databricks.com",
            "main",
            "staging",
            "ingest",
        );

        assert_eq!(source.volume_path(), "/Volumes/main/staging/ingest");
    }

    #[test]
    fn test_unity_source_secure_display() {
        let source = UnityVolumeSource::new(
            "https://workspace.cloud.databricks.com",
            "main",
            "default",
            "data",
        )
        .with_token("dapi123secret456");

        // Display should not contain the token
        let display = format!("{}", source);
        assert!(!display.contains("dapi123"));
        assert!(!display.contains("secret"));
    }
}
