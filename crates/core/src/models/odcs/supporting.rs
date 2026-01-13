//! Supporting types for ODCS native data structures
//!
//! These types are used across ODCSContract, SchemaObject, and Property
//! to represent shared concepts like quality rules, custom properties, and relationships.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Quality rule for data validation (ODCS v3.1.0)
///
/// Quality rules can be defined at contract, schema, or property level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct QualityRule {
    /// Type of quality rule (e.g., "sql", "custom", "library")
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub rule_type: Option<String>,
    /// Quality dimension (e.g., "accuracy", "completeness", "timeliness")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    /// Business impact description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_impact: Option<String>,
    /// Metric name for the rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    /// Description of the quality rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Condition that must be true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_be: Option<serde_json::Value>,
    /// Condition that must be false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_not_be: Option<serde_json::Value>,
    /// Greater than condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_be_greater_than: Option<serde_json::Value>,
    /// Less than condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_be_less_than: Option<serde_json::Value>,
    /// Greater than or equal condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_be_greater_than_or_equal: Option<serde_json::Value>,
    /// Less than or equal condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_be_less_than_or_equal: Option<serde_json::Value>,
    /// Value must be in this set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_be_in: Option<Vec<serde_json::Value>>,
    /// Value must not be in this set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_not_be_in: Option<Vec<serde_json::Value>>,
    /// SQL query for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Scheduler type for quality checks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
    /// Schedule expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    /// Engine for running the quality check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// URL to quality tool or dashboard
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Additional properties not explicitly modeled
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Custom property for format-specific metadata (ODCS v3.1.0)
///
/// Used to store metadata that doesn't fit into the standard ODCS fields,
/// such as Avro-specific or Protobuf-specific attributes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomProperty {
    /// Property name
    pub property: String,
    /// Property value (flexible type)
    pub value: serde_json::Value,
}

impl CustomProperty {
    /// Create a new custom property
    pub fn new(property: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            property: property.into(),
            value,
        }
    }

    /// Create a string custom property
    pub fn string(property: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            property: property.into(),
            value: serde_json::Value::String(value.into()),
        }
    }
}

/// Authoritative definition reference (ODCS v3.1.0)
///
/// Links to external authoritative sources for definitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoritativeDefinition {
    /// Type of the reference (e.g., "businessDefinition", "transformationImplementation")
    #[serde(rename = "type")]
    pub definition_type: String,
    /// URL to the authoritative definition
    pub url: String,
}

impl AuthoritativeDefinition {
    /// Create a new authoritative definition
    pub fn new(definition_type: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            definition_type: definition_type.into(),
            url: url.into(),
        }
    }
}

/// Schema-level relationship (ODCS v3.1.0)
///
/// Represents relationships between schema objects (tables).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaRelationship {
    /// Relationship type (e.g., "foreignKey", "parent", "child")
    #[serde(rename = "type")]
    pub relationship_type: String,
    /// Source properties (column names) in this schema
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from_properties: Vec<String>,
    /// Target schema object name
    pub to_schema: String,
    /// Target properties (column names) in the target schema
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub to_properties: Vec<String>,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Property-level relationship (ODCS v3.1.0)
///
/// Represents relationships from a property to other definitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyRelationship {
    /// Relationship type (e.g., "foreignKey", "parent", "child")
    #[serde(rename = "type")]
    pub relationship_type: String,
    /// Target reference (e.g., "definitions/order_id", "schema/id/properties/id")
    pub to: String,
}

impl PropertyRelationship {
    /// Create a new property relationship
    pub fn new(relationship_type: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            relationship_type: relationship_type.into(),
            to: to.into(),
        }
    }
}

/// Logical type options for additional type metadata (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogicalTypeOptions {
    /// Minimum length for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    /// Maximum length for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
    /// Regex pattern for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Format hint (e.g., "email", "uuid", "uri")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Minimum value for numbers/dates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<serde_json::Value>,
    /// Maximum value for numbers/dates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<serde_json::Value>,
    /// Exclusive minimum for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<serde_json::Value>,
    /// Exclusive maximum for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<serde_json::Value>,
    /// Precision for decimals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<i32>,
    /// Scale for decimals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<i32>,
}

impl LogicalTypeOptions {
    /// Check if all options are empty/None
    pub fn is_empty(&self) -> bool {
        self.min_length.is_none()
            && self.max_length.is_none()
            && self.pattern.is_none()
            && self.format.is_none()
            && self.minimum.is_none()
            && self.maximum.is_none()
            && self.exclusive_minimum.is_none()
            && self.exclusive_maximum.is_none()
            && self.precision.is_none()
            && self.scale.is_none()
    }
}

/// Team information (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    /// Team name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Team members
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<TeamMember>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Team member information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    /// Member name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Member email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Member role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Support information (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Support {
    /// Support channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Support URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Support email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Server configuration (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    /// Server name/identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// Server type (e.g., "BigQuery", "Snowflake", "S3")
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    /// Server environment (e.g., "production", "development")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    /// Server description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Database name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    /// Project name (for cloud platforms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Schema name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Catalog name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog: Option<String>,
    /// Dataset name (for BigQuery)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    /// Account name (for Snowflake)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Host URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Location/Region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Format for file-based servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Delimiter for CSV files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Topic name for streaming
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Role definition (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    /// Role name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Role description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Principal (user/group)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    /// Access level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Service level definition (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceLevel {
    /// Service level property name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    /// Value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// Unit of measurement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Element this applies to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    /// Driver for this SLA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Scheduler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
    /// Schedule expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Price information (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    /// Price amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<serde_json::Value>,
    /// Currency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Billing frequency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_frequency: Option<String>,
    /// Price model type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_model: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Terms and conditions (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Terms {
    /// Terms description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Usage limitations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limitations: Option<String>,
    /// URL to full terms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Link to external resource (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// Link type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
    /// URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Description that can be string or structured object (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Description {
    /// Simple string description
    Simple(String),
    /// Structured description object
    Structured(StructuredDescription),
}

impl Default for Description {
    fn default() -> Self {
        Description::Simple(String::new())
    }
}

impl Description {
    /// Get the description as a simple string
    pub fn as_string(&self) -> String {
        match self {
            Description::Simple(s) => s.clone(),
            Description::Structured(d) => d.purpose.clone().unwrap_or_default(),
        }
    }
}

/// Structured description with multiple fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StructuredDescription {
    /// Purpose of the data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Limitations of the data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limitations: Option<String>,
    /// Usage guidelines
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    /// Additional properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_rule_serialization() {
        let rule = QualityRule {
            dimension: Some("accuracy".to_string()),
            must_be: Some(serde_json::json!(true)),
            ..Default::default()
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("dimension"));
        assert!(json.contains("accuracy"));
    }

    #[test]
    fn test_custom_property() {
        let prop = CustomProperty::string("source_format", "avro");
        assert_eq!(prop.property, "source_format");
        assert_eq!(prop.value, serde_json::json!("avro"));
    }

    #[test]
    fn test_description_variants() {
        let simple: Description = serde_json::from_str(r#""A simple description""#).unwrap();
        assert_eq!(simple.as_string(), "A simple description");

        let structured: Description =
            serde_json::from_str(r#"{"purpose": "Data analysis", "usage": "Read-only"}"#).unwrap();
        assert_eq!(structured.as_string(), "Data analysis");
    }

    #[test]
    fn test_logical_type_options_is_empty() {
        let empty = LogicalTypeOptions::default();
        assert!(empty.is_empty());

        let with_length = LogicalTypeOptions {
            max_length: Some(100),
            ..Default::default()
        };
        assert!(!with_length.is_empty());
    }
}
