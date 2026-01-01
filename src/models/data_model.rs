//! DataModel for the SDK

use super::relationship::Relationship;
use super::table::Table;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Data model representing a complete data model with tables and relationships
///
/// A `DataModel` is a container for a collection of tables and their relationships.
/// It represents a workspace or domain within a larger data modeling system.
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::models::DataModel;
///
/// let model = DataModel::new(
///     "MyModel".to_string(),
///     "/path/to/git".to_string(),
///     "control.yaml".to_string(),
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataModel {
    /// Unique identifier for the model (UUIDv5 based on name and path)
    pub id: Uuid,
    /// Model name
    pub name: String,
    /// Optional description of the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Path to the Git repository directory
    pub git_directory_path: String,
    /// Tables in this model
    #[serde(default)]
    pub tables: Vec<Table>,
    /// Relationships between tables
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    /// Path to the control file (relationships.yaml)
    pub control_file_path: String,
    /// Path to diagram file if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_file_path: Option<String>,
    /// Whether this model is in a subfolder
    #[serde(default)]
    pub is_subfolder: bool,
    /// Parent Git directory if this is a subfolder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_git_directory: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl DataModel {
    /// Create a new data model with the given name and paths
    ///
    /// # Arguments
    ///
    /// * `name` - The model name
    /// * `git_directory_path` - Path to the Git repository directory
    /// * `control_file_path` - Path to the control file (typically "relationships.yaml")
    ///
    /// # Returns
    ///
    /// A new `DataModel` instance with a UUIDv5 ID (deterministic based on name and path)
    /// and current timestamps.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::models::DataModel;
    ///
    /// let model = DataModel::new(
    ///     "MyModel".to_string(),
    ///     "/workspace/models".to_string(),
    ///     "relationships.yaml".to_string(),
    /// );
    /// ```
    pub fn new(name: String, git_directory_path: String, control_file_path: String) -> Self {
        let now = Utc::now();
        // Use deterministic UUID v5 based on model name and git path
        // This avoids requiring random number generation (getrandom/wasm_js)
        let key = format!("{}:{}", git_directory_path, name);
        let id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, key.as_bytes());
        Self {
            id,
            name,
            description: None,
            git_directory_path,
            tables: Vec::new(),
            relationships: Vec::new(),
            control_file_path,
            diagram_file_path: None,
            is_subfolder: false,
            parent_git_directory: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get a table by its ID
    ///
    /// # Arguments
    ///
    /// * `table_id` - The UUID of the table to find
    ///
    /// # Returns
    ///
    /// A reference to the table if found, `None` otherwise.
    pub fn get_table_by_id(&self, table_id: Uuid) -> Option<&Table> {
        self.tables.iter().find(|t| t.id == table_id)
    }

    /// Get a mutable reference to a table by its ID
    ///
    /// # Arguments
    ///
    /// * `table_id` - The UUID of the table to find
    ///
    /// # Returns
    ///
    /// A mutable reference to the table if found, `None` otherwise.
    pub fn get_table_by_id_mut(&mut self, table_id: Uuid) -> Option<&mut Table> {
        self.tables.iter_mut().find(|t| t.id == table_id)
    }

    /// Get a table by its name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the table to find
    ///
    /// # Returns
    ///
    /// A reference to the first table with the given name if found, `None` otherwise.
    ///
    /// # Note
    ///
    /// If multiple tables have the same name (different database_type/catalog/schema),
    /// use `get_table_by_unique_key` instead.
    pub fn get_table_by_name(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }

    /// Get a table by its unique key (database_type, name, catalog, schema)
    ///
    /// # Arguments
    ///
    /// * `database_type` - Optional database type
    /// * `name` - Table name
    /// * `catalog_name` - Optional catalog name
    /// * `schema_name` - Optional schema name
    ///
    /// # Returns
    ///
    /// A reference to the table if found, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_sdk::models::DataModel;
    /// # let model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// // Find table in specific schema
    /// let table = model.get_table_by_unique_key(
    ///     Some("PostgreSQL"),
    ///     "users",
    ///     Some("mydb"),
    ///     Some("public"),
    /// );
    /// ```
    pub fn get_table_by_unique_key(
        &self,
        database_type: Option<&str>,
        name: &str,
        catalog_name: Option<&str>,
        schema_name: Option<&str>,
    ) -> Option<&Table> {
        let target_key = (
            database_type.map(|s| s.to_string()),
            name.to_string(),
            catalog_name.map(|s| s.to_string()),
            schema_name.map(|s| s.to_string()),
        );
        self.tables
            .iter()
            .find(|t| t.get_unique_key() == target_key)
    }

    /// Get all relationships involving a specific table
    ///
    /// # Arguments
    ///
    /// * `table_id` - The UUID of the table
    ///
    /// # Returns
    ///
    /// A vector of references to relationships where the table is either the source or target.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_sdk::models::DataModel;
    /// # let model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// # let table_id = uuid::Uuid::new_v4();
    /// // Get all relationships for a table
    /// let relationships = model.get_relationships_for_table(table_id);
    /// ```
    pub fn get_relationships_for_table(&self, table_id: Uuid) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| r.source_table_id == table_id || r.target_table_id == table_id)
            .collect()
    }
}
