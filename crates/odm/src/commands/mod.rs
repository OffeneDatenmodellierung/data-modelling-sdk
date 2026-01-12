//! CLI command implementations

pub mod decision;
pub mod export;
pub mod import;
pub mod knowledge;
pub mod validate;

#[cfg(feature = "duckdb-backend")]
pub mod db;
#[cfg(feature = "duckdb-backend")]
pub mod query;

#[cfg(feature = "staging")]
pub mod staging;

#[cfg(all(feature = "inference", feature = "staging"))]
pub mod inference;

#[cfg(feature = "mapping")]
pub mod mapping;

#[cfg(feature = "pipeline")]
pub mod pipeline;
