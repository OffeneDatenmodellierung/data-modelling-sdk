//! DataFlow to Domain schema migration utility
//!
//! Converts DataFlow YAML format to Business Domain schema format.
//! DataFlow nodes become Systems, and DataFlow relationships become SystemConnections.

use crate::models::domain::{Domain, System, SystemConnection};
use crate::models::enums::InfrastructureType;
use crate::models::table::{ContactDetails, SlaProperty};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use uuid::Uuid;

/// DataFlow format structure for YAML parsing (internal, for migration only)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowFormat {
    nodes: Option<Vec<DataFlowNode>>,
    relationships: Option<Vec<DataFlowRelationship>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowNode {
    id: Option<String>,
    name: String,
    #[serde(rename = "type")]
    node_type: Option<String>,
    columns: Option<Vec<DataFlowColumn>>,
    metadata: Option<DataFlowMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowColumn {
    name: String,
    #[serde(rename = "type")]
    data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowRelationship {
    id: Option<String>,
    source_node_id: Option<String>,
    target_node_id: Option<String>,
    metadata: Option<DataFlowMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataFlowMetadata {
    owner: Option<String>,
    sla: Option<Vec<SlaProperty>>,
    contact_details: Option<ContactDetails>,
    infrastructure_type: Option<String>,
    notes: Option<String>,
}

/// Error during DataFlow migration
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid infrastructure type: {0}")]
    InvalidInfrastructureType(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Migrate DataFlow YAML to Domain schema format
///
/// # Arguments
///
/// * `dataflow_yaml` - DataFlow format YAML content as a string
/// * `domain_name` - Name for the new Domain (optional, defaults to "MigratedDomain")
///
/// # Returns
///
/// A `Domain` containing Systems and SystemConnections migrated from DataFlow format
///
/// # Example
///
/// ```rust
/// use data_modelling_core::convert::migrate_dataflow::migrate_dataflow_to_domain;
/// use uuid::Uuid;
///
/// let node1_id = Uuid::new_v4();
/// let node2_id = Uuid::new_v4();
/// let dataflow_yaml = format!(r#"
/// nodes:
///   - id: {}
///     name: kafka-cluster
///     metadata:
///       owner: "Data Engineering Team"
///       infrastructure_type: "Kafka"
///   - id: {}
///     name: postgres-db
///     metadata:
///       infrastructure_type: "PostgreSQL"
/// relationships:
///   - source_node_id: "{}"
///     target_node_id: "{}"
/// "#, node1_id, node2_id, node1_id, node2_id);
///
/// let domain = migrate_dataflow_to_domain(&dataflow_yaml, Some("customer-service")).unwrap();
/// assert_eq!(domain.systems.len(), 2);
/// assert_eq!(domain.system_connections.len(), 1);
/// ```
pub fn migrate_dataflow_to_domain(
    dataflow_yaml: &str,
    domain_name: Option<&str>,
) -> Result<Domain, MigrationError> {
    // Parse DataFlow YAML
    let data_flow: DataFlowFormat = serde_yaml::from_str(dataflow_yaml)
        .map_err(|e| MigrationError::ParseError(format!("Failed to parse DataFlow YAML: {}", e)))?;

    // Create new Domain
    let mut domain = Domain::new(domain_name.unwrap_or("MigratedDomain").to_string());

    // Map of old node IDs to new system IDs
    let mut node_id_to_system_id: HashMap<String, Uuid> = HashMap::new();

    // Migrate nodes to Systems
    if let Some(nodes) = data_flow.nodes {
        for node in nodes {
            let system = migrate_node_to_system(&node, domain.id)?;
            let system_id = system.id;

            // Store mapping for relationships
            let node_id = node.id.unwrap_or_else(|| system_id.to_string());
            node_id_to_system_id.insert(node_id, system_id);

            domain.add_system(system);
        }
    }

    // Migrate relationships to SystemConnections
    if let Some(relationships) = data_flow.relationships {
        for rel in relationships {
            let connection =
                migrate_relationship_to_system_connection(&rel, &node_id_to_system_id)?;
            domain.add_system_connection(connection);
        }
    }

    Ok(domain)
}

/// Migrate a DataFlow node to a System
fn migrate_node_to_system(node: &DataFlowNode, domain_id: Uuid) -> Result<System, MigrationError> {
    // Parse system ID
    let system_id = if let Some(id_str) = &node.id {
        Uuid::parse_str(id_str)
            .map_err(|e| MigrationError::ParseError(format!("Invalid node UUID: {}", e)))?
    } else {
        // Generate ID from name if not provided
        Uuid::new_v4()
    };

    // Parse infrastructure type
    let infrastructure_type = if let Some(infra_str) = node
        .metadata
        .as_ref()
        .and_then(|m| m.infrastructure_type.as_ref())
    {
        parse_infrastructure_type(infra_str)?
    } else {
        // Default to Kafka if not specified (common for DataFlow)
        InfrastructureType::Kafka
    };

    // Create System with DataFlow metadata
    let mut system = System::new(node.name.clone(), infrastructure_type, domain_id);
    system.id = system_id;

    // Preserve all DataFlow metadata
    if let Some(metadata) = &node.metadata {
        system.owner = metadata.owner.clone();
        system.sla = metadata.sla.clone();
        system.contact_details = metadata.contact_details.clone();
        system.notes = metadata.notes.clone();

        // If infrastructure_type was in metadata, it's already set above
        // But if it wasn't, we keep the default
    }

    // Add description if node has columns (indicates it might be a data store)
    if let Some(columns) = &node.columns
        && !columns.is_empty()
    {
        system.description = Some(format!(
            "Migrated from DataFlow node with {} columns",
            columns.len()
        ));
    }

    Ok(system)
}

/// Migrate a DataFlow relationship to a SystemConnection
fn migrate_relationship_to_system_connection(
    rel: &DataFlowRelationship,
    node_id_to_system_id: &HashMap<String, Uuid>,
) -> Result<SystemConnection, MigrationError> {
    // Parse connection ID
    let connection_id = if let Some(id_str) = &rel.id {
        Uuid::parse_str(id_str)
            .map_err(|e| MigrationError::ParseError(format!("Invalid relationship UUID: {}", e)))?
    } else {
        Uuid::new_v4()
    };

    // Get source and target system IDs
    let source_node_id = rel
        .source_node_id
        .as_ref()
        .ok_or_else(|| MigrationError::MissingField("source_node_id".to_string()))?;
    let target_node_id = rel
        .target_node_id
        .as_ref()
        .ok_or_else(|| MigrationError::MissingField("target_node_id".to_string()))?;

    let source_system_id = *node_id_to_system_id.get(source_node_id).ok_or_else(|| {
        MigrationError::ParseError(format!("Source node ID not found: {}", source_node_id))
    })?;
    let target_system_id = *node_id_to_system_id.get(target_node_id).ok_or_else(|| {
        MigrationError::ParseError(format!("Target node ID not found: {}", target_node_id))
    })?;

    // Create SystemConnection
    let mut connection = SystemConnection {
        id: connection_id,
        source_system_id,
        target_system_id,
        connection_type: "data_flow".to_string(), // Default for DataFlow relationships
        bidirectional: false,                     // Default to unidirectional
        metadata: HashMap::new(),
        created_at: None,
        updated_at: None,
    };

    // Preserve relationship metadata if present
    if let Some(metadata) = &rel.metadata {
        // Store metadata in connection.metadata HashMap
        if let Some(owner) = &metadata.owner {
            connection.metadata.insert(
                "owner".to_string(),
                serde_json::Value::String(owner.clone()),
            );
        }
        if let Some(notes) = &metadata.notes {
            connection.metadata.insert(
                "notes".to_string(),
                serde_json::Value::String(notes.clone()),
            );
        }
        if let Some(sla) = &metadata.sla {
            connection.metadata.insert(
                "sla".to_string(),
                serde_json::to_value(sla).unwrap_or(serde_json::Value::Null),
            );
        }
        if let Some(contact_details) = &metadata.contact_details {
            connection.metadata.insert(
                "contact_details".to_string(),
                serde_json::to_value(contact_details).unwrap_or(serde_json::Value::Null),
            );
        }
        if let Some(infra_type) = &metadata.infrastructure_type {
            connection.metadata.insert(
                "infrastructure_type".to_string(),
                serde_json::Value::String(infra_type.clone()),
            );
        }
    }

    Ok(connection)
}

/// Parse infrastructure type string to InfrastructureType enum
fn parse_infrastructure_type(infra_str: &str) -> Result<InfrastructureType, MigrationError> {
    // Try to match the string to InfrastructureType enum
    // Using serde deserialization which handles PascalCase
    match serde_json::from_str::<InfrastructureType>(&format!("\"{}\"", infra_str)) {
        Ok(infra_type) => Ok(infra_type),
        Err(_) => Err(MigrationError::InvalidInfrastructureType(format!(
            "Invalid infrastructure type: {}. Must be one of the valid InfrastructureType values.",
            infra_str
        ))),
    }
}
