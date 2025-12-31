//! Storage backend abstraction
//!
//! Defines the StorageBackend trait and implementations for different storage systems:
//! - FileSystemStorageBackend: Native file system (for native apps)
//! - BrowserStorageBackend: Browser storage APIs (for WASM apps)
//! - ApiStorageBackend: HTTP API (for online mode, default)

use async_trait::async_trait;

/// Error type for storage operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Storage backend error: {0}")]
    BackendError(String),
}

/// Trait for storage backends
///
/// This trait abstracts file operations, directory operations, and model-specific operations
/// across different storage systems (file system, browser storage, HTTP API).
#[async_trait(?Send)]
pub trait StorageBackend: Send + Sync {
    /// Read a file from storage
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError>;

    /// Write a file to storage
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError>;

    /// List files in a directory
    async fn list_files(&self, dir: &str) -> Result<Vec<String>, StorageError>;

    /// Check if a file exists
    async fn file_exists(&self, path: &str) -> Result<bool, StorageError>;

    /// Delete a file
    async fn delete_file(&self, path: &str) -> Result<(), StorageError>;

    /// Create a directory
    async fn create_dir(&self, path: &str) -> Result<(), StorageError>;

    /// Check if a directory exists
    async fn dir_exists(&self, path: &str) -> Result<bool, StorageError>;
}

// Storage backend implementations
#[cfg(feature = "native-fs")]
pub mod filesystem;

#[cfg(feature = "api-backend")]
pub mod api;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub mod browser;
