//! LLM-enhanced schema matching
//!
//! This module provides LLM-based field matching to complement
//! algorithmic matching with semantic understanding.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::config::MappingConfig;
use super::error::{MappingError, MappingResult};
use super::types::{FieldMapping, MatchMethod, SchemaMapping, TransformMapping, TransformType};

#[cfg(feature = "llm")]
use crate::llm::LlmClient;

/// Configuration for LLM-enhanced matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMatcherConfig {
    /// Minimum confidence for LLM suggestions (0.0-1.0)
    pub min_confidence: f64,
    /// Include field descriptions in prompt
    pub include_descriptions: bool,
    /// Include example values in prompt
    pub include_examples: bool,
    /// Maximum fields to include in a single prompt
    pub max_fields_per_prompt: usize,
    /// Temperature for LLM generation
    pub temperature: f32,
    /// Maximum retries on LLM failure
    pub max_retries: usize,
    /// Create TransformMappings from LLM hints
    pub create_transforms: bool,
}

impl Default for LlmMatcherConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            include_descriptions: true,
            include_examples: true,
            max_fields_per_prompt: 50,
            temperature: 0.3,
            max_retries: 2,
            create_transforms: true,
        }
    }
}

impl LlmMatcherConfig {
    /// Create a new LLM matcher config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum confidence threshold
    pub fn with_min_confidence(mut self, confidence: f64) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Enable/disable field descriptions
    pub fn with_descriptions(mut self, enabled: bool) -> Self {
        self.include_descriptions = enabled;
        self
    }

    /// Enable/disable example values
    pub fn with_examples(mut self, enabled: bool) -> Self {
        self.include_examples = enabled;
        self
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Enable/disable transform creation from hints
    pub fn with_transforms(mut self, enabled: bool) -> Self {
        self.create_transforms = enabled;
        self
    }
}

/// LLM suggestion for a field mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFieldSuggestion {
    /// Source field path
    pub source_field: String,
    /// Target field path
    pub target_field: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Reasoning for the match
    pub reasoning: String,
    /// Whether type conversion is needed
    pub requires_transform: bool,
    /// Suggested transformation if needed
    pub transform_hint: Option<String>,
}

/// Response from LLM matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMatchResponse {
    /// Field mapping suggestions
    pub suggestions: Vec<LlmFieldSuggestion>,
    /// Fields that couldn't be matched
    pub unmatched_source: Vec<String>,
    /// Target fields with no source match
    pub unmatched_target: Vec<String>,
    /// Overall confidence in the mapping
    pub overall_confidence: f64,
}

/// LLM-enhanced schema matcher
#[cfg(feature = "llm")]
pub struct LlmSchemaMatcher<C: LlmClient> {
    client: C,
    config: LlmMatcherConfig,
    mapping_config: MappingConfig,
}

#[cfg(feature = "llm")]
impl<C: LlmClient> LlmSchemaMatcher<C> {
    /// Create a new LLM schema matcher
    pub fn new(client: C) -> Self {
        Self {
            client,
            config: LlmMatcherConfig::default(),
            mapping_config: MappingConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(client: C, config: LlmMatcherConfig, mapping_config: MappingConfig) -> Self {
        Self {
            client,
            config,
            mapping_config,
        }
    }

    /// Match schemas using LLM
    pub async fn match_schemas(
        &self,
        source: &Value,
        target: &Value,
    ) -> MappingResult<SchemaMapping> {
        // First, run algorithmic matching
        let algo_matcher = super::SchemaMatcher::with_config(self.mapping_config.clone());
        let mut mapping = algo_matcher.match_schemas(source, target)?;

        // If there are gaps, try LLM matching for unmatched fields
        if !mapping.gaps.is_empty() || !mapping.extras.is_empty() {
            let llm_suggestions = self.get_llm_suggestions(source, target, &mapping).await?;
            self.merge_llm_suggestions(&mut mapping, llm_suggestions)?;
        }

        Ok(mapping)
    }

    /// Get LLM suggestions for unmatched fields with retry logic
    async fn get_llm_suggestions(
        &self,
        source: &Value,
        target: &Value,
        current_mapping: &SchemaMapping,
    ) -> MappingResult<LlmMatchResponse> {
        // Collect all unmatched fields
        let source_fields = extract_field_info(source)?;
        let target_fields = extract_field_info(target)?;

        let matched_sources: std::collections::HashSet<_> = current_mapping
            .direct_mappings
            .iter()
            .map(|m| &m.source_path)
            .collect();

        let matched_targets: std::collections::HashSet<_> = current_mapping
            .direct_mappings
            .iter()
            .map(|m| &m.target_path)
            .collect();

        let unmatched_source: Vec<_> = source_fields
            .iter()
            .filter(|(path, _)| !matched_sources.contains(*path))
            .collect();

        let unmatched_target: Vec<_> = target_fields
            .iter()
            .filter(|(path, _)| !matched_targets.contains(*path))
            .collect();

        // Batch if there are too many fields
        let max_per_batch = self.config.max_fields_per_prompt;
        let source_batches: Vec<_> = unmatched_source.chunks(max_per_batch).collect();
        let target_batches: Vec<_> = unmatched_target.chunks(max_per_batch).collect();

        let mut combined_response = LlmMatchResponse {
            suggestions: Vec::new(),
            unmatched_source: Vec::new(),
            unmatched_target: Vec::new(),
            overall_confidence: 0.0,
        };

        // Process batches - pair source and target batches together
        let num_batches = source_batches.len().max(target_batches.len()).max(1);
        let mut total_confidence = 0.0;
        let mut batch_count = 0;

        for i in 0..num_batches {
            let source_batch = source_batches.get(i).copied().unwrap_or(&[]);
            let target_batch = target_batches.get(i).copied().unwrap_or(&[]);

            if source_batch.is_empty() && target_batch.is_empty() {
                continue;
            }

            let prompt = self.build_matching_prompt_for_batch(source_batch, target_batch)?;
            let batch_response = self.call_llm_with_retry(&prompt).await?;

            combined_response
                .suggestions
                .extend(batch_response.suggestions);
            combined_response
                .unmatched_source
                .extend(batch_response.unmatched_source);
            combined_response
                .unmatched_target
                .extend(batch_response.unmatched_target);
            total_confidence += batch_response.overall_confidence;
            batch_count += 1;
        }

        if batch_count > 0 {
            combined_response.overall_confidence = total_confidence / batch_count as f64;
        }

        Ok(combined_response)
    }

    /// Call LLM with retry logic and exponential backoff
    async fn call_llm_with_retry(&self, prompt: &str) -> MappingResult<LlmMatchResponse> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.client.complete(prompt).await {
                Ok(response) => {
                    match self.parse_llm_response(&response) {
                        Ok(parsed) => return Ok(parsed),
                        Err(e) => {
                            last_error = Some(e);
                            // Parse error - might be worth retrying
                            if attempt < self.config.max_retries {
                                // Exponential backoff: 100ms, 200ms, 400ms, ...
                                let delay_ms = 100 * (1 << attempt);
                                tokio::time::sleep(std::time::Duration::from_millis(delay_ms))
                                    .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(MappingError::LlmError(e.to_string()));
                    if attempt < self.config.max_retries {
                        // Exponential backoff
                        let delay_ms = 100 * (1 << attempt);
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| MappingError::LlmError("LLM call failed after retries".to_string())))
    }

    /// Build the prompt for a batch of fields
    fn build_matching_prompt_for_batch(
        &self,
        source_fields: &[(&String, &FieldPromptInfo)],
        target_fields: &[(&String, &FieldPromptInfo)],
    ) -> MappingResult<String> {
        let prompt = format!(
            r#"You are a schema mapping expert. Match source fields to target fields based on semantic meaning.

## Source Schema Fields (unmatched)
{}

## Target Schema Fields (unmatched)
{}

## Instructions
1. For each unmatched source field, find the best matching target field based on semantic meaning
2. Consider field names, types, and descriptions
3. Assign a confidence score (0.0-1.0) for each match
4. Note if type conversion is required
5. If type conversion is needed, provide a transform_hint describing the transformation (e.g., "uppercase", "parse_date", "to_string")

## Response Format
Respond with valid JSON only, no markdown or explanation:
{{
  "suggestions": [
    {{
      "source_field": "field_name",
      "target_field": "matching_target",
      "confidence": 0.85,
      "reasoning": "brief explanation",
      "requires_transform": false,
      "transform_hint": null
    }}
  ],
  "unmatched_source": ["fields", "that", "have", "no", "match"],
  "unmatched_target": ["target", "fields", "without", "source"],
  "overall_confidence": 0.8
}}"#,
            format_fields_for_prompt(
                source_fields,
                self.config.include_descriptions,
                self.config.include_examples
            ),
            format_fields_for_prompt(
                target_fields,
                self.config.include_descriptions,
                self.config.include_examples
            ),
        );

        Ok(prompt)
    }

    /// Parse the LLM response into structured suggestions
    fn parse_llm_response(&self, response: &str) -> MappingResult<LlmMatchResponse> {
        // Try to extract JSON from the response
        let json_str = extract_json_from_response(response)?;

        serde_json::from_str(&json_str)
            .map_err(|e| MappingError::LlmError(format!("Failed to parse LLM response: {}", e)))
    }

    /// Merge LLM suggestions into the existing mapping
    fn merge_llm_suggestions(
        &self,
        mapping: &mut SchemaMapping,
        suggestions: LlmMatchResponse,
    ) -> MappingResult<()> {
        for suggestion in suggestions.suggestions {
            // Only accept suggestions above confidence threshold
            if suggestion.confidence < self.config.min_confidence {
                continue;
            }

            // Check if this mapping doesn't conflict with existing ones
            let source_already_mapped = mapping
                .direct_mappings
                .iter()
                .any(|m| m.source_path == suggestion.source_field);

            let target_already_mapped = mapping
                .direct_mappings
                .iter()
                .any(|m| m.target_path == suggestion.target_field);

            if source_already_mapped || target_already_mapped {
                continue;
            }

            // If transform is required and we should create transforms, add to transformations
            if suggestion.requires_transform && self.config.create_transforms {
                if let Some(ref hint) = suggestion.transform_hint {
                    let transform = self.create_transform_from_hint(
                        &suggestion.source_field,
                        &suggestion.target_field,
                        hint,
                        suggestion.confidence,
                    );
                    mapping.transformations.push(transform);

                    // Remove from gaps if present
                    mapping
                        .gaps
                        .retain(|g| g.target_path != suggestion.target_field);

                    // Remove from extras if present
                    mapping.extras.retain(|e| *e != suggestion.source_field);

                    continue;
                }
            }

            // Add the LLM-suggested mapping as a direct mapping
            let field_mapping = FieldMapping::new(
                suggestion.source_field.clone(),
                suggestion.target_field.clone(),
            )
            .with_confidence(suggestion.confidence)
            .with_type_compatible(!suggestion.requires_transform)
            .with_match_method(MatchMethod::Llm);

            mapping.direct_mappings.push(field_mapping);

            // Remove from gaps if present
            mapping
                .gaps
                .retain(|g| g.target_path != suggestion.target_field);

            // Remove from extras if present
            mapping.extras.retain(|e| *e != suggestion.source_field);
        }

        // Recalculate stats
        mapping.stats.direct_mapped = mapping.direct_mappings.len();
        mapping.stats.transform_mapped = mapping.transformations.len();
        mapping.stats.gaps_count = mapping.gaps.len();
        mapping.stats.required_gaps = mapping.gaps.iter().filter(|g| g.required).count();
        mapping.stats.extras_count = mapping.extras.len();

        // Recalculate compatibility score
        mapping.compatibility_score = calculate_compatibility_score(mapping);

        Ok(())
    }

    /// Create a TransformMapping from an LLM hint
    fn create_transform_from_hint(
        &self,
        source_field: &str,
        target_field: &str,
        hint: &str,
        confidence: f64,
    ) -> TransformMapping {
        let hint_lower = hint.to_lowercase();

        // Parse common transform hints into TransformTypes
        let transform_type = if hint_lower.contains("uppercase")
            || hint_lower.contains("upper")
            || hint_lower.contains("lowercase")
            || hint_lower.contains("lower")
            || hint_lower.contains("trim")
        {
            // String transformations as custom expressions
            TransformType::Custom {
                expression: hint.to_string(),
            }
        } else if hint_lower.contains("date") || hint_lower.contains("parse_date") {
            // Date format changes
            TransformType::FormatChange {
                from_format: "auto".to_string(),
                to_format: "ISO8601".to_string(),
            }
        } else if hint_lower.contains("to_string")
            || hint_lower.contains("tostring")
            || hint_lower.contains("stringify")
        {
            TransformType::TypeCast {
                from_type: "any".to_string(),
                to_type: "string".to_string(),
            }
        } else if hint_lower.contains("to_int")
            || hint_lower.contains("toint")
            || hint_lower.contains("parse_int")
        {
            TransformType::TypeCast {
                from_type: "string".to_string(),
                to_type: "integer".to_string(),
            }
        } else if hint_lower.contains("to_float")
            || hint_lower.contains("tofloat")
            || hint_lower.contains("parse_float")
            || hint_lower.contains("to_number")
        {
            TransformType::TypeCast {
                from_type: "string".to_string(),
                to_type: "number".to_string(),
            }
        } else if hint_lower.contains("to_bool")
            || hint_lower.contains("tobool")
            || hint_lower.contains("parse_bool")
        {
            TransformType::TypeCast {
                from_type: "string".to_string(),
                to_type: "boolean".to_string(),
            }
        } else if hint_lower.contains("split") {
            TransformType::Split {
                delimiter: ",".to_string(),
                target_paths: vec![target_field.to_string()],
            }
        } else if hint_lower.contains("join")
            || hint_lower.contains("concat")
            || hint_lower.contains("merge")
        {
            TransformType::Merge {
                separator: Some(",".to_string()),
            }
        } else if hint_lower.contains("extract") || hint_lower.contains("json_path") {
            TransformType::Extract {
                json_path: format!("$.{}", source_field),
            }
        } else if hint_lower.contains("default") || hint_lower.contains("fallback") {
            TransformType::Default { value: Value::Null }
        } else if hint_lower.contains("rename") {
            TransformType::Rename
        } else {
            // Use custom expression for unrecognized hints
            TransformType::Custom {
                expression: hint.to_string(),
            }
        };

        TransformMapping {
            source_paths: vec![source_field.to_string()],
            target_path: target_field.to_string(),
            transform_type,
            description: format!("LLM suggested: {}", hint),
            confidence,
        }
    }
}

/// Field information for prompt building
#[derive(Debug, Clone)]
struct FieldPromptInfo {
    field_type: String,
    description: Option<String>,
    format: Option<String>,
    required: bool,
    example: Option<Value>,
}

/// Extract field information for prompt building
fn extract_field_info(schema: &Value) -> MappingResult<HashMap<String, FieldPromptInfo>> {
    let mut fields = HashMap::new();

    let properties = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .ok_or_else(|| {
            MappingError::InvalidSchema("Schema must have 'properties' object".to_string())
        })?;

    let required_fields: std::collections::HashSet<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    extract_field_info_recursive(properties, &required_fields, "", &mut fields);

    Ok(fields)
}

fn extract_field_info_recursive(
    properties: &serde_json::Map<String, Value>,
    required: &std::collections::HashSet<&str>,
    prefix: &str,
    fields: &mut HashMap<String, FieldPromptInfo>,
) {
    for (name, prop) in properties {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", prefix, name)
        };

        let field_type = prop
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("any")
            .to_string();

        let description = prop
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);

        let format = prop
            .get("format")
            .and_then(|f| f.as_str())
            .map(String::from);

        let is_required = required.contains(name.as_str());

        // Extract example values - check both "example" and "examples" fields
        let example = prop
            .get("example")
            .cloned()
            .or_else(|| {
                prop.get("examples")
                    .and_then(|e| e.as_array())
                    .and_then(|arr| arr.first())
                    .cloned()
            })
            .or_else(|| {
                // Also check for "default" as a hint for typical values
                prop.get("default").cloned()
            });

        fields.insert(
            path.clone(),
            FieldPromptInfo {
                field_type: field_type.clone(),
                description,
                format,
                required: is_required,
                example,
            },
        );

        // Recurse into nested objects
        if field_type == "object" {
            if let Some(nested_props) = prop.get("properties").and_then(|p| p.as_object()) {
                let nested_required: std::collections::HashSet<&str> = prop
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();

                extract_field_info_recursive(nested_props, &nested_required, &path, fields);
            }
        }
    }
}

/// Format fields for the prompt
fn format_fields_for_prompt(
    fields: &[(&String, &FieldPromptInfo)],
    include_descriptions: bool,
    include_examples: bool,
) -> String {
    fields
        .iter()
        .map(|(path, info)| {
            let mut line = format!("- {}: {} ", path, info.field_type);

            if info.required {
                line.push_str("[required] ");
            }

            if let Some(ref format) = info.format {
                line.push_str(&format!("(format: {}) ", format));
            }

            if include_examples {
                if let Some(ref example) = info.example {
                    // Format example compactly
                    let example_str = match example {
                        Value::String(s) => format!("\"{}\"", s),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        _ => serde_json::to_string(example).unwrap_or_default(),
                    };
                    // Truncate long examples
                    let truncated = if example_str.len() > 50 {
                        format!("{}...", &example_str[..47])
                    } else {
                        example_str
                    };
                    line.push_str(&format!("(example: {}) ", truncated));
                }
            }

            if include_descriptions {
                if let Some(ref desc) = info.description {
                    line.push_str(&format!("- {}", desc));
                }
            }

            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract JSON from LLM response (handles markdown code blocks)
fn extract_json_from_response(response: &str) -> MappingResult<String> {
    let trimmed = response.trim();

    // Try direct parse first
    if trimmed.starts_with('{') {
        return Ok(trimmed.to_string());
    }

    // Try to extract from markdown code block
    if let Some(start) = trimmed.find("```json") {
        let json_start = start + 7;
        if let Some(end) = trimmed[json_start..].find("```") {
            return Ok(trimmed[json_start..json_start + end].trim().to_string());
        }
    }

    // Try generic code block
    if let Some(start) = trimmed.find("```") {
        let json_start = start + 3;
        // Skip optional language identifier
        let content = &trimmed[json_start..];
        let actual_start = content.find('\n').map(|i| i + 1).unwrap_or(0);
        if let Some(end) = content[actual_start..].find("```") {
            return Ok(content[actual_start..actual_start + end].trim().to_string());
        }
    }

    // Try to find JSON object in response
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return Ok(trimmed[start..=end].to_string());
        }
    }

    Err(MappingError::LlmError(
        "Could not extract JSON from LLM response".to_string(),
    ))
}

/// Calculate compatibility score for a mapping
fn calculate_compatibility_score(mapping: &SchemaMapping) -> f64 {
    if mapping.stats.target_fields == 0 {
        return 1.0;
    }

    let direct_score: f64 = mapping
        .direct_mappings
        .iter()
        .map(|m| m.confidence * if m.type_compatible { 1.0 } else { 0.8 })
        .sum();

    let transform_score: f64 = mapping
        .transformations
        .iter()
        .map(|t| t.confidence * 0.9)
        .sum();

    let total_mapped = direct_score + transform_score;
    let gap_penalty = mapping.stats.required_gaps as f64 * 0.2;

    let raw_score = total_mapped / mapping.stats.target_fields as f64 - gap_penalty;
    raw_score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_llm_matcher_config_default() {
        let config = LlmMatcherConfig::default();
        assert_eq!(config.min_confidence, 0.7);
        assert!(config.include_descriptions);
        assert!(config.include_examples);
    }

    #[test]
    fn test_llm_matcher_config_builder() {
        let config = LlmMatcherConfig::new()
            .with_min_confidence(0.8)
            .with_descriptions(false)
            .with_examples(false);

        assert_eq!(config.min_confidence, 0.8);
        assert!(!config.include_descriptions);
        assert!(!config.include_examples);
    }

    #[test]
    fn test_extract_json_from_response_direct() {
        let response = r#"{"suggestions": [], "overall_confidence": 0.9}"#;
        let json = extract_json_from_response(response).unwrap();
        assert!(json.starts_with('{'));
    }

    #[test]
    fn test_extract_json_from_response_markdown() {
        let response = r#"Here's the mapping:

```json
{"suggestions": [], "overall_confidence": 0.9}
```

That's my analysis."#;
        let json = extract_json_from_response(response).unwrap();
        assert!(json.contains("suggestions"));
    }

    #[test]
    fn test_extract_json_from_response_embedded() {
        let response = r#"The result is {"suggestions": [], "overall_confidence": 0.9} as shown."#;
        let json = extract_json_from_response(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_field_info() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The user's full name"
                },
                "email": {
                    "type": "string",
                    "format": "email"
                }
            },
            "required": ["name"]
        });

        let fields = extract_field_info(&schema).unwrap();
        assert_eq!(fields.len(), 2);
        assert!(fields.get("name").unwrap().required);
        assert!(!fields.get("email").unwrap().required);
        assert_eq!(
            fields.get("email").unwrap().format,
            Some("email".to_string())
        );
    }

    #[test]
    fn test_format_fields_for_prompt() {
        let fields: HashMap<String, FieldPromptInfo> = [
            (
                "name".to_string(),
                FieldPromptInfo {
                    field_type: "string".to_string(),
                    description: Some("User name".to_string()),
                    format: None,
                    required: true,
                    example: Some(Value::String("John Doe".to_string())),
                },
            ),
            (
                "age".to_string(),
                FieldPromptInfo {
                    field_type: "integer".to_string(),
                    description: None,
                    format: None,
                    required: false,
                    example: Some(Value::Number(serde_json::Number::from(25))),
                },
            ),
        ]
        .into_iter()
        .collect();

        let field_refs: Vec<_> = fields.iter().collect();
        let output = format_fields_for_prompt(&field_refs, true, true);

        assert!(output.contains("name"));
        assert!(output.contains("string"));
        assert!(output.contains("[required]"));
        assert!(output.contains("John Doe"));
        assert!(output.contains("25"));
    }

    #[test]
    fn test_format_fields_without_examples() {
        let fields: HashMap<String, FieldPromptInfo> = [(
            "email".to_string(),
            FieldPromptInfo {
                field_type: "string".to_string(),
                description: Some("Email address".to_string()),
                format: Some("email".to_string()),
                required: true,
                example: Some(Value::String("user@example.com".to_string())),
            },
        )]
        .into_iter()
        .collect();

        let field_refs: Vec<_> = fields.iter().collect();

        // With examples disabled, shouldn't contain the example value
        let output_no_examples = format_fields_for_prompt(&field_refs, true, false);
        assert!(output_no_examples.contains("email"));
        assert!(output_no_examples.contains("Email address"));
        assert!(!output_no_examples.contains("user@example.com"));

        // With examples enabled, should contain the example value
        let output_with_examples = format_fields_for_prompt(&field_refs, true, true);
        assert!(output_with_examples.contains("user@example.com"));
    }

    #[test]
    fn test_llm_field_suggestion_serialization() {
        let suggestion = LlmFieldSuggestion {
            source_field: "customer_name".to_string(),
            target_field: "name".to_string(),
            confidence: 0.85,
            reasoning: "Semantic match".to_string(),
            requires_transform: false,
            transform_hint: None,
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        let parsed: LlmFieldSuggestion = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.source_field, "customer_name");
        assert_eq!(parsed.confidence, 0.85);
    }

    #[test]
    fn test_llm_match_response_parsing() {
        let json = r#"{
            "suggestions": [
                {
                    "source_field": "cust_name",
                    "target_field": "customer_name",
                    "confidence": 0.9,
                    "reasoning": "Abbreviation match",
                    "requires_transform": false,
                    "transform_hint": null
                }
            ],
            "unmatched_source": ["internal_id"],
            "unmatched_target": ["created_at"],
            "overall_confidence": 0.85
        }"#;

        let response: LlmMatchResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.suggestions.len(), 1);
        assert_eq!(response.suggestions[0].source_field, "cust_name");
        assert_eq!(response.unmatched_source.len(), 1);
        assert_eq!(response.unmatched_target.len(), 1);
        assert_eq!(response.overall_confidence, 0.85);
    }

    #[cfg(feature = "llm")]
    mod llm_tests {
        use super::*;
        use crate::llm::client::MockLlmClient;

        #[tokio::test]
        async fn test_llm_matcher_with_mock() {
            let mock_response = r#"{
                "suggestions": [
                    {
                        "source_field": "usr_email",
                        "target_field": "email_address",
                        "confidence": 0.9,
                        "reasoning": "Both represent user email",
                        "requires_transform": false,
                        "transform_hint": null
                    }
                ],
                "unmatched_source": [],
                "unmatched_target": [],
                "overall_confidence": 0.9
            }"#;

            let client = MockLlmClient::new(mock_response);
            let matcher = LlmSchemaMatcher::new(client);

            let source = json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "usr_email": {"type": "string"}
                }
            });

            let target = json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "email_address": {"type": "string"}
                }
            });

            let mapping = matcher.match_schemas(&source, &target).await.unwrap();

            // Should have matched 'name' exactly and 'usr_email' -> 'email_address' via LLM
            assert_eq!(mapping.direct_mappings.len(), 2);
            assert!(mapping.gaps.is_empty());
        }

        #[tokio::test]
        async fn test_llm_matcher_respects_confidence_threshold() {
            let mock_response = r#"{
                "suggestions": [
                    {
                        "source_field": "x",
                        "target_field": "y",
                        "confidence": 0.5,
                        "reasoning": "Low confidence match",
                        "requires_transform": false,
                        "transform_hint": null
                    }
                ],
                "unmatched_source": [],
                "unmatched_target": ["y"],
                "overall_confidence": 0.5
            }"#;

            let client = MockLlmClient::new(mock_response);
            let config = LlmMatcherConfig::new().with_min_confidence(0.7);
            let matcher = LlmSchemaMatcher::with_config(client, config, MappingConfig::strict());

            let source = json!({
                "type": "object",
                "properties": {
                    "x": {"type": "string"}
                }
            });

            let target = json!({
                "type": "object",
                "properties": {
                    "y": {"type": "string"}
                }
            });

            let mapping = matcher.match_schemas(&source, &target).await.unwrap();

            // Low confidence suggestion should be rejected
            assert!(mapping.direct_mappings.is_empty());
            assert_eq!(mapping.gaps.len(), 1);
        }
    }
}
