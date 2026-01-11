//! Staging database for raw JSON data ingestion
//!
//! This module provides a staging area for ingesting raw JSON data from various sources
//! (local files, S3, Unity Catalog Volumes) into an embedded database for processing.
//!
//! ## Features
//!
//! - **Large dataset handling** - Process millions of records without loading into memory
//! - **Deduplication** - Skip already-ingested files by path or content hash
//! - **Batch tracking** - Resume interrupted ingestions
//! - **SQL queries** - Analyze staged data before export
//!
//! ## Example
//!
//! ```rust,ignore
//! use data_modelling_core::staging::{StagingDb, IngestConfig, DedupStrategy};
//!
//! // Open or create staging database
//! let db = StagingDb::open("pipeline.duckdb")?;
//! db.init()?;
//!
//! // Configure ingestion
//! let config = IngestConfig::builder()
//!     .source("./data/")
//!     .pattern("*.json")
//!     .partition("2024-01")
//!     .dedup(DedupStrategy::ByPath)
//!     .build();
//!
//! // Ingest files
//! let stats = db.ingest(&config).await?;
//! println!("Ingested {} records from {} files", stats.records_ingested, stats.files_processed);
//! ```

mod batch;
mod config;
mod db;
mod error;
mod ingest;
mod schema;

pub use batch::{BatchStatus, ProcessingBatch};
pub use config::{DedupStrategy, IngestConfig, IngestConfigBuilder, SourceType};
#[cfg(feature = "duckdb-backend")]
pub use db::StagingDb;
#[cfg(feature = "postgres-backend")]
pub use db::StagingDbPostgres;
pub use error::{IngestError, StagingError};
pub use ingest::IngestStats;
pub use schema::StagingSchema;
