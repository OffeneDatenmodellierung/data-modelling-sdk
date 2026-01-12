//! Type inference for JSON values

#![allow(clippy::collapsible_if)]
#![allow(clippy::only_used_in_recursion)]

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::formats::Format;

/// Inferred JSON type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InferredType {
    /// Null type
    Null,
    /// Boolean type
    Boolean,
    /// Integer type (whole numbers)
    Integer,
    /// Number type (floating point)
    Number,
    /// String type with optional format
    String { format: Option<Format> },
    /// Array type with item type
    Array { items: Box<InferredType> },
    /// Object type with properties
    Object {
        properties: BTreeMap<String, InferredField>,
    },
    /// Mixed types (oneOf in JSON Schema)
    Mixed { types: Vec<InferredType> },
    /// Unknown (no samples)
    Unknown,
}

impl InferredType {
    /// Get the JSON Schema type name
    pub fn type_name(&self) -> &'static str {
        match self {
            InferredType::Null => "null",
            InferredType::Boolean => "boolean",
            InferredType::Integer => "integer",
            InferredType::Number => "number",
            InferredType::String { .. } => "string",
            InferredType::Array { .. } => "array",
            InferredType::Object { .. } => "object",
            InferredType::Mixed { .. } => "mixed",
            InferredType::Unknown => "unknown",
        }
    }

    /// Check if this type can be merged with another type
    pub fn can_merge_with(&self, other: &InferredType) -> bool {
        match (self, other) {
            // Same types can always merge
            (a, b) if a == b => true,
            // Integer can be promoted to Number
            (InferredType::Integer, InferredType::Number) => true,
            (InferredType::Number, InferredType::Integer) => true,
            // Null can be combined with any type (makes it nullable)
            (InferredType::Null, _) | (_, InferredType::Null) => true,
            // Arrays with compatible items
            (InferredType::Array { items: a }, InferredType::Array { items: b }) => {
                a.can_merge_with(b)
            }
            // Objects can be merged (union of properties)
            (InferredType::Object { .. }, InferredType::Object { .. }) => true,
            // Different types become Mixed
            _ => true,
        }
    }

    /// Merge this type with another type
    pub fn merge_with(self, other: InferredType) -> InferredType {
        if self == other {
            return self;
        }

        match (self, other) {
            // Null + X = nullable X (handle at field level)
            (InferredType::Null, other) | (other, InferredType::Null) => other,

            // Integer + Number = Number
            (InferredType::Integer, InferredType::Number)
            | (InferredType::Number, InferredType::Integer) => InferredType::Number,

            // String formats: prefer more specific or drop format
            (InferredType::String { format: f1 }, InferredType::String { format: f2 }) => {
                InferredType::String {
                    format: if f1 == f2 { f1 } else { None },
                }
            }

            // Arrays: merge item types
            (InferredType::Array { items: a }, InferredType::Array { items: b }) => {
                InferredType::Array {
                    items: Box::new((*a).merge_with(*b)),
                }
            }

            // Objects: merge properties
            (
                InferredType::Object { properties: mut p1 },
                InferredType::Object { properties: p2 },
            ) => {
                for (key, field2) in p2 {
                    if let Some(field1) = p1.get_mut(&key) {
                        *field1 = field1.clone().merge_with(field2);
                    } else {
                        // Field only in second object - mark as optional
                        let mut optional_field = field2;
                        optional_field.required = false;
                        p1.insert(key, optional_field);
                    }
                }
                // Mark fields not in p2 as optional
                for field in p1.values_mut() {
                    if !field.required {
                        continue;
                    }
                    // The field exists in p1 but might not have been in p2
                    // This is handled above, but for safety:
                }
                InferredType::Object { properties: p1 }
            }

            // Mixed types: combine
            (InferredType::Mixed { mut types }, other) => {
                if !types.contains(&other) {
                    types.push(other);
                }
                InferredType::Mixed { types }
            }
            (other, InferredType::Mixed { mut types }) => {
                if !types.contains(&other) {
                    types.push(other);
                }
                InferredType::Mixed { types }
            }

            // Unknown + anything = anything
            (InferredType::Unknown, other) | (other, InferredType::Unknown) => other,

            // Different primitive types become Mixed
            (a, b) => InferredType::Mixed { types: vec![a, b] },
        }
    }
}

/// An inferred field in a schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferredField {
    /// The inferred type of this field
    pub field_type: InferredType,
    /// Whether this field is required (appears in all records)
    pub required: bool,
    /// Whether this field can be null
    pub nullable: bool,
    /// Number of occurrences
    pub occurrences: usize,
    /// Example values (if collection enabled)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<serde_json::Value>,
    /// Description (can be set by LLM later)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl InferredField {
    /// Create a new inferred field
    pub fn new(field_type: InferredType) -> Self {
        Self {
            field_type,
            required: true,
            nullable: false,
            occurrences: 1,
            examples: Vec::new(),
            description: None,
        }
    }

    /// Mark this field as nullable
    pub fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Mark this field as optional
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Add an example value
    pub fn add_example(&mut self, value: serde_json::Value, max_examples: usize) {
        if self.examples.len() < max_examples && !self.examples.contains(&value) {
            self.examples.push(value);
        }
    }

    /// Merge with another field
    pub fn merge_with(self, other: InferredField) -> InferredField {
        InferredField {
            field_type: self.field_type.merge_with(other.field_type),
            required: self.required && other.required,
            nullable: self.nullable || other.nullable,
            occurrences: self.occurrences + other.occurrences,
            examples: {
                let mut examples = self.examples;
                for ex in other.examples {
                    if !examples.contains(&ex) {
                        examples.push(ex);
                    }
                }
                examples
            },
            description: self.description.or(other.description),
        }
    }
}

/// Complete inferred schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferredSchema {
    /// Schema name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Schema description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Root type (usually Object)
    pub root: InferredType,
    /// Number of records analyzed
    pub record_count: usize,
    /// Partition key (if from staging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    /// Field statistics
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub field_stats: HashMap<String, FieldStats>,
}

/// Statistics for a field
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldStats {
    /// Number of occurrences
    pub occurrences: usize,
    /// Number of null values
    pub null_count: usize,
    /// Number of distinct values (if tracked)
    pub distinct_count: Option<usize>,
    /// Minimum value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Average value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg: Option<f64>,
}

impl InferredSchema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self {
            name: None,
            description: None,
            root: InferredType::Unknown,
            record_count: 0,
            partition: None,
            field_stats: HashMap::new(),
        }
    }

    /// Convert to JSON Schema format
    pub fn to_json_schema(&self) -> serde_json::Value {
        let mut schema = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema"
        });

        if let Some(ref name) = self.name {
            schema["title"] = serde_json::Value::String(name.clone());
        }
        if let Some(ref desc) = self.description {
            schema["description"] = serde_json::Value::String(desc.clone());
        }

        self.type_to_json_schema(&self.root, &mut schema);

        schema
    }

    fn type_to_json_schema(&self, inferred: &InferredType, schema: &mut serde_json::Value) {
        match inferred {
            InferredType::Null => {
                schema["type"] = serde_json::json!("null");
            }
            InferredType::Boolean => {
                schema["type"] = serde_json::json!("boolean");
            }
            InferredType::Integer => {
                schema["type"] = serde_json::json!("integer");
            }
            InferredType::Number => {
                schema["type"] = serde_json::json!("number");
            }
            InferredType::String { format } => {
                schema["type"] = serde_json::json!("string");
                if let Some(fmt) = format {
                    if let Some(fmt_str) = fmt.as_json_schema_format() {
                        schema["format"] = serde_json::json!(fmt_str);
                    }
                }
            }
            InferredType::Array { items } => {
                schema["type"] = serde_json::json!("array");
                let mut items_schema = serde_json::json!({});
                self.type_to_json_schema(items, &mut items_schema);
                schema["items"] = items_schema;
            }
            InferredType::Object { properties } => {
                schema["type"] = serde_json::json!("object");
                let mut props = serde_json::Map::new();
                let mut required = Vec::new();

                for (name, field) in properties {
                    let mut prop_schema = serde_json::json!({});
                    self.type_to_json_schema(&field.field_type, &mut prop_schema);

                    if let Some(ref desc) = field.description {
                        prop_schema["description"] = serde_json::json!(desc);
                    }
                    if !field.examples.is_empty() {
                        prop_schema["examples"] = serde_json::json!(field.examples);
                    }

                    props.insert(name.clone(), prop_schema);

                    if field.required && !field.nullable {
                        required.push(serde_json::Value::String(name.clone()));
                    }
                }

                schema["properties"] = serde_json::Value::Object(props);
                if !required.is_empty() {
                    schema["required"] = serde_json::Value::Array(required);
                }
            }
            InferredType::Mixed { types } => {
                let mut one_of = Vec::new();
                for t in types {
                    let mut sub_schema = serde_json::json!({});
                    self.type_to_json_schema(t, &mut sub_schema);
                    one_of.push(sub_schema);
                }
                schema["oneOf"] = serde_json::json!(one_of);
            }
            InferredType::Unknown => {
                // Empty schema accepts anything
            }
        }
    }
}

impl Default for InferredSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_merge_same() {
        let t1 = InferredType::Integer;
        let t2 = InferredType::Integer;
        assert_eq!(t1.merge_with(t2), InferredType::Integer);
    }

    #[test]
    fn test_type_merge_integer_number() {
        let t1 = InferredType::Integer;
        let t2 = InferredType::Number;
        assert_eq!(t1.merge_with(t2), InferredType::Number);
    }

    #[test]
    fn test_type_merge_null() {
        let t1 = InferredType::Null;
        let t2 = InferredType::String { format: None };
        assert_eq!(t1.merge_with(t2), InferredType::String { format: None });
    }

    #[test]
    fn test_type_merge_mixed() {
        let t1 = InferredType::Boolean;
        let t2 = InferredType::String { format: None };
        let merged = t1.merge_with(t2);
        match merged {
            InferredType::Mixed { types } => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Mixed type"),
        }
    }

    #[test]
    fn test_field_merge() {
        let f1 = InferredField::new(InferredType::Integer).with_required(true);
        let f2 = InferredField::new(InferredType::Integer).with_required(false);
        let merged = f1.merge_with(f2);
        assert!(!merged.required); // One was optional, so merged is optional
    }

    #[test]
    fn test_schema_to_json_schema() {
        let mut properties = BTreeMap::new();
        properties.insert(
            "name".to_string(),
            InferredField::new(InferredType::String { format: None }),
        );
        properties.insert("age".to_string(), InferredField::new(InferredType::Integer));

        let schema = InferredSchema {
            name: Some("Person".to_string()),
            description: None,
            root: InferredType::Object { properties },
            record_count: 10,
            partition: None,
            field_stats: HashMap::new(),
        };

        let json_schema = schema.to_json_schema();
        assert_eq!(json_schema["title"], "Person");
        assert_eq!(json_schema["type"], "object");
        assert!(json_schema["properties"]["name"].is_object());
        assert!(json_schema["properties"]["age"].is_object());
    }
}
