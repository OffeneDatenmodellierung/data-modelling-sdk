//! Types for schema mapping results

use serde::{Deserialize, Serialize};

/// Result of mapping a source schema to a target schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMapping {
    /// Direct field-to-field mappings
    pub direct_mappings: Vec<FieldMapping>,
    /// Mappings that require transformations
    pub transformations: Vec<TransformMapping>,
    /// Target fields that have no source (gaps)
    pub gaps: Vec<FieldGap>,
    /// Source fields that have no target (extras)
    pub extras: Vec<String>,
    /// Overall compatibility score (0.0-1.0)
    pub compatibility_score: f64,
    /// Summary statistics
    pub stats: MappingStats,
}

impl SchemaMapping {
    /// Create an empty mapping result
    pub fn empty() -> Self {
        Self {
            direct_mappings: Vec::new(),
            transformations: Vec::new(),
            gaps: Vec::new(),
            extras: Vec::new(),
            compatibility_score: 0.0,
            stats: MappingStats::default(),
        }
    }

    /// Check if the mapping is complete (no gaps in required fields)
    pub fn is_complete(&self) -> bool {
        self.gaps.iter().all(|g| !g.required)
    }

    /// Get all mapped target fields
    pub fn mapped_targets(&self) -> Vec<&str> {
        let mut targets: Vec<&str> = self
            .direct_mappings
            .iter()
            .map(|m| m.target_path.as_str())
            .collect();
        targets.extend(self.transformations.iter().map(|t| t.target_path.as_str()));
        targets
    }

    /// Get all mapped source fields
    pub fn mapped_sources(&self) -> Vec<&str> {
        let mut sources: Vec<&str> = self
            .direct_mappings
            .iter()
            .map(|m| m.source_path.as_str())
            .collect();
        for t in &self.transformations {
            sources.extend(t.source_paths.iter().map(|s| s.as_str()));
        }
        sources
    }
}

/// A direct field-to-field mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    /// Path in source schema
    pub source_path: String,
    /// Path in target schema
    pub target_path: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Whether types are directly compatible
    pub type_compatible: bool,
    /// Match method used
    pub match_method: MatchMethod,
}

impl FieldMapping {
    /// Create a new field mapping
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source_path: source.into(),
            target_path: target.into(),
            confidence: 1.0,
            type_compatible: true,
            match_method: MatchMethod::Exact,
        }
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set type compatibility
    pub fn with_type_compatible(mut self, compatible: bool) -> Self {
        self.type_compatible = compatible;
        self
    }

    /// Set match method
    pub fn with_match_method(mut self, method: MatchMethod) -> Self {
        self.match_method = method;
        self
    }
}

/// Method used to match fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMethod {
    /// Exact name match
    Exact,
    /// Case-insensitive match
    CaseInsensitive,
    /// Fuzzy (Levenshtein) match
    Fuzzy,
    /// Semantic match (LLM-assisted)
    Semantic,
    /// LLM-based matching
    Llm,
    /// Manual/user-defined
    Manual,
}

impl std::fmt::Display for MatchMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchMethod::Exact => write!(f, "exact"),
            MatchMethod::CaseInsensitive => write!(f, "case_insensitive"),
            MatchMethod::Fuzzy => write!(f, "fuzzy"),
            MatchMethod::Semantic => write!(f, "semantic"),
            MatchMethod::Llm => write!(f, "llm"),
            MatchMethod::Manual => write!(f, "manual"),
        }
    }
}

/// A mapping that requires transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformMapping {
    /// Source field path(s) - may be multiple for merge operations
    pub source_paths: Vec<String>,
    /// Target field path
    pub target_path: String,
    /// Type of transformation
    pub transform_type: TransformType,
    /// Human-readable description
    pub description: String,
    /// Confidence score
    pub confidence: f64,
}

impl TransformMapping {
    /// Create a new transform mapping
    pub fn new(sources: Vec<String>, target: impl Into<String>, transform: TransformType) -> Self {
        let desc = transform.describe();
        Self {
            source_paths: sources,
            target_path: target.into(),
            transform_type: transform,
            description: desc,
            confidence: 0.8,
        }
    }

    /// Set custom description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Types of transformations between fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransformType {
    /// Cast from one type to another
    TypeCast { from_type: String, to_type: String },
    /// Rename field (different name, same type)
    Rename,
    /// Merge multiple fields into one
    Merge { separator: Option<String> },
    /// Split one field into multiple
    Split {
        delimiter: String,
        target_paths: Vec<String>,
    },
    /// Change format (e.g., date format)
    FormatChange {
        from_format: String,
        to_format: String,
    },
    /// Custom expression
    Custom { expression: String },
    /// Nested field extraction
    Extract { json_path: String },
    /// Default value for missing field
    Default { value: serde_json::Value },
}

impl TransformType {
    /// Get human-readable description
    pub fn describe(&self) -> String {
        match self {
            TransformType::TypeCast { from_type, to_type } => {
                format!("Cast from {} to {}", from_type, to_type)
            }
            TransformType::Rename => "Rename field".to_string(),
            TransformType::Merge { separator } => {
                let sep = separator.as_deref().unwrap_or(" ");
                format!("Merge fields with separator '{}'", sep)
            }
            TransformType::Split { delimiter, .. } => {
                format!("Split field by '{}'", delimiter)
            }
            TransformType::FormatChange {
                from_format,
                to_format,
            } => {
                format!("Change format from {} to {}", from_format, to_format)
            }
            TransformType::Custom { expression } => {
                format!("Custom: {}", expression)
            }
            TransformType::Extract { json_path } => {
                format!("Extract from JSON path: {}", json_path)
            }
            TransformType::Default { value } => {
                format!("Default value: {}", value)
            }
        }
    }
}

/// A gap in the mapping (target field with no source)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldGap {
    /// Target field path
    pub target_path: String,
    /// Target field type
    pub target_type: String,
    /// Whether the field is required
    pub required: bool,
    /// Suggested similar source fields
    pub suggestions: Vec<String>,
    /// Suggested default value
    pub suggested_default: Option<serde_json::Value>,
}

impl FieldGap {
    /// Create a new field gap
    pub fn new(target: impl Into<String>, target_type: impl Into<String>, required: bool) -> Self {
        Self {
            target_path: target.into(),
            target_type: target_type.into(),
            required,
            suggestions: Vec::new(),
            suggested_default: None,
        }
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Set suggested default
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.suggested_default = Some(default);
        self
    }
}

/// Statistics about the mapping
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MappingStats {
    /// Total source fields
    pub source_fields: usize,
    /// Total target fields
    pub target_fields: usize,
    /// Fields directly mapped
    pub direct_mapped: usize,
    /// Fields mapped with transformation
    pub transform_mapped: usize,
    /// Target fields with gaps
    pub gaps_count: usize,
    /// Required gaps (blocking)
    pub required_gaps: usize,
    /// Extra source fields
    pub extras_count: usize,
}

impl MappingStats {
    /// Calculate coverage percentage
    pub fn coverage(&self) -> f64 {
        if self.target_fields == 0 {
            return 1.0;
        }
        let mapped = self.direct_mapped + self.transform_mapped;
        mapped as f64 / self.target_fields as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_mapping() {
        let mapping = FieldMapping::new("source.name", "target.name")
            .with_confidence(0.95)
            .with_match_method(MatchMethod::CaseInsensitive);

        assert_eq!(mapping.source_path, "source.name");
        assert_eq!(mapping.target_path, "target.name");
        assert_eq!(mapping.confidence, 0.95);
        assert_eq!(mapping.match_method, MatchMethod::CaseInsensitive);
    }

    #[test]
    fn test_transform_mapping() {
        let transform = TransformMapping::new(
            vec!["first_name".to_string(), "last_name".to_string()],
            "full_name",
            TransformType::Merge {
                separator: Some(" ".to_string()),
            },
        );

        assert_eq!(transform.source_paths.len(), 2);
        assert_eq!(transform.target_path, "full_name");
        assert!(transform.description.contains("Merge"));
    }

    #[test]
    fn test_field_gap() {
        let gap = FieldGap::new("required_field", "string", true)
            .with_suggestion("similar_field")
            .with_default(serde_json::json!("default"));

        assert!(gap.required);
        assert_eq!(gap.suggestions.len(), 1);
        assert!(gap.suggested_default.is_some());
    }

    #[test]
    fn test_schema_mapping_complete() {
        let mut mapping = SchemaMapping::empty();
        mapping.direct_mappings.push(FieldMapping::new("a", "b"));
        mapping
            .gaps
            .push(FieldGap::new("optional", "string", false));

        assert!(mapping.is_complete());

        mapping.gaps.push(FieldGap::new("required", "string", true));
        assert!(!mapping.is_complete());
    }

    #[test]
    fn test_mapping_stats_coverage() {
        let stats = MappingStats {
            source_fields: 10,
            target_fields: 8,
            direct_mapped: 5,
            transform_mapped: 2,
            gaps_count: 1,
            required_gaps: 0,
            extras_count: 3,
        };

        assert!((stats.coverage() - 0.875).abs() < 0.001);
    }

    #[test]
    fn test_transform_type_describe() {
        let cast = TransformType::TypeCast {
            from_type: "string".to_string(),
            to_type: "integer".to_string(),
        };
        assert!(cast.describe().contains("string"));
        assert!(cast.describe().contains("integer"));

        let merge = TransformType::Merge {
            separator: Some(", ".to_string()),
        };
        assert!(merge.describe().contains("Merge"));
    }
}
