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

/// Column model representing a field in a table
///
/// A column defines a single field with a data type, constraints, and optional metadata.
/// Columns can be primary keys, foreign keys, nullable, and have various constraints.
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::models::Column;
///
/// let column = Column::new("id".to_string(), "INT".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Column {
    /// Column name
    pub name: String,
    /// Data type (e.g., "INT", "VARCHAR(100)", "TIMESTAMP")
    pub data_type: String,
    /// Whether the column allows NULL values (default: true)
    #[serde(default = "default_true")]
    pub nullable: bool,
    /// Whether this column is part of the primary key (default: false)
    #[serde(default)]
    pub primary_key: bool,
    /// Whether this column is a secondary key (default: false)
    #[serde(default)]
    pub secondary_key: bool,
    /// Composite key name if this column is part of a composite key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite_key: Option<String>,
    /// Foreign key reference if this column references another table
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key: Option<ForeignKey>,
    /// Additional constraints (e.g., "CHECK", "UNIQUE")
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Column description/documentation
    #[serde(default)]
    pub description: String,
    /// Validation errors and warnings
    #[serde(default)]
    pub errors: Vec<HashMap<String, serde_json::Value>>,
    /// Quality rules and checks
    #[serde(default)]
    pub quality: Vec<HashMap<String, serde_json::Value>>,
    /// JSON Schema $ref reference path
    #[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]
    pub ref_path: Option<String>,
    /// Enum values if this column is an enumeration type
    #[serde(default)]
    pub enum_values: Vec<String>,
    /// Display order for UI rendering
    #[serde(default)]
    pub column_order: i32,
}

fn default_true() -> bool {
    true
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
    pub fn new(name: String, data_type: String) -> Self {
        Self {
            name,
            data_type: normalize_data_type(&data_type),
            nullable: true,
            primary_key: false,
            secondary_key: false,
            composite_key: None,
            foreign_key: None,
            constraints: Vec::new(),
            description: String::new(),
            errors: Vec::new(),
            quality: Vec::new(),
            ref_path: None,
            enum_values: Vec::new(),
            column_order: 0,
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
