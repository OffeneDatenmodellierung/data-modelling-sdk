//! File system storage backend
//!
//! Implements StorageBackend for native file system operations.
//! Used by native macOS/Windows/Linux apps.
//!
//! ## Security
//!
//! All path operations are validated to prevent path traversal attacks.
//! Paths containing ".." are rejected, and all resolved paths are verified
//! to remain within the base directory.

use super::{StorageBackend, StorageError};
use async_trait::async_trait;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
// Note: tracing imports are available for future use
#[allow(unused_imports)]
use tracing::{info, warn};

/// File system storage backend
pub struct FileSystemStorageBackend {
    base_path: PathBuf,
}

impl FileSystemStorageBackend {
    /// Create a new file system storage backend
    ///
    /// # Arguments
    ///
    /// * `base_path` - Base directory path for all file operations
    ///
    /// # Security
    ///
    /// All file operations are restricted to the base path. Path traversal attempts
    /// (containing "..") are rejected.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::storage::filesystem::FileSystemStorageBackend;
    ///
    /// let backend = FileSystemStorageBackend::new("/workspace/data");
    /// ```
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Resolve a path relative to the base path with security checks.
    ///
    /// # Security
    ///
    /// - Rejects paths containing ".." components
    /// - Verifies the resolved path stays within base_path
    /// - Handles both existing and non-existing paths safely
    fn resolve_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        // Normalize: strip leading slashes
        let normalized = path.trim_start_matches('/');

        // Check for path traversal attempts in the input
        if normalized.contains("..") {
            return Err(StorageError::PermissionDenied(
                "Path traversal (..) not allowed".to_string(),
            ));
        }

        // Build the full path
        let full = self.base_path.join(normalized);

        // Validate each component to catch edge cases
        for component in full.components() {
            if matches!(component, Component::ParentDir) {
                return Err(StorageError::PermissionDenied(
                    "Path traversal not allowed".to_string(),
                ));
            }
        }

        // For existing paths, canonicalize and verify containment
        if full.exists() {
            let canonical = full
                .canonicalize()
                .map_err(|e| StorageError::IoError(format!("Failed to resolve path: {}", e)))?;

            let base_canonical = self
                .base_path
                .canonicalize()
                .unwrap_or_else(|_| self.base_path.clone());

            if !canonical.starts_with(&base_canonical) {
                return Err(StorageError::PermissionDenied(
                    "Path escapes base directory".to_string(),
                ));
            }

            return Ok(canonical);
        }

        // For non-existing paths, check that the parent is valid
        if let Some(parent) = full.parent()
            && parent.exists()
        {
            let parent_canonical = parent.canonicalize().map_err(|e| {
                StorageError::IoError(format!("Failed to resolve parent path: {}", e))
            })?;

            let base_canonical = self
                .base_path
                .canonicalize()
                .unwrap_or_else(|_| self.base_path.clone());

            if !parent_canonical.starts_with(&base_canonical) {
                return Err(StorageError::PermissionDenied(
                    "Path escapes base directory".to_string(),
                ));
            }
        }

        // Return the non-canonicalized path for non-existing files
        // (write operations will create it within the validated structure)
        Ok(full)
    }
}

#[async_trait(?Send)]
impl StorageBackend for FileSystemStorageBackend {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let full_path = self.resolve_path(path)?;

        fs::read(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::FileNotFound(path.to_string())
            } else {
                StorageError::IoError(format!("Failed to read file {}: {}", path, e))
            }
        })
    }

    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::IoError(format!("Failed to create directory for {}: {}", path, e))
            })?;
        }

        fs::write(&full_path, content)
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to write file {}: {}", path, e)))
    }

    async fn list_files(&self, dir: &str) -> Result<Vec<String>, StorageError> {
        let full_path = self.resolve_path(dir)?;

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::DirectoryNotFound(dir.to_string())
            } else {
                StorageError::IoError(format!("Failed to read directory {}: {}", dir, e))
            }
        })?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to read directory entry: {}", e)))?
        {
            if let Ok(file_type) = entry.file_type().await
                && file_type.is_file()
                && let Some(file_name) = entry.file_name().to_str()
            {
                entries.push(file_name.to_string());
            }
        }

        Ok(entries)
    }

    async fn file_exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path)?;

        match fs::metadata(&full_path).await {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(StorageError::IoError(format!(
                        "Failed to check file existence {}: {}",
                        path, e
                    )))
                }
            }
        }
    }

    async fn delete_file(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path)?;

        fs::remove_file(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::FileNotFound(path.to_string())
            } else {
                StorageError::IoError(format!("Failed to delete file {}: {}", path, e))
            }
        })
    }

    async fn create_dir(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path)?;

        fs::create_dir_all(&full_path).await.map_err(|e| {
            StorageError::IoError(format!("Failed to create directory {}: {}", path, e))
        })
    }

    async fn dir_exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path)?;

        match fs::metadata(&full_path).await {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(StorageError::IoError(format!(
                        "Failed to check directory existence {}: {}",
                        path, e
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_path_traversal_blocked() {
        let temp = TempDir::new().unwrap();
        let backend = FileSystemStorageBackend::new(temp.path());

        // Test ".." in path
        let result = backend.resolve_path("../etc/passwd");
        assert!(matches!(result, Err(StorageError::PermissionDenied(_))));

        // Test with leading slash and ".."
        let result = backend.resolve_path("/foo/../../../etc/passwd");
        assert!(matches!(result, Err(StorageError::PermissionDenied(_))));

        // Test valid paths work
        let result = backend.resolve_path("valid/path/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_paths_allowed() {
        let temp = TempDir::new().unwrap();
        let backend = FileSystemStorageBackend::new(temp.path());

        // Simple file
        let result = backend.resolve_path("file.txt");
        assert!(result.is_ok());

        // Nested path
        let result = backend.resolve_path("foo/bar/baz.txt");
        assert!(result.is_ok());

        // With leading slash (should be stripped)
        let result = backend.resolve_path("/file.txt");
        assert!(result.is_ok());
    }
}
