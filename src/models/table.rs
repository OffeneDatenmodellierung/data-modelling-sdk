//! Table model for the SDK

use super::column::Column;
use super::enums::{
    DataVaultClassification, DatabaseType, InfrastructureType, MedallionLayer, ModelingLevel,
    SCDPattern,
};
use super::tag::Tag;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json;
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/// Deserialize tags with backward compatibility (supports Vec<String> and Vec<Tag>)
fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<Tag>, D::Error>
where
    D: Deserializer<'de>,
{
    // Accept either Vec<String> (backward compatibility) or Vec<Tag>
    struct TagVisitor;

    impl<'de> serde::de::Visitor<'de> for TagVisitor {
        type Value = Vec<Tag>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a vector of tags (strings or Tag objects)")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut tags = Vec::new();
            while let Some(item) = seq.next_element::<serde_json::Value>()? {
                match item {
                    serde_json::Value::String(s) => {
                        // Backward compatibility: parse string as Tag
                        if let Ok(tag) = Tag::from_str(&s) {
                            tags.push(tag);
                        }
                    }
                    _ => {
                        // Try to deserialize as Tag directly (if it's a string in JSON)
                        if let serde_json::Value::String(s) = item
                            && let Ok(tag) = Tag::from_str(&s)
                        {
                            tags.push(tag);
                        }
                    }
                }
            }
            Ok(tags)
        }
    }

    deserializer.deserialize_seq(TagVisitor)
}

/// Position coordinates for table placement on canvas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

/// SLA (Service Level Agreement) property following ODCS-inspired structure
///
/// Represents a single SLA property for Data Flow nodes and relationships.
/// Uses a lightweight format inspired by ODCS servicelevels but separate from ODCS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlaProperty {
    /// SLA attribute name (e.g., "latency", "availability", "throughput")
    pub property: String,
    /// Metric value (flexible type to support numbers, strings, etc.)
    pub value: serde_json::Value,
    /// Measurement unit (e.g., "hours", "percent", "requests_per_second")
    pub unit: String,
    /// Optional: Data elements this SLA applies to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    /// Optional: Importance driver (e.g., "regulatory", "analytics", "operational")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    /// Optional: Description of the SLA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional: Scheduler type for monitoring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
    /// Optional: Schedule expression (e.g., cron format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
}

/// Contact details for Data Flow node/relationship owners/responsible parties
///
/// Structured contact information for operational and governance purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactDetails {
    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Phone number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Contact name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Role or title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Other contact methods or additional information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
}

/// Table model representing a database table or data contract
///
/// A table represents a structured data entity with columns, metadata, and relationships.
/// Tables can be imported from various formats (SQL, ODCS, JSON Schema, etc.) and exported
/// to multiple formats.
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::models::{Table, Column};
///
/// let table = Table::new(
///     "users".to_string(),
///     vec![
///         Column::new("id".to_string(), "INT".to_string()),
///         Column::new("name".to_string(), "VARCHAR(100)".to_string()),
///     ],
/// );
/// ```
///
/// # Example with Metadata (Data Flow Node)
///
/// ```rust
/// use data_modelling_sdk::models::{Table, Column, InfrastructureType, ContactDetails, SlaProperty};
/// use serde_json::json;
///
/// let mut table = Table::new(
///     "user_events".to_string(),
///     vec![Column::new("id".to_string(), "UUID".to_string())],
/// );
/// table.owner = Some("Data Engineering Team".to_string());
/// table.infrastructure_type = Some(InfrastructureType::Kafka);
/// table.contact_details = Some(ContactDetails {
///     email: Some("team@example.com".to_string()),
///     phone: None,
///     name: Some("Data Team".to_string()),
///     role: Some("Data Owner".to_string()),
///     other: None,
/// });
/// table.sla = Some(vec![SlaProperty {
///     property: "latency".to_string(),
///     value: json!(4),
///     unit: "hours".to_string(),
///     description: Some("Data must be available within 4 hours".to_string()),
///     element: None,
///     driver: Some("operational".to_string()),
///     scheduler: None,
///     schedule: None,
/// }]);
/// table.notes = Some("User interaction events from web application".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table {
    /// Unique identifier for the table (UUIDv4)
    pub id: Uuid,
    /// Table name (must be unique within database_type/catalog/schema scope)
    pub name: String,
    /// List of columns in the table
    pub columns: Vec<Column>,
    /// Database type (PostgreSQL, MySQL, etc.) if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_type: Option<DatabaseType>,
    /// Catalog name (database name in some systems)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    /// Schema name (namespace within catalog)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    /// Medallion architecture layers (Bronze, Silver, Gold)
    #[serde(default)]
    pub medallion_layers: Vec<MedallionLayer>,
    /// Slowly Changing Dimension pattern (Type 1, Type 2, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scd_pattern: Option<SCDPattern>,
    /// Data Vault classification (Hub, Link, Satellite)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_vault_classification: Option<DataVaultClassification>,
    /// Modeling level (Conceptual, Logical, Physical)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modeling_level: Option<ModelingLevel>,
    /// Tags for categorization and filtering (supports Simple, Pair, and List formats)
    #[serde(default, deserialize_with = "deserialize_tags")]
    pub tags: Vec<Tag>,
    /// ODCL/ODCS metadata (legacy format support)
    #[serde(default)]
    pub odcl_metadata: HashMap<String, serde_json::Value>,
    /// Owner information (person, team, or organization name) for Data Flow nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// SLA (Service Level Agreement) information (ODCS-inspired but lightweight format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<Vec<SlaProperty>>,
    /// Contact details for responsible parties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_details: Option<ContactDetails>,
    /// Infrastructure type (hosting platform, service, or tool) for Data Flow nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_type: Option<InfrastructureType>,
    /// Additional notes and context for Data Flow nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Canvas position for visual representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    /// Path to YAML file if loaded from file system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yaml_file_path: Option<String>,
    /// Draw.io cell ID for diagram integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawio_cell_id: Option<String>,
    /// Quality rules and checks
    #[serde(default)]
    pub quality: Vec<HashMap<String, serde_json::Value>>,
    /// Validation errors and warnings
    #[serde(default)]
    pub errors: Vec<HashMap<String, serde_json::Value>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Table {
    /// Create a new table with the given name and columns
    ///
    /// # Arguments
    ///
    /// * `name` - The table name (must be valid according to naming conventions)
    /// * `columns` - Vector of columns for the table
    ///
    /// # Returns
    ///
    /// A new `Table` instance with a generated UUIDv4 ID and current timestamps.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let table = Table::new(
    ///     "users".to_string(),
    ///     vec![Column::new("id".to_string(), "INT".to_string())],
    /// );
    /// ```
    pub fn new(name: String, columns: Vec<Column>) -> Self {
        let now = Utc::now();
        // UUIDv4 everywhere (do not derive ids from natural keys like name).
        let id = Self::generate_id(&name, None, None, None);
        Self {
            id,
            name,
            columns,
            database_type: None,
            catalog_name: None,
            schema_name: None,
            medallion_layers: Vec::new(),
            scd_pattern: None,
            data_vault_classification: None,
            modeling_level: None,
            tags: Vec::new(),
            odcl_metadata: HashMap::new(),
            owner: None,
            sla: None,
            contact_details: None,
            infrastructure_type: None,
            notes: None,
            position: None,
            yaml_file_path: None,
            drawio_cell_id: None,
            quality: Vec::new(),
            errors: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the unique key tuple for this table
    ///
    /// Returns a tuple of (database_type, name, catalog_name, schema_name) that uniquely
    /// identifies this table within its scope. Used for detecting naming conflicts.
    ///
    /// # Returns
    ///
    /// A tuple containing the database type (as string), name, catalog name, and schema name.
    pub fn get_unique_key(&self) -> (Option<String>, String, Option<String>, Option<String>) {
        (
            self.database_type.as_ref().map(|dt| format!("{:?}", dt)),
            self.name.clone(),
            self.catalog_name.clone(),
            self.schema_name.clone(),
        )
    }

    /// Generate a UUIDv4 for a new table id.
    ///
    /// Note: params are retained for backward-compatibility with previous deterministic-v5 API.
    pub fn generate_id(
        _name: &str,
        _database_type: Option<&DatabaseType>,
        _catalog_name: Option<&str>,
        _schema_name: Option<&str>,
    ) -> Uuid {
        Uuid::new_v4()
    }
}
