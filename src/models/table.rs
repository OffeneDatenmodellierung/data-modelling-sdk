//! Table model for the SDK

use super::column::Column;
use super::enums::{
    DataVaultClassification, DatabaseType, MedallionLayer, ModelingLevel, SCDPattern,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Position coordinates for table placement on canvas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
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
    /// Tags for categorization and filtering
    #[serde(default)]
    pub tags: Vec<String>,
    /// ODCL/ODCS metadata (legacy format support)
    #[serde(default)]
    pub odcl_metadata: HashMap<String, serde_json::Value>,
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
