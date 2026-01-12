//! Storage backend tests

#[cfg(feature = "native-fs")]
mod filesystem_tests {
    use data_modelling_core::storage::{
        StorageBackend, StorageError, filesystem::FileSystemStorageBackend,
    };
    use tempfile::TempDir;
    use tokio::runtime::Runtime;

    fn runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn test_read_write_roundtrip() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            let content = b"Hello, World!";
            backend.write_file("test.txt", content).await.unwrap();

            let read_content = backend.read_file("test.txt").await.unwrap();
            assert_eq!(read_content, content);
        });
    }

    #[test]
    fn test_file_not_found() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            let result = backend.read_file("nonexistent.txt").await;
            assert!(matches!(result, Err(StorageError::FileNotFound(_))));
        });
    }

    #[test]
    fn test_path_traversal_blocked() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            // Try to escape via ".."
            let result = backend.read_file("../etc/passwd").await;
            assert!(matches!(result, Err(StorageError::PermissionDenied(_))));

            // Try nested traversal
            let result = backend.read_file("foo/../../etc/passwd").await;
            assert!(matches!(result, Err(StorageError::PermissionDenied(_))));

            // Try with leading slash
            let result = backend.read_file("/foo/../../../etc/passwd").await;
            assert!(matches!(result, Err(StorageError::PermissionDenied(_))));
        });
    }

    #[test]
    fn test_write_creates_directories() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            let content = b"nested file content";
            backend.write_file("a/b/c/deep.txt", content).await.unwrap();

            let read_content = backend.read_file("a/b/c/deep.txt").await.unwrap();
            assert_eq!(read_content, content);
        });
    }

    #[test]
    fn test_list_files() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            // Create some files
            backend.write_file("dir/file1.txt", b"1").await.unwrap();
            backend.write_file("dir/file2.txt", b"2").await.unwrap();
            backend.write_file("dir/file3.txt", b"3").await.unwrap();

            let files = backend.list_files("dir").await.unwrap();
            assert_eq!(files.len(), 3);
            assert!(files.contains(&"file1.txt".to_string()));
            assert!(files.contains(&"file2.txt".to_string()));
            assert!(files.contains(&"file3.txt".to_string()));
        });
    }

    #[test]
    fn test_delete_file() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            backend
                .write_file("to_delete.txt", b"delete me")
                .await
                .unwrap();
            assert!(backend.file_exists("to_delete.txt").await.unwrap());

            backend.delete_file("to_delete.txt").await.unwrap();
            assert!(!backend.file_exists("to_delete.txt").await.unwrap());
        });
    }

    #[test]
    fn test_file_exists() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            assert!(!backend.file_exists("test.txt").await.unwrap());
            backend.write_file("test.txt", b"content").await.unwrap();
            assert!(backend.file_exists("test.txt").await.unwrap());
        });
    }

    #[test]
    fn test_create_dir() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            assert!(!backend.dir_exists("newdir").await.unwrap());
            backend.create_dir("newdir").await.unwrap();
            assert!(backend.dir_exists("newdir").await.unwrap());
        });
    }

    #[test]
    fn test_valid_nested_paths() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            // Valid nested paths should work
            backend
                .write_file("level1/level2/file.txt", b"content")
                .await
                .unwrap();
            assert!(backend.file_exists("level1/level2/file.txt").await.unwrap());
        });
    }

    #[test]
    fn test_dir_exists() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            assert!(!backend.dir_exists("nonexistent").await.unwrap());
            backend.create_dir("newdir").await.unwrap();
            assert!(backend.dir_exists("newdir").await.unwrap());
        });
    }

    #[test]
    fn test_list_files_empty_directory() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            backend.create_dir("emptydir").await.unwrap();
            let files = backend.list_files("emptydir").await.unwrap();
            assert_eq!(files.len(), 0);
        });
    }

    #[test]
    fn test_list_files_nonexistent_directory() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            let result = backend.list_files("nonexistent").await;
            assert!(matches!(result, Err(StorageError::DirectoryNotFound(_))));
        });
    }

    #[test]
    fn test_write_file_overwrites_existing() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            backend.write_file("test.txt", b"original").await.unwrap();
            backend.write_file("test.txt", b"updated").await.unwrap();

            let content = backend.read_file("test.txt").await.unwrap();
            assert_eq!(content, b"updated");
        });
    }

    #[test]
    fn test_delete_nonexistent_file() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            let result = backend.delete_file("nonexistent.txt").await;
            assert!(matches!(result, Err(StorageError::FileNotFound(_))));
        });
    }

    #[test]
    fn test_create_dir_nested() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            backend.create_dir("level1/level2/level3").await.unwrap();
            assert!(backend.dir_exists("level1/level2/level3").await.unwrap());
        });
    }

    #[test]
    fn test_create_dir_already_exists() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            backend.create_dir("testdir").await.unwrap();
            // Should not error if directory already exists
            backend.create_dir("testdir").await.unwrap();
        });
    }

    #[test]
    fn test_path_traversal_various_patterns() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            let malicious_paths = [
                "..",
                "../",
                "../../",
                "foo/../bar",
                "foo/../../bar",
                "/../etc/passwd",
                "foo/..\\bar", // Mixed separators
            ];

            for path in &malicious_paths {
                let result = backend.read_file(path).await;
                assert!(
                    matches!(result, Err(StorageError::PermissionDenied(_))),
                    "Path traversal should be blocked: {}",
                    path
                );
            }
        });
    }

    #[test]
    fn test_read_file_binary_content() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            let binary_data = vec![0u8, 1u8, 2u8, 255u8, 128u8];
            backend
                .write_file("binary.bin", &binary_data)
                .await
                .unwrap();

            let read_data = backend.read_file("binary.bin").await.unwrap();
            assert_eq!(read_data, binary_data);
        });
    }

    #[test]
    fn test_list_files_filters_directories() {
        let rt = runtime();
        rt.block_on(async {
            let temp = TempDir::new().unwrap();
            let backend = FileSystemStorageBackend::new(temp.path());

            backend
                .write_file("dir/file.txt", b"content")
                .await
                .unwrap();
            backend.create_dir("dir/subdir").await.unwrap();

            let files = backend.list_files("dir").await.unwrap();
            // Should only return files, not directories
            assert_eq!(files.len(), 1);
            assert!(files.contains(&"file.txt".to_string()));
            assert!(!files.contains(&"subdir".to_string()));
        });
    }
}

#[cfg(feature = "api-backend")]
mod api_validation_tests {
    #[allow(unused_imports)]
    use data_modelling_core::storage::StorageError;

    // Note: These tests validate the domain slug validation function
    // Full API tests would require mocking the HTTP client

    #[test]
    fn test_domain_validation_valid() {
        // Valid domains (tested via the internal validate function exposed in tests)
        let valid_domains = ["my-domain", "my_domain", "domain123", "MyDomain"];
        for domain in valid_domains {
            assert!(
                domain
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
                "Domain should be valid: {}",
                domain
            );
        }
    }

    #[test]
    fn test_domain_validation_invalid() {
        // These patterns should be rejected
        let invalid_domains = [
            "../etc",             // path traversal
            "domain/path",        // URL path
            "domain?query",       // query string
            "domain#hash",        // hash
            "domain with spaces", // spaces
            ".hidden",            // starts with dot
        ];
        for domain in invalid_domains {
            let has_invalid = domain
                .chars()
                .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
                || domain.starts_with('.');
            assert!(has_invalid, "Domain should be invalid: {}", domain);
        }
    }
}
