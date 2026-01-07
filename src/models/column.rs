//! Column model for the SDK

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Foreign key reference to another table's column
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKey {
    /// Target table ID (UUID as string)
    pub table_id: String,
    /// Column name in the target table
    pub column_name: String,
}

/// ODCS v3.1.0 Relationship at property level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyRelationship {
    /// Relationship type (e.g., "foreignKey", "parent", "child")
    #[serde(rename = "type")]
    pub relationship_type: String,
    /// Target reference (e.g., "definitions/order_id", "schema/id/properties/id")
    pub to: String,
}

/// ODCS v3.1.0 logicalTypeOptions for additional type metadata
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

/// Authoritative definition reference (ODCS v3.1.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthoritativeDefinition {
    /// Type of the reference (e.g., "businessDefinition", "transformationImplementation")
    #[serde(rename = "type")]
    pub definition_type: String,
    /// URL to the authoritative definition
    pub url: String,
}

/// Column model representing a field in a table
///
/// A column defines a single field with a data type, constraints, and optional metadata.
/// This model supports all ODCS v3.1.0 property fields to ensure no data loss during import/export.
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::models::Column;
///
/// let column = Column::new("id".to_string(), "INT".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    // === Core Identity Fields ===
    /// Stable technical identifier (ODCS: id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Column name (ODCS: name)
    pub name: String,
    /// Business name for the column (ODCS: businessName)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_name: Option<String>,
    /// Column description/documentation (ODCS: description)
    #[serde(default)]
    pub description: String,

    // === Type Information ===
    /// Logical data type (ODCS: logicalType - e.g., "string", "integer", "number")
    #[serde(rename = "dataType")]
    pub data_type: String,
    /// Physical database type (ODCS: physicalType - e.g., "VARCHAR(100)", "BIGINT")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_type: Option<String>,
    /// Physical name in the data source (ODCS: physicalName)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_name: Option<String>,
    /// Additional type options (ODCS: logicalTypeOptions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_type_options: Option<LogicalTypeOptions>,

    // === Key Constraints ===
    /// Whether this column is part of the primary key (ODCS: primaryKey)
    #[serde(default)]
    pub primary_key: bool,
    /// Position in composite primary key, 1-based (ODCS: primaryKeyPosition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key_position: Option<i32>,
    /// Whether the column contains unique values (ODCS: unique)
    #[serde(default)]
    pub unique: bool,
    /// Whether the column allows NULL values (inverse of ODCS: required)
    #[serde(default = "default_true")]
    pub nullable: bool,

    // === Partitioning & Clustering ===
    /// Whether the column is used for partitioning (ODCS: partitioned)
    #[serde(default)]
    pub partitioned: bool,
    /// Position in partition key, 1-based (ODCS: partitionKeyPosition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_key_position: Option<i32>,
    /// Whether the column is used for clustering
    #[serde(default)]
    pub clustered: bool,

    // === Data Classification & Security ===
    /// Data classification level (ODCS: classification - e.g., "confidential", "public")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    /// Whether this is a critical data element (ODCS: criticalDataElement)
    #[serde(default)]
    pub critical_data_element: bool,
    /// Name of the encrypted version of this column (ODCS: encryptedName)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_name: Option<String>,

    // === Transformation Metadata ===
    /// Source objects used in transformation (ODCS: transformSourceObjects)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transform_source_objects: Vec<String>,
    /// Transformation logic/expression (ODCS: transformLogic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_logic: Option<String>,
    /// Human-readable transformation description (ODCS: transformDescription)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_description: Option<String>,

    // === Examples & Documentation ===
    /// Example values for this column (ODCS: examples)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<serde_json::Value>,
    /// Default value for the column
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,

    // === Relationships & References ===
    /// ODCS v3.1.0 relationships (property-level references)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<PropertyRelationship>,
    /// Authoritative definitions (ODCS: authoritativeDefinitions)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authoritative_definitions: Vec<AuthoritativeDefinition>,

    // === Quality & Validation ===
    /// Quality rules and checks (ODCS: quality)
    #[serde(default)]
    pub quality: Vec<HashMap<String, serde_json::Value>>,
    /// Enum values if this column is an enumeration type
    #[serde(default)]
    pub enum_values: Vec<String>,

    // === Tags & Custom Properties ===
    /// Property-level tags (ODCS: tags)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Custom properties for format-specific metadata not covered by ODCS
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_properties: HashMap<String, serde_json::Value>,

    // === Legacy/Internal Fields ===
    /// Whether this column is a secondary/business key
    #[serde(default)]
    pub secondary_key: bool,
    /// Composite key name if this column is part of a composite key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite_key: Option<String>,
    /// Foreign key reference (legacy - prefer relationships)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key: Option<ForeignKey>,
    /// Additional constraints (e.g., "CHECK", "UNIQUE")
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Validation errors and warnings
    #[serde(default)]
    pub errors: Vec<HashMap<String, serde_json::Value>>,
    /// Display order for UI rendering
    #[serde(default)]
    pub column_order: i32,
    /// Nested data type for ARRAY<STRUCT> or MAP types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nested_data: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Column {
    fn default() -> Self {
        Self {
            // Core Identity
            id: None,
            name: String::new(),
            business_name: None,
            description: String::new(),
            // Type Information
            data_type: String::new(),
            physical_type: None,
            physical_name: None,
            logical_type_options: None,
            // Key Constraints
            primary_key: false,
            primary_key_position: None,
            unique: false,
            nullable: true, // Default to nullable
            // Partitioning & Clustering
            partitioned: false,
            partition_key_position: None,
            clustered: false,
            // Data Classification & Security
            classification: None,
            critical_data_element: false,
            encrypted_name: None,
            // Transformation Metadata
            transform_source_objects: Vec::new(),
            transform_logic: None,
            transform_description: None,
            // Examples & Documentation
            examples: Vec::new(),
            default_value: None,
            // Relationships & References
            relationships: Vec::new(),
            authoritative_definitions: Vec::new(),
            // Quality & Validation
            quality: Vec::new(),
            enum_values: Vec::new(),
            // Tags & Custom Properties
            tags: Vec::new(),
            custom_properties: HashMap::new(),
            // Legacy/Internal Fields
            secondary_key: false,
            composite_key: None,
            foreign_key: None,
            constraints: Vec::new(),
            errors: Vec::new(),
            column_order: 0,
            nested_data: None,
        }
    }
}

impl Column {
    /// Create a new column with the given name and data type
    ///
    /// # Arguments
    ///
    /// * `name` - The column name (must be valid according to naming conventions)
    /// * `data_type` - The data type string (e.g., "INT", "VARCHAR(100)")
    ///
    /// # Returns
    ///
    /// A new `Column` instance with default values (nullable=true, primary_key=false).
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::models::Column;
    ///
    /// let col = Column::new("user_id".to_string(), "BIGINT".to_string());
    /// ```
    #[allow(deprecated)]
    pub fn new(name: String, data_type: String) -> Self {
        Self {
            // Core Identity
            id: None,
            name,
            business_name: None,
            description: String::new(),
            // Type Information
            data_type: normalize_data_type(&data_type),
            physical_type: None,
            physical_name: None,
            logical_type_options: None,
            // Key Constraints
            primary_key: false,
            primary_key_position: None,
            unique: false,
            nullable: true,
            // Partitioning & Clustering
            partitioned: false,
            partition_key_position: None,
            clustered: false,
            // Data Classification & Security
            classification: None,
            critical_data_element: false,
            encrypted_name: None,
            // Transformation Metadata
            transform_source_objects: Vec::new(),
            transform_logic: None,
            transform_description: None,
            // Examples & Documentation
            examples: Vec::new(),
            default_value: None,
            // Relationships & References
            relationships: Vec::new(),
            authoritative_definitions: Vec::new(),
            // Quality & Validation
            quality: Vec::new(),
            enum_values: Vec::new(),
            // Tags & Custom Properties
            tags: Vec::new(),
            custom_properties: HashMap::new(),
            // Legacy/Internal Fields
            secondary_key: false,
            composite_key: None,
            foreign_key: None,
            constraints: Vec::new(),
            errors: Vec::new(),
            column_order: 0,
            nested_data: None,
        }
    }
}

fn normalize_data_type(data_type: &str) -> String {
    if data_type.is_empty() {
        return data_type.to_string();
    }

    let upper = data_type.to_uppercase();

    // Handle STRUCT<...>, ARRAY<...>, MAP<...> preserving inner content
    if upper.starts_with("STRUCT") {
        if let Some(start) = data_type.find('<')
            && let Some(end) = data_type.rfind('>')
        {
            let inner = &data_type[start + 1..end];
            return format!("STRUCT<{}>", inner);
        }
        return format!("STRUCT{}", &data_type[6..]);
    } else if upper.starts_with("ARRAY") {
        if let Some(start) = data_type.find('<')
            && let Some(end) = data_type.rfind('>')
        {
            let inner = &data_type[start + 1..end];
            return format!("ARRAY<{}>", inner);
        }
        return format!("ARRAY{}", &data_type[5..]);
    } else if upper.starts_with("MAP") {
        if let Some(start) = data_type.find('<')
            && let Some(end) = data_type.rfind('>')
        {
            let inner = &data_type[start + 1..end];
            return format!("MAP<{}>", inner);
        }
        return format!("MAP{}", &data_type[3..]);
    }

    upper
}
