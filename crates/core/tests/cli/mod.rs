//! CLI tests module

#[cfg(feature = "cli")]
pub mod export_tests;
#[cfg(feature = "cli")]
pub mod import_tests;
#[cfg(feature = "cli")]
pub mod integration_tests;
#[cfg(all(feature = "cli", feature = "odps-validation"))]
pub mod validation_tests;
