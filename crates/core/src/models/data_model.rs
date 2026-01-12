//! DataModel for the SDK

use super::domain::{CADSNode, Domain, NodeConnection, ODCSNode, System, SystemConnection};
use super::enums::InfrastructureType;
use super::relationship::Relationship;
use super::table::Table;
use super::tag::Tag;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// Data model representing a complete data model with tables and relationships
///
/// A `DataModel` is a container for a collection of tables and their relationships.
/// It represents a workspace or domain within a larger data modeling system.
///
/// # Example
///
/// ```rust
/// use data_modelling_core::models::DataModel;
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
    /// Business domains in this model
    #[serde(default)]
    pub domains: Vec<Domain>,
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
    /// use data_modelling_core::models::DataModel;
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
            domains: Vec::new(),
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
    /// # use data_modelling_core::models::DataModel;
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
    /// # use data_modelling_core::models::DataModel;
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

    /// Filter Data Flow nodes (tables) by owner
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner name to filter by (case-sensitive exact match)
    ///
    /// # Returns
    ///
    /// A vector of references to tables matching the owner.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_core::models::{DataModel, Table, Column};
    /// # let mut model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// # let mut table = Table::new("test_table".to_string(), vec![Column::new("id".to_string(), "INT".to_string())]);
    /// # table.owner = Some("Data Engineering Team".to_string());
    /// # model.tables.push(table);
    /// let owned_nodes = model.filter_nodes_by_owner("Data Engineering Team");
    /// ```
    pub fn filter_nodes_by_owner(&self, owner: &str) -> Vec<&Table> {
        self.tables
            .iter()
            .filter(|t| t.owner.as_deref() == Some(owner))
            .collect()
    }

    /// Filter Data Flow relationships by owner
    ///
    /// # Arguments
    ///
    /// * `owner` - The owner name to filter by (case-sensitive exact match)
    ///
    /// # Returns
    ///
    /// A vector of references to relationships matching the owner.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_core::models::{DataModel, Relationship};
    /// # use uuid::Uuid;
    /// # let mut model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// # let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
    /// # rel.owner = Some("Data Engineering Team".to_string());
    /// # model.relationships.push(rel);
    /// let owned_relationships = model.filter_relationships_by_owner("Data Engineering Team");
    /// ```
    pub fn filter_relationships_by_owner(&self, owner: &str) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| r.owner.as_deref() == Some(owner))
            .collect()
    }

    /// Filter Data Flow nodes (tables) by infrastructure type
    ///
    /// # Arguments
    ///
    /// * `infra_type` - The infrastructure type to filter by
    ///
    /// # Returns
    ///
    /// A vector of references to tables matching the infrastructure type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_core::models::{DataModel, Table, Column, InfrastructureType};
    /// # let mut model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// # let mut table = Table::new("test_table".to_string(), vec![Column::new("id".to_string(), "INT".to_string())]);
    /// # table.infrastructure_type = Some(InfrastructureType::Kafka);
    /// # model.tables.push(table);
    /// let kafka_nodes = model.filter_nodes_by_infrastructure_type(InfrastructureType::Kafka);
    /// ```
    pub fn filter_nodes_by_infrastructure_type(
        &self,
        infra_type: InfrastructureType,
    ) -> Vec<&Table> {
        self.tables
            .iter()
            .filter(|t| t.infrastructure_type == Some(infra_type))
            .collect()
    }

    /// Filter Data Flow relationships by infrastructure type
    ///
    /// # Arguments
    ///
    /// * `infra_type` - The infrastructure type to filter by
    ///
    /// # Returns
    ///
    /// A vector of references to relationships matching the infrastructure type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_core::models::{DataModel, Relationship, InfrastructureType};
    /// # use uuid::Uuid;
    /// # let mut model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// # let mut rel = Relationship::new(Uuid::new_v4(), Uuid::new_v4());
    /// # rel.infrastructure_type = Some(InfrastructureType::Kafka);
    /// # model.relationships.push(rel);
    /// let kafka_relationships = model.filter_relationships_by_infrastructure_type(InfrastructureType::Kafka);
    /// ```
    pub fn filter_relationships_by_infrastructure_type(
        &self,
        infra_type: InfrastructureType,
    ) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| r.infrastructure_type == Some(infra_type))
            .collect()
    }

    /// Filter Data Flow nodes and relationships by tag
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to filter by
    ///
    /// # Returns
    ///
    /// A tuple containing vectors of references to tables and relationships containing the tag.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_core::models::{DataModel, Table, Column, Tag};
    /// # let mut model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// # let mut table = Table::new("test_table".to_string(), vec![Column::new("id".to_string(), "INT".to_string())]);
    /// # table.tags.push(Tag::Simple("production".to_string()));
    /// # model.tables.push(table);
    /// let (tagged_nodes, tagged_relationships) = model.filter_by_tags("production");
    /// ```
    pub fn filter_by_tags(&self, tag: &str) -> (Vec<&Table>, Vec<&Relationship>) {
        // Parse the tag string to Tag enum for comparison
        let search_tag = Tag::from_str(tag).unwrap_or_else(|_| {
            // If parsing fails, create a Simple tag
            Tag::Simple(tag.to_string())
        });

        let tagged_tables: Vec<&Table> = self
            .tables
            .iter()
            .filter(|t| t.tags.contains(&search_tag))
            .collect();
        let tagged_relationships: Vec<&Relationship> = self
            .relationships
            .iter()
            .filter(|_r| {
                // Relationships don't have tags field, so we return empty for now
                // This maintains the API contract but relationships don't support tags yet
                false
            })
            .collect();
        (tagged_tables, tagged_relationships)
    }

    /// Add a domain to the model
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain to add
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_core::models::{DataModel, Domain};
    /// # let mut model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// let domain = Domain::new("customer-service".to_string());
    /// model.add_domain(domain);
    /// ```
    pub fn add_domain(&mut self, domain: Domain) {
        self.domains.push(domain);
        self.updated_at = Utc::now();
    }

    /// Get a domain by its ID
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The UUID of the domain to find
    ///
    /// # Returns
    ///
    /// A reference to the domain if found, `None` otherwise.
    pub fn get_domain_by_id(&self, domain_id: Uuid) -> Option<&Domain> {
        self.domains.iter().find(|d| d.id == domain_id)
    }

    /// Get a mutable reference to a domain by its ID
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The UUID of the domain to find
    ///
    /// # Returns
    ///
    /// A mutable reference to the domain if found, `None` otherwise.
    pub fn get_domain_by_id_mut(&mut self, domain_id: Uuid) -> Option<&mut Domain> {
        self.domains.iter_mut().find(|d| d.id == domain_id)
    }

    /// Get a domain by its name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the domain to find
    ///
    /// # Returns
    ///
    /// A reference to the first domain with the given name if found, `None` otherwise.
    pub fn get_domain_by_name(&self, name: &str) -> Option<&Domain> {
        self.domains.iter().find(|d| d.name == name)
    }

    /// Add a system to a domain
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The UUID of the domain
    /// * `system` - The system to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if the domain was found and the system was added, `Err` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use data_modelling_core::models::{DataModel, Domain, System, InfrastructureType};
    /// # use uuid::Uuid;
    /// # let mut model = DataModel::new("test".to_string(), "/path".to_string(), "control.yaml".to_string());
    /// # let domain = Domain::new("customer-service".to_string());
    /// # let domain_id = domain.id;
    /// # model.add_domain(domain);
    /// let system = System::new("kafka-cluster".to_string(), InfrastructureType::Kafka, domain_id);
    /// model.add_system_to_domain(domain_id, system).unwrap();
    /// ```
    pub fn add_system_to_domain(&mut self, domain_id: Uuid, system: System) -> Result<(), String> {
        let domain = self
            .get_domain_by_id_mut(domain_id)
            .ok_or_else(|| format!("Domain with ID {} not found", domain_id))?;
        domain.add_system(system);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a CADS node to a domain
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The UUID of the domain
    /// * `node` - The CADS node to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if the domain was found and the node was added, `Err` otherwise.
    pub fn add_cads_node_to_domain(
        &mut self,
        domain_id: Uuid,
        node: CADSNode,
    ) -> Result<(), String> {
        let domain = self
            .get_domain_by_id_mut(domain_id)
            .ok_or_else(|| format!("Domain with ID {} not found", domain_id))?;
        domain.add_cads_node(node);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add an ODCS node to a domain
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The UUID of the domain
    /// * `node` - The ODCS node to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if the domain was found and the node was added, `Err` otherwise.
    pub fn add_odcs_node_to_domain(
        &mut self,
        domain_id: Uuid,
        node: ODCSNode,
    ) -> Result<(), String> {
        let domain = self
            .get_domain_by_id_mut(domain_id)
            .ok_or_else(|| format!("Domain with ID {} not found", domain_id))?;
        domain.add_odcs_node(node);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a system connection to a domain
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The UUID of the domain
    /// * `connection` - The system connection to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if the domain was found and the connection was added, `Err` otherwise.
    pub fn add_system_connection_to_domain(
        &mut self,
        domain_id: Uuid,
        connection: SystemConnection,
    ) -> Result<(), String> {
        let domain = self
            .get_domain_by_id_mut(domain_id)
            .ok_or_else(|| format!("Domain with ID {} not found", domain_id))?;
        domain.add_system_connection(connection);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a node connection to a domain
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The UUID of the domain
    /// * `connection` - The node connection to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if the domain was found and the connection was added, `Err` otherwise.
    pub fn add_node_connection_to_domain(
        &mut self,
        domain_id: Uuid,
        connection: NodeConnection,
    ) -> Result<(), String> {
        let domain = self
            .get_domain_by_id_mut(domain_id)
            .ok_or_else(|| format!("Domain with ID {} not found", domain_id))?;
        domain.add_node_connection(connection);
        self.updated_at = Utc::now();
        Ok(())
    }
}
