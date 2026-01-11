//! Checkpointing for pipeline resume functionality

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::config::PipelineStage;
use super::error::{PipelineError, PipelineResult};

/// Pipeline checkpoint state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique pipeline run ID
    pub run_id: String,
    /// Pipeline name
    pub name: Option<String>,
    /// When the pipeline started
    pub started_at: DateTime<Utc>,
    /// When checkpoint was last updated
    pub updated_at: DateTime<Utc>,
    /// Current status
    pub status: PipelineStatus,
    /// Completed stages
    pub completed_stages: Vec<PipelineStage>,
    /// Current stage (if running)
    pub current_stage: Option<PipelineStage>,
    /// Stage outputs (paths to artifacts)
    pub stage_outputs: HashMap<String, StageOutput>,
    /// Error message if failed
    pub error: Option<String>,
    /// Configuration hash for validation
    pub config_hash: String,
}

impl Checkpoint {
    /// Create a new checkpoint for a pipeline run
    pub fn new(run_id: impl Into<String>, config_hash: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            run_id: run_id.into(),
            name: None,
            started_at: now,
            updated_at: now,
            status: PipelineStatus::Running,
            completed_stages: Vec::new(),
            current_stage: None,
            stage_outputs: HashMap::new(),
            error: None,
            config_hash: config_hash.into(),
        }
    }

    /// Set pipeline name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Mark a stage as started
    pub fn start_stage(&mut self, stage: PipelineStage) {
        self.current_stage = Some(stage);
        self.updated_at = Utc::now();
    }

    /// Mark a stage as completed
    pub fn complete_stage(&mut self, stage: PipelineStage, output: StageOutput) {
        self.completed_stages.push(stage);
        self.stage_outputs.insert(stage.name().to_string(), output);
        self.current_stage = None;
        self.updated_at = Utc::now();
    }

    /// Mark a stage as skipped
    pub fn skip_stage(&mut self, stage: PipelineStage, reason: impl Into<String>) {
        self.stage_outputs
            .insert(stage.name().to_string(), StageOutput::skipped(reason));
        self.current_stage = None;
        self.updated_at = Utc::now();
    }

    /// Mark pipeline as completed
    pub fn complete(&mut self) {
        self.status = PipelineStatus::Completed;
        self.current_stage = None;
        self.updated_at = Utc::now();
    }

    /// Mark pipeline as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = PipelineStatus::Failed;
        self.error = Some(error.into());
        self.updated_at = Utc::now();
    }

    /// Check if a stage has been completed
    pub fn is_stage_completed(&self, stage: PipelineStage) -> bool {
        self.completed_stages.contains(&stage)
    }

    /// Get the next stage to run
    pub fn next_stage(&self, all_stages: &[PipelineStage]) -> Option<PipelineStage> {
        for stage in all_stages {
            if !self.is_stage_completed(*stage) {
                return Some(*stage);
            }
        }
        None
    }

    /// Get output from a completed stage
    pub fn get_stage_output(&self, stage: PipelineStage) -> Option<&StageOutput> {
        self.stage_outputs.get(stage.name())
    }

    /// Calculate duration so far
    pub fn duration(&self) -> chrono::Duration {
        self.updated_at - self.started_at
    }

    /// Save checkpoint to file
    pub fn save(&self, path: &Path) -> PipelineResult<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load checkpoint from file
    pub fn load(path: &Path) -> PipelineResult<Self> {
        if !path.exists() {
            return Err(PipelineError::FileNotFound(path.to_path_buf()));
        }
        let json = std::fs::read_to_string(path)?;
        let checkpoint: Self = serde_json::from_str(&json)?;
        Ok(checkpoint)
    }

    /// Get default checkpoint path for a database
    pub fn default_path(database: &Path) -> PathBuf {
        let mut path = database.to_path_buf();
        path.set_extension("checkpoint.json");
        path
    }
}

/// Pipeline execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    /// Pipeline is running
    Running,
    /// Pipeline completed successfully
    Completed,
    /// Pipeline failed
    Failed,
    /// Pipeline was cancelled
    Cancelled,
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Output from a pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutput {
    /// Whether the stage was successful
    pub success: bool,
    /// Whether the stage was skipped
    pub skipped: bool,
    /// Reason for skipping (if applicable)
    pub skip_reason: Option<String>,
    /// Output file paths
    pub files: Vec<PathBuf>,
    /// Stage-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl StageOutput {
    /// Create a successful stage output
    pub fn success() -> Self {
        Self {
            success: true,
            skipped: false,
            skip_reason: None,
            files: Vec::new(),
            metadata: HashMap::new(),
            duration_ms: 0,
            timestamp: Utc::now(),
        }
    }

    /// Create a skipped stage output
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            success: true,
            skipped: true,
            skip_reason: Some(reason.into()),
            files: Vec::new(),
            metadata: HashMap::new(),
            duration_ms: 0,
            timestamp: Utc::now(),
        }
    }

    /// Create a failed stage output
    pub fn failed() -> Self {
        Self {
            success: false,
            skipped: false,
            skip_reason: None,
            files: Vec::new(),
            metadata: HashMap::new(),
            duration_ms: 0,
            timestamp: Utc::now(),
        }
    }

    /// Add an output file
    pub fn with_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.files.push(path.into());
        self
    }

    /// Add multiple output files
    pub fn with_files(mut self, paths: Vec<PathBuf>) -> Self {
        self.files.extend(paths);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_new() {
        let checkpoint = Checkpoint::new("run-123", "config-hash");
        assert_eq!(checkpoint.run_id, "run-123");
        assert_eq!(checkpoint.status, PipelineStatus::Running);
        assert!(checkpoint.completed_stages.is_empty());
    }

    #[test]
    fn test_checkpoint_stage_lifecycle() {
        let mut checkpoint = Checkpoint::new("run-123", "hash");

        checkpoint.start_stage(PipelineStage::Ingest);
        assert_eq!(checkpoint.current_stage, Some(PipelineStage::Ingest));

        checkpoint.complete_stage(
            PipelineStage::Ingest,
            StageOutput::success().with_metadata("records", serde_json::json!(1000)),
        );
        assert!(checkpoint.is_stage_completed(PipelineStage::Ingest));
        assert!(checkpoint.current_stage.is_none());
    }

    #[test]
    fn test_checkpoint_next_stage() {
        let mut checkpoint = Checkpoint::new("run-123", "hash");
        let stages = vec![
            PipelineStage::Ingest,
            PipelineStage::Infer,
            PipelineStage::Export,
        ];

        assert_eq!(checkpoint.next_stage(&stages), Some(PipelineStage::Ingest));

        checkpoint.complete_stage(PipelineStage::Ingest, StageOutput::success());
        assert_eq!(checkpoint.next_stage(&stages), Some(PipelineStage::Infer));

        checkpoint.complete_stage(PipelineStage::Infer, StageOutput::success());
        assert_eq!(checkpoint.next_stage(&stages), Some(PipelineStage::Export));

        checkpoint.complete_stage(PipelineStage::Export, StageOutput::success());
        assert_eq!(checkpoint.next_stage(&stages), None);
    }

    #[test]
    fn test_checkpoint_skip_stage() {
        let mut checkpoint = Checkpoint::new("run-123", "hash");
        checkpoint.skip_stage(PipelineStage::Refine, "LLM not configured");

        let output = checkpoint.get_stage_output(PipelineStage::Refine).unwrap();
        assert!(output.skipped);
        assert_eq!(output.skip_reason, Some("LLM not configured".to_string()));
    }

    #[test]
    fn test_stage_output() {
        let output = StageOutput::success()
            .with_file("/output/schema.json")
            .with_metadata("fields", serde_json::json!(10))
            .with_duration(1500);

        assert!(output.success);
        assert!(!output.skipped);
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.duration_ms, 1500);
    }

    #[test]
    fn test_checkpoint_complete() {
        let mut checkpoint = Checkpoint::new("run-123", "hash");
        checkpoint.complete();
        assert_eq!(checkpoint.status, PipelineStatus::Completed);
    }

    #[test]
    fn test_checkpoint_fail() {
        let mut checkpoint = Checkpoint::new("run-123", "hash");
        checkpoint.fail("Database connection failed");
        assert_eq!(checkpoint.status, PipelineStatus::Failed);
        assert_eq!(
            checkpoint.error,
            Some("Database connection failed".to_string())
        );
    }

    #[test]
    fn test_default_checkpoint_path() {
        let path = Checkpoint::default_path(Path::new("/data/staging.duckdb"));
        assert_eq!(path, PathBuf::from("/data/staging.checkpoint.json"));
    }
}
