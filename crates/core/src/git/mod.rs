//! Git operations for managing Git repositories
//!
//! Provides Git repository management (init, open, commit, push, status) that can be used
//! by both the API and native app.

#[cfg(feature = "git")]
mod git_service;

#[cfg(feature = "git")]
pub use git_service::{GitCredentials, GitError, GitService, GitStatus};
