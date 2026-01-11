//! Prompt templates for LLM-based schema refinement
//!
//! This module provides prompt templates and utilities for constructing
//! prompts that guide the LLM to refine inferred schemas.

use serde::{Deserialize, Serialize};

/// Prompt template for schema refinement
pub const REFINEMENT_PROMPT_TEMPLATE: &str = r#"You are a data modeling expert. Your task is to refine an automatically inferred JSON schema based on the provided context.

## Rules
1. NEVER rename any fields - preserve all original field names exactly
2. NEVER remove any fields from the schema
3. Only make ADDITIVE changes:
   - Add descriptions to fields
   - Add format hints (e.g., "date-time", "email", "uri")
   - Narrow types where appropriate (e.g., "string" -> "string with format")
   - Add enum values if a field has a known set of values
   - Add constraints (minLength, maxLength, minimum, maximum, pattern)
4. If documentation mentions a field's purpose, add it as a description
5. If sample data shows a pattern, add appropriate format or constraints

## Input Schema
```json
{schema}
```

{documentation_section}

{samples_section}

## Output
Return ONLY a valid JSON object with the refined schema. Do not include any explanation or markdown formatting.
The output must be valid JSON that can be parsed directly."#;

/// Prompt template for describing a single field
pub const FIELD_DESCRIPTION_PROMPT: &str = r#"Based on the following context, provide a brief description (1-2 sentences) for a database field.

Field name: {field_name}
Field type: {field_type}
Sample values: {sample_values}

{documentation_context}

Return only the description text, no JSON or formatting."#;

/// Context for building a refinement prompt
#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// The inferred schema as JSON
    pub schema_json: String,
    /// Optional documentation text
    pub documentation: Option<String>,
    /// Sample records as JSON
    pub samples: Vec<String>,
    /// Maximum tokens for the prompt
    pub max_tokens: usize,
}

impl PromptContext {
    /// Create a new prompt context
    pub fn new(schema_json: impl Into<String>) -> Self {
        Self {
            schema_json: schema_json.into(),
            documentation: None,
            samples: Vec::new(),
            max_tokens: 4096,
        }
    }

    /// Add documentation context
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Add sample records
    pub fn with_samples(mut self, samples: Vec<String>) -> Self {
        self.samples = samples;
        self
    }

    /// Set maximum tokens
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_tokens = max;
        self
    }

    /// Build the refinement prompt
    pub fn build_prompt(&self) -> String {
        let documentation_section = if let Some(doc) = &self.documentation {
            format!(
                "## Documentation Context\n```\n{}\n```\n",
                truncate_to_tokens(doc, self.max_tokens / 4)
            )
        } else {
            String::new()
        };

        let samples_section = if !self.samples.is_empty() {
            let samples_text = self
                .samples
                .iter()
                .take(5)
                .enumerate()
                .map(|(i, s)| format!("Sample {}: {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "## Sample Records\n```json\n{}\n```\n",
                truncate_to_tokens(&samples_text, self.max_tokens / 4)
            )
        } else {
            String::new()
        };

        REFINEMENT_PROMPT_TEMPLATE
            .replace("{schema}", &self.schema_json)
            .replace("{documentation_section}", &documentation_section)
            .replace("{samples_section}", &samples_section)
    }
}

/// Result of parsing an LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRefinement {
    /// The refined schema
    pub schema: serde_json::Value,
    /// Any warnings during parsing
    pub warnings: Vec<String>,
}

/// Parse the LLM response to extract the refined schema
///
/// This function handles various response formats:
/// - Pure JSON
/// - JSON wrapped in markdown code blocks
/// - JSON with leading/trailing text
pub fn parse_llm_response(response: &str) -> Result<ParsedRefinement, String> {
    let warnings = Vec::new();

    // Try to extract JSON from the response
    let json_str = extract_json(response);

    // Parse as JSON
    let schema: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
        format!(
            "Failed to parse JSON: {}. Response was: {}",
            e,
            &response[..response.len().min(200)]
        )
    })?;

    // Verify it's an object
    if !schema.is_object() {
        return Err("Response is not a JSON object".to_string());
    }

    Ok(ParsedRefinement { schema, warnings })
}

/// Extract JSON from a response that may contain markdown or other text
fn extract_json(response: &str) -> String {
    let trimmed = response.trim();

    // Try to find JSON in code blocks
    if let Some(start) = trimmed.find("```json") {
        let content_start = start + 7;
        if let Some(end) = trimmed[content_start..].find("```") {
            return trimmed[content_start..content_start + end]
                .trim()
                .to_string();
        }
    }

    // Try to find generic code blocks
    if let Some(start) = trimmed.find("```") {
        let content_start = start + 3;
        // Skip language identifier if present
        let content_start = trimmed[content_start..]
            .find('\n')
            .map(|n| content_start + n + 1)
            .unwrap_or(content_start);
        if let Some(end) = trimmed[content_start..].find("```") {
            return trimmed[content_start..content_start + end]
                .trim()
                .to_string();
        }
    }

    // Try to find JSON object directly
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return trimmed[start..=end].to_string();
            }
        }
    }

    // Return as-is
    trimmed.to_string()
}

/// Truncate text to approximately the given number of tokens
///
/// Uses a rough estimate of 4 characters per token for English text
fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4;
    if text.len() <= max_chars {
        return text.to_string();
    }

    // Truncate at word boundary
    let truncated = &text[..max_chars];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &truncated[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

/// Estimate the token count for a piece of text
///
/// Uses a rough estimate of 4 characters per token
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_context_basic() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
        let context = PromptContext::new(schema);
        let prompt = context.build_prompt();

        assert!(prompt.contains("data modeling expert"));
        assert!(prompt.contains(schema));
        assert!(!prompt.contains("Documentation Context"));
        assert!(!prompt.contains("Sample Records"));
    }

    #[test]
    fn test_prompt_context_with_documentation() {
        let schema = r#"{"type": "object"}"#;
        let context =
            PromptContext::new(schema).with_documentation("This table stores customer information");
        let prompt = context.build_prompt();

        assert!(prompt.contains("Documentation Context"));
        assert!(prompt.contains("customer information"));
    }

    #[test]
    fn test_prompt_context_with_samples() {
        let schema = r#"{"type": "object"}"#;
        let samples = vec![
            r#"{"name": "Alice", "age": 30}"#.to_string(),
            r#"{"name": "Bob", "age": 25}"#.to_string(),
        ];
        let context = PromptContext::new(schema).with_samples(samples);
        let prompt = context.build_prompt();

        assert!(prompt.contains("Sample Records"));
        assert!(prompt.contains("Alice"));
        assert!(prompt.contains("Sample 1"));
        assert!(prompt.contains("Sample 2"));
    }

    #[test]
    fn test_parse_llm_response_pure_json() {
        let response = r#"{"type": "object", "properties": {}}"#;
        let result = parse_llm_response(response).unwrap();
        assert!(result.schema.is_object());
    }

    #[test]
    fn test_parse_llm_response_markdown_json() {
        let response = r#"Here's the refined schema:

```json
{"type": "object", "properties": {"name": {"type": "string"}}}
```

This schema includes..."#;

        let result = parse_llm_response(response).unwrap();
        assert!(result.schema.is_object());
        assert!(result.schema.get("properties").is_some());
    }

    #[test]
    fn test_parse_llm_response_generic_code_block() {
        let response = r#"```
{"type": "object"}
```"#;

        let result = parse_llm_response(response).unwrap();
        assert!(result.schema.is_object());
    }

    #[test]
    fn test_parse_llm_response_with_text() {
        let response = r#"Based on my analysis, the refined schema is:
{"type": "object", "properties": {"id": {"type": "integer"}}}
I've added the integer type for the id field."#;

        let result = parse_llm_response(response).unwrap();
        assert!(result.schema.is_object());
    }

    #[test]
    fn test_parse_llm_response_invalid() {
        let response = "This is not valid JSON";
        let result = parse_llm_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_json_code_block() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        let result = extract_json(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_direct() {
        let input = "Some text {\"key\": \"value\"} more text";
        let result = extract_json(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_truncate_to_tokens() {
        let short = "Hello world";
        assert_eq!(truncate_to_tokens(short, 100), short);

        let long = "a ".repeat(100);
        let truncated = truncate_to_tokens(&long, 10);
        assert!(truncated.len() < long.len());
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("hello world"), 3);
        assert_eq!(estimate_tokens(&"a".repeat(100)), 25);
    }
}
