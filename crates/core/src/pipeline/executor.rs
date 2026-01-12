//! Pipeline executor for running the full data pipeline

use std::time::Instant;

use sha2::{Digest, Sha256};
use tracing::{debug, error, info, info_span, warn};
use uuid::Uuid;

use super::checkpoint::{Checkpoint, PipelineStatus, StageOutput};
use super::config::{PipelineConfig, PipelineStage};
use super::error::{PipelineError, PipelineResult};

/// Pipeline executor that runs all stages
pub struct PipelineExecutor {
    config: PipelineConfig,
    checkpoint: Checkpoint,
}

impl PipelineExecutor {
    /// Create a new pipeline executor
    pub fn new(config: PipelineConfig) -> PipelineResult<Self> {
        config.validate().map_err(PipelineError::ConfigError)?;

        let config_hash = Self::hash_config(&config);
        let run_id = Uuid::new_v4().to_string();

        let checkpoint = if config.resume {
            Self::load_or_create_checkpoint(&config, &run_id, &config_hash)?
        } else {
            Checkpoint::new(&run_id, &config_hash)
        };

        Ok(Self { config, checkpoint })
    }

    /// Create executor with existing checkpoint (for resume)
    pub fn with_checkpoint(config: PipelineConfig, checkpoint: Checkpoint) -> PipelineResult<Self> {
        config.validate().map_err(PipelineError::ConfigError)?;
        Ok(Self { config, checkpoint })
    }

    /// Get the current checkpoint
    pub fn checkpoint(&self) -> &Checkpoint {
        &self.checkpoint
    }

    /// Run the pipeline
    pub fn run(&mut self) -> PipelineResult<PipelineReport> {
        let _span = info_span!(
            "pipeline_run",
            run_id = %self.checkpoint.run_id,
            dry_run = self.config.dry_run
        )
        .entered();

        let start = Instant::now();
        let stages = self.config.effective_stages();

        info!(
            run_id = %self.checkpoint.run_id,
            stages = ?stages.iter().map(|s| s.name()).collect::<Vec<_>>(),
            dry_run = self.config.dry_run,
            "Starting pipeline"
        );

        if self.config.verbose {
            eprintln!("Pipeline run: {}", self.checkpoint.run_id);
            eprintln!(
                "Stages to run: {:?}",
                stages.iter().map(|s| s.name()).collect::<Vec<_>>()
            );
            if self.config.dry_run {
                eprintln!("DRY RUN MODE - no changes will be made");
            }
        }

        // Validate inputs in dry-run mode
        if self.config.dry_run {
            return self.dry_run(&stages);
        }

        // Run each stage
        for stage in &stages {
            // Skip if already completed (resume mode)
            if self.checkpoint.is_stage_completed(*stage) {
                debug!(stage = stage.name(), "Stage already completed, skipping");
                if self.config.verbose {
                    eprintln!("Stage {} already completed, skipping", stage.name());
                }
                continue;
            }

            // Check if stage should be skipped
            if let Some(reason) = self.should_skip_stage(*stage) {
                debug!(stage = stage.name(), reason = %reason, "Skipping stage");
                if self.config.verbose {
                    eprintln!("Skipping stage {}: {}", stage.name(), reason);
                }
                self.checkpoint.skip_stage(*stage, &reason);
                self.save_checkpoint()?;
                continue;
            }

            // Run the stage
            let _stage_span = info_span!("pipeline_stage", stage = stage.name()).entered();
            info!(stage = stage.name(), "Starting stage");

            if self.config.verbose {
                eprintln!("Running stage {}...", stage.name());
            }

            self.checkpoint.start_stage(*stage);
            self.save_checkpoint()?;

            match self.run_stage(*stage) {
                Ok(output) => {
                    info!(
                        stage = stage.name(),
                        duration_ms = output.duration_ms,
                        "Stage completed"
                    );
                    if self.config.verbose {
                        eprintln!(
                            "Stage {} completed in {}ms",
                            stage.name(),
                            output.duration_ms
                        );
                    }
                    self.checkpoint.complete_stage(*stage, output);
                    self.save_checkpoint()?;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    error!(stage = stage.name(), error = %error_msg, "Stage failed");
                    eprintln!("Stage {} failed: {}", stage.name(), error_msg);
                    self.checkpoint.fail(&error_msg);
                    self.save_checkpoint()?;
                    return Err(e);
                }
            }
        }

        self.checkpoint.complete();
        self.save_checkpoint()?;

        let duration = start.elapsed();
        info!(
            run_id = %self.checkpoint.run_id,
            duration_ms = duration.as_millis() as u64,
            stages_completed = self.checkpoint.completed_stages.len(),
            "Pipeline completed"
        );

        Ok(PipelineReport {
            run_id: self.checkpoint.run_id.clone(),
            status: self.checkpoint.status,
            stages_completed: self.checkpoint.completed_stages.clone(),
            duration_ms: duration.as_millis() as u64,
            outputs: self
                .checkpoint
                .stage_outputs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        })
    }

    /// Run a single stage
    fn run_stage(&self, stage: PipelineStage) -> PipelineResult<StageOutput> {
        let start = Instant::now();

        let output = match stage {
            PipelineStage::Ingest => self.run_ingest()?,
            PipelineStage::Infer => self.run_infer()?,
            PipelineStage::Refine => self.run_refine()?,
            PipelineStage::Map => self.run_map()?,
            PipelineStage::Export => self.run_export()?,
            PipelineStage::Generate => self.run_generate()?,
        };

        Ok(output.with_duration(start.elapsed().as_millis() as u64))
    }

    /// Run the ingest stage
    fn run_ingest(&self) -> PipelineResult<StageOutput> {
        let source = self
            .config
            .source
            .as_ref()
            .ok_or_else(|| PipelineError::MissingInput("source path".to_string()))?;

        debug!(source = %source.display(), pattern = %self.config.pattern, "Starting ingestion");

        // In a real implementation, this would use the staging module
        // For now, we just validate the source exists
        if !source.exists() {
            warn!(source = %source.display(), "Source path not found");
            return Err(PipelineError::FileNotFound(source.clone()));
        }

        let mut output = StageOutput::success();
        output = output.with_metadata("source", serde_json::json!(source.display().to_string()));
        output = output.with_metadata("pattern", serde_json::json!(self.config.pattern));

        // Count files matching pattern
        let pattern = source.join(&self.config.pattern);
        let files: Vec<_> = glob::glob(pattern.to_str().unwrap_or(""))
            .map(|paths| paths.filter_map(|p| p.ok()).collect())
            .unwrap_or_default();

        output = output.with_metadata("files_found", serde_json::json!(files.len()));

        debug!(files_found = files.len(), "Ingestion scan complete");
        if self.config.verbose {
            eprintln!("  Found {} files matching pattern", files.len());
        }

        Ok(output)
    }

    /// Run the infer stage
    fn run_infer(&self) -> PipelineResult<StageOutput> {
        // Get schema output path
        let schema_path = self.config.output_dir.join("inferred_schema.json");
        debug!(output = %schema_path.display(), "Running schema inference");

        let mut output = StageOutput::success();
        output = output.with_file(&schema_path);
        output = output.with_metadata(
            "schema_path",
            serde_json::json!(schema_path.display().to_string()),
        );

        Ok(output)
    }

    /// Run the refine stage
    fn run_refine(&self) -> PipelineResult<StageOutput> {
        if !self.config.llm.is_enabled() {
            debug!("LLM not configured, skipping refinement");
            return Ok(StageOutput::skipped("LLM not configured"));
        }

        debug!(
            mode = ?self.config.llm.mode,
            model = %self.config.llm.model,
            "Running LLM refinement"
        );

        let refined_path = self.config.output_dir.join("refined_schema.json");

        let mut output = StageOutput::success();
        output = output.with_file(&refined_path);
        output = output.with_metadata("model", serde_json::json!(self.config.llm.model));

        Ok(output)
    }

    /// Run the map stage
    fn run_map(&self) -> PipelineResult<StageOutput> {
        let target_schema = self
            .config
            .target_schema
            .as_ref()
            .ok_or_else(|| PipelineError::MissingInput("target schema".to_string()))?;

        debug!(target = %target_schema.display(), "Running schema mapping");

        if !target_schema.exists() {
            warn!(target = %target_schema.display(), "Target schema not found");
            return Err(PipelineError::FileNotFound(target_schema.clone()));
        }

        let mapping_path = self.config.output_dir.join("mapping.json");
        let transform_path = self.config.output_dir.join("transform.sql");

        let mut output = StageOutput::success();
        output = output.with_file(&mapping_path);
        output = output.with_file(&transform_path);
        output = output.with_metadata(
            "target_schema",
            serde_json::json!(target_schema.display().to_string()),
        );

        debug!(
            mapping = %mapping_path.display(),
            transform = %transform_path.display(),
            "Mapping complete"
        );

        Ok(output)
    }

    /// Run the export stage
    fn run_export(&self) -> PipelineResult<StageOutput> {
        let export_path = self.config.output_dir.join("data.parquet");
        debug!(output = %export_path.display(), format = "parquet", "Running export");

        let mut output = StageOutput::success();
        output = output.with_file(&export_path);
        output = output.with_metadata("format", serde_json::json!("parquet"));

        Ok(output)
    }

    /// Run the generate stage
    fn run_generate(&self) -> PipelineResult<StageOutput> {
        let contract_path = self.config.output_dir.join("contract.odcs.yaml");
        debug!(output = %contract_path.display(), format = "odcs", "Generating contract");

        let mut output = StageOutput::success();
        output = output.with_file(&contract_path);
        output = output.with_metadata("format", serde_json::json!("odcs"));

        Ok(output)
    }

    /// Check if a stage should be skipped
    fn should_skip_stage(&self, stage: PipelineStage) -> Option<String> {
        match stage {
            PipelineStage::Refine if !self.config.llm.is_enabled() => {
                Some("LLM not configured".to_string())
            }
            PipelineStage::Map if self.config.target_schema.is_none() => {
                Some("No target schema specified".to_string())
            }
            _ => None,
        }
    }

    /// Run in dry-run mode (validation only)
    fn dry_run(&self, stages: &[PipelineStage]) -> PipelineResult<PipelineReport> {
        let mut validation_errors = Vec::new();

        for stage in stages {
            if let Err(e) = self.validate_stage(*stage) {
                validation_errors.push(format!("{}: {}", stage.name(), e));
            }
        }

        if !validation_errors.is_empty() {
            return Err(PipelineError::ConfigError(format!(
                "Validation errors:\n  {}",
                validation_errors.join("\n  ")
            )));
        }

        eprintln!("Dry run validation passed for all stages");

        Ok(PipelineReport {
            run_id: self.checkpoint.run_id.clone(),
            status: PipelineStatus::Completed,
            stages_completed: Vec::new(),
            duration_ms: 0,
            outputs: std::collections::HashMap::new(),
        })
    }

    /// Validate a stage's inputs
    fn validate_stage(&self, stage: PipelineStage) -> PipelineResult<()> {
        match stage {
            PipelineStage::Ingest => {
                let source = self
                    .config
                    .source
                    .as_ref()
                    .ok_or_else(|| PipelineError::MissingInput("source path".to_string()))?;
                if !source.exists() {
                    return Err(PipelineError::FileNotFound(source.clone()));
                }
            }
            PipelineStage::Map => {
                if let Some(ref target) = self.config.target_schema {
                    if !target.exists() {
                        return Err(PipelineError::FileNotFound(target.clone()));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Save checkpoint to disk
    fn save_checkpoint(&self) -> PipelineResult<()> {
        let path = Checkpoint::default_path(&self.config.database);
        self.checkpoint.save(&path)
    }

    /// Load existing checkpoint or create new one
    fn load_or_create_checkpoint(
        config: &PipelineConfig,
        run_id: &str,
        config_hash: &str,
    ) -> PipelineResult<Checkpoint> {
        let path = Checkpoint::default_path(&config.database);

        if path.exists() {
            let checkpoint = Checkpoint::load(&path)?;

            // Validate config hash matches
            if checkpoint.config_hash != config_hash {
                return Err(PipelineError::ResumeError(
                    "Configuration has changed since last run. Use --no-resume to start fresh."
                        .to_string(),
                ));
            }

            // Check if resumable
            if checkpoint.status == PipelineStatus::Completed {
                return Err(PipelineError::ResumeError(
                    "Previous run already completed. Use --no-resume to start fresh.".to_string(),
                ));
            }

            Ok(checkpoint)
        } else {
            Ok(Checkpoint::new(run_id, config_hash))
        }
    }

    /// Hash the config for change detection
    fn hash_config(config: &PipelineConfig) -> String {
        let mut hasher = Sha256::new();
        hasher.update(config.database.display().to_string().as_bytes());
        if let Some(ref source) = config.source {
            hasher.update(source.display().to_string().as_bytes());
        }
        hasher.update(config.pattern.as_bytes());
        if let Some(ref partition) = config.partition {
            hasher.update(partition.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}

/// Report from a pipeline run
#[derive(Debug, Clone)]
pub struct PipelineReport {
    /// Run ID
    pub run_id: String,
    /// Final status
    pub status: PipelineStatus,
    /// Completed stages
    pub stages_completed: Vec<PipelineStage>,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Stage outputs
    pub outputs: std::collections::HashMap<String, StageOutput>,
}

impl PipelineReport {
    /// Check if pipeline was successful
    pub fn is_success(&self) -> bool {
        self.status == PipelineStatus::Completed
    }

    /// Get formatted duration
    pub fn duration_formatted(&self) -> String {
        let secs = self.duration_ms / 1000;
        let mins = secs / 60;
        let remaining_secs = secs % 60;

        if mins > 0 {
            format!("{}m {}s", mins, remaining_secs)
        } else {
            format!("{}s", secs)
        }
    }

    /// Print summary to stderr
    pub fn print_summary(&self) {
        eprintln!();
        eprintln!("Pipeline {} - {}", self.run_id, self.status);
        eprintln!("Duration: {}", self.duration_formatted());
        eprintln!("Stages completed: {}", self.stages_completed.len());

        for stage in &self.stages_completed {
            if let Some(output) = self.outputs.get(stage.name()) {
                let status = if output.skipped {
                    "skipped"
                } else if output.success {
                    "ok"
                } else {
                    "failed"
                };
                eprintln!(
                    "  - {}: {} ({}ms)",
                    stage.name(),
                    status,
                    output.duration_ms
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pipeline_executor_creation() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("input");
        std::fs::create_dir(&source).unwrap();

        // Use specific stages to avoid Map requiring target schema
        let config = PipelineConfig::new()
            .with_source(&source)
            .with_database(temp.path().join("staging.duckdb"))
            .with_output_dir(temp.path().join("output"))
            .with_stages(vec![
                PipelineStage::Ingest,
                PipelineStage::Infer,
                PipelineStage::Export,
            ]);

        let executor = PipelineExecutor::new(config).unwrap();
        assert_eq!(executor.checkpoint().status, PipelineStatus::Running);
    }

    #[test]
    fn test_config_hash() {
        let config1 = PipelineConfig::new()
            .with_source("/data/input")
            .with_pattern("*.json");

        let config2 = PipelineConfig::new()
            .with_source("/data/input")
            .with_pattern("*.json");

        let config3 = PipelineConfig::new()
            .with_source("/data/other")
            .with_pattern("*.json");

        assert_eq!(
            PipelineExecutor::hash_config(&config1),
            PipelineExecutor::hash_config(&config2)
        );
        assert_ne!(
            PipelineExecutor::hash_config(&config1),
            PipelineExecutor::hash_config(&config3)
        );
    }

    #[test]
    fn test_pipeline_report() {
        let report = PipelineReport {
            run_id: "test-123".to_string(),
            status: PipelineStatus::Completed,
            stages_completed: vec![PipelineStage::Ingest, PipelineStage::Infer],
            duration_ms: 65000,
            outputs: std::collections::HashMap::new(),
        };

        assert!(report.is_success());
        assert_eq!(report.duration_formatted(), "1m 5s");
    }
}
