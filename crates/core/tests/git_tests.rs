//! Git service tests

#[cfg(feature = "git")]
mod git_service_tests {
    use data_modelling_core::git::{GitCredentials, GitService};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_git_service_new() {
        let service = GitService::new();
        assert!(service.repository().is_none());
        assert!(service.git_directory().is_none());
    }

    #[test]
    fn test_git_service_default() {
        let service = GitService::default();
        assert!(service.repository().is_none());
        assert!(service.git_directory().is_none());
    }

    #[test]
    fn test_open_or_init_new_repository() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();

        // Should initialize a new repository
        service.open_or_init(&git_dir).unwrap();

        assert!(service.repository().is_some());
        assert_eq!(service.git_directory(), Some(&git_dir));
        assert!(git_dir.join(".git").exists());
    }

    #[test]
    fn test_open_or_init_existing_repository() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service1 = GitService::new();

        // Initialize repository
        service1.open_or_init(&git_dir).unwrap();

        // Open existing repository with new service
        let mut service2 = GitService::new();
        service2.open_or_init(&git_dir).unwrap();

        assert!(service2.repository().is_some());
        assert_eq!(service2.git_directory(), Some(&git_dir));
    }

    #[test]
    fn test_stage_files_empty() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create a file
        let file_path = git_dir.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        // Stage all files (empty paths means stage all)
        service.stage_files(&[]).unwrap();

        // Verify file is staged
        let status = service.status().unwrap();
        assert!(status.staged_files.contains(&"test.txt".to_string()));
    }

    #[test]
    fn test_stage_files_specific() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create files
        fs::write(git_dir.join("file1.txt"), "content1").unwrap();
        fs::write(git_dir.join("file2.txt"), "content2").unwrap();

        // Stage only file1
        service.stage_files(&["file1.txt"]).unwrap();

        let status = service.status().unwrap();
        assert!(status.staged_files.contains(&"file1.txt".to_string()));
        // file2 should be untracked, not staged
        assert!(!status.staged_files.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_stage_files_repository_not_opened() {
        let service = GitService::new();
        let result = service.stage_files(&["file.txt"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_commit() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create and stage a file
        fs::write(git_dir.join("test.txt"), "content").unwrap();
        service.stage_files(&["test.txt"]).unwrap();

        // Commit
        service
            .commit("Test commit", "Test User", "test@example.com")
            .unwrap();

        // Verify commit was created
        let repo = service.repository().unwrap();
        let head = repo.head().unwrap();
        assert!(head.peel_to_commit().is_ok());
    }

    #[test]
    fn test_commit_repository_not_opened() {
        let service = GitService::new();
        let result = service.commit("message", "author", "email");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_clean_repository() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Initial commit to have a clean state
        fs::write(git_dir.join("README.md"), "readme").unwrap();
        service.stage_files(&["README.md"]).unwrap();
        service
            .commit("Initial commit", "Test", "test@example.com")
            .unwrap();

        let status = service.status().unwrap();
        assert!(!status.has_changes);
        assert!(status.staged_files.is_empty());
        assert!(status.unstaged_files.is_empty());
        assert!(status.untracked_files.is_empty());
    }

    #[test]
    fn test_status_with_changes() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create initial commit
        fs::write(git_dir.join("file.txt"), "original").unwrap();
        service.stage_files(&["file.txt"]).unwrap();
        service
            .commit("Initial", "Test", "test@example.com")
            .unwrap();

        // Make changes
        fs::write(git_dir.join("file.txt"), "modified").unwrap(); // Modified
        fs::write(git_dir.join("new.txt"), "new").unwrap(); // New file

        let status = service.status().unwrap();
        assert!(status.has_changes);
        assert!(status.unstaged_files.contains(&"file.txt".to_string()));
        assert!(status.untracked_files.contains(&"new.txt".to_string()));
    }

    #[test]
    fn test_status_repository_not_opened() {
        let service = GitService::new();
        let result = service.status();
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_all() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create files
        fs::write(git_dir.join("file1.txt"), "content1").unwrap();
        fs::write(git_dir.join("file2.txt"), "content2").unwrap();

        // Commit all
        service
            .commit_all("Commit all", "Test", "test@example.com")
            .unwrap();

        let status = service.status().unwrap();
        assert!(!status.has_changes);
    }

    #[test]
    fn test_git_status_fields() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create and stage a file
        fs::write(git_dir.join("staged.txt"), "staged").unwrap();
        service.stage_files(&["staged.txt"]).unwrap();

        // Create a file, stage it, then modify it to make it unstaged
        fs::write(git_dir.join("unstaged.txt"), "original").unwrap();
        service.stage_files(&["unstaged.txt"]).unwrap();
        fs::write(git_dir.join("unstaged.txt"), "modified").unwrap();

        // Create an untracked file
        fs::write(git_dir.join("untracked.txt"), "untracked").unwrap();

        let status = service.status().unwrap();
        assert!(status.has_changes);
        assert!(status.staged_files.contains(&"staged.txt".to_string()));
        assert!(status.unstaged_files.contains(&"unstaged.txt".to_string()));
        assert!(
            status
                .untracked_files
                .contains(&"untracked.txt".to_string())
        );
    }

    #[test]
    fn test_git_credentials_default() {
        let creds = GitCredentials::default();
        assert!(creds.ssh_key_path.is_none());
        assert!(creds.username.is_none());
        assert!(creds.token.is_none());
    }

    #[test]
    fn test_git_service_with_credentials() {
        let creds = GitCredentials {
            username: Some("testuser".to_string()),
            token: Some("testtoken".to_string()),
            ssh_key_path: None,
        };
        let service = GitService::with_credentials(creds);
        assert!(service.repository().is_none());
    }

    #[test]
    fn test_set_credentials() {
        let mut service = GitService::new();
        let creds = GitCredentials {
            username: Some("user".to_string()),
            token: Some("pass".to_string()),
            ssh_key_path: None,
        };
        service.set_credentials(creds);
        // Credentials are stored internally, can't easily test without clone/push
    }

    #[test]
    fn test_set_remote() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Set remote
        service
            .set_remote("origin", "https://example.com/repo.git")
            .unwrap();

        // Verify remote was set
        let repo = service.repository().unwrap();
        let remote = repo.find_remote("origin").unwrap();
        assert_eq!(remote.url(), Some("https://example.com/repo.git"));
    }

    #[test]
    fn test_set_remote_repository_not_opened() {
        let mut service = GitService::new();
        let result = service.set_remote("origin", "https://example.com/repo.git");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_conflicts_no_conflicts() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create and commit a file
        fs::write(git_dir.join("file.txt"), "content").unwrap();
        service.stage_files(&["file.txt"]).unwrap();
        service
            .commit("Initial", "Test", "test@example.com")
            .unwrap();

        // No conflicts
        assert!(!service.has_conflicts().unwrap());
    }

    #[test]
    fn test_has_conflicts_repository_not_opened() {
        let service = GitService::new();
        let result = service.has_conflicts();
        assert!(result.is_err());
    }

    #[test]
    fn test_remote_status_no_remote() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("repo");
        let mut service = GitService::new();
        service.open_or_init(&git_dir).unwrap();

        // Create initial commit
        fs::write(git_dir.join("file.txt"), "content").unwrap();
        service.stage_files(&["file.txt"]).unwrap();
        service
            .commit("Initial", "Test", "test@example.com")
            .unwrap();

        // No remote configured, should return (false, false)
        let (unpushed, unpulled) = service.remote_status("origin", "main").unwrap();
        assert!(!unpushed);
        assert!(!unpulled);
    }

    #[test]
    fn test_remote_status_repository_not_opened() {
        let service = GitService::new();
        let result = service.remote_status("origin", "main");
        assert!(result.is_err());
    }

    // Note: Tests for clone_repository, fetch, and pull would require
    // actual remote repositories or mocking, which is more complex.
    // These are better suited for integration tests.
}
