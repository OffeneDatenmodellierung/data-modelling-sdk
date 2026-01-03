//! Tests for ODPS (Open Data Product Standard) import/export

use data_modelling_sdk::export::odps::ODPSExporter;
use data_modelling_sdk::import::odps::ODPSImporter;
use data_modelling_sdk::models::Tag;
use data_modelling_sdk::models::odps::*;

#[test]
fn test_odps_import_basic() {
    let yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: customer-data-product
version: 1.0.0
status: active
domain: customer-service
tenant: acme-corp
tags:
  - customer
  - data-product
description:
  purpose: Customer data product for analytics
  usage: Use for customer analytics and reporting
  limitations: Data is updated daily
"#;

    let importer = ODPSImporter::new();
    let product = importer.import(yaml).unwrap();

    assert_eq!(product.api_version, "v1.0.0");
    assert_eq!(product.kind, "DataProduct");
    assert_eq!(product.id, "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(product.name, Some("customer-data-product".to_string()));
    assert_eq!(product.version, Some("1.0.0".to_string()));
    assert_eq!(product.status, ODPSStatus::Active);
    assert_eq!(product.domain, Some("customer-service".to_string()));
    assert_eq!(product.tenant, Some("acme-corp".to_string()));
    assert_eq!(product.tags.len(), 2);

    if let Some(description) = &product.description {
        assert_eq!(
            description.purpose,
            Some("Customer data product for analytics".to_string())
        );
        assert_eq!(
            description.usage,
            Some("Use for customer analytics and reporting".to_string())
        );
        assert_eq!(
            description.limitations,
            Some("Data is updated daily".to_string())
        );
    } else {
        panic!("Description should be present");
    }
}

#[test]
fn test_odps_import_with_ports() {
    let yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: customer-data-product
version: 1.0.0
status: active
inputPorts:
  - name: customer-input
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440001
outputPorts:
  - name: customer-output
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440002
    description: Processed customer data
"#;

    let importer = ODPSImporter::new();
    let product = importer.import(yaml).unwrap();

    assert!(product.input_ports.is_some());
    assert_eq!(product.input_ports.as_ref().unwrap().len(), 1);
    assert_eq!(
        product.input_ports.as_ref().unwrap()[0].name,
        "customer-input"
    );
    assert_eq!(
        product.input_ports.as_ref().unwrap()[0].contract_id,
        "660e8400-e29b-41d4-a716-446655440001"
    );

    assert!(product.output_ports.is_some());
    assert_eq!(product.output_ports.as_ref().unwrap().len(), 1);
    assert_eq!(
        product.output_ports.as_ref().unwrap()[0].name,
        "customer-output"
    );
    assert_eq!(
        product.output_ports.as_ref().unwrap()[0].contract_id,
        Some("660e8400-e29b-41d4-a716-446655440002".to_string())
    );
    assert_eq!(
        product.output_ports.as_ref().unwrap()[0].description,
        Some("Processed customer data".to_string())
    );
}

#[test]
fn test_odps_contractid_validation() {
    let yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: customer-data-product
version: 1.0.0
status: active
inputPorts:
  - name: customer-input
    version: 1.0.0
    contractId: unknown-contract-id
"#;

    // Without validation, should succeed
    let importer = ODPSImporter::new();
    let product = importer.import(yaml).unwrap();
    assert_eq!(
        product.input_ports.as_ref().unwrap()[0].contract_id,
        "unknown-contract-id"
    );

    // With validation, should fail
    let importer = ODPSImporter::with_table_ids(vec!["known-contract-id".to_string()]);
    let result = importer.import(yaml);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("unknown contractId")
    );
}

#[test]
fn test_odps_export_basic() {
    let product = ODPSDataProduct {
        api_version: "v1.0.0".to_string(),
        kind: "DataProduct".to_string(),
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        name: Some("customer-data-product".to_string()),
        version: Some("1.0.0".to_string()),
        status: ODPSStatus::Active,
        domain: Some("customer-service".to_string()),
        tenant: Some("acme-corp".to_string()),
        authoritative_definitions: None,
        description: Some(ODPSDescription {
            purpose: Some("Customer data product for analytics".to_string()),
            limitations: Some("Data is updated daily".to_string()),
            usage: Some("Use for customer analytics and reporting".to_string()),
            authoritative_definitions: None,
            custom_properties: None,
        }),
        custom_properties: None,
        tags: vec![
            Tag::Simple("customer".to_string()),
            Tag::Simple("data-product".to_string()),
        ],
        input_ports: None,
        output_ports: None,
        management_ports: None,
        support: None,
        team: None,
        product_created_ts: None,
        created_at: None,
        updated_at: None,
    };

    let yaml = ODPSExporter::export_product(&product);

    assert!(yaml.contains("apiVersion: v1.0.0"));
    assert!(yaml.contains("kind: DataProduct"));
    assert!(yaml.contains("id: 550e8400-e29b-41d4-a716-446655440000"));
    assert!(yaml.contains("name: customer-data-product"));
    assert!(yaml.contains("version: 1.0.0"));
    assert!(yaml.contains("status: active"));
    assert!(yaml.contains("domain: customer-service"));
    assert!(yaml.contains("tenant: acme-corp"));
    assert!(yaml.contains("tags:"));
    assert!(yaml.contains("description:"));
}

#[test]
fn test_odps_round_trip() {
    let original_yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: customer-data-product
version: 1.0.0
status: active
domain: customer-service
inputPorts:
  - name: customer-input
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440001
    tags:
      - input
      - customer
outputPorts:
  - name: customer-output
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440002
    description: Processed customer data
    type: batch
managementPorts:
  - name: api
    content: discoverability
    type: rest
    url: https://api.example.com/products/customer
support:
  - channel: slack
    url: https://slack.example.com/channels/customer-support
    tool: slack
    scope: interactive
team:
  name: Customer Data Team
  description: Team responsible for customer data products
  members:
    - username: john.doe@example.com
      name: John Doe
      role: Data Product Owner
"#;

    // Import
    let importer = ODPSImporter::new();
    let product = importer.import(original_yaml).unwrap();

    // Verify import
    assert_eq!(product.name, Some("customer-data-product".to_string()));
    assert!(product.input_ports.is_some());
    assert!(product.output_ports.is_some());
    assert!(product.management_ports.is_some());
    assert!(product.support.is_some());
    assert!(product.team.is_some());

    // Export
    let exported_yaml = ODPSExporter::export_product(&product);

    // Import again
    let product2 = importer.import(&exported_yaml).unwrap();

    // Verify round-trip
    assert_eq!(product.id, product2.id);
    assert_eq!(product.name, product2.name);
    assert_eq!(product.version, product2.version);
    assert_eq!(product.status, product2.status);
    assert_eq!(product.domain, product2.domain);
    assert_eq!(
        product.input_ports.as_ref().map(|p| p.len()),
        product2.input_ports.as_ref().map(|p| p.len())
    );
    assert_eq!(
        product.output_ports.as_ref().map(|p| p.len()),
        product2.output_ports.as_ref().map(|p| p.len())
    );
}

#[test]
fn test_odps_enhanced_tags() {
    let yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: test-product
version: 1.0.0
status: active
tags:
  - simple-tag
  - Environment:Production
  - SecondaryDomains:[finance, operations]
inputPorts:
  - name: input
    version: 1.0.0
    contractId: test-contract-id
    tags:
      - input-tag
      - Type:DataContract
"#;

    let importer = ODPSImporter::new();
    let product = importer.import(yaml).unwrap();

    assert_eq!(product.tags.len(), 3);
    assert_eq!(product.tags[0], Tag::Simple("simple-tag".to_string()));
    assert_eq!(
        product.tags[1],
        Tag::Pair("Environment".to_string(), "Production".to_string())
    );
    assert_eq!(
        product.tags[2],
        Tag::List(
            "SecondaryDomains".to_string(),
            vec!["finance".to_string(), "operations".to_string()]
        )
    );

    // Check input port tags
    if let Some(input_ports) = &product.input_ports {
        assert_eq!(input_ports[0].tags.len(), 2);
        assert_eq!(input_ports[0].tags[0], Tag::Simple("input-tag".to_string()));
        assert_eq!(
            input_ports[0].tags[1],
            Tag::Pair("Type".to_string(), "DataContract".to_string())
        );
    }
}

#[test]
fn test_odps_all_statuses() {
    let statuses = vec![
        ("proposed", ODPSStatus::Proposed),
        ("draft", ODPSStatus::Draft),
        ("active", ODPSStatus::Active),
        ("deprecated", ODPSStatus::Deprecated),
        ("retired", ODPSStatus::Retired),
    ];

    for (status_str, status_enum) in statuses {
        let yaml = format!(
            r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: test-product
version: 1.0.0
status: {}
"#,
            status_str
        );

        let importer = ODPSImporter::new();
        let product = importer.import(&yaml).unwrap();
        assert_eq!(product.status, status_enum);
    }
}
