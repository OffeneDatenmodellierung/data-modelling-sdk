//! CLI module for data-modelling-cli binary

#[cfg(feature = "cli")]
pub mod commands;
#[cfg(feature = "cli")]
pub mod error;
#[cfg(feature = "cli")]
pub mod output;
#[cfg(feature = "cli")]
pub mod reference;
#[cfg(feature = "cli")]
pub mod validation;

#[cfg(feature = "cli")]
pub use error::CliError;
