//! Relationship model for the SDK

use super::enums::{Cardinality, InfrastructureType, RelationshipType};
use super::table::{ContactDetails, SlaProperty};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Foreign key column mapping details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKeyDetails {
    /// Column name in the source table
    pub source_column: String,
    /// Column name in the target table
    pub target_column: String,
}

/// ETL job metadata for data flow relationships
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ETLJobMetadata {
    /// Name of the ETL job that creates this relationship
    pub job_name: String,
    /// Optional notes about the ETL job
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Job execution frequency (e.g., "daily", "hourly")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
}

/// Connection point coordinates for relationship visualization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionPoint {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

/// Visual metadata for relationship rendering on canvas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualMetadata {
    /// Connection point identifier on source table
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_connection_point: Option<String>,
    /// Connection point identifier on target table
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_connection_point: Option<String>,
    /// Waypoints for routing the relationship line
    #[serde(default)]
    pub routing_waypoints: Vec<ConnectionPoint>,
    /// Position for the relationship label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_position: Option<ConnectionPoint>,
}

/// Relationship model representing a connection between two tables
///
/// Relationships can represent foreign keys, data flows, dependencies, or ETL transformations.
/// They connect a source table to a target table with optional metadata about cardinality,
/// foreign key details, and ETL job information.
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::models::Relationship;
///
/// let source_id = uuid::Uuid::new_v4();
/// let target_id = uuid::Uuid::new_v4();
/// let relationship = Relationship::new(source_id, target_id);
/// ```
///
/// # Example with Metadata (Data Flow Relationship)
///
/// ```rust
/// use data_modelling_sdk::models::{Relationship, InfrastructureType, ContactDetails, SlaProperty};
/// use serde_json::json;
/// use uuid::Uuid;
///
/// let source_id = Uuid::new_v4();
/// let target_id = Uuid::new_v4();
/// let mut relationship = Relationship::new(source_id, target_id);
/// relationship.owner = Some("Data Engineering Team".to_string());
/// relationship.infrastructure_type = Some(InfrastructureType::Kafka);
/// relationship.contact_details = Some(ContactDetails {
///     email: Some("team@example.com".to_string()),
///     phone: None,
///     name: Some("Data Team".to_string()),
///     role: Some("Data Owner".to_string()),
///     other: None,
/// });
/// relationship.sla = Some(vec![SlaProperty {
///     property: "latency".to_string(),
///     value: json!(2),
///     unit: "hours".to_string(),
///     description: Some("Data flow must complete within 2 hours".to_string()),
///     element: None,
///     driver: Some("operational".to_string()),
///     scheduler: None,
///     schedule: None,
/// }]);
/// relationship.notes = Some("ETL pipeline from source to target".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    /// Unique identifier for the relationship (UUIDv4)
    pub id: Uuid,
    /// ID of the source table
    pub source_table_id: Uuid,
    /// ID of the target table
    pub target_table_id: Uuid,
    /// Cardinality (OneToOne, OneToMany, ManyToMany)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardinality: Option<Cardinality>,
    /// Whether the source side is optional (nullable foreign key)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_optional: Option<bool>,
    /// Whether the target side is optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_optional: Option<bool>,
    /// Foreign key column mapping details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key_details: Option<ForeignKeyDetails>,
    /// ETL job metadata for data flow relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etl_job_metadata: Option<ETLJobMetadata>,
    /// Type of relationship (ForeignKey, DataFlow, Dependency, ETL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_type: Option<RelationshipType>,
    /// Optional notes about the relationship
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Owner information (person, team, or organization name) for Data Flow relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// SLA (Service Level Agreement) information (ODCS-inspired but lightweight format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<Vec<SlaProperty>>,
    /// Contact details for responsible parties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_details: Option<ContactDetails>,
    /// Infrastructure type (hosting platform, service, or tool) for Data Flow relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_type: Option<InfrastructureType>,
    /// Visual metadata for canvas rendering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_metadata: Option<VisualMetadata>,
    /// Draw.io edge ID for diagram integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawio_edge_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Relationship {
    /// Create a new relationship between two tables
    ///
    /// # Arguments
    ///
    /// * `source_table_id` - UUID of the source table
    /// * `target_table_id` - UUID of the target table
    ///
    /// # Returns
    ///
    /// A new `Relationship` instance with a generated UUIDv4 ID and current timestamps.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::models::Relationship;
    ///
    /// let source_id = uuid::Uuid::new_v4();
    /// let target_id = uuid::Uuid::new_v4();
    /// let rel = Relationship::new(source_id, target_id);
    /// ```
    pub fn new(source_table_id: Uuid, target_table_id: Uuid) -> Self {
        let now = Utc::now();
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
            owner: None,
            sla: None,
            contact_details: None,
            infrastructure_type: None,
            visual_metadata: None,
            drawio_edge_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate a UUIDv4 for a new relationship id.
    ///
    /// Note: params are retained for backward-compatibility with previous deterministic-v5 API.
    pub fn generate_id(_source_table_id: Uuid, _target_table_id: Uuid) -> Uuid {
        Uuid::new_v4()
    }
}
