//! Schema inference engine

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::config::InferenceConfig;
use super::error::InferenceError;
use super::formats::{Format, detect_format};
use super::types::{FieldStats, InferredField, InferredSchema, InferredType};

/// Statistics from schema inference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceStats {
    /// Total records processed
    pub records_processed: usize,
    /// Records skipped (invalid JSON)
    pub records_skipped: usize,
    /// Total fields discovered
    pub fields_discovered: usize,
    /// Maximum nesting depth encountered
    pub max_depth: usize,
    /// Types detected by field path
    pub type_distribution: HashMap<String, HashMap<String, usize>>,
}

/// Schema inference engine
///
/// Analyzes JSON records and builds a schema definition.
pub struct SchemaInferrer {
    config: InferenceConfig,
    /// Field type tracking: path -> list of observed types
    field_types: HashMap<String, Vec<InferredType>>,
    /// Field occurrence counts
    field_occurrences: HashMap<String, usize>,
    /// Null occurrences per field
    field_nulls: HashMap<String, usize>,
    /// Example values per field
    field_examples: HashMap<String, Vec<Value>>,
    /// Numeric stats per field
    field_numeric_stats: HashMap<String, NumericStats>,
    /// Total records processed
    record_count: usize,
    /// Records skipped
    skipped_count: usize,
    /// Maximum depth seen
    max_depth_seen: usize,
}

#[derive(Debug, Clone, Default)]
struct NumericStats {
    min: f64,
    max: f64,
    sum: f64,
    count: usize,
}

impl NumericStats {
    fn add(&mut self, value: f64) {
        if self.count == 0 {
            self.min = value;
            self.max = value;
        } else {
            self.min = self.min.min(value);
            self.max = self.max.max(value);
        }
        self.sum += value;
        self.count += 1;
    }

    fn avg(&self) -> Option<f64> {
        if self.count > 0 {
            Some(self.sum / self.count as f64)
        } else {
            None
        }
    }
}

impl SchemaInferrer {
    /// Create a new schema inferrer with default configuration
    pub fn new() -> Self {
        Self::with_config(InferenceConfig::default())
    }

    /// Create a new schema inferrer with custom configuration
    pub fn with_config(config: InferenceConfig) -> Self {
        Self {
            config,
            field_types: HashMap::new(),
            field_occurrences: HashMap::new(),
            field_nulls: HashMap::new(),
            field_examples: HashMap::new(),
            field_numeric_stats: HashMap::new(),
            record_count: 0,
            skipped_count: 0,
            max_depth_seen: 0,
        }
    }

    /// Add a single JSON string for analysis
    pub fn add_json(&mut self, json: &str) -> Result<(), InferenceError> {
        // Check sample size limit
        if self.config.sample_size > 0 && self.record_count >= self.config.sample_size {
            return Ok(());
        }

        let value: Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(_) => {
                self.skipped_count += 1;
                return Ok(());
            }
        };

        self.add_value(&value)
    }

    /// Add a parsed JSON value for analysis
    pub fn add_value(&mut self, value: &Value) -> Result<(), InferenceError> {
        // Check sample size limit
        if self.config.sample_size > 0 && self.record_count >= self.config.sample_size {
            return Ok(());
        }

        // Root must be an object
        if !value.is_object() {
            return Err(InferenceError::InvalidStructure(
                value_type_name(value).to_string(),
            ));
        }

        self.record_count += 1;
        self.analyze_value(value, "$", 0)?;

        Ok(())
    }

    /// Add a batch of JSON strings
    pub fn add_json_batch(&mut self, records: &[String]) -> Result<(), InferenceError> {
        for json in records {
            self.add_json(json)?;
        }
        Ok(())
    }

    /// Analyze a JSON value at a given path
    fn analyze_value(
        &mut self,
        value: &Value,
        path: &str,
        depth: usize,
    ) -> Result<(), InferenceError> {
        if depth > self.config.max_depth {
            return Err(InferenceError::MaxDepthExceeded {
                depth,
                max: self.config.max_depth,
            });
        }

        self.max_depth_seen = self.max_depth_seen.max(depth);

        // Track field occurrence
        *self.field_occurrences.entry(path.to_string()).or_insert(0) += 1;

        // Infer and track type
        let inferred_type = self.infer_type(value, path, depth)?;
        self.field_types
            .entry(path.to_string())
            .or_default()
            .push(inferred_type);

        // Track nulls
        if value.is_null() {
            *self.field_nulls.entry(path.to_string()).or_insert(0) += 1;
        }

        // Collect examples
        if self.config.collect_examples && !value.is_object() && !value.is_array() {
            let examples = self.field_examples.entry(path.to_string()).or_default();
            if examples.len() < self.config.max_examples && !examples.contains(value) {
                examples.push(value.clone());
            }
        }

        // Track numeric stats
        if let Some(n) = value.as_f64() {
            self.field_numeric_stats
                .entry(path.to_string())
                .or_default()
                .add(n);
        }

        Ok(())
    }

    /// Infer the type of a JSON value
    fn infer_type(
        &mut self,
        value: &Value,
        path: &str,
        depth: usize,
    ) -> Result<InferredType, InferenceError> {
        match value {
            Value::Null => Ok(InferredType::Null),
            Value::Bool(_) => Ok(InferredType::Boolean),
            Value::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    Ok(InferredType::Integer)
                } else {
                    Ok(InferredType::Number)
                }
            }
            Value::String(s) => {
                let format = if self.config.detect_formats {
                    let detected = detect_format(s);
                    if detected != Format::None {
                        Some(detected)
                    } else {
                        None
                    }
                } else {
                    None
                };
                Ok(InferredType::String { format })
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    Ok(InferredType::Array {
                        items: Box::new(InferredType::Unknown),
                    })
                } else {
                    // Analyze all items and merge types
                    let mut item_type = InferredType::Unknown;
                    let item_path = format!("{}[]", path);

                    for item in arr {
                        self.analyze_value(item, &item_path, depth + 1)?;
                        let t = self.infer_type(item, &item_path, depth + 1)?;
                        item_type = item_type.merge_with(t);
                    }

                    Ok(InferredType::Array {
                        items: Box::new(item_type),
                    })
                }
            }
            Value::Object(obj) => {
                let mut properties = BTreeMap::new();

                for (key, val) in obj {
                    let field_path = format!("{}.{}", path, key);
                    self.analyze_value(val, &field_path, depth + 1)?;

                    let field_type = self.infer_type(val, &field_path, depth + 1)?;
                    let nullable = val.is_null();

                    let mut field = InferredField::new(field_type);
                    field.nullable = nullable;

                    // Add example if configured
                    if self.config.collect_examples && !val.is_object() && !val.is_array() {
                        field.add_example(val.clone(), self.config.max_examples);
                    }

                    properties.insert(key.clone(), field);
                }

                Ok(InferredType::Object { properties })
            }
        }
    }

    /// Finalize inference and generate schema
    pub fn finalize(self) -> Result<InferredSchema, InferenceError> {
        if self.record_count == 0 {
            return Err(InferenceError::NoRecords);
        }

        // Build the root type by analyzing collected data
        let root = self.build_root_type()?;

        // Build field stats
        let mut field_stats = HashMap::new();
        for (path, occurrences) in &self.field_occurrences {
            let null_count = self.field_nulls.get(path).copied().unwrap_or(0);
            let numeric = self.field_numeric_stats.get(path);

            field_stats.insert(
                path.clone(),
                FieldStats {
                    occurrences: *occurrences,
                    null_count,
                    distinct_count: self.field_examples.get(path).map(|e| e.len()),
                    min: numeric.map(|n| n.min),
                    max: numeric.map(|n| n.max),
                    avg: numeric.and_then(|n| n.avg()),
                },
            );
        }

        Ok(InferredSchema {
            name: None,
            description: None,
            root,
            record_count: self.record_count,
            partition: None,
            field_stats,
        })
    }

    /// Build the root type from collected data
    fn build_root_type(&self) -> Result<InferredType, InferenceError> {
        // Find all top-level fields ($.fieldname)
        let mut properties = BTreeMap::new();

        for (path, types) in &self.field_types {
            // Only process direct children of root
            if !path.starts_with("$.") || path.matches('.').count() != 1 {
                continue;
            }

            let field_name = path.strip_prefix("$.").unwrap();
            if field_name.contains('.') || field_name.contains('[') {
                continue;
            }

            // Merge all observed types
            let mut primary_type = InferredType::Unknown;
            for t in types {
                primary_type = primary_type.merge_with(t.clone());
            }

            // If we have object types, recursively build nested structure
            if matches!(primary_type, InferredType::Object { .. }) {
                primary_type = self.build_nested_type(path)?;
            }

            // Calculate field properties
            let occurrences = self.field_occurrences.get(path).copied().unwrap_or(0);
            let null_count = self.field_nulls.get(path).copied().unwrap_or(0);
            let frequency = occurrences as f64 / self.record_count as f64;

            let mut field = InferredField::new(primary_type);
            field.required = frequency >= 1.0 - f64::EPSILON;
            field.nullable = null_count > 0;
            field.occurrences = occurrences;

            // Check frequency threshold
            if frequency < self.config.min_field_frequency {
                continue;
            }

            // Add examples
            if let Some(examples) = self.field_examples.get(path) {
                field.examples = examples.clone();
            }

            properties.insert(field_name.to_string(), field);
        }

        Ok(InferredType::Object { properties })
    }

    /// Build nested type structure
    fn build_nested_type(&self, parent_path: &str) -> Result<InferredType, InferenceError> {
        let prefix = format!("{}.", parent_path);
        let mut properties = BTreeMap::new();

        for (path, types) in &self.field_types {
            if !path.starts_with(&prefix) {
                continue;
            }

            let rest = path.strip_prefix(&prefix).unwrap();
            // Only direct children
            if rest.contains('.') || rest.contains('[') {
                continue;
            }

            // Merge all observed types
            let mut primary_type = InferredType::Unknown;
            for t in types {
                primary_type = primary_type.merge_with(t.clone());
            }

            // Recursively build if object
            if matches!(primary_type, InferredType::Object { .. }) {
                primary_type = self.build_nested_type(path)?;
            }

            let occurrences = self.field_occurrences.get(path).copied().unwrap_or(0);
            let null_count = self.field_nulls.get(path).copied().unwrap_or(0);

            let mut field = InferredField::new(primary_type);
            field.nullable = null_count > 0;
            field.occurrences = occurrences;

            if let Some(examples) = self.field_examples.get(path) {
                field.examples = examples.clone();
            }

            properties.insert(rest.to_string(), field);
        }

        Ok(InferredType::Object { properties })
    }

    /// Get current inference statistics
    pub fn stats(&self) -> InferenceStats {
        let mut type_distribution = HashMap::new();

        for (path, types) in &self.field_types {
            let mut dist: HashMap<String, usize> = HashMap::new();
            for t in types {
                *dist.entry(t.type_name().to_string()).or_insert(0) += 1;
            }
            type_distribution.insert(path.clone(), dist);
        }

        InferenceStats {
            records_processed: self.record_count,
            records_skipped: self.skipped_count,
            fields_discovered: self.field_occurrences.len(),
            max_depth: self.max_depth_seen,
            type_distribution,
        }
    }

    /// Get the number of records processed
    pub fn record_count(&self) -> usize {
        self.record_count
    }
}

impl Default for SchemaInferrer {
    fn default() -> Self {
        Self::new()
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_simple_object() {
        let mut inferrer = SchemaInferrer::new();

        inferrer
            .add_json(r#"{"name": "Alice", "age": 30}"#)
            .unwrap();
        inferrer.add_json(r#"{"name": "Bob", "age": 25}"#).unwrap();

        let schema = inferrer.finalize().unwrap();
        assert_eq!(schema.record_count, 2);

        if let InferredType::Object { properties } = schema.root {
            assert!(properties.contains_key("name"));
            assert!(properties.contains_key("age"));
            assert_eq!(
                properties["name"].field_type,
                InferredType::String { format: None }
            );
            assert_eq!(properties["age"].field_type, InferredType::Integer);
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_infer_optional_fields() {
        let mut inferrer = SchemaInferrer::new();

        inferrer
            .add_json(r#"{"name": "Alice", "email": "alice@example.com"}"#)
            .unwrap();
        inferrer.add_json(r#"{"name": "Bob"}"#).unwrap();

        let schema = inferrer.finalize().unwrap();

        if let InferredType::Object { properties } = schema.root {
            assert!(properties["name"].required);
            // email only appears in 50% of records
            assert!(!properties["email"].required);
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_infer_nested_object() {
        let mut inferrer = SchemaInferrer::new();

        inferrer
            .add_json(r#"{"user": {"name": "Alice", "age": 30}}"#)
            .unwrap();

        let schema = inferrer.finalize().unwrap();

        if let InferredType::Object { properties } = &schema.root {
            if let InferredType::Object { properties: nested } = &properties["user"].field_type {
                assert!(nested.contains_key("name"));
                assert!(nested.contains_key("age"));
            } else {
                panic!("Expected nested object");
            }
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_infer_array() {
        let mut inferrer = SchemaInferrer::new();

        inferrer.add_json(r#"{"tags": ["a", "b", "c"]}"#).unwrap();

        let schema = inferrer.finalize().unwrap();

        if let InferredType::Object { properties } = &schema.root {
            if let InferredType::Array { items } = &properties["tags"].field_type {
                assert_eq!(**items, InferredType::String { format: None });
            } else {
                panic!("Expected array type");
            }
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_infer_format_detection() {
        let mut inferrer = SchemaInferrer::new();

        inferrer
            .add_json(r#"{"id": "550e8400-e29b-41d4-a716-446655440000", "date": "2024-01-15"}"#)
            .unwrap();

        let schema = inferrer.finalize().unwrap();

        if let InferredType::Object { properties } = &schema.root {
            if let InferredType::String { format } = &properties["id"].field_type {
                assert_eq!(*format, Some(super::super::formats::Format::Uuid));
            } else {
                panic!("Expected string type for id");
            }
            if let InferredType::String { format } = &properties["date"].field_type {
                assert_eq!(*format, Some(super::super::formats::Format::Date));
            } else {
                panic!("Expected string type for date");
            }
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_infer_nullable() {
        let mut inferrer = SchemaInferrer::new();

        inferrer
            .add_json(r#"{"name": "Alice", "nickname": null}"#)
            .unwrap();
        inferrer
            .add_json(r#"{"name": "Bob", "nickname": "Bobby"}"#)
            .unwrap();

        let schema = inferrer.finalize().unwrap();

        if let InferredType::Object { properties } = schema.root {
            assert!(!properties["name"].nullable);
            assert!(properties["nickname"].nullable);
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_infer_mixed_types() {
        let mut inferrer = SchemaInferrer::new();

        inferrer.add_json(r#"{"value": 42}"#).unwrap();
        inferrer.add_json(r#"{"value": "text"}"#).unwrap();

        let schema = inferrer.finalize().unwrap();

        if let InferredType::Object { properties } = schema.root {
            // Should be a mixed type
            match &properties["value"].field_type {
                InferredType::Mixed { types } => {
                    assert!(types.len() >= 2);
                }
                _ => panic!("Expected mixed type"),
            }
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_stats() {
        let mut inferrer = SchemaInferrer::new();

        inferrer
            .add_json(r#"{"name": "Alice", "age": 30}"#)
            .unwrap();
        inferrer.add_json(r#"{"name": "Bob", "age": 25}"#).unwrap();

        let stats = inferrer.stats();
        assert_eq!(stats.records_processed, 2);
        assert!(stats.fields_discovered > 0);
    }

    #[test]
    fn test_sample_size_limit() {
        let config = InferenceConfig::builder().sample_size(2).build();
        let mut inferrer = SchemaInferrer::with_config(config);

        inferrer.add_json(r#"{"a": 1}"#).unwrap();
        inferrer.add_json(r#"{"a": 2}"#).unwrap();
        inferrer.add_json(r#"{"a": 3}"#).unwrap(); // Should be ignored

        assert_eq!(inferrer.record_count(), 2);
    }
}
