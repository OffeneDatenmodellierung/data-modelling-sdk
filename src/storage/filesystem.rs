//! File system storage backend
//! 
//! Implements StorageBackend for native file system operations.
//! Used by native macOS/Windows/Linux apps.

use super::{StorageBackend, StorageError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

/// File system storage backend
pub struct FileSystemStorageBackend {
    base_path: PathBuf,
}

impl FileSystemStorageBackend {
    /// Create a new file system storage backend
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Resolve a path relative to the base path
    fn resolve_path(&self, path: &str) -> PathBuf {
        if path.starts_with('/') {
            // Absolute path - use as-is (relative to base_path root)
            self.base_path.join(path.strip_prefix('/').unwrap_or(path))
        } else {
            // Relative path
            self.base_path.join(path)
        }
    }
}

#[async_trait(?Send)]
impl StorageBackend for FileSystemStorageBackend {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let full_path = self.resolve_path(path);
        
        fs::read(&full_path)
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    StorageError::FileNotFound(path.to_string())
                } else {
                    StorageError::IoError(format!("Failed to read file {}: {}", path, e))
                }
            })
    }

    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::IoError(format!("Failed to create directory for {}: {}", path, e)))?;
        }
        
        fs::write(&full_path, content)
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to write file {}: {}", path, e)))
    }

    async fn list_files(&self, dir: &str) -> Result<Vec<String>, StorageError> {
        let full_path = self.resolve_path(dir);
        
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&full_path)
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    StorageError::DirectoryNotFound(dir.to_string())
                } else {
                    StorageError::IoError(format!("Failed to read directory {}: {}", dir, e))
                }
            })?;
        
        while let Some(entry) = read_dir.next_entry().await
            .map_err(|e| StorageError::IoError(format!("Failed to read directory entry: {}", e)))?
        {
            if let Ok(file_type) = entry.file_type().await {
                if file_type.is_file() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        entries.push(file_name.to_string());
                    }
                }
            }
        }
        
        Ok(entries)
    }

    async fn file_exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path);
        
        match fs::metadata(&full_path).await {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(StorageError::IoError(format!("Failed to check file existence {}: {}", path, e)))
                }
            }
        }
    }

    async fn delete_file(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path);
        
        fs::remove_file(&full_path)
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    StorageError::FileNotFound(path.to_string())
                } else {
                    StorageError::IoError(format!("Failed to delete file {}: {}", path, e))
                }
            })
    }

    async fn create_dir(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path);
        
        fs::create_dir_all(&full_path)
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to create directory {}: {}", path, e)))
    }

    async fn dir_exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path);
        
        match fs::metadata(&full_path).await {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(StorageError::IoError(format!("Failed to check directory existence {}: {}", path, e)))
                }
            }
        }
    }
}
