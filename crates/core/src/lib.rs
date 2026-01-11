//! Data Modelling Core - Shared library for model operations across platforms
//!
//! Provides unified interfaces for:
//! - File/folder operations (via storage backends)
//! - Model loading/saving
//! - Import/export functionality
//! - Validation logic
//! - Authentication types (shared across web, desktop, mobile)
//! - Workspace management types

pub mod auth;
pub mod convert;
#[cfg(feature = "database")]
pub mod database;
pub mod export;
#[cfg(feature = "git")]
pub mod git;
pub mod import;
#[cfg(feature = "inference")]
pub mod inference;
#[cfg(any(feature = "llm", feature = "llm-online", feature = "llm-offline"))]
pub mod llm;
#[cfg(feature = "mapping")]
pub mod mapping;
pub mod model;
pub mod models;
#[cfg(feature = "pipeline")]
pub mod pipeline;
#[cfg(any(feature = "staging", feature = "staging-postgres"))]
pub mod staging;
pub mod storage;
pub mod validation;
pub mod workspace;

// Re-export commonly used types
#[cfg(feature = "api-backend")]
pub use storage::api::ApiStorageBackend;
#[cfg(feature = "native-fs")]
pub use storage::filesystem::FileSystemStorageBackend;
pub use storage::{StorageBackend, StorageError};

pub use convert::{ConversionError, convert_to_odcs};
#[cfg(feature = "png-export")]
pub use export::PNGExporter;
pub use export::{
    AvroExporter, ExportError, ExportResult, JSONSchemaExporter, ODCSExporter, ProtobufExporter,
    SQLExporter,
};
pub use import::{
    AvroImporter, ImportError, ImportResult, JSONSchemaImporter, ODCSImporter, ProtobufImporter,
    SQLImporter,
};
#[cfg(feature = "api-backend")]
pub use model::ApiModelLoader;
pub use model::{ModelLoader, ModelSaver};
pub use validation::{
    RelationshipValidationError, RelationshipValidationResult, TableValidationError,
    TableValidationResult,
};

// Re-export models
pub use models::enums::*;
pub use models::{Column, ContactDetails, DataModel, ForeignKey, Relationship, SlaProperty, Table};

// Re-export auth types
pub use auth::{
    AuthMode, AuthState, GitHubEmail, InitiateOAuthRequest, InitiateOAuthResponse,
    SelectEmailRequest,
};

// Re-export workspace types
pub use workspace::{
    CreateWorkspaceRequest, CreateWorkspaceResponse, ListProfilesResponse, LoadProfileRequest,
    ProfileInfo, WorkspaceInfo,
};

// Re-export Git types
#[cfg(feature = "git")]
pub use git::{GitError, GitService, GitStatus};
