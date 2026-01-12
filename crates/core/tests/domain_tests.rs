//! Tests for Business Domain schema

use data_modelling_core::models::cads::CADSKind;
use data_modelling_core::models::domain::*;
use data_modelling_core::models::enums::InfrastructureType;
use data_modelling_core::models::table::{ContactDetails, SlaProperty};
use serde_json::json;
use uuid::Uuid;

#[test]
fn test_domain_creation() {
    let domain = Domain::new("customer-service".to_string());

    assert_eq!(domain.name, "customer-service");
    assert!(domain.systems.is_empty());
    assert!(domain.cads_nodes.is_empty());
    assert!(domain.odcs_nodes.is_empty());
    assert!(domain.system_connections.is_empty());
    assert!(domain.node_connections.is_empty());
}

#[test]
fn test_system_creation_with_dataflow_metadata() {
    let domain_id = Uuid::new_v4();
    let mut system = System::new(
        "kafka-cluster".to_string(),
        InfrastructureType::Kafka,
        domain_id,
    );

    // Add DataFlow metadata
    system.owner = Some("Data Engineering Team".to_string());
    system.sla = Some(vec![SlaProperty {
        property: "availability".to_string(),
        value: json!(99.9),
        unit: "percent".to_string(),
        element: None,
        driver: Some("operational".to_string()),
        description: Some("99.9% uptime SLA".to_string()),
        scheduler: None,
        schedule: None,
    }]);
    system.contact_details = Some(ContactDetails {
        email: Some("data-eng@example.com".to_string()),
        phone: None,
        name: Some("Data Engineering Team".to_string()),
        role: Some("System Owner".to_string()),
        other: None,
    });
    system.notes = Some("Primary Kafka cluster for customer events".to_string());
    system.version = Some("1.0.0".to_string());

    assert_eq!(system.name, "kafka-cluster");
    assert_eq!(system.infrastructure_type, InfrastructureType::Kafka);
    assert_eq!(system.domain_id, domain_id);
    assert!(system.owner.is_some());
    assert!(system.sla.is_some());
    assert!(system.contact_details.is_some());
    assert!(system.notes.is_some());
    assert_eq!(system.version, Some("1.0.0".to_string()));
}

#[test]
fn test_system_connection_erd_style() {
    let source_system_id = Uuid::new_v4();
    let target_system_id = Uuid::new_v4();

    let connection = SystemConnection {
        id: Uuid::new_v4(),
        source_system_id,
        target_system_id,
        connection_type: "data_flow".to_string(),
        bidirectional: true,
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert("protocol".to_string(), json!("kafka"));
            m.insert("topic".to_string(), json!("customer-events"));
            m
        },
        created_at: None,
        updated_at: None,
    };

    assert_eq!(connection.source_system_id, source_system_id);
    assert_eq!(connection.target_system_id, target_system_id);
    assert_eq!(connection.connection_type, "data_flow");
    assert!(connection.bidirectional);
    assert_eq!(connection.metadata.get("protocol"), Some(&json!("kafka")));
}

#[test]
fn test_node_connection_crowsfeet_notation() {
    let source_node_id = Uuid::new_v4();
    let target_node_id = Uuid::new_v4();

    let connection = NodeConnection {
        id: Uuid::new_v4(),
        source_node_id,
        target_node_id,
        cardinality: CrowsfeetCardinality::OneToMany,
        relationship_type: "foreign_key".to_string(),
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert("foreign_key_column".to_string(), json!("customer_id"));
            m
        },
        created_at: None,
        updated_at: None,
    };

    assert_eq!(connection.source_node_id, source_node_id);
    assert_eq!(connection.target_node_id, target_node_id);
    assert_eq!(connection.cardinality, CrowsfeetCardinality::OneToMany);
    assert_eq!(connection.relationship_type, "foreign_key");
}

#[test]
fn test_cads_node_local() {
    let system_id = Uuid::new_v4();
    let cads_asset_id = Uuid::new_v4();

    let node = CADSNode::new_local(system_id, cads_asset_id, CADSKind::AIModel);

    assert_eq!(node.system_id, system_id);
    assert_eq!(node.cads_asset_id, Some(cads_asset_id));
    assert_eq!(node.kind, CADSKind::AIModel);
    assert!(node.shared_reference.is_none());
}

#[test]
fn test_cads_node_shared() {
    let system_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let node_id = Uuid::new_v4();

    let shared_ref = SharedNodeReference {
        domain_id,
        node_id,
        node_version: "1.0.0".to_string(),
    };

    let node = CADSNode::new_shared(system_id, CADSKind::MLPipeline, shared_ref.clone());

    assert_eq!(node.system_id, system_id);
    assert_eq!(node.cads_asset_id, None);
    assert_eq!(node.kind, CADSKind::MLPipeline);
    assert_eq!(node.shared_reference, Some(shared_ref));
}

#[test]
fn test_odcs_node_local() {
    let system_id = Uuid::new_v4();
    let table_id = Uuid::new_v4();

    let node = ODCSNode::new_local(system_id, table_id, "source".to_string());

    assert_eq!(node.system_id, system_id);
    assert_eq!(node.table_id, Some(table_id));
    assert_eq!(node.role, "source");
    assert!(node.shared_reference.is_none());
}

#[test]
fn test_odcs_node_shared() {
    let system_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let node_id = Uuid::new_v4();

    let shared_ref = SharedNodeReference {
        domain_id,
        node_id,
        node_version: "2.1.0".to_string(),
    };

    let node = ODCSNode::new_shared(system_id, "destination".to_string(), shared_ref.clone());

    assert_eq!(node.system_id, system_id);
    assert_eq!(node.table_id, None);
    assert_eq!(node.role, "destination");
    assert_eq!(node.shared_reference, Some(shared_ref));
}

#[test]
fn test_domain_add_operations() {
    let mut domain = Domain::new("test-domain".to_string());
    let domain_id = domain.id;

    // Add system
    let system = System::new(
        "test-system".to_string(),
        InfrastructureType::Kafka,
        domain_id,
    );
    domain.add_system(system);
    assert_eq!(domain.systems.len(), 1);
    assert_eq!(domain.systems[0].domain_id, domain_id);

    // Add CADS node
    let system_id = domain.systems[0].id;
    let cads_asset_id = Uuid::new_v4();
    let cads_node = CADSNode::new_local(system_id, cads_asset_id, CADSKind::Application);
    domain.add_cads_node(cads_node);
    assert_eq!(domain.cads_nodes.len(), 1);

    // Add ODCS node
    let table_id = Uuid::new_v4();
    let odcs_node = ODCSNode::new_local(system_id, table_id, "source".to_string());
    domain.add_odcs_node(odcs_node);
    assert_eq!(domain.odcs_nodes.len(), 1);

    // Add system connection
    let target_system_id = Uuid::new_v4();
    let connection = SystemConnection {
        id: Uuid::new_v4(),
        source_system_id: system_id,
        target_system_id,
        connection_type: "api_call".to_string(),
        bidirectional: false,
        metadata: std::collections::HashMap::new(),
        created_at: None,
        updated_at: None,
    };
    domain.add_system_connection(connection);
    assert_eq!(domain.system_connections.len(), 1);

    // Add node connection
    let source_node_id = domain.odcs_nodes[0].id;
    let target_node_id = Uuid::new_v4();
    let node_connection = NodeConnection {
        id: Uuid::new_v4(),
        source_node_id,
        target_node_id,
        cardinality: CrowsfeetCardinality::OneToMany,
        relationship_type: "foreign_key".to_string(),
        metadata: std::collections::HashMap::new(),
        created_at: None,
        updated_at: None,
    };
    domain.add_node_connection(node_connection);
    assert_eq!(domain.node_connections.len(), 1);
}

#[test]
fn test_domain_yaml_serialization() {
    let mut domain = Domain::new("test-domain".to_string());
    let domain_id = domain.id;

    // Add a system
    let system = System::new(
        "test-system".to_string(),
        InfrastructureType::PostgreSQL,
        domain_id,
    );
    domain.add_system(system);

    // Serialize to YAML
    let yaml = domain.to_yaml().unwrap();
    assert!(yaml.contains("name: test-domain"));
    assert!(yaml.contains("test-system"));

    // Deserialize from YAML
    let domain2 = Domain::from_yaml(&yaml).unwrap();
    assert_eq!(domain.id, domain2.id);
    assert_eq!(domain.name, domain2.name);
    assert_eq!(domain.systems.len(), domain2.systems.len());
}

#[test]
fn test_all_cardinality_types() {
    let cardinalities = vec![
        CrowsfeetCardinality::OneToOne,
        CrowsfeetCardinality::OneToMany,
        CrowsfeetCardinality::ZeroOrOne,
        CrowsfeetCardinality::ZeroOrMany,
    ];

    for cardinality in cardinalities {
        let connection = NodeConnection {
            id: Uuid::new_v4(),
            source_node_id: Uuid::new_v4(),
            target_node_id: Uuid::new_v4(),
            cardinality,
            relationship_type: "test".to_string(),
            metadata: std::collections::HashMap::new(),
            created_at: None,
            updated_at: None,
        };

        // Verify serialization
        let json = serde_json::to_string(&connection).unwrap();
        assert!(json.contains("cardinality"));

        // Verify deserialization
        let connection2: NodeConnection = serde_json::from_str(&json).unwrap();
        assert_eq!(connection.cardinality, connection2.cardinality);
    }
}

#[test]
fn test_shared_node_reference() {
    let domain_id = Uuid::new_v4();
    let node_id = Uuid::new_v4();

    let shared_ref = SharedNodeReference {
        domain_id,
        node_id,
        node_version: "1.2.3".to_string(),
    };

    // Verify serialization
    let json = serde_json::to_string(&shared_ref).unwrap();
    assert!(json.contains("domain_id"));
    assert!(json.contains("node_id"));
    assert!(json.contains("node_version"));
    assert!(json.contains("1.2.3"));

    // Verify deserialization
    let shared_ref2: SharedNodeReference = serde_json::from_str(&json).unwrap();
    assert_eq!(shared_ref.domain_id, shared_ref2.domain_id);
    assert_eq!(shared_ref.node_id, shared_ref2.node_id);
    assert_eq!(shared_ref.node_version, shared_ref2.node_version);
}

#[test]
fn test_local_metadata_overrides() {
    let system_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let node_id = Uuid::new_v4();

    let shared_ref = SharedNodeReference {
        domain_id,
        node_id,
        node_version: "1.0.0".to_string(),
    };

    let mut cads_node = CADSNode::new_shared(system_id, CADSKind::AIModel, shared_ref);

    // Add local metadata override
    let mut override1 = std::collections::HashMap::new();
    override1.insert("custom_field".to_string(), json!("custom_value"));
    override1.insert("environment".to_string(), json!("production"));
    cads_node.custom_metadata.push(override1);

    assert_eq!(cads_node.custom_metadata.len(), 1);
    assert_eq!(
        cads_node.custom_metadata[0].get("custom_field"),
        Some(&json!("custom_value"))
    );
}
