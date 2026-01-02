//! Data Flow format exporter for lightweight Data Flow nodes and relationships.
//!
//! This module exports Data Flow format YAML files (lightweight format separate from ODCS).
//! ODCS format is only for Data Models (tables), while this format is for Data Flow nodes and relationships.

use crate::models::{DataModel, Relationship, Table};
use serde_yaml;

/// Data Flow format exporter
pub struct DataFlowExporter;

impl Default for DataFlowExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl DataFlowExporter {
    /// Create a new Data Flow exporter instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::dataflow::DataFlowExporter;
    ///
    /// let exporter = DataFlowExporter::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Export a DataModel to Data Flow format YAML.
    ///
    /// # Arguments
    ///
    /// * `model` - The DataModel to export
    ///
    /// # Returns
    ///
    /// A YAML string in Data Flow format.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::dataflow::DataFlowExporter;
    /// use data_modelling_sdk::models::{DataModel, Table, Column};
    ///
    /// let mut model = DataModel::new("test".to_string(), "/tmp".to_string(), "control.yaml".to_string());
    /// let table = Table::new("users".to_string(), vec![Column::new("id".to_string(), "INT".to_string())]);
    /// model.tables.push(table);
    ///
    /// let yaml = DataFlowExporter::export_model(&model);
    /// assert!(yaml.contains("nodes:"));
    /// ```
    pub fn export_model(model: &DataModel) -> String {
        let mut yaml = serde_yaml::Mapping::new();

        // Export nodes
        let nodes: Vec<serde_yaml::Value> =
            model.tables.iter().map(Self::export_node_to_yaml).collect();
        if !nodes.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("nodes".to_string()),
                serde_yaml::Value::Sequence(nodes),
            );
        }

        // Export relationships
        let relationships: Vec<serde_yaml::Value> = model
            .relationships
            .iter()
            .map(Self::export_relationship_to_yaml)
            .collect();
        if !relationships.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("relationships".to_string()),
                serde_yaml::Value::Sequence(relationships),
            );
        }

        serde_yaml::to_string(&yaml).unwrap_or_default()
    }

    /// Export a single node (Table) to Data Flow format YAML.
    ///
    /// # Arguments
    ///
    /// * `table` - The table to export
    ///
    /// # Returns
    ///
    /// A YAML string for a single node.
    pub fn export_node(table: &Table) -> String {
        let yaml_value = Self::export_node_to_yaml(table);
        serde_yaml::to_string(&yaml_value).unwrap_or_default()
    }

    /// Export a single relationship to Data Flow format YAML.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship to export
    ///
    /// # Returns
    ///
    /// A YAML string for a single relationship.
    pub fn export_relationship(relationship: &Relationship) -> String {
        let yaml_value = Self::export_relationship_to_yaml(relationship);
        serde_yaml::to_string(&yaml_value).unwrap_or_default()
    }

    fn export_node_to_yaml(table: &Table) -> serde_yaml::Value {
        let mut node = serde_yaml::Mapping::new();

        node.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(table.id.to_string()),
        );
        node.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(table.name.clone()),
        );
        node.insert(
            serde_yaml::Value::String("type".to_string()),
            serde_yaml::Value::String("table".to_string()),
        );

        // Export columns
        let columns: Vec<serde_yaml::Value> = table
            .columns
            .iter()
            .map(|col| {
                let mut col_map = serde_yaml::Mapping::new();
                col_map.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String(col.name.clone()),
                );
                col_map.insert(
                    serde_yaml::Value::String("type".to_string()),
                    serde_yaml::Value::String(col.data_type.clone()),
                );
                serde_yaml::Value::Mapping(col_map)
            })
            .collect();
        if !columns.is_empty() {
            node.insert(
                serde_yaml::Value::String("columns".to_string()),
                serde_yaml::Value::Sequence(columns),
            );
        }

        // Export metadata
        let metadata = Self::export_metadata_to_yaml(
            table.owner.as_deref(),
            table.sla.as_ref(),
            table.contact_details.as_ref(),
            table.infrastructure_type,
            table.notes.as_deref(),
        );
        if let Some(meta) = metadata {
            node.insert(serde_yaml::Value::String("metadata".to_string()), meta);
        }

        serde_yaml::Value::Mapping(node)
    }

    fn export_relationship_to_yaml(relationship: &Relationship) -> serde_yaml::Value {
        let mut rel = serde_yaml::Mapping::new();

        rel.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(relationship.id.to_string()),
        );
        rel.insert(
            serde_yaml::Value::String("source_node_id".to_string()),
            serde_yaml::Value::String(relationship.source_table_id.to_string()),
        );
        rel.insert(
            serde_yaml::Value::String("target_node_id".to_string()),
            serde_yaml::Value::String(relationship.target_table_id.to_string()),
        );

        // Export metadata
        let metadata = Self::export_metadata_to_yaml(
            relationship.owner.as_deref(),
            relationship.sla.as_ref(),
            relationship.contact_details.as_ref(),
            relationship.infrastructure_type,
            relationship.notes.as_deref(),
        );
        if let Some(meta) = metadata {
            rel.insert(serde_yaml::Value::String("metadata".to_string()), meta);
        }

        serde_yaml::Value::Mapping(rel)
    }

    fn export_metadata_to_yaml(
        owner: Option<&str>,
        sla: Option<&Vec<crate::models::SlaProperty>>,
        contact_details: Option<&crate::models::ContactDetails>,
        infrastructure_type: Option<crate::models::enums::InfrastructureType>,
        notes: Option<&str>,
    ) -> Option<serde_yaml::Value> {
        let mut has_metadata = false;
        let mut metadata = serde_yaml::Mapping::new();

        if let Some(owner_str) = owner {
            metadata.insert(
                serde_yaml::Value::String("owner".to_string()),
                serde_yaml::Value::String(owner_str.to_string()),
            );
            has_metadata = true;
        }

        if let Some(sla_vec) = sla {
            let sla_yaml: Vec<serde_yaml::Value> = sla_vec
                .iter()
                .map(|sla_prop| {
                    let mut sla_map = serde_yaml::Mapping::new();
                    sla_map.insert(
                        serde_yaml::Value::String("property".to_string()),
                        serde_yaml::Value::String(sla_prop.property.clone()),
                    );
                    sla_map.insert(
                        serde_yaml::Value::String("value".to_string()),
                        Self::json_value_to_yaml(&sla_prop.value),
                    );
                    sla_map.insert(
                        serde_yaml::Value::String("unit".to_string()),
                        serde_yaml::Value::String(sla_prop.unit.clone()),
                    );
                    if let Some(ref element) = sla_prop.element {
                        sla_map.insert(
                            serde_yaml::Value::String("element".to_string()),
                            serde_yaml::Value::String(element.clone()),
                        );
                    }
                    if let Some(ref driver) = sla_prop.driver {
                        sla_map.insert(
                            serde_yaml::Value::String("driver".to_string()),
                            serde_yaml::Value::String(driver.clone()),
                        );
                    }
                    if let Some(ref description) = sla_prop.description {
                        sla_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(description.clone()),
                        );
                    }
                    if let Some(ref scheduler) = sla_prop.scheduler {
                        sla_map.insert(
                            serde_yaml::Value::String("scheduler".to_string()),
                            serde_yaml::Value::String(scheduler.clone()),
                        );
                    }
                    if let Some(ref schedule) = sla_prop.schedule {
                        sla_map.insert(
                            serde_yaml::Value::String("schedule".to_string()),
                            serde_yaml::Value::String(schedule.clone()),
                        );
                    }
                    serde_yaml::Value::Mapping(sla_map)
                })
                .collect();
            metadata.insert(
                serde_yaml::Value::String("sla".to_string()),
                serde_yaml::Value::Sequence(sla_yaml),
            );
            has_metadata = true;
        }

        if let Some(contact) = contact_details {
            let mut contact_map = serde_yaml::Mapping::new();
            if let Some(ref email) = contact.email {
                contact_map.insert(
                    serde_yaml::Value::String("email".to_string()),
                    serde_yaml::Value::String(email.clone()),
                );
            }
            if let Some(ref phone) = contact.phone {
                contact_map.insert(
                    serde_yaml::Value::String("phone".to_string()),
                    serde_yaml::Value::String(phone.clone()),
                );
            }
            if let Some(ref name) = contact.name {
                contact_map.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String(name.clone()),
                );
            }
            if let Some(ref role) = contact.role {
                contact_map.insert(
                    serde_yaml::Value::String("role".to_string()),
                    serde_yaml::Value::String(role.clone()),
                );
            }
            if let Some(ref other) = contact.other {
                contact_map.insert(
                    serde_yaml::Value::String("other".to_string()),
                    serde_yaml::Value::String(other.clone()),
                );
            }
            if !contact_map.is_empty() {
                metadata.insert(
                    serde_yaml::Value::String("contact_details".to_string()),
                    serde_yaml::Value::Mapping(contact_map),
                );
                has_metadata = true;
            }
        }

        if let Some(infra_type) = infrastructure_type {
            // Serialize enum as PascalCase string
            let infra_str = serde_json::to_string(&infra_type)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            metadata.insert(
                serde_yaml::Value::String("infrastructure_type".to_string()),
                serde_yaml::Value::String(infra_str),
            );
            has_metadata = true;
        }

        if let Some(notes_str) = notes {
            metadata.insert(
                serde_yaml::Value::String("notes".to_string()),
                serde_yaml::Value::String(notes_str.to_string()),
            );
            has_metadata = true;
        }

        if has_metadata {
            Some(serde_yaml::Value::Mapping(metadata))
        } else {
            None
        }
    }

    fn json_value_to_yaml(json: &serde_json::Value) -> serde_yaml::Value {
        match json {
            serde_json::Value::Null => serde_yaml::Value::Null,
            serde_json::Value::Bool(b) => serde_yaml::Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    serde_yaml::Value::Number(serde_yaml::Number::from(i))
                } else if let Some(f) = n.as_f64() {
                    serde_yaml::Value::Number(serde_yaml::Number::from(f))
                } else {
                    serde_yaml::Value::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => serde_yaml::Value::String(s.clone()),
            serde_json::Value::Array(arr) => {
                let yaml_arr: Vec<serde_yaml::Value> =
                    arr.iter().map(Self::json_value_to_yaml).collect();
                serde_yaml::Value::Sequence(yaml_arr)
            }
            serde_json::Value::Object(obj) => {
                let mut yaml_map = serde_yaml::Mapping::new();
                for (k, v) in obj {
                    yaml_map.insert(
                        serde_yaml::Value::String(k.clone()),
                        Self::json_value_to_yaml(v),
                    );
                }
                serde_yaml::Value::Mapping(yaml_map)
            }
        }
    }
}
