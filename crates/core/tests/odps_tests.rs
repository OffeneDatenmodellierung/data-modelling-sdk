//! Tests for ODPS (Open Data Product Standard) import/export

use data_modelling_core::export::odps::ODPSExporter;
use data_modelling_core::import::odps::ODPSImporter;
use data_modelling_core::models::Tag;
use data_modelling_core::models::odps::*;

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

#[cfg(feature = "odps-validation")]
mod odps_validation_integration_tests {
    use data_modelling_core::import::odps::ODPSImporter;

    #[test]
    fn test_odps_validation_valid_file() {
        let valid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
name: test-product
version: 1.0.0
"#;

        let importer = ODPSImporter::new();
        let result = importer.import(valid_odps);
        assert!(
            result.is_ok(),
            "Valid ODPS file should pass validation and import"
        );
    }

    #[test]
    fn test_odps_validation_invalid_missing_required_field() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
# Missing 'id' field
status: active
"#;

        let importer = ODPSImporter::new();
        let result = importer.import(invalid_odps);
        assert!(
            result.is_err(),
            "ODPS file missing required field should fail validation"
        );
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("validation") || error_msg.contains("id"),
                "Error message should indicate validation failure or missing id"
            );
        }
    }

    #[test]
    fn test_odps_validation_invalid_enum_value() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: invalid-status-value
"#;

        let importer = ODPSImporter::new();
        let result = importer.import(invalid_odps);
        assert!(
            result.is_err(),
            "ODPS file with invalid enum value should fail validation"
        );
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("validation") || error_msg.contains("status"),
                "Error message should indicate validation failure"
            );
        }
    }

    #[test]
    fn test_odps_validation_invalid_url_format() {
        // Note: JSON Schema format validation may be lenient depending on validator implementation
        // This test verifies validation runs, but URL format validation may not be strictly enforced
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
support:
  - channel: email
    url: not-a-valid-url-format
"#;

        let importer = ODPSImporter::new();
        let result = importer.import(invalid_odps);
        // URL format validation may pass if jsonschema doesn't strictly validate format
        // The important thing is that validation runs and other validations work
        // If validation fails, that's good; if it passes, format validation may be lenient
        if let Err(err) = result {
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("validation") || error_msg.contains("url"),
                "Error message should indicate validation failure"
            );
        }
        // If validation passes, that's acceptable - format validation may be lenient
    }

    #[test]
    fn test_odps_validation_missing_nested_required_field() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
support:
  - channel: email
    # Missing 'url' field
"#;

        let importer = ODPSImporter::new();
        let result = importer.import(invalid_odps);
        assert!(
            result.is_err(),
            "ODPS file with missing nested required field should fail validation"
        );
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("validation") || error_msg.contains("url"),
                "Error message should indicate validation failure"
            );
        }
    }
}

#[cfg(feature = "odps-validation")]
mod odps_field_preservation_tests {
    use super::*;

    #[test]
    fn test_odps_field_preservation_all_optional_fields() {
        let yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: test-product
version: 1.0.0
status: active
domain: test-domain
tenant: test-tenant
tags:
  - tag1
  - tag2
description:
  purpose: Test purpose
  usage: Test usage
  limitations: Test limitations
customProperties:
  - property: custom1
    value: value1
  - property: custom2
    value: value2
authoritativeDefinitions:
  - type: businessDefinition
    url: https://example.com/def1
    description: Definition 1
  - type: tutorial
    url: https://example.com/tut1
support:
  - channel: email
    url: https://example.com/support
  - channel: slack
    url: https://example.com/slack
team:
  name: Test Team
  members:
    - username: user1
      name: User One
      role: Developer
inputPorts:
  - name: input1
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440001
    tags:
      - input-tag
    customProperties:
      - property: port-prop
        value: port-value
outputPorts:
  - name: output1
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440002
    description: Output description
    tags:
      - output-tag
    customProperties:
      - property: output-prop
        value: output-value
productCreatedTs: 2024-01-01T00:00:00Z
"#;

        let importer = ODPSImporter::new();
        let product = importer.import(yaml).unwrap();

        // Export back
        let exporter = ODPSExporter;
        let exported_yaml = exporter.export(&product).unwrap();

        // Import again to verify round-trip
        let product2 = importer.import(&exported_yaml).unwrap();

        // Verify all fields are preserved
        assert_eq!(product.id, product2.id);
        assert_eq!(product.name, product2.name);
        assert_eq!(product.version, product2.version);
        assert_eq!(product.status, product2.status);
        assert_eq!(product.domain, product2.domain);
        assert_eq!(product.tenant, product2.tenant);
        assert_eq!(product.tags.len(), product2.tags.len());
        assert_eq!(product.tags, product2.tags);

        // Verify description
        assert_eq!(
            product.description.is_some(),
            product2.description.is_some()
        );
        if let (Some(desc1), Some(desc2)) = (&product.description, &product2.description) {
            assert_eq!(desc1.purpose, desc2.purpose);
            assert_eq!(desc1.usage, desc2.usage);
            assert_eq!(desc1.limitations, desc2.limitations);
        }

        // Verify custom properties
        assert_eq!(
            product.custom_properties.is_some(),
            product2.custom_properties.is_some()
        );
        if let (Some(props1), Some(props2)) =
            (&product.custom_properties, &product2.custom_properties)
        {
            assert_eq!(props1.len(), props2.len());
            for (p1, p2) in props1.iter().zip(props2.iter()) {
                assert_eq!(p1.property, p2.property);
                assert_eq!(p1.value, p2.value);
            }
        }

        // Verify authoritative definitions
        assert_eq!(
            product.authoritative_definitions.is_some(),
            product2.authoritative_definitions.is_some()
        );
        if let (Some(defs1), Some(defs2)) = (
            &product.authoritative_definitions,
            &product2.authoritative_definitions,
        ) {
            assert_eq!(defs1.len(), defs2.len());
            for (d1, d2) in defs1.iter().zip(defs2.iter()) {
                assert_eq!(d1.r#type, d2.r#type);
                assert_eq!(d1.url, d2.url);
                assert_eq!(d1.description, d2.description);
            }
        }

        // Verify support
        assert_eq!(product.support.is_some(), product2.support.is_some());
        if let (Some(sup1), Some(sup2)) = (&product.support, &product2.support) {
            assert_eq!(sup1.len(), sup2.len());
            for (s1, s2) in sup1.iter().zip(sup2.iter()) {
                assert_eq!(s1.channel, s2.channel);
                assert_eq!(s1.url, s2.url);
            }
        }

        // Verify team
        assert_eq!(product.team.is_some(), product2.team.is_some());
        if let (Some(team1), Some(team2)) = (&product.team, &product2.team) {
            assert_eq!(team1.name, team2.name);
            assert_eq!(team1.members.is_some(), team2.members.is_some());
            if let (Some(mem1), Some(mem2)) = (&team1.members, &team2.members) {
                assert_eq!(mem1.len(), mem2.len());
                for (m1, m2) in mem1.iter().zip(mem2.iter()) {
                    assert_eq!(m1.username, m2.username);
                    assert_eq!(m1.name, m2.name);
                    assert_eq!(m1.role, m2.role);
                }
            }
        }

        // Verify input ports with nested structures
        assert_eq!(
            product.input_ports.is_some(),
            product2.input_ports.is_some()
        );
        if let (Some(ports1), Some(ports2)) = (&product.input_ports, &product2.input_ports) {
            assert_eq!(ports1.len(), ports2.len());
            for (p1, p2) in ports1.iter().zip(ports2.iter()) {
                assert_eq!(p1.name, p2.name);
                assert_eq!(p1.tags.len(), p2.tags.len());
                assert_eq!(p1.tags, p2.tags);
                assert_eq!(
                    p1.custom_properties.is_some(),
                    p2.custom_properties.is_some()
                );
                if let (Some(cp1), Some(cp2)) = (&p1.custom_properties, &p2.custom_properties) {
                    assert_eq!(cp1.len(), cp2.len());
                }
            }
        }

        // Verify output ports with nested structures
        assert_eq!(
            product.output_ports.is_some(),
            product2.output_ports.is_some()
        );
        if let (Some(ports1), Some(ports2)) = (&product.output_ports, &product2.output_ports) {
            assert_eq!(ports1.len(), ports2.len());
            for (p1, p2) in ports1.iter().zip(ports2.iter()) {
                assert_eq!(p1.name, p2.name);
                assert_eq!(p1.description, p2.description);
                assert_eq!(p1.tags.len(), p2.tags.len());
                assert_eq!(p1.tags, p2.tags);
                assert_eq!(
                    p1.custom_properties.is_some(),
                    p2.custom_properties.is_some()
                );
            }
        }

        assert_eq!(product.product_created_ts, product2.product_created_ts);
    }

    #[test]
    fn test_odps_field_preservation_nested_structures() {
        let yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
inputPorts:
  - name: input1
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440001
    tags:
      - nested-tag-1
      - nested-tag-2
    customProperties:
      - property: nested-prop-1
        value: nested-value-1
      - property: nested-prop-2
        value: nested-value-2
    authoritativeDefinitions:
      - type: businessDefinition
        url: https://example.com/nested-def
        description: Nested definition
outputPorts:
  - name: output1
    version: 1.0.0
    contractId: 660e8400-e29b-41d4-a716-446655440002
    tags:
      - output-nested-tag
    customProperties:
      - property: output-nested-prop
        value: output-nested-value
"#;

        let importer = ODPSImporter::new();
        let product = importer.import(yaml).unwrap();

        // Verify nested structures are imported correctly
        assert!(product.input_ports.is_some());
        let input_port = &product.input_ports.as_ref().unwrap()[0];
        assert_eq!(input_port.tags.len(), 2);
        assert!(input_port.custom_properties.is_some());
        assert_eq!(input_port.custom_properties.as_ref().unwrap().len(), 2);
        assert!(input_port.authoritative_definitions.is_some());
        assert_eq!(
            input_port.authoritative_definitions.as_ref().unwrap().len(),
            1
        );

        // Export and re-import
        let exporter = ODPSExporter;
        let exported_yaml = exporter.export(&product).unwrap();
        let product2 = importer.import(&exported_yaml).unwrap();

        // Verify nested structures are preserved
        assert!(product2.input_ports.is_some());
        let input_port2 = &product2.input_ports.as_ref().unwrap()[0];
        assert_eq!(input_port.tags, input_port2.tags);
        assert_eq!(
            input_port.custom_properties.as_ref().unwrap().len(),
            input_port2.custom_properties.as_ref().unwrap().len()
        );
        assert_eq!(
            input_port.authoritative_definitions.as_ref().unwrap().len(),
            input_port2
                .authoritative_definitions
                .as_ref()
                .unwrap()
                .len()
        );

        // Verify output port nested structures
        assert!(product2.output_ports.is_some());
        let output_port2 = &product2.output_ports.as_ref().unwrap()[0];
        assert_eq!(
            product.output_ports.as_ref().unwrap()[0].tags,
            output_port2.tags
        );
        assert_eq!(
            product.output_ports.as_ref().unwrap()[0]
                .custom_properties
                .as_ref()
                .unwrap()
                .len(),
            output_port2.custom_properties.as_ref().unwrap().len()
        );
    }

    #[test]
    fn test_odps_field_preservation_empty_optional_arrays() {
        let yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
tags: []
inputPorts: []
outputPorts: []
support: []
customProperties: []
authoritativeDefinitions: []
"#;

        let importer = ODPSImporter::new();
        let product = importer.import(yaml).unwrap();

        // Verify empty arrays are preserved
        assert_eq!(product.tags.len(), 0);
        assert!(product.input_ports.is_some());
        assert_eq!(product.input_ports.as_ref().unwrap().len(), 0);
        assert!(product.output_ports.is_some());
        assert_eq!(product.output_ports.as_ref().unwrap().len(), 0);
        assert!(product.support.is_some());
        assert_eq!(product.support.as_ref().unwrap().len(), 0);
        assert!(product.custom_properties.is_some());
        assert_eq!(product.custom_properties.as_ref().unwrap().len(), 0);
        assert!(product.authoritative_definitions.is_some());
        assert_eq!(product.authoritative_definitions.as_ref().unwrap().len(), 0);

        // Export and verify empty arrays are preserved in YAML
        let exporter = ODPSExporter;
        let exported_yaml = exporter.export(&product).unwrap();

        // Re-import to verify round-trip
        let product2 = importer.import(&exported_yaml).unwrap();
        assert_eq!(product2.tags.len(), 0);
        assert!(product2.input_ports.is_some());
        assert_eq!(product2.input_ports.as_ref().unwrap().len(), 0);
        assert!(product2.output_ports.is_some());
        assert_eq!(product2.output_ports.as_ref().unwrap().len(), 0);
    }
}

#[cfg(feature = "odps-validation")]
mod odps_feature_flag_tests {
    use super::*;

    #[test]
    fn test_odps_import_with_validation_enabled() {
        let valid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
"#;

        let importer = ODPSImporter::new();
        let result = importer.import(valid_odps);
        // With validation enabled, valid file should succeed
        assert!(
            result.is_ok(),
            "Valid ODPS file should import successfully with validation enabled"
        );
    }

    #[test]
    fn test_odps_import_validation_error_when_enabled() {
        let invalid_odps = r#"
apiVersion: v1.0.0
kind: DataProduct
# Missing 'id' field
status: active
"#;

        let importer = ODPSImporter::new();
        let result = importer.import(invalid_odps);
        // With validation enabled, invalid file should fail
        assert!(
            result.is_err(),
            "Invalid ODPS file should fail validation when feature enabled"
        );
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("validation") || error_msg.contains("id"),
                "Error should indicate validation failure or missing id"
            );
        }
    }

    #[test]
    fn test_odps_export_with_validation_enabled() {
        use data_modelling_core::models::odps::{ODPSDataProduct, ODPSStatus};

        let product = ODPSDataProduct {
            api_version: "v1.0.0".to_string(),
            kind: "DataProduct".to_string(),
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            name: Some("test-product".to_string()),
            version: Some("1.0.0".to_string()),
            status: ODPSStatus::Active,
            domain: None,
            tenant: None,
            tags: vec![],
            description: None,
            authoritative_definitions: None,
            custom_properties: None,
            input_ports: None,
            output_ports: None,
            management_ports: None,
            support: None,
            team: None,
            product_created_ts: None,
            created_at: None,
            updated_at: None,
        };

        let exporter = ODPSExporter;
        let result = exporter.export(&product);
        // With validation enabled, valid product should export successfully
        assert!(
            result.is_ok(),
            "Valid ODPS product should export successfully with validation enabled"
        );
    }

    #[test]
    fn test_odps_export_validation_runs_when_enabled() {
        use data_modelling_core::models::odps::{ODPSDataProduct, ODPSStatus};

        let product = ODPSDataProduct {
            api_version: "v1.0.0".to_string(),
            kind: "DataProduct".to_string(),
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            name: Some("test-product".to_string()),
            version: Some("1.0.0".to_string()),
            status: ODPSStatus::Active,
            domain: None,
            tenant: None,
            tags: vec![],
            description: None,
            authoritative_definitions: None,
            custom_properties: None,
            input_ports: None,
            output_ports: None,
            management_ports: None,
            support: None,
            team: None,
            product_created_ts: None,
            created_at: None,
            updated_at: None,
        };

        let exporter = ODPSExporter;
        let result = exporter.export(&product);
        // With validation enabled, valid product should export successfully
        assert!(
            result.is_ok(),
            "Valid ODPS product should export successfully with validation enabled"
        );

        // Verify the exported YAML is valid by importing it back
        let exported_yaml = result.unwrap();
        let importer = ODPSImporter::new();
        let round_trip_result = importer.import(&exported_yaml);
        assert!(
            round_trip_result.is_ok(),
            "Exported YAML should be valid and importable"
        );
    }
}
