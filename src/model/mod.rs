//! Model loading and saving functionality
//!
//! Provides high-level operations for loading and saving data models
//! using storage backends.

#[cfg(feature = "api-backend")]
pub mod api_loader;
pub mod loader;
pub mod saver;

#[cfg(feature = "api-backend")]
pub use api_loader::ApiModelLoader;
pub use loader::ModelLoader;
pub use saver::ModelSaver;
