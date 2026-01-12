//! Pipeline configuration types

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Main pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Name of the pipeline run
    pub name: Option<String>,
    /// Staging database path
    pub database: PathBuf,
    /// Source path for ingestion
    pub source: Option<PathBuf>,
    /// File pattern for ingestion
    pub pattern: String,
    /// Partition key
    pub partition: Option<String>,
    /// Output directory for exports
    pub output_dir: PathBuf,
    /// Target schema file for mapping (optional)
    pub target_schema: Option<PathBuf>,
    /// LLM configuration for refinement
    pub llm: LlmPipelineConfig,
    /// Stages to run (empty = all)
    pub stages: Vec<PipelineStage>,
    /// Enable dry-run mode
    pub dry_run: bool,
    /// Resume from checkpoint
    pub resume: bool,
    /// Verbose output
    pub verbose: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            name: None,
            database: PathBuf::from("staging.duckdb"),
            source: None,
            pattern: "*.json".to_string(),
            partition: None,
            output_dir: PathBuf::from("output"),
            target_schema: None,
            llm: LlmPipelineConfig::default(),
            stages: Vec::new(),
            dry_run: false,
            resume: false,
            verbose: false,
        }
    }
}

impl PipelineConfig {
    /// Create a new pipeline config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database path
    pub fn with_database(mut self, path: impl Into<PathBuf>) -> Self {
        self.database = path.into();
        self
    }

    /// Set the source path
    pub fn with_source(mut self, path: impl Into<PathBuf>) -> Self {
        self.source = Some(path.into());
        self
    }

    /// Set the file pattern
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = pattern.into();
        self
    }

    /// Set the partition key
    pub fn with_partition(mut self, partition: impl Into<String>) -> Self {
        self.partition = Some(partition.into());
        self
    }

    /// Set the output directory
    pub fn with_output_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_dir = path.into();
        self
    }

    /// Set the target schema
    pub fn with_target_schema(mut self, path: impl Into<PathBuf>) -> Self {
        self.target_schema = Some(path.into());
        self
    }

    /// Set LLM configuration
    pub fn with_llm(mut self, llm: LlmPipelineConfig) -> Self {
        self.llm = llm;
        self
    }

    /// Set specific stages to run
    pub fn with_stages(mut self, stages: Vec<PipelineStage>) -> Self {
        self.stages = stages;
        self
    }

    /// Enable dry-run mode
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable resume from checkpoint
    pub fn with_resume(mut self, resume: bool) -> Self {
        self.resume = resume;
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Get stages to run (all if empty)
    pub fn effective_stages(&self) -> Vec<PipelineStage> {
        if self.stages.is_empty() {
            PipelineStage::all()
        } else {
            self.stages.clone()
        }
    }

    /// Check if a specific stage should run
    pub fn should_run_stage(&self, stage: PipelineStage) -> bool {
        if self.stages.is_empty() {
            true
        } else {
            self.stages.contains(&stage)
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Source is required for ingest stage
        if self.should_run_stage(PipelineStage::Ingest) && self.source.is_none() {
            return Err("Source path is required for ingest stage".to_string());
        }

        // Target schema required for map stage
        if self.should_run_stage(PipelineStage::Map) && self.target_schema.is_none() {
            return Err("Target schema is required for map stage".to_string());
        }

        Ok(())
    }
}

/// LLM configuration for the pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPipelineConfig {
    /// LLM mode (none, online, offline)
    pub mode: String,
    /// Ollama URL for online mode
    pub ollama_url: String,
    /// Model name
    pub model: String,
    /// Model path for offline mode
    pub model_path: Option<PathBuf>,
    /// Documentation path for context
    pub doc_path: Option<PathBuf>,
    /// Temperature for generation
    pub temperature: f32,
}

impl Default for LlmPipelineConfig {
    fn default() -> Self {
        Self {
            mode: "none".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            model_path: None,
            doc_path: None,
            temperature: 0.3,
        }
    }
}

impl LlmPipelineConfig {
    /// Check if LLM is enabled
    pub fn is_enabled(&self) -> bool {
        self.mode != "none"
    }

    /// Create online LLM config
    pub fn online(model: impl Into<String>) -> Self {
        Self {
            mode: "online".to_string(),
            model: model.into(),
            ..Default::default()
        }
    }

    /// Create offline LLM config
    pub fn offline(model_path: impl Into<PathBuf>) -> Self {
        Self {
            mode: "offline".to_string(),
            model_path: Some(model_path.into()),
            ..Default::default()
        }
    }
}

/// Pipeline stages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStage {
    /// Stage 1: Ingest data into staging database
    Ingest,
    /// Stage 2: Infer schema from staged data
    Infer,
    /// Stage 3: Refine schema with LLM (optional)
    Refine,
    /// Stage 4: Map to target schema (optional)
    Map,
    /// Stage 5: Export to Parquet/target format
    Export,
    /// Stage 6: Generate ODCS contracts
    Generate,
}

impl PipelineStage {
    /// Get all stages in execution order
    pub fn all() -> Vec<Self> {
        vec![
            Self::Ingest,
            Self::Infer,
            Self::Refine,
            Self::Map,
            Self::Export,
            Self::Generate,
        ]
    }

    /// Get stage name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ingest => "ingest",
            Self::Infer => "infer",
            Self::Refine => "refine",
            Self::Map => "map",
            Self::Export => "export",
            Self::Generate => "generate",
        }
    }

    /// Get stage description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Ingest => "Ingest data into staging database",
            Self::Infer => "Infer schema from staged data",
            Self::Refine => "Refine schema with LLM",
            Self::Map => "Map to target schema",
            Self::Export => "Export to Parquet/target format",
            Self::Generate => "Generate ODCS contracts",
        }
    }

    /// Get stage index (1-based)
    pub fn index(&self) -> usize {
        match self {
            Self::Ingest => 1,
            Self::Infer => 2,
            Self::Refine => 3,
            Self::Map => 4,
            Self::Export => 5,
            Self::Generate => 6,
        }
    }

    /// Check if this stage is optional
    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Refine | Self::Map)
    }
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for PipelineStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ingest" | "1" => Ok(Self::Ingest),
            "infer" | "2" => Ok(Self::Infer),
            "refine" | "3" => Ok(Self::Refine),
            "map" | "4" => Ok(Self::Map),
            "export" | "5" => Ok(Self::Export),
            "generate" | "6" => Ok(Self::Generate),
            _ => Err(format!("Unknown stage: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.database, PathBuf::from("staging.duckdb"));
        assert_eq!(config.pattern, "*.json");
        assert!(!config.dry_run);
    }

    #[test]
    fn test_pipeline_config_builder() {
        let config = PipelineConfig::new()
            .with_database("test.duckdb")
            .with_source("/data/input")
            .with_output_dir("/data/output")
            .with_dry_run(true);

        assert_eq!(config.database, PathBuf::from("test.duckdb"));
        assert_eq!(config.source, Some(PathBuf::from("/data/input")));
        assert!(config.dry_run);
    }

    #[test]
    fn test_effective_stages() {
        let config = PipelineConfig::default();
        assert_eq!(config.effective_stages().len(), 6);

        let config = PipelineConfig::default()
            .with_stages(vec![PipelineStage::Ingest, PipelineStage::Infer]);
        assert_eq!(config.effective_stages().len(), 2);
    }

    #[test]
    fn test_pipeline_stage_parse() {
        assert_eq!(
            "ingest".parse::<PipelineStage>().unwrap(),
            PipelineStage::Ingest
        );
        assert_eq!("1".parse::<PipelineStage>().unwrap(), PipelineStage::Ingest);
        assert_eq!(
            "refine".parse::<PipelineStage>().unwrap(),
            PipelineStage::Refine
        );
        assert!("invalid".parse::<PipelineStage>().is_err());
    }

    #[test]
    fn test_pipeline_stage_properties() {
        assert_eq!(PipelineStage::Ingest.index(), 1);
        assert!(!PipelineStage::Ingest.is_optional());
        assert!(PipelineStage::Refine.is_optional());
        assert!(PipelineStage::Map.is_optional());
    }

    #[test]
    fn test_llm_config() {
        let config = LlmPipelineConfig::default();
        assert!(!config.is_enabled());

        let config = LlmPipelineConfig::online("llama3.2");
        assert!(config.is_enabled());
        assert_eq!(config.mode, "online");
    }

    #[test]
    fn test_config_validation() {
        let config = PipelineConfig::default();
        // Ingest needs source
        assert!(config.validate().is_err());

        let config = PipelineConfig::default()
            .with_source("/data")
            .with_stages(vec![PipelineStage::Ingest, PipelineStage::Infer]);
        assert!(config.validate().is_ok());

        let config = PipelineConfig::default()
            .with_source("/data")
            .with_stages(vec![PipelineStage::Map]);
        // Map needs target schema
        assert!(config.validate().is_err());
    }
}
