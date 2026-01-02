//! Data Flow format importer for lightweight Data Flow nodes and relationships.
//!
//! This module imports Data Flow format YAML files (lightweight format separate from ODCS).
//! ODCS format is only for Data Models (tables), while this format is for Data Flow nodes and relationships.

use super::ImportError;
use crate::models::enums::InfrastructureType;
use crate::models::{Column, ContactDetails, DataModel, Relationship, SlaProperty, Table};
use serde::{Deserialize, Serialize};
use serde_yaml;
use uuid::Uuid;

/// Data Flow format structure for YAML parsing
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

/// Data Flow format importer
pub struct DataFlowImporter;

impl Default for DataFlowImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl DataFlowImporter {
    /// Create a new Data Flow importer instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::dataflow::DataFlowImporter;
    ///
    /// let importer = DataFlowImporter::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Import Data Flow format YAML content and create DataModel.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Data Flow format YAML content as a string
    ///
    /// # Returns
    ///
    /// A `DataModel` containing the extracted nodes and relationships.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::dataflow::DataFlowImporter;
    ///
    /// let importer = DataFlowImporter::new();
    /// let yaml = r#"
    /// nodes:
    ///   - name: user_events
    ///     metadata:
    ///       owner: "Data Engineering Team"
    ///       infrastructure_type: "Kafka"
    /// "#;
    /// let model = importer.import(yaml).unwrap();
    /// ```
    pub fn import(&self, yaml_content: &str) -> Result<DataModel, ImportError> {
        let data_flow: DataFlowFormat = serde_yaml::from_str(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse YAML: {}", e)))?;

        let mut model = DataModel::new(
            "DataFlow".to_string(),
            "/tmp".to_string(),
            "relationships.yaml".to_string(),
        );

        // Import nodes
        if let Some(nodes) = data_flow.nodes {
            for node in nodes {
                let table = self.parse_node(node)?;
                model.tables.push(table);
            }
        }

        // Import relationships
        if let Some(relationships) = data_flow.relationships {
            for rel in relationships {
                let relationship = self.parse_relationship(rel)?;
                model.relationships.push(relationship);
            }
        }

        Ok(model)
    }

    fn parse_node(&self, node: DataFlowNode) -> Result<Table, ImportError> {
        let id = if let Some(id_str) = node.id {
            Uuid::parse_str(&id_str)
                .map_err(|e| ImportError::ParseError(format!("Invalid UUID: {}", e)))?
        } else {
            Table::generate_id(&node.name, None, None, None)
        };

        let columns: Vec<Column> = node
            .columns
            .unwrap_or_default()
            .into_iter()
            .map(|c| Column::new(c.name, c.data_type))
            .collect();

        let mut table = Table::new(node.name, columns);
        table.id = id;

        // Extract metadata
        if let Some(metadata) = node.metadata {
            table.owner = metadata.owner;
            table.sla = metadata.sla;
            table.contact_details = metadata.contact_details;
            table.notes = metadata.notes;

            // Parse infrastructure type
            if let Some(infra_str) = metadata.infrastructure_type {
                table.infrastructure_type = self.parse_infrastructure_type(&infra_str)?;
            }
        }

        Ok(table)
    }

    fn parse_relationship(&self, rel: DataFlowRelationship) -> Result<Relationship, ImportError> {
        let id = if let Some(id_str) = rel.id {
            Uuid::parse_str(&id_str)
                .map_err(|e| ImportError::ParseError(format!("Invalid UUID: {}", e)))?
        } else {
            Uuid::new_v4()
        };

        let source_id = rel
            .source_node_id
            .ok_or_else(|| ImportError::ParseError("Missing source_node_id".to_string()))?;
        let target_id = rel
            .target_node_id
            .ok_or_else(|| ImportError::ParseError("Missing target_node_id".to_string()))?;

        let source_uuid = Uuid::parse_str(&source_id)
            .map_err(|e| ImportError::ParseError(format!("Invalid source UUID: {}", e)))?;
        let target_uuid = Uuid::parse_str(&target_id)
            .map_err(|e| ImportError::ParseError(format!("Invalid target UUID: {}", e)))?;

        let mut relationship = Relationship::new(source_uuid, target_uuid);
        relationship.id = id;

        // Extract metadata
        if let Some(metadata) = rel.metadata {
            relationship.owner = metadata.owner;
            relationship.sla = metadata.sla;
            relationship.contact_details = metadata.contact_details;
            relationship.notes = metadata.notes;

            // Parse infrastructure type
            if let Some(infra_str) = metadata.infrastructure_type {
                relationship.infrastructure_type = self.parse_infrastructure_type(&infra_str)?;
            }
        }

        Ok(relationship)
    }

    fn parse_infrastructure_type(
        &self,
        infra_str: &str,
    ) -> Result<Option<InfrastructureType>, ImportError> {
        // Try to match the string to InfrastructureType enum
        // Using serde deserialization which handles PascalCase
        match serde_json::from_str::<InfrastructureType>(&format!("\"{}\"", infra_str)) {
            Ok(infra_type) => Ok(Some(infra_type)),
            Err(_) => Err(ImportError::ParseError(format!(
                "Invalid infrastructure type: {}. Must be one of the valid InfrastructureType values.",
                infra_str
            ))),
        }
    }
}
