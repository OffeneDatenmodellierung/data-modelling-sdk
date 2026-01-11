//! Business Domain schema models
//!
//! Defines structures for organizing systems, CADS nodes, and ODCS nodes within business domains.

use super::cads::CADSKind;
use super::enums::InfrastructureType;
use super::table::{ContactDetails, SlaProperty};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Cardinality for Crowsfeet notation relationships between ODCS nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CrowsfeetCardinality {
    /// One-to-one relationship (1:1)
    OneToOne,
    /// One-to-many relationship (1:N)
    OneToMany,
    /// Zero-or-one relationship (0:1)
    ZeroOrOne,
    /// Zero-or-many relationship (0:N)
    ZeroOrMany,
}

/// System - Physical infrastructure entity
///
/// Systems are physical entities like Kafka, Cassandra, EKS, EC2, etc.
/// They inherit DataFlow node metadata (owner, SLA, contact_details, infrastructure_type, notes).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct System {
    /// Unique identifier
    pub id: Uuid,
    /// System name
    pub name: String,
    /// Infrastructure type (Kafka, Cassandra, EKS, EC2, etc.)
    pub infrastructure_type: InfrastructureType,
    /// Parent domain ID
    pub domain_id: Uuid,
    /// System description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// System endpoints
    #[serde(default)]
    pub endpoints: Vec<String>,
    /// Owner (from DataFlow metadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// SLA properties (from DataFlow metadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<Vec<SlaProperty>>,
    /// Contact details (from DataFlow metadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_details: Option<ContactDetails>,
    /// Notes (from DataFlow metadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Version (semantic version) - required when sharing, optional for local systems
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// SystemConnection - ERD-style connection between systems
///
/// Represents bidirectional connections between systems with connection metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemConnection {
    /// Unique identifier
    pub id: Uuid,
    /// Source system ID
    pub source_system_id: Uuid,
    /// Target system ID
    pub target_system_id: Uuid,
    /// Connection type (e.g., "data_flow", "api_call", "message_queue")
    pub connection_type: String,
    /// Whether the connection is bidirectional
    #[serde(default)]
    pub bidirectional: bool,
    /// Connection metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Shared node reference
///
/// Used for referencing nodes shared from other domains.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedNodeReference {
    /// Domain ID where the node is defined
    pub domain_id: Uuid,
    /// Node ID in the source domain
    pub node_id: Uuid,
    /// Node version (semantic version)
    pub node_version: String,
}

/// CADSNode - Reference to a CADS asset
///
/// Can be a local CADS asset or a shared reference from another domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSNode {
    /// Unique identifier
    pub id: Uuid,
    /// Parent system ID
    pub system_id: Uuid,
    /// CADS asset ID (if local)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cads_asset_id: Option<Uuid>,
    /// CADS asset kind
    pub kind: CADSKind,
    /// Shared reference (if shared from another domain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_reference: Option<SharedNodeReference>,
    /// Local metadata overrides (for shared nodes)
    #[serde(default)]
    pub custom_metadata: Vec<HashMap<String, serde_json::Value>>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// ODCSNode - Reference to an ODCS Table
///
/// Can be a local ODCS Table or a shared reference from another domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ODCSNode {
    /// Unique identifier
    pub id: Uuid,
    /// Parent system ID
    pub system_id: Uuid,
    /// ODCS Table ID (if local)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<Uuid>,
    /// Node role (e.g., "source", "destination", "intermediate")
    pub role: String,
    /// Shared reference (if shared from another domain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_reference: Option<SharedNodeReference>,
    /// Local metadata overrides (for shared nodes)
    #[serde(default)]
    pub custom_metadata: Vec<HashMap<String, serde_json::Value>>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// NodeConnection - Crowsfeet notation relationship between ODCS nodes
///
/// Represents cardinality-based relationships (1:1, 1:N, 0:1, 0:N) between ODCS nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeConnection {
    /// Unique identifier
    pub id: Uuid,
    /// Source node ID (ODCSNode)
    pub source_node_id: Uuid,
    /// Target node ID (ODCSNode)
    pub target_node_id: Uuid,
    /// Cardinality (OneToOne, OneToMany, ZeroOrOne, ZeroOrMany)
    pub cardinality: CrowsfeetCardinality,
    /// Relationship type (e.g., "foreign_key", "data_flow", "derived_from")
    pub relationship_type: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Domain - Top-level container for a business domain
///
/// Organizes systems, CADS nodes, and ODCS nodes within a business domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Domain {
    /// Unique identifier
    pub id: Uuid,
    /// Domain name
    pub name: String,
    /// Domain description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Systems in this domain
    #[serde(default)]
    pub systems: Vec<System>,
    /// CADS nodes in this domain
    #[serde(default)]
    pub cads_nodes: Vec<CADSNode>,
    /// ODCS nodes in this domain
    #[serde(default)]
    pub odcs_nodes: Vec<ODCSNode>,
    /// System connections (ERD-style)
    #[serde(default)]
    pub system_connections: Vec<SystemConnection>,
    /// Node connections (Crowsfeet notation)
    #[serde(default)]
    pub node_connections: Vec<NodeConnection>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Domain {
    /// Create a new Domain
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            systems: Vec::new(),
            cads_nodes: Vec::new(),
            odcs_nodes: Vec::new(),
            system_connections: Vec::new(),
            node_connections: Vec::new(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }

    /// Add a system to the domain
    pub fn add_system(&mut self, mut system: System) {
        system.domain_id = self.id;
        system.created_at = Some(chrono::Utc::now());
        system.updated_at = Some(chrono::Utc::now());
        self.systems.push(system);
        self.updated_at = Some(chrono::Utc::now());
    }

    /// Add a CADS node to the domain
    pub fn add_cads_node(&mut self, mut node: CADSNode) {
        node.created_at = Some(chrono::Utc::now());
        node.updated_at = Some(chrono::Utc::now());
        self.cads_nodes.push(node);
        self.updated_at = Some(chrono::Utc::now());
    }

    /// Add an ODCS node to the domain
    pub fn add_odcs_node(&mut self, mut node: ODCSNode) {
        node.created_at = Some(chrono::Utc::now());
        node.updated_at = Some(chrono::Utc::now());
        self.odcs_nodes.push(node);
        self.updated_at = Some(chrono::Utc::now());
    }

    /// Add a system connection
    pub fn add_system_connection(&mut self, mut connection: SystemConnection) {
        connection.created_at = Some(chrono::Utc::now());
        connection.updated_at = Some(chrono::Utc::now());
        self.system_connections.push(connection);
        self.updated_at = Some(chrono::Utc::now());
    }

    /// Add a node connection
    pub fn add_node_connection(&mut self, mut connection: NodeConnection) {
        connection.created_at = Some(chrono::Utc::now());
        connection.updated_at = Some(chrono::Utc::now());
        self.node_connections.push(connection);
        self.updated_at = Some(chrono::Utc::now());
    }

    /// Import domain from YAML
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }

    /// Export domain to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

impl System {
    /// Create a new System
    pub fn new(name: String, infrastructure_type: InfrastructureType, domain_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            infrastructure_type,
            domain_id,
            description: None,
            endpoints: Vec::new(),
            owner: None,
            sla: None,
            contact_details: None,
            notes: None,
            version: None,
            metadata: HashMap::new(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }
}

impl CADSNode {
    /// Create a new local CADS node
    pub fn new_local(system_id: Uuid, cads_asset_id: Uuid, kind: CADSKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            system_id,
            cads_asset_id: Some(cads_asset_id),
            kind,
            shared_reference: None,
            custom_metadata: Vec::new(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }

    /// Create a new shared CADS node reference
    pub fn new_shared(
        system_id: Uuid,
        kind: CADSKind,
        shared_reference: SharedNodeReference,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            system_id,
            cads_asset_id: None,
            kind,
            shared_reference: Some(shared_reference),
            custom_metadata: Vec::new(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }
}

impl ODCSNode {
    /// Create a new local ODCS node
    pub fn new_local(system_id: Uuid, table_id: Uuid, role: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            system_id,
            table_id: Some(table_id),
            role,
            shared_reference: None,
            custom_metadata: Vec::new(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }

    /// Create a new shared ODCS node reference
    pub fn new_shared(
        system_id: Uuid,
        role: String,
        shared_reference: SharedNodeReference,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            system_id,
            table_id: None,
            role,
            shared_reference: Some(shared_reference),
            custom_metadata: Vec::new(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }
}
