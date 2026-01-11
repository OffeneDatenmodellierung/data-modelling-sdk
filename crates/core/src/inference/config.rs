//! Configuration for schema inference

use serde::{Deserialize, Serialize};

/// Configuration for schema inference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceConfig {
    /// Maximum number of records to sample (0 = all)
    pub sample_size: usize,

    /// Minimum field occurrence frequency for inclusion (0.0 - 1.0)
    /// Fields appearing less frequently are marked as optional
    pub min_field_frequency: f64,

    /// Enable format detection (date, uuid, email, etc.)
    pub detect_formats: bool,

    /// Maximum nesting depth for objects
    pub max_depth: usize,

    /// Collect example values for documentation
    pub collect_examples: bool,

    /// Maximum number of examples to collect per field
    pub max_examples: usize,

    /// Treat all fields as nullable by default
    pub assume_nullable: bool,

    /// Minimum confidence threshold for format detection (0.0 - 1.0)
    pub format_confidence_threshold: f64,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            sample_size: 0,           // All records
            min_field_frequency: 0.0, // Include all fields
            detect_formats: true,
            max_depth: 10,
            collect_examples: true,
            max_examples: 5,
            assume_nullable: false,
            format_confidence_threshold: 0.9,
        }
    }
}

impl InferenceConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for custom configuration
    pub fn builder() -> InferenceConfigBuilder {
        InferenceConfigBuilder::default()
    }
}

/// Builder for InferenceConfig
#[derive(Debug, Default)]
pub struct InferenceConfigBuilder {
    config: InferenceConfig,
}

impl InferenceConfigBuilder {
    /// Set the sample size (0 = all records)
    pub fn sample_size(mut self, size: usize) -> Self {
        self.config.sample_size = size;
        self
    }

    /// Set the minimum field frequency for inclusion
    pub fn min_field_frequency(mut self, freq: f64) -> Self {
        self.config.min_field_frequency = freq.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable format detection
    pub fn detect_formats(mut self, detect: bool) -> Self {
        self.config.detect_formats = detect;
        self
    }

    /// Set the maximum nesting depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Enable or disable example collection
    pub fn collect_examples(mut self, collect: bool) -> Self {
        self.config.collect_examples = collect;
        self
    }

    /// Set the maximum number of examples per field
    pub fn max_examples(mut self, max: usize) -> Self {
        self.config.max_examples = max;
        self
    }

    /// Set whether to assume all fields are nullable
    pub fn assume_nullable(mut self, nullable: bool) -> Self {
        self.config.assume_nullable = nullable;
        self
    }

    /// Set the format detection confidence threshold
    pub fn format_confidence_threshold(mut self, threshold: f64) -> Self {
        self.config.format_confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Build the configuration
    pub fn build(self) -> InferenceConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = InferenceConfig::default();
        assert_eq!(config.sample_size, 0);
        assert!(config.detect_formats);
        assert_eq!(config.max_depth, 10);
    }

    #[test]
    fn test_builder() {
        let config = InferenceConfig::builder()
            .sample_size(1000)
            .min_field_frequency(0.5)
            .detect_formats(false)
            .max_depth(5)
            .build();

        assert_eq!(config.sample_size, 1000);
        assert_eq!(config.min_field_frequency, 0.5);
        assert!(!config.detect_formats);
        assert_eq!(config.max_depth, 5);
    }

    #[test]
    fn test_frequency_clamping() {
        let config = InferenceConfig::builder()
            .min_field_frequency(1.5) // Should clamp to 1.0
            .build();

        assert_eq!(config.min_field_frequency, 1.0);
    }
}
