//! CLI commands for pipeline operations
//!
//! Note: Some fields in argument structs are defined for future configuration
//! file support but are not yet used.

#![allow(dead_code)]

use std::path::PathBuf;

use crate::error::CliError;
use data_modelling_core::pipeline::{
    LlmPipelineConfig, PipelineConfig, PipelineExecutor, PipelineStage,
};

/// Arguments for the `pipeline run` command
pub struct PipelineRunArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Source directory for ingestion
    pub source: Option<PathBuf>,
    /// File pattern for ingestion
    pub pattern: String,
    /// Partition key
    pub partition: Option<String>,
    /// Output directory
    pub output_dir: PathBuf,
    /// Target schema file for mapping
    pub target_schema: Option<PathBuf>,
    /// Stages to run (empty = all)
    pub stages: Vec<String>,
    /// LLM mode (none, online, offline)
    pub llm_mode: String,
    /// Ollama URL
    pub ollama_url: String,
    /// Model name
    pub model: String,
    /// Model path for offline mode
    pub model_path: Option<PathBuf>,
    /// Documentation path
    pub doc_path: Option<PathBuf>,
    /// Temperature
    pub temperature: f32,
    /// Configuration file
    pub config_file: Option<PathBuf>,
    /// Dry run mode
    pub dry_run: bool,
    /// Resume from checkpoint
    pub resume: bool,
    /// Verbose output
    pub verbose: bool,
}

/// Arguments for the `pipeline status` command
pub struct PipelineStatusArgs {
    /// Path to the staging database file
    pub database: PathBuf,
}

/// Handle the `pipeline run` command
pub fn handle_pipeline_run(args: &PipelineRunArgs) -> Result<(), CliError> {
    // Build LLM config
    let llm = LlmPipelineConfig {
        mode: args.llm_mode.clone(),
        ollama_url: args.ollama_url.clone(),
        model: args.model.clone(),
        model_path: args.model_path.clone(),
        doc_path: args.doc_path.clone(),
        temperature: args.temperature,
    };

    // Parse stages
    let stages: Vec<PipelineStage> = if args.stages.is_empty() {
        Vec::new() // Empty means all stages
    } else {
        args.stages
            .iter()
            .map(|s| {
                s.parse::<PipelineStage>()
                    .map_err(|e| CliError::InvalidArgument(e))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    // Build config
    let mut config = PipelineConfig::new()
        .with_database(&args.database)
        .with_pattern(&args.pattern)
        .with_output_dir(&args.output_dir)
        .with_llm(llm)
        .with_stages(stages)
        .with_dry_run(args.dry_run)
        .with_resume(args.resume)
        .with_verbose(args.verbose);

    if let Some(ref source) = args.source {
        config = config.with_source(source);
    }

    if let Some(ref partition) = args.partition {
        config = config.with_partition(partition);
    }

    if let Some(ref target) = args.target_schema {
        config = config.with_target_schema(target);
    }

    // Create executor
    let mut executor =
        PipelineExecutor::new(config).map_err(|e| CliError::PipelineError(e.to_string()))?;

    eprintln!("Starting pipeline run: {}", executor.checkpoint().run_id);

    // Run pipeline
    let report = executor
        .run()
        .map_err(|e| CliError::PipelineError(e.to_string()))?;

    // Print summary
    report.print_summary();

    if report.is_success() {
        eprintln!();
        eprintln!("Pipeline completed successfully!");
        Ok(())
    } else {
        Err(CliError::PipelineError("Pipeline failed".to_string()))
    }
}

/// Handle the `pipeline status` command
pub fn handle_pipeline_status(args: &PipelineStatusArgs) -> Result<(), CliError> {
    use data_modelling_core::pipeline::Checkpoint;

    let checkpoint_path = Checkpoint::default_path(&args.database);

    if !checkpoint_path.exists() {
        eprintln!(
            "No pipeline checkpoint found for database: {}",
            args.database.display()
        );
        eprintln!("Run 'odm pipeline run' to start a new pipeline.");
        return Ok(());
    }

    let checkpoint = Checkpoint::load(&checkpoint_path)
        .map_err(|e| CliError::PipelineError(format!("Failed to load checkpoint: {}", e)))?;

    eprintln!("Pipeline Status");
    eprintln!("===============");
    eprintln!();
    eprintln!("Run ID:   {}", checkpoint.run_id);
    eprintln!("Status:   {}", checkpoint.status);
    eprintln!(
        "Started:  {}",
        checkpoint.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    eprintln!(
        "Updated:  {}",
        checkpoint.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    eprintln!();
    eprintln!("Completed Stages:");
    for stage in &checkpoint.completed_stages {
        if let Some(output) = checkpoint.stage_outputs.get(stage.name()) {
            let status = if output.skipped {
                "skipped"
            } else if output.success {
                "completed"
            } else {
                "failed"
            };
            eprintln!(
                "  - {}: {} ({}ms)",
                stage.name(),
                status,
                output.duration_ms
            );
        } else {
            eprintln!("  - {}: completed", stage.name());
        }
    }

    if let Some(stage) = &checkpoint.current_stage {
        eprintln!();
        eprintln!("Current Stage: {}", stage.name());
    }

    if let Some(ref error) = checkpoint.error {
        eprintln!();
        eprintln!("Error: {}", error);
    }

    Ok(())
}
