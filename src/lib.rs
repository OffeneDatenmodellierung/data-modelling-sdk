//! Data Modelling SDK - Shared library for model operations across platforms
//! 
//! Provides unified interfaces for:
//! - File/folder operations (via storage backends)
//! - Model loading/saving
//! - Import/export functionality
//! - Validation logic
//! - Authentication types (shared across web, desktop, mobile)
//! - Workspace management types

pub mod storage;
pub mod model;
pub mod import;
pub mod export;
pub mod validation;
pub mod models;
pub mod auth;
pub mod workspace;

// Re-export commonly used types
pub use storage::{StorageBackend, StorageError};
#[cfg(feature = "native-fs")]
pub use storage::filesystem::FileSystemStorageBackend;
#[cfg(feature = "api-backend")]
pub use storage::api::ApiStorageBackend;
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub use storage::browser::BrowserStorageBackend;

pub use model::{ModelLoader, ModelSaver};
#[cfg(feature = "api-backend")]
pub use model::ApiModelLoader;
pub use import::{ImportResult, ImportError, SQLImporter, JSONSchemaImporter, AvroImporter, ProtobufImporter, ODCSImporter};
pub use export::{ExportResult, ExportError, SQLExporter, JSONSchemaExporter, AvroExporter, ProtobufExporter, ODCSExporter};
#[cfg(feature = "png-export")]
pub use export::PNGExporter;
pub use validation::{
    TableValidationError, TableValidationResult,
    RelationshipValidationError, RelationshipValidationResult,
};

// Re-export models
pub use models::{Column, Table, Relationship, DataModel, ForeignKey};
pub use models::enums::*;

// Re-export auth types
pub use auth::{AuthMode, AuthState, GitHubEmail, InitiateOAuthRequest, InitiateOAuthResponse, SelectEmailRequest};

// Re-export workspace types
pub use workspace::{WorkspaceInfo, ProfileInfo, CreateWorkspaceRequest, CreateWorkspaceResponse, ListProfilesResponse, LoadProfileRequest};
