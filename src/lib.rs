//! Data Modelling SDK - Shared library for model operations across platforms
//!
//! This crate re-exports everything from `data-modelling-core` for backward compatibility.
//!
//! Provides unified interfaces for:
//! - File/folder operations (via storage backends)
//! - Model loading/saving
//! - Import/export functionality
//! - Validation logic
//! - Authentication types (shared across web, desktop, mobile)
//! - Workspace management types

// Re-export everything from core
pub use data_modelling_core::*;
