//! Relationship model for the SDK

use super::enums::{Cardinality, RelationshipType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKeyDetails {
    pub source_column: String,
    pub target_column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ETLJobMetadata {
    pub job_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_connection_point: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_connection_point: Option<String>,
    #[serde(default)]
    pub routing_waypoints: Vec<ConnectionPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_position: Option<ConnectionPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    pub id: Uuid,
    pub source_table_id: Uuid,
    pub target_table_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardinality: Option<Cardinality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key_details: Option<ForeignKeyDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etl_job_metadata: Option<ETLJobMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_type: Option<RelationshipType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_metadata: Option<VisualMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawio_edge_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Relationship {
    pub fn new(source_table_id: Uuid, target_table_id: Uuid) -> Self {
        let now = Utc::now();
        // Use deterministic UUID v5 based on source and target table IDs
        // This avoids requiring random number generation (getrandom/wasm_js)
        let id = Self::generate_id(source_table_id, target_table_id);
        Self {
            id,
            source_table_id,
            target_table_id,
            cardinality: None,
            source_optional: None,
            target_optional: None,
            foreign_key_details: None,
            etl_job_metadata: None,
            relationship_type: None,
            notes: None,
            visual_metadata: None,
            drawio_edge_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate a deterministic UUID v5 for a relationship based on source and target table IDs
    /// This avoids requiring random number generation (getrandom/wasm_js)
    pub fn generate_id(source_table_id: Uuid, target_table_id: Uuid) -> Uuid {
        // Create a deterministic string from the relationship endpoints
        // Sort IDs to ensure same relationship gets same UUID regardless of direction
        let (id1, id2) = if source_table_id < target_table_id {
            (source_table_id, target_table_id)
        } else {
            (target_table_id, source_table_id)
        };
        let key = format!("{}:{}", id1, id2);
        // Use UUID v5 (deterministic) with a namespace UUID for relationships
        Uuid::new_v5(&Uuid::NAMESPACE_URL, key.as_bytes())
    }
}



