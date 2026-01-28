//! Property type for ODCS native data structures
//!
//! Represents a column/field in an ODCS schema object with full support
//! for nested properties (OBJECT and ARRAY types).

use super::supporting::{
    AuthoritativeDefinition, CustomProperty, LogicalTypeOptions, PropertyRelationship, QualityRule,
};
use serde::{Deserialize, Serialize};

/// Helper function to skip serializing false boolean values
fn is_false(b: &bool) -> bool {
    !*b
}

/// Property - one column in a schema object (ODCS v3.1.0)
///
/// Properties represent individual fields in a schema. They support nested
/// structures through the `properties` field (for OBJECT types) and the
/// `items` field (for ARRAY types).
///
/// # Example
///
/// ```rust
/// use data_modelling_core::models::odcs::{Property, LogicalTypeOptions};
///
/// // Simple property
/// let id_prop = Property::new("id", "integer")
///     .with_primary_key(true)
///     .with_required(true);
///
/// // Nested object property
/// let address_prop = Property::new("address", "object")
///     .with_nested_properties(vec![
///         Property::new("street", "string"),
///         Property::new("city", "string"),
///         Property::new("zip", "string"),
///     ]);
///
/// // Array property
/// let tags_prop = Property::new("tags", "array")
///     .with_items(Property::new("", "string"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    // === Core Identity Fields ===
    /// Stable technical identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Property name
    pub name: String,
    /// Business name for the property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_name: Option<String>,
    /// Property description/documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // === Type Information ===
    /// Logical data type (e.g., "string", "integer", "number", "boolean", "object", "array")
    pub logical_type: String,
    /// Physical database type (e.g., "VARCHAR(100)", "BIGINT")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_type: Option<String>,
    /// Physical name in the data source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_name: Option<String>,
    /// Additional type options (min/max length, pattern, precision, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_type_options: Option<LogicalTypeOptions>,

    // === Key Constraints ===
    /// Whether the property is required (inverse of nullable)
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    /// Whether this property is part of the primary key
    #[serde(default, skip_serializing_if = "is_false")]
    pub primary_key: bool,
    /// Position in composite primary key, 1-based
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key_position: Option<i32>,
    /// Whether the property contains unique values
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique: bool,

    // === Partitioning & Clustering ===
    /// Whether the property is used for partitioning
    #[serde(default, skip_serializing_if = "is_false")]
    pub partitioned: bool,
    /// Position in partition key, 1-based
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_key_position: Option<i32>,
    /// Whether the property is used for clustering
    #[serde(default, skip_serializing_if = "is_false")]
    pub clustered: bool,

    // === Data Classification & Security ===
    /// Data classification level (e.g., "confidential", "public")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    /// Whether this is a critical data element
    #[serde(default, skip_serializing_if = "is_false")]
    pub critical_data_element: bool,
    /// Name of the encrypted version of this property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_name: Option<String>,

    // === Transformation Metadata ===
    /// Source objects used in transformation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transform_source_objects: Vec<String>,
    /// Transformation logic/expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_logic: Option<String>,
    /// Human-readable transformation description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_description: Option<String>,

    // === Examples & Defaults ===
    /// Example values for this property
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<serde_json::Value>,
    /// Default value for the property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,

    // === Relationships & References ===
    /// Property-level relationships
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<PropertyRelationship>,
    /// Authoritative definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authoritative_definitions: Vec<AuthoritativeDefinition>,

    // === Quality & Validation ===
    /// Quality rules and checks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quality: Vec<QualityRule>,
    /// Enum values if this property is an enumeration type
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,

    // === Tags & Custom Properties ===
    /// Property-level tags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Custom properties for format-specific metadata
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_properties: Vec<CustomProperty>,

    // === Nested Properties (for OBJECT/ARRAY types) ===
    /// For ARRAY types: the item type definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Property>>,
    /// For OBJECT types: nested property definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
}

impl Property {
    /// Create a new property with the given name and logical type
    pub fn new(name: impl Into<String>, logical_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            logical_type: logical_type.into(),
            ..Default::default()
        }
    }

    /// Set the property as required
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set the property as a primary key
    pub fn with_primary_key(mut self, primary_key: bool) -> Self {
        self.primary_key = primary_key;
        self
    }

    /// Set the primary key position
    pub fn with_primary_key_position(mut self, position: i32) -> Self {
        self.primary_key_position = Some(position);
        self
    }

    /// Set the property description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the business name
    pub fn with_business_name(mut self, business_name: impl Into<String>) -> Self {
        self.business_name = Some(business_name.into());
        self
    }

    /// Set the physical type
    pub fn with_physical_type(mut self, physical_type: impl Into<String>) -> Self {
        self.physical_type = Some(physical_type.into());
        self
    }

    /// Set the physical name
    pub fn with_physical_name(mut self, physical_name: impl Into<String>) -> Self {
        self.physical_name = Some(physical_name.into());
        self
    }

    /// Set nested properties (for OBJECT types)
    pub fn with_nested_properties(mut self, properties: Vec<Property>) -> Self {
        self.properties = properties;
        self
    }

    /// Set items property (for ARRAY types)
    pub fn with_items(mut self, items: Property) -> Self {
        self.items = Some(Box::new(items));
        self
    }

    /// Set enum values
    pub fn with_enum_values(mut self, values: Vec<String>) -> Self {
        self.enum_values = values;
        self
    }

    /// Add a custom property
    pub fn with_custom_property(mut self, property: CustomProperty) -> Self {
        self.custom_properties.push(property);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set unique constraint
    pub fn with_unique(mut self, unique: bool) -> Self {
        self.unique = unique;
        self
    }

    /// Set classification
    pub fn with_classification(mut self, classification: impl Into<String>) -> Self {
        self.classification = Some(classification.into());
        self
    }

    /// Check if this property has nested structure (OBJECT or ARRAY type)
    pub fn has_nested_structure(&self) -> bool {
        !self.properties.is_empty() || self.items.is_some()
    }

    /// Check if this is an object type
    pub fn is_object(&self) -> bool {
        self.logical_type.to_lowercase() == "object"
            || self.logical_type.to_lowercase() == "struct"
            || !self.properties.is_empty()
    }

    /// Check if this is an array type
    pub fn is_array(&self) -> bool {
        self.logical_type.to_lowercase() == "array" || self.items.is_some()
    }

    /// Get all nested properties recursively, returning (path, property) pairs
    /// Path uses dot notation for nested objects and `[]` for arrays
    pub fn flatten_to_paths(&self) -> Vec<(String, &Property)> {
        let mut result = Vec::new();
        self.flatten_recursive(&self.name, &mut result);
        result
    }

    fn flatten_recursive<'a>(
        &'a self,
        current_path: &str,
        result: &mut Vec<(String, &'a Property)>,
    ) {
        // Add current property
        result.push((current_path.to_string(), self));

        // Recurse into nested object properties
        for nested in &self.properties {
            let nested_path = if current_path.is_empty() {
                nested.name.clone()
            } else {
                format!("{}.{}", current_path, nested.name)
            };
            nested.flatten_recursive(&nested_path, result);
        }

        // Recurse into array items
        if let Some(ref items) = self.items {
            let items_path = if current_path.is_empty() {
                "[]".to_string()
            } else {
                format!("{}.[]", current_path)
            };
            items.flatten_recursive(&items_path, result);
        }
    }

    /// Create a property tree from a list of flattened columns with dot-notation names
    ///
    /// This reconstructs the hierarchical structure from paths like:
    /// - "address.street" -> nested object
    /// - "tags.[]" -> array items
    /// - "items.[].name" -> array of objects
    pub fn from_flat_paths(paths: &[(String, Property)]) -> Vec<Property> {
        use std::collections::HashMap;

        // Group by top-level name
        let mut top_level: HashMap<String, Vec<(String, &Property)>> = HashMap::new();

        for (path, prop) in paths {
            let parts: Vec<&str> = path.split('.').collect();
            if parts.is_empty() {
                continue;
            }

            let top_name = parts[0].to_string();
            let remaining_path = if parts.len() > 1 {
                parts[1..].join(".")
            } else {
                String::new()
            };

            top_level
                .entry(top_name)
                .or_default()
                .push((remaining_path, prop));
        }

        // Build properties from grouped paths
        let mut result = Vec::new();
        for (name, children) in top_level {
            // Find the root property (empty remaining path)
            let root = children
                .iter()
                .find(|(path, _)| path.is_empty())
                .map(|(_, p)| (*p).clone());

            let mut prop = root.unwrap_or_else(|| Property::new(&name, "object"));
            prop.name = name;

            // Process nested paths
            let nested_paths: Vec<(String, Property)> = children
                .iter()
                .filter(|(path, _)| !path.is_empty())
                .map(|(path, p)| (path.clone(), (*p).clone()))
                .collect();

            if !nested_paths.is_empty() {
                // Check if it's an array type (has [] in path)
                let has_array_items = nested_paths.iter().any(|(p, _)| p.starts_with("[]"));

                if has_array_items {
                    // Build array items
                    let items_paths: Vec<(String, Property)> = nested_paths
                        .iter()
                        .filter(|(p, _)| p.starts_with("[]"))
                        .map(|(p, prop)| {
                            let remaining = if p == "[]" {
                                String::new()
                            } else {
                                p.strip_prefix("[].").unwrap_or("").to_string()
                            };
                            (remaining, prop.clone())
                        })
                        .collect();

                    if !items_paths.is_empty() {
                        // Find the array item type
                        let item_root = items_paths
                            .iter()
                            .find(|(p, _)| p.is_empty())
                            .map(|(_, p)| p.clone());

                        let mut items_prop =
                            item_root.unwrap_or_else(|| Property::new("", "object"));

                        // Recursively build nested items
                        let nested_item_paths: Vec<(String, Property)> = items_paths
                            .into_iter()
                            .filter(|(p, _)| !p.is_empty())
                            .collect();

                        if !nested_item_paths.is_empty() {
                            items_prop.properties = Property::from_flat_paths(&nested_item_paths);
                        }

                        prop.items = Some(Box::new(items_prop));
                    }
                }

                // Build object properties (non-array paths)
                let object_paths: Vec<(String, Property)> = nested_paths
                    .into_iter()
                    .filter(|(p, _)| !p.starts_with("[]"))
                    .collect();

                if !object_paths.is_empty() {
                    prop.properties = Property::from_flat_paths(&object_paths);
                }
            }

            result.push(prop);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_creation() {
        let prop = Property::new("id", "integer")
            .with_primary_key(true)
            .with_required(true)
            .with_description("Unique identifier");

        assert_eq!(prop.name, "id");
        assert_eq!(prop.logical_type, "integer");
        assert!(prop.primary_key);
        assert!(prop.required);
        assert_eq!(prop.description, Some("Unique identifier".to_string()));
    }

    #[test]
    fn test_nested_object_property() {
        let address = Property::new("address", "object").with_nested_properties(vec![
            Property::new("street", "string"),
            Property::new("city", "string"),
            Property::new("zip", "string"),
        ]);

        assert!(address.is_object());
        assert!(!address.is_array());
        assert!(address.has_nested_structure());
        assert_eq!(address.properties.len(), 3);
    }

    #[test]
    fn test_array_property() {
        let tags = Property::new("tags", "array").with_items(Property::new("", "string"));

        assert!(tags.is_array());
        assert!(!tags.is_object());
        assert!(tags.has_nested_structure());
        assert!(tags.items.is_some());
    }

    #[test]
    fn test_flatten_to_paths() {
        let address = Property::new("address", "object").with_nested_properties(vec![
            Property::new("street", "string"),
            Property::new("city", "string"),
        ]);

        let paths = address.flatten_to_paths();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].0, "address");
        assert!(paths.iter().any(|(p, _)| p == "address.street"));
        assert!(paths.iter().any(|(p, _)| p == "address.city"));
    }

    #[test]
    fn test_flatten_array_to_paths() {
        let items = Property::new("items", "array").with_items(
            Property::new("", "object").with_nested_properties(vec![
                Property::new("name", "string"),
                Property::new("quantity", "integer"),
            ]),
        );

        let paths = items.flatten_to_paths();
        assert!(paths.iter().any(|(p, _)| p == "items"));
        assert!(paths.iter().any(|(p, _)| p == "items.[]"));
        assert!(paths.iter().any(|(p, _)| p == "items.[].name"));
        assert!(paths.iter().any(|(p, _)| p == "items.[].quantity"));
    }

    #[test]
    fn test_serialization() {
        let prop = Property::new("name", "string")
            .with_required(true)
            .with_description("User name");

        let json = serde_json::to_string_pretty(&prop).unwrap();
        assert!(json.contains("\"name\": \"name\""));
        assert!(json.contains("\"logicalType\": \"string\""));
        assert!(json.contains("\"required\": true"));

        // Verify camelCase
        assert!(json.contains("logicalType"));
        assert!(!json.contains("logical_type"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "name": "email",
            "logicalType": "string",
            "required": true,
            "logicalTypeOptions": {
                "format": "email",
                "maxLength": 255
            }
        }"#;

        let prop: Property = serde_json::from_str(json).unwrap();
        assert_eq!(prop.name, "email");
        assert_eq!(prop.logical_type, "string");
        assert!(prop.required);
        assert!(prop.logical_type_options.is_some());
        let opts = prop.logical_type_options.unwrap();
        assert_eq!(opts.format, Some("email".to_string()));
        assert_eq!(opts.max_length, Some(255));
    }
}
