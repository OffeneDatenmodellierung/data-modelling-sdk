//! Configuration for schema mapping operations

use serde::{Deserialize, Serialize};

/// Configuration for schema mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    /// Minimum confidence threshold for fuzzy matching (0.0-1.0)
    pub min_confidence: f64,
    /// Enable case-insensitive matching
    pub case_insensitive: bool,
    /// Enable fuzzy (Levenshtein) matching
    pub fuzzy_matching: bool,
    /// Maximum Levenshtein distance for fuzzy matches
    pub max_edit_distance: usize,
    /// Enable type coercion suggestions
    pub suggest_type_coercions: bool,
    /// Include unmapped source fields in extras
    pub track_extras: bool,
    /// Include unmapped required target fields in gaps
    pub track_gaps: bool,
    /// Output format for transformations
    pub transform_format: TransformFormat,
}

impl Default for MappingConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            case_insensitive: true,
            fuzzy_matching: true,
            max_edit_distance: 3,
            suggest_type_coercions: true,
            track_extras: true,
            track_gaps: true,
            transform_format: TransformFormat::Sql,
        }
    }
}

impl MappingConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict config (exact matches only)
    pub fn strict() -> Self {
        Self {
            min_confidence: 1.0,
            case_insensitive: false,
            fuzzy_matching: false,
            max_edit_distance: 0,
            suggest_type_coercions: false,
            track_extras: true,
            track_gaps: true,
            transform_format: TransformFormat::Sql,
        }
    }

    /// Set minimum confidence threshold
    pub fn with_min_confidence(mut self, confidence: f64) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Enable/disable case-insensitive matching
    pub fn with_case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        self
    }

    /// Enable/disable fuzzy matching
    pub fn with_fuzzy_matching(mut self, enabled: bool) -> Self {
        self.fuzzy_matching = enabled;
        self
    }

    /// Set maximum edit distance for fuzzy matches
    pub fn with_max_edit_distance(mut self, distance: usize) -> Self {
        self.max_edit_distance = distance;
        self
    }

    /// Set transform output format
    pub fn with_transform_format(mut self, format: TransformFormat) -> Self {
        self.transform_format = format;
        self
    }
}

/// Output format for generated transformations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransformFormat {
    /// SQL (DuckDB compatible)
    Sql,
    /// JQ filter expressions
    Jq,
    /// Python script
    Python,
    /// PySpark transformation
    Spark,
}

impl Default for TransformFormat {
    fn default() -> Self {
        Self::Sql
    }
}

impl std::fmt::Display for TransformFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransformFormat::Sql => write!(f, "sql"),
            TransformFormat::Jq => write!(f, "jq"),
            TransformFormat::Python => write!(f, "python"),
            TransformFormat::Spark => write!(f, "spark"),
        }
    }
}

impl std::str::FromStr for TransformFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sql" => Ok(TransformFormat::Sql),
            "jq" => Ok(TransformFormat::Jq),
            "python" | "py" => Ok(TransformFormat::Python),
            "spark" | "pyspark" => Ok(TransformFormat::Spark),
            _ => Err(format!("Unknown transform format: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MappingConfig::default();
        assert_eq!(config.min_confidence, 0.7);
        assert!(config.case_insensitive);
        assert!(config.fuzzy_matching);
    }

    #[test]
    fn test_strict_config() {
        let config = MappingConfig::strict();
        assert_eq!(config.min_confidence, 1.0);
        assert!(!config.case_insensitive);
        assert!(!config.fuzzy_matching);
    }

    #[test]
    fn test_builder() {
        let config = MappingConfig::new()
            .with_min_confidence(0.8)
            .with_fuzzy_matching(false)
            .with_transform_format(TransformFormat::Python);

        assert_eq!(config.min_confidence, 0.8);
        assert!(!config.fuzzy_matching);
        assert_eq!(config.transform_format, TransformFormat::Python);
    }

    #[test]
    fn test_transform_format_parse() {
        assert_eq!(
            "sql".parse::<TransformFormat>().unwrap(),
            TransformFormat::Sql
        );
        assert_eq!(
            "python".parse::<TransformFormat>().unwrap(),
            TransformFormat::Python
        );
        assert_eq!(
            "py".parse::<TransformFormat>().unwrap(),
            TransformFormat::Python
        );
        assert_eq!(
            "spark".parse::<TransformFormat>().unwrap(),
            TransformFormat::Spark
        );
        assert!("invalid".parse::<TransformFormat>().is_err());
    }
}
