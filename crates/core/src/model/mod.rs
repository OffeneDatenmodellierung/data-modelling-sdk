//! Model loading and saving functionality
//!
//! Provides high-level operations for loading and saving data models
//! using storage backends.
//!
//! File structure:
//! - Base directory (workspace_path)
//!   - Domain directories (e.g., `domain1/`, `domain2/`)
//!     - `domain.yaml` - Domain definition
//!     - `{name}.odcs.yaml` - ODCS table files
//!     - `{name}.odps.yaml` - ODPS product files
//!     - `{name}.cads.yaml` - CADS asset files
//!   - `tables/` - Legacy: tables not in any domain (backward compatibility)

#[cfg(feature = "api-backend")]
pub mod api_loader;
pub mod loader;
pub mod saver;

#[cfg(feature = "api-backend")]
pub use api_loader::ApiModelLoader;
pub use loader::{DomainLoadResult, ModelLoader};
pub use saver::ModelSaver;
