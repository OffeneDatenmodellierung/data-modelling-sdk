//! Full pipeline integration for data ingestion, inference, and export
//!
//! This module provides a complete pipeline that orchestrates:
//! - Data ingestion into staging database
//! - Schema inference from staged data
//! - LLM-enhanced schema refinement (optional)
//! - Target schema mapping (optional)
//! - Data export to Parquet/target format
//! - ODCS contract generation
//!
//! # Example
//!
//! ```rust,ignore
//! use data_modelling_core::pipeline::{PipelineConfig, PipelineExecutor, PipelineStage};
//!
//! let config = PipelineConfig::new()
//!     .with_source("/data/input")
//!     .with_database("staging.duckdb")
//!     .with_output_dir("/data/output")
//!     .with_stages(vec![
//!         PipelineStage::Ingest,
//!         PipelineStage::Infer,
//!         PipelineStage::Export,
//!     ]);
//!
//! let mut executor = PipelineExecutor::new(config)?;
//! let report = executor.run()?;
//!
//! println!("Pipeline completed in {}", report.duration_formatted());
//! ```
//!
//! # Pipeline Stages
//!
//! 1. **Ingest**: Load JSON/JSONL files into staging database
//! 2. **Infer**: Infer schema from staged data using statistical analysis
//! 3. **Refine** (optional): Enhance schema using LLM with documentation context
//! 4. **Map** (optional): Map inferred schema to target schema
//! 5. **Export**: Export data to Parquet or other target format
//! 6. **Generate**: Generate ODCS data contracts
//!
//! # Checkpointing
//!
//! The pipeline supports checkpointing for resume functionality:
//!
//! ```rust,ignore
//! // Resume from previous run
//! let config = PipelineConfig::new()
//!     .with_source("/data/input")
//!     .with_resume(true);
//!
//! let mut executor = PipelineExecutor::new(config)?;
//! let report = executor.run()?; // Continues from last checkpoint
//! ```
//!
//! # Dry Run
//!
//! Validate inputs without executing:
//!
//! ```rust,ignore
//! let config = PipelineConfig::new()
//!     .with_source("/data/input")
//!     .with_dry_run(true);
//!
//! let mut executor = PipelineExecutor::new(config)?;
//! let report = executor.run()?; // Validates but doesn't execute
//! ```

mod checkpoint;
mod config;
mod error;
mod executor;

pub use checkpoint::{Checkpoint, PipelineStatus, StageOutput};
pub use config::{LlmPipelineConfig, PipelineConfig, PipelineStage};
pub use error::{PipelineError, PipelineResult};
pub use executor::{PipelineExecutor, PipelineReport};

/// Run a pipeline with the given configuration
///
/// This is a convenience function for simple pipeline execution.
pub fn run_pipeline(config: PipelineConfig) -> PipelineResult<PipelineReport> {
    let mut executor = PipelineExecutor::new(config)?;
    executor.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_run_pipeline_dry_run() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("input");
        std::fs::create_dir(&source).unwrap();

        // Create a test file
        std::fs::write(source.join("test.json"), r#"{"name": "test"}"#).unwrap();

        let config = PipelineConfig::new()
            .with_source(&source)
            .with_database(temp.path().join("staging.duckdb"))
            .with_output_dir(temp.path().join("output"))
            .with_dry_run(true)
            .with_stages(vec![PipelineStage::Ingest]);

        let report = run_pipeline(config).unwrap();
        assert!(report.is_success());
    }
}
