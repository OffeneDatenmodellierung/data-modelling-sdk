//! Git service for managing Git repositories
//!
//! Provides Git operations that can be used by both API and native app.

use anyhow::{Context, Result};
use git2::{
    Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository, RepositoryInitOptions, Signature,
};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Error type for Git operations
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git repository error: {0}")]
    Repository(String),
    #[error("Git operation failed: {0}")]
    Operation(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Git status information
#[derive(Debug, Clone)]
pub struct GitStatus {
    pub has_changes: bool,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

/// Configuration for Git credentials
#[derive(Debug, Clone, Default)]
pub struct GitCredentials {
    /// SSH key path for authentication (optional)
    pub ssh_key_path: Option<PathBuf>,
    /// Username for HTTPS authentication (optional)
    pub username: Option<String>,
    /// Token/password for HTTPS authentication (optional)
    pub token: Option<String>,
}

/// Service for Git repository management
pub struct GitService {
    /// Git repository instance
    repo: Option<Repository>,
    /// Git directory path
    git_directory: Option<PathBuf>,
    /// Credentials for remote operations
    credentials: GitCredentials,
}

impl GitService {
    /// Create a new Git service instance
    pub fn new() -> Self {
        Self {
            repo: None,
            git_directory: None,
            credentials: GitCredentials::default(),
        }
    }

    /// Create a new Git service instance with credentials
    pub fn with_credentials(credentials: GitCredentials) -> Self {
        Self {
            repo: None,
            git_directory: None,
            credentials,
        }
    }

    /// Set credentials for remote operations
    pub fn set_credentials(&mut self, credentials: GitCredentials) {
        self.credentials = credentials;
    }

    /// Initialize or open a Git repository at the given path
    ///
    /// If the repository doesn't exist, it will be initialized.
    /// If it exists, it will be opened.
    pub fn open_or_init(&mut self, git_directory_path: &Path) -> Result<()> {
        // Validate directory exists (or create parent if initializing)
        if !git_directory_path.exists()
            && let Some(parent) = git_directory_path.parent()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }

        // Try to open existing repository
        let repo = match Repository::open(git_directory_path) {
            Ok(repo) => {
                info!("Opened existing Git repository at {:?}", git_directory_path);
                repo
            }
            Err(_) => {
                // Initialize new Git repository
                let mut opts = RepositoryInitOptions::new();
                opts.bare(false);
                let repo = Repository::init_opts(git_directory_path, &opts)
                    .with_context(|| {
                        format!(
                            "Failed to initialize Git repository at {:?}",
                            git_directory_path
                        )
                    })
                    .map_err(|e| GitError::Repository(format!("Failed to initialize: {}", e)))?;
                info!("Initialized new Git repository at {:?}", git_directory_path);
                repo
            }
        };

        self.repo = Some(repo);
        self.git_directory = Some(git_directory_path.to_path_buf());
        Ok(())
    }

    /// Get the repository instance (if opened)
    pub fn repository(&self) -> Option<&Repository> {
        self.repo.as_ref()
    }

    /// Get the Git directory path (if set)
    pub fn git_directory(&self) -> Option<&PathBuf> {
        self.git_directory.as_ref()
    }

    /// Stage files for commit
    ///
    /// `paths` can be:
    /// - Specific file paths relative to repository root
    /// - Empty vec to stage all changes
    pub fn stage_files(&self, paths: &[&str]) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        let mut index = repo
            .index()
            .map_err(|e| GitError::Operation(format!("Failed to get index: {}", e)))?;

        if paths.is_empty() {
            // Stage all changes - add all files in the working directory
            index
                .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
                .map_err(|e| GitError::Operation(format!("Failed to stage all files: {}", e)))?;
        } else {
            // Stage specific files
            for path in paths {
                index
                    .add_path(Path::new(path))
                    .map_err(|e| GitError::Operation(format!("Failed to add {}: {}", path, e)))?;
            }
        }

        index
            .write()
            .map_err(|e| GitError::Operation(format!("Failed to write index: {}", e)))?;

        let count_msg = if paths.is_empty() {
            "all".to_string()
        } else {
            paths.len().to_string()
        };
        info!("Staged {} files", count_msg);
        Ok(())
    }

    /// Commit staged changes
    ///
    /// `message` is the commit message
    /// `author_name` and `author_email` are used for the commit signature
    pub fn commit(&self, message: &str, author_name: &str, author_email: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        let signature = Signature::now(author_name, author_email)
            .map_err(|e| GitError::Operation(format!("Failed to create signature: {}", e)))?;

        let mut index = repo
            .index()
            .map_err(|e| GitError::Operation(format!("Failed to get index: {}", e)))?;

        let tree_id = index
            .write_tree()
            .map_err(|e| GitError::Operation(format!("Failed to write tree: {}", e)))?;

        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| GitError::Operation(format!("Failed to find tree: {}", e)))?;

        // Get HEAD reference for parent commit
        let mut parents: Vec<git2::Commit> = Vec::new();
        if let Ok(head) = repo.head()
            && let Ok(parent) = head.peel_to_commit()
        {
            parents.push(parent);
        }

        let parents_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents_refs,
        )
        .map_err(|e| GitError::Operation(format!("Failed to commit: {}", e)))?;

        info!("Committed changes: {}", message);
        Ok(())
    }

    /// Clone a remote repository
    ///
    /// `remote_url` is the URL of the remote repository
    /// `local_path` is where to clone the repository
    /// `branch` is the branch to checkout (defaults to "main")
    pub fn clone_repository(
        &mut self,
        remote_url: &str,
        local_path: &Path,
        branch: Option<&str>,
    ) -> Result<()> {
        let branch = branch.unwrap_or("main");

        // Create parent directory if needed
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }

        // Clone credential values before creating callbacks to avoid borrow issues
        let ssh_key_path = self.credentials.ssh_key_path.clone();
        let username = self.credentials.username.clone();
        let token = self.credentials.token.clone();

        // Set up callbacks for authentication
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, username_from_url, allowed_types| {
            if allowed_types.contains(git2::CredentialType::SSH_KEY)
                && let Some(ref key_path) = ssh_key_path
            {
                let user = username_from_url.unwrap_or("git");
                return Cred::ssh_key(user, None, key_path, None);
            }

            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT)
                && let (Some(user), Some(pass)) = (&username, &token)
            {
                return Cred::userpass_plaintext(user, pass);
            }

            Cred::default()
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);
        builder.branch(branch);

        let repo = builder
            .clone(remote_url, local_path)
            .with_context(|| format!("Failed to clone repository from {}", remote_url))
            .map_err(|e| GitError::Operation(format!("Failed to clone: {}", e)))?;

        info!("Cloned repository from {} to {:?}", remote_url, local_path);

        self.repo = Some(repo);
        self.git_directory = Some(local_path.to_path_buf());
        Ok(())
    }

    /// Set remote URL for the repository
    pub fn set_remote(&mut self, remote_name: &str, url: &str) -> Result<()> {
        let repo = self
            .repo
            .as_mut()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        // Remove existing remote if present
        let _ = repo.remote_delete(remote_name);
        // Add new remote
        repo.remote(remote_name, url)
            .map_err(|e| GitError::Operation(format!("Failed to set remote: {}", e)))?;

        info!("Set remote {} to {}", remote_name, url);
        Ok(())
    }

    /// Push changes to remote repository
    ///
    /// `remote_name` is typically "origin"
    /// `branch_name` is typically "main" or "master"
    pub fn push(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        let mut remote = repo.find_remote(remote_name).map_err(|e| {
            GitError::Operation(format!("Failed to find remote {}: {}", remote_name, e))
        })?;

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

        let callbacks = self.create_callbacks()?;
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote
            .push(&[&refspec], Some(&mut push_options))
            .map_err(|e| GitError::Operation(format!("Failed to push: {}", e)))?;

        info!("Pushed {} to {}", branch_name, remote_name);
        Ok(())
    }

    /// Fetch changes from remote repository
    pub fn fetch(&self, remote_name: &str, branch_name: Option<&str>) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        let mut remote = repo.find_remote(remote_name).map_err(|e| {
            GitError::Operation(format!("Failed to find remote {}: {}", remote_name, e))
        })?;

        let callbacks = self.create_callbacks()?;
        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let refspecs: Vec<&str> = if let Some(branch) = branch_name {
            vec![branch]
        } else {
            vec![] // Fetch all branches
        };

        remote
            .fetch(&refspecs, Some(&mut fetch_opts), None)
            .map_err(|e| GitError::Operation(format!("Failed to fetch: {}", e)))?;

        info!("Fetched from {}", remote_name);
        Ok(())
    }

    /// Pull changes from remote (fetch + merge)
    ///
    /// Returns true if merge was successful, false if conflicts were detected
    pub fn pull(&mut self, remote_name: &str, branch_name: &str) -> Result<bool> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        // First fetch - inline to avoid borrow checker issues
        let mut remote = repo.find_remote(remote_name).map_err(|e| {
            GitError::Operation(format!("Failed to find remote {}: {}", remote_name, e))
        })?;

        let callbacks = self.create_callbacks()?;
        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        remote
            .fetch(&[branch_name], Some(&mut fetch_opts), None)
            .map_err(|e| GitError::Operation(format!("Failed to fetch: {}", e)))?;

        // Get fetch head
        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .map_err(|e| GitError::Operation(format!("Failed to find FETCH_HEAD: {}", e)))?;
        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|e| GitError::Operation(format!("Failed to get commit: {}", e)))?;

        // Perform merge analysis
        let (merge_analysis, _) = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(|e| GitError::Operation(format!("Failed to analyze merge: {}", e)))?;

        if merge_analysis.is_up_to_date() {
            info!("Already up to date");
            return Ok(true);
        }

        if merge_analysis.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = repo
                .find_reference(&refname)
                .map_err(|e| GitError::Operation(format!("Failed to find reference: {}", e)))?;
            reference
                .set_target(fetch_commit.id(), "Fast-forward")
                .map_err(|e| GitError::Operation(format!("Failed to set target: {}", e)))?;
            repo.set_head(&refname)
                .map_err(|e| GitError::Operation(format!("Failed to set HEAD: {}", e)))?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| GitError::Operation(format!("Failed to checkout: {}", e)))?;

            info!("Fast-forward merge completed");
            return Ok(true);
        }

        // Normal merge required
        repo.merge(&[&fetch_commit], None, None)
            .map_err(|e| GitError::Operation(format!("Failed to merge: {}", e)))?;

        // Check for conflicts
        let mut index = repo
            .index()
            .map_err(|e| GitError::Operation(format!("Failed to get index: {}", e)))?;
        let has_conflicts = index.has_conflicts();

        if has_conflicts {
            warn!("Merge has conflicts");
            return Ok(false);
        }

        // Complete merge with commit
        let sig = Signature::now("Git Sync", "sync@modelling.local")
            .map_err(|e| GitError::Operation(format!("Failed to create signature: {}", e)))?;
        let head = repo
            .head()?
            .peel_to_commit()
            .map_err(|e| GitError::Operation(format!("Failed to get HEAD commit: {}", e)))?;
        let fetch_commit_obj = repo
            .find_commit(fetch_commit.id())
            .map_err(|e| GitError::Operation(format!("Failed to find commit: {}", e)))?;

        let tree_id = index
            .write_tree()
            .map_err(|e| GitError::Operation(format!("Failed to write tree: {}", e)))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| GitError::Operation(format!("Failed to find tree: {}", e)))?;

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Merge remote changes",
            &tree,
            &[&head, &fetch_commit_obj],
        )
        .map_err(|e| GitError::Operation(format!("Failed to commit merge: {}", e)))?;

        repo.cleanup_state()
            .map_err(|e| GitError::Operation(format!("Failed to cleanup state: {}", e)))?;

        info!("Merge completed");
        Ok(true)
    }

    /// Check if there are merge conflicts
    pub fn has_conflicts(&self) -> Result<bool> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        let index = repo
            .index()
            .map_err(|e| GitError::Operation(format!("Failed to get index: {}", e)))?;

        Ok(index.has_conflicts())
    }

    /// Get remote status (unpushed/unpulled commits)
    pub fn remote_status(&self, remote_name: &str, branch_name: &str) -> Result<(bool, bool)> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        let local_ref = format!("refs/heads/{}", branch_name);
        let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch_name);

        let local_oid = repo
            .find_reference(&local_ref)
            .ok()
            .and_then(|r| r.target());
        let remote_oid = repo
            .find_reference(&remote_ref)
            .ok()
            .and_then(|r| r.target());

        match (local_oid, remote_oid) {
            (Some(local), Some(remote)) => {
                if local == remote {
                    Ok((false, false))
                } else {
                    // Check if local is ahead or behind
                    let (ahead, behind) = repo.graph_ahead_behind(local, remote).map_err(|e| {
                        GitError::Operation(format!("Failed to compare commits: {}", e))
                    })?;
                    Ok((ahead > 0, behind > 0))
                }
            }
            (Some(_), None) => Ok((true, false)), // Local exists, remote doesn't
            (None, Some(_)) => Ok((false, true)), // Remote exists, local doesn't
            (None, None) => Ok((false, false)),
        }
    }

    /// Create authentication callbacks
    fn create_callbacks(&self) -> Result<RemoteCallbacks<'_>> {
        let mut callbacks = RemoteCallbacks::new();

        // Clone credential values for the closure
        let ssh_key_path = self.credentials.ssh_key_path.clone();
        let username = self.credentials.username.clone();
        let token = self.credentials.token.clone();

        callbacks.credentials(move |_url, username_from_url, allowed_types| {
            if allowed_types.contains(git2::CredentialType::SSH_KEY)
                && let Some(ref key_path) = ssh_key_path
            {
                let user = username_from_url.unwrap_or("git");
                return Cred::ssh_key(user, None, key_path, None);
            }

            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT)
                && let (Some(user), Some(pass)) = (&username, &token)
            {
                return Cred::userpass_plaintext(user, pass);
            }

            Cred::default()
        });

        Ok(callbacks)
    }

    /// Get Git status (staged, unstaged, untracked files)
    pub fn status(&self) -> Result<GitStatus> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| GitError::Operation("Repository not opened".to_string()))?;

        let mut status_options = git2::StatusOptions::new();
        status_options.include_untracked(true);
        status_options.include_ignored(false);

        let statuses = repo
            .statuses(Some(&mut status_options))
            .map_err(|e| GitError::Operation(format!("Failed to get status: {}", e)))?;

        let mut staged_files = Vec::new();
        let mut unstaged_files = Vec::new();
        let mut untracked_files = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
                staged_files.push(path.clone());
            }

            if status.is_wt_modified() || status.is_wt_deleted() {
                unstaged_files.push(path.clone());
            }

            if status.is_wt_new() {
                untracked_files.push(path.clone());
            }
        }

        let has_changes =
            !staged_files.is_empty() || !unstaged_files.is_empty() || !untracked_files.is_empty();

        Ok(GitStatus {
            has_changes,
            staged_files,
            unstaged_files,
            untracked_files,
        })
    }

    /// Stage all changes and commit
    ///
    /// Convenience method that stages all changes and commits them.
    pub fn commit_all(&self, message: &str, author_name: &str, author_email: &str) -> Result<()> {
        self.stage_files(&[])?;
        self.commit(message, author_name, author_email)?;
        Ok(())
    }

    /// Stage all changes, commit, and push
    ///
    /// Convenience method for the common workflow.
    pub fn commit_and_push(
        &self,
        message: &str,
        author_name: &str,
        author_email: &str,
        remote_name: &str,
        branch_name: &str,
    ) -> Result<()> {
        self.commit_all(message, author_name, author_email)?;
        self.push(remote_name, branch_name)?;
        Ok(())
    }
}

impl Default for GitService {
    fn default() -> Self {
        Self::new()
    }
}
