//! SchemaObject type for ODCS native data structures
//!
//! Represents a table/view/topic in an ODCS contract with full support
//! for all schema-level metadata fields.

use super::property::Property;
use super::supporting::{AuthoritativeDefinition, CustomProperty, QualityRule, SchemaRelationship};
use serde::{Deserialize, Serialize};

/// SchemaObject - one table/view/topic in a contract (ODCS v3.1.0)
///
/// Schema objects represent individual data structures within a contract.
/// Each schema object contains properties (columns) and can have its own
/// metadata like quality rules, relationships, and authoritative definitions.
///
/// # Example
///
/// ```rust
/// use data_modelling_core::models::odcs::{SchemaObject, Property};
///
/// let users_table = SchemaObject::new("users")
///     .with_physical_name("tbl_users")
///     .with_physical_type("table")
///     .with_business_name("User Accounts")
///     .with_description("Contains registered user information")
///     .with_properties(vec![
///         Property::new("id", "integer").with_primary_key(true),
///         Property::new("email", "string").with_required(true),
///         Property::new("name", "string"),
///     ]);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SchemaObject {
    // === Core Identity Fields ===
    /// Stable technical identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Schema object name (table/view name)
    pub name: String,
    /// Physical name in the data source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_name: Option<String>,
    /// Physical type ("table", "view", "topic", "file", "object", "stream")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_type: Option<String>,
    /// Business name for the schema object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_name: Option<String>,
    /// Schema object description/documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // === Granularity ===
    /// Description of the data granularity (e.g., "One row per customer per day")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_granularity_description: Option<String>,

    // === Properties (Columns) ===
    /// List of properties/columns in this schema object
    #[serde(default)]
    pub properties: Vec<Property>,

    // === Relationships ===
    /// Schema-level relationships to other schema objects
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<SchemaRelationship>,

    // === Quality & Validation ===
    /// Quality rules and checks at schema level
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quality: Vec<QualityRule>,

    // === References ===
    /// Authoritative definitions for this schema object
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authoritative_definitions: Vec<AuthoritativeDefinition>,

    // === Tags & Custom Properties ===
    /// Schema-level tags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Custom properties for format-specific metadata
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_properties: Vec<CustomProperty>,
}

impl SchemaObject {
    /// Create a new schema object with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the physical name
    pub fn with_physical_name(mut self, physical_name: impl Into<String>) -> Self {
        self.physical_name = Some(physical_name.into());
        self
    }

    /// Set the physical type
    pub fn with_physical_type(mut self, physical_type: impl Into<String>) -> Self {
        self.physical_type = Some(physical_type.into());
        self
    }

    /// Set the business name
    pub fn with_business_name(mut self, business_name: impl Into<String>) -> Self {
        self.business_name = Some(business_name.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the data granularity description
    pub fn with_data_granularity_description(mut self, description: impl Into<String>) -> Self {
        self.data_granularity_description = Some(description.into());
        self
    }

    /// Set the properties (columns)
    pub fn with_properties(mut self, properties: Vec<Property>) -> Self {
        self.properties = properties;
        self
    }

    /// Add a property
    pub fn with_property(mut self, property: Property) -> Self {
        self.properties.push(property);
        self
    }

    /// Set the relationships
    pub fn with_relationships(mut self, relationships: Vec<SchemaRelationship>) -> Self {
        self.relationships = relationships;
        self
    }

    /// Add a relationship
    pub fn with_relationship(mut self, relationship: SchemaRelationship) -> Self {
        self.relationships.push(relationship);
        self
    }

    /// Set the quality rules
    pub fn with_quality(mut self, quality: Vec<QualityRule>) -> Self {
        self.quality = quality;
        self
    }

    /// Add a quality rule
    pub fn with_quality_rule(mut self, rule: QualityRule) -> Self {
        self.quality.push(rule);
        self
    }

    /// Set the authoritative definitions
    pub fn with_authoritative_definitions(
        mut self,
        definitions: Vec<AuthoritativeDefinition>,
    ) -> Self {
        self.authoritative_definitions = definitions;
        self
    }

    /// Add an authoritative definition
    pub fn with_authoritative_definition(mut self, definition: AuthoritativeDefinition) -> Self {
        self.authoritative_definitions.push(definition);
        self
    }

    /// Set the tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the custom properties
    pub fn with_custom_properties(mut self, custom_properties: Vec<CustomProperty>) -> Self {
        self.custom_properties = custom_properties;
        self
    }

    /// Add a custom property
    pub fn with_custom_property(mut self, custom_property: CustomProperty) -> Self {
        self.custom_properties.push(custom_property);
        self
    }

    /// Set the ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Get the primary key properties
    pub fn primary_key_properties(&self) -> Vec<&Property> {
        let mut pk_props: Vec<&Property> =
            self.properties.iter().filter(|p| p.primary_key).collect();
        pk_props.sort_by_key(|p| p.primary_key_position.unwrap_or(i32::MAX));
        pk_props
    }

    /// Get the required properties
    pub fn required_properties(&self) -> Vec<&Property> {
        self.properties.iter().filter(|p| p.required).collect()
    }

    /// Get a property by name
    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// Get a mutable property by name
    pub fn get_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.properties.iter_mut().find(|p| p.name == name)
    }

    /// Count of properties
    pub fn property_count(&self) -> usize {
        self.properties.len()
    }

    /// Check if this schema has any nested/complex properties
    pub fn has_nested_properties(&self) -> bool {
        self.properties.iter().any(|p| p.has_nested_structure())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_object_creation() {
        let schema = SchemaObject::new("users")
            .with_physical_name("tbl_users")
            .with_physical_type("table")
            .with_business_name("User Accounts")
            .with_description("Contains user data");

        assert_eq!(schema.name, "users");
        assert_eq!(schema.physical_name, Some("tbl_users".to_string()));
        assert_eq!(schema.physical_type, Some("table".to_string()));
        assert_eq!(schema.business_name, Some("User Accounts".to_string()));
        assert_eq!(schema.description, Some("Contains user data".to_string()));
    }

    #[test]
    fn test_schema_with_properties() {
        let schema = SchemaObject::new("orders").with_properties(vec![
            Property::new("id", "integer")
                .with_primary_key(true)
                .with_primary_key_position(1),
            Property::new("customer_id", "integer").with_required(true),
            Property::new("total", "number"),
        ]);

        assert_eq!(schema.property_count(), 3);

        let pk_props = schema.primary_key_properties();
        assert_eq!(pk_props.len(), 1);
        assert_eq!(pk_props[0].name, "id");

        let required_props = schema.required_properties();
        assert_eq!(required_props.len(), 1);
        assert_eq!(required_props[0].name, "customer_id");
    }

    #[test]
    fn test_get_property() {
        let schema = SchemaObject::new("products")
            .with_property(Property::new("id", "integer"))
            .with_property(Property::new("name", "string"));

        let id_prop = schema.get_property("id");
        assert!(id_prop.is_some());
        assert_eq!(id_prop.unwrap().name, "id");

        let missing = schema.get_property("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_serialization() {
        let schema = SchemaObject::new("events")
            .with_physical_type("topic")
            .with_properties(vec![
                Property::new("event_id", "string").with_primary_key(true),
                Property::new("timestamp", "timestamp"),
            ]);

        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("\"name\": \"events\""));
        assert!(json.contains("\"physicalType\": \"topic\""));
        assert!(json.contains("\"properties\""));

        // Verify camelCase
        assert!(json.contains("physicalType"));
        assert!(!json.contains("physical_type"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "name": "customers",
            "physicalName": "customer_table",
            "physicalType": "table",
            "businessName": "Customer Records",
            "description": "All customer information",
            "dataGranularityDescription": "One row per customer",
            "properties": [
                {
                    "name": "id",
                    "logicalType": "integer",
                    "primaryKey": true
                },
                {
                    "name": "email",
                    "logicalType": "string",
                    "required": true
                }
            ],
            "tags": ["pii", "customer-data"]
        }"#;

        let schema: SchemaObject = serde_json::from_str(json).unwrap();
        assert_eq!(schema.name, "customers");
        assert_eq!(schema.physical_name, Some("customer_table".to_string()));
        assert_eq!(schema.physical_type, Some("table".to_string()));
        assert_eq!(schema.business_name, Some("Customer Records".to_string()));
        assert_eq!(
            schema.data_granularity_description,
            Some("One row per customer".to_string())
        );
        assert_eq!(schema.properties.len(), 2);
        assert_eq!(schema.tags, vec!["pii", "customer-data"]);
    }

    #[test]
    fn test_has_nested_properties() {
        let simple_schema = SchemaObject::new("simple")
            .with_property(Property::new("id", "integer"))
            .with_property(Property::new("name", "string"));

        assert!(!simple_schema.has_nested_properties());

        let nested_schema = SchemaObject::new("nested")
            .with_property(Property::new("id", "integer"))
            .with_property(
                Property::new("address", "object")
                    .with_nested_properties(vec![Property::new("city", "string")]),
            );

        assert!(nested_schema.has_nested_properties());
    }
}
