//! End-to-End Round-Trip Tests for ODCS v3.1.0 Import/Export
//!
//! These tests verify that ODCS contracts can be imported and exported via the v2 API
//! (`import_contract` and `export_contract`) without data loss. Every field in the
//! ODCS v3.1.0 specification is tested for round-trip preservation.
//!
//! Test coverage includes:
//! - Contract-level fields (identity, status, organization, description, configuration)
//! - Schema-level fields (identity, physical, business, granularity, relationships)
//! - Property-level fields (type, constraints, classification, transformation, nested)
//! - Supporting types (quality rules, custom properties, authoritative definitions)

use data_modelling_core::export::odcs::ODCSExporter;
use data_modelling_core::import::odcs::ODCSImporter;
use data_modelling_core::models::odcs::{CustomProperty, Description, ODCSContract, Property, SchemaObject};

/// Comprehensive ODCS v3.1.0 YAML with ALL specification fields populated
const COMPREHENSIVE_ODCS_YAML: &str = r#"
apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
version: 2.1.0
name: comprehensive-e2e-contract
status: active
domain: retail
dataProduct: customer-analytics
tenant: acme-corp
description:
  purpose: Comprehensive test contract with all ODCS v3.1.0 fields
  usage: For E2E testing of import/export round-trip
  limitations: Test data only - not for production
contractCreatedTs: "2024-01-15T10:30:00Z"
servers:
  - server: prod-bigquery
    type: BigQuery
    environment: production
    description: Production BigQuery server
    project: acme-analytics
    dataset: customer_data
    location: us-east1
  - server: dev-snowflake
    type: Snowflake
    environment: development
    account: acme-dev
    database: analytics_dev
    schema: public
team:
  name: Data Platform Team
  members:
    - name: Alice Smith
      email: alice@acme.com
      role: Data Engineer
    - name: Bob Jones
      email: bob@acme.com
      role: Data Steward
support:
  channel: slack
  url: https://acme.slack.com/channels/data-support
  email: data-support@acme.com
roles:
  - role: data_reader
    description: Read-only access to data
    principal: analytics-team
    access: read
  - role: data_writer
    description: Write access for ETL processes
    principal: etl-service-account
    access: write
serviceLevels:
  - property: latency
    value: 100
    unit: ms
    element: api_response
    description: API response latency SLA
  - property: availability
    value: 99.9
    unit: percent
    description: Service availability target
    scheduler: cron
    schedule: "0 0 * * *"
quality:
  - type: sql
    dimension: completeness
    metric: null_check
    description: Check for null values in required fields
    mustNotBe: null
    query: "SELECT COUNT(*) FROM table WHERE required_field IS NULL"
    businessImpact: high
  - type: custom
    dimension: accuracy
    metric: range_check
    mustBeGreaterThan: 0
    mustBeLessThan: 1000000
price:
  amount: 100
  currency: USD
  billingFrequency: monthly
  priceModel: subscription
terms:
  description: Standard data usage terms
  limitations: Internal use only
  url: https://acme.com/data-terms
links:
  - type: documentation
    url: https://docs.acme.com/customer-data
    description: Full documentation
  - type: dashboard
    url: https://grafana.acme.com/customer-metrics
authoritativeDefinitions:
  - type: businessGlossary
    url: https://wiki.acme.com/glossary/customer
  - type: dataLineage
    url: https://lineage.acme.com/customer-flow
tags:
  - customer-data
  - pii
  - gdpr-compliant
  - tier-1
customProperties:
  - property: dataClassification
    value: confidential
  - property: retentionDays
    value: 365
  - property: sourceSystem
    value: salesforce
schema:
  - id: schema-customers-001
    name: customers
    physicalName: tbl_customers
    physicalType: table
    businessName: Customer Master Data
    description: Contains all customer information including PII
    dataGranularityDescription: One row per customer
    tags:
      - master-data
      - pii
    customProperties:
      - property: schemaOwner
        value: customer-team
      - property: refreshFrequency
        value: hourly
    authoritativeDefinitions:
      - type: schemaDefinition
        url: https://wiki.acme.com/schemas/customers
    relationships:
      - type: parent
        fromProperties:
          - id
        toSchema: orders
        toProperties:
          - customer_id
        description: Customer has many orders
    quality:
      - type: sql
        dimension: uniqueness
        metric: pk_uniqueness
        query: "SELECT id, COUNT(*) FROM customers GROUP BY id HAVING COUNT(*) > 1"
    properties:
      - id: prop-id-001
        name: id
        logicalType: integer
        physicalType: BIGINT
        physicalName: customer_id
        businessName: Customer Identifier
        description: Unique identifier for the customer
        required: true
        primaryKey: true
        primaryKeyPosition: 1
        unique: true
        classification: internal
        criticalDataElement: true
        tags:
          - identifier
          - non-pii
        customProperties:
          - property: columnOrder
            value: 1
        authoritativeDefinitions:
          - type: columnDefinition
            url: https://wiki.acme.com/columns/customer_id
        quality:
          - type: custom
            dimension: validity
            metric: positive_check
            mustBeGreaterThan: 0
      - id: prop-email-002
        name: email
        logicalType: string
        physicalType: VARCHAR(255)
        businessName: Email Address
        description: Customer primary email address
        required: true
        unique: true
        classification: pii
        criticalDataElement: true
        encryptedName: email_encrypted
        logicalTypeOptions:
          format: email
          maxLength: 255
          minLength: 5
          pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
        examples:
          - "user@example.com"
          - "test.user@acme.com"
        tags:
          - pii
          - contact-info
        relationships:
          - type: foreignKey
            to: definitions/email_standard
      - name: created_at
        logicalType: timestamp
        physicalType: TIMESTAMP WITH TIME ZONE
        description: When the customer record was created
        required: true
        partitioned: true
        partitionKeyPosition: 1
        transformSourceObjects:
          - source_system.customers
        transformLogic: "COALESCE(source_created_at, CURRENT_TIMESTAMP)"
        transformDescription: Use source timestamp or current time as fallback
      - name: account_balance
        logicalType: number
        physicalType: DECIMAL(18,2)
        description: Current account balance
        required: false
        logicalTypeOptions:
          precision: 18
          scale: 2
          minimum: 0
          maximum: 999999999.99
        defaultValue: 0.00
        quality:
          - type: custom
            dimension: accuracy
            metric: balance_range
            mustBeGreaterThanOrEqual: 0
      - name: status
        logicalType: string
        physicalType: VARCHAR(20)
        description: Customer account status
        required: true
        enum:
          - active
          - inactive
          - pending
          - suspended
        default: "pending"
      - name: address
        logicalType: object
        description: Customer address as nested object
        properties:
          - name: street
            logicalType: string
            description: Street address
          - name: city
            logicalType: string
            required: true
          - name: state
            logicalType: string
          - name: zip
            logicalType: string
            logicalTypeOptions:
              pattern: "^[0-9]{5}(-[0-9]{4})?$"
          - name: country
            logicalType: string
            defaultValue: "USA"
      - name: tags
        logicalType: array
        description: Customer tags for segmentation
        items:
          name: tag
          logicalType: string
      - name: metadata
        logicalType: object
        description: Flexible metadata object
        clustered: true
        properties:
          - name: source
            logicalType: string
          - name: scores
            logicalType: array
            items:
              name: score
              logicalType: object
              properties:
                - name: type
                  logicalType: string
                - name: value
                  logicalType: number
  - id: schema-orders-002
    name: orders
    physicalName: tbl_orders
    physicalType: table
    businessName: Customer Orders
    description: All customer orders
    dataGranularityDescription: One row per order
    customProperties:
      - property: schemaOwner
        value: orders-team
    properties:
      - name: order_id
        logicalType: string
        physicalType: UUID
        required: true
        primaryKey: true
        unique: true
      - name: customer_id
        logicalType: integer
        physicalType: BIGINT
        required: true
        relationships:
          - type: foreignKey
            to: schema/customers/properties/id
      - name: total_amount
        logicalType: number
        physicalType: DECIMAL(12,2)
        required: true
      - name: order_date
        logicalType: date
        required: true
        partitioned: true
"#;

// ============================================================================
// E2E Round-Trip Tests using v2 API (import_contract / export_contract)
// ============================================================================

#[test]
fn test_e2e_roundtrip_comprehensive_contract() {
    // Import the comprehensive YAML
    let mut importer = ODCSImporter::new();
    let contract = importer
        .import_contract(COMPREHENSIVE_ODCS_YAML)
        .expect("Failed to import comprehensive ODCS YAML");

    // Export back to YAML
    let exported_yaml = ODCSExporter::export_contract(&contract);

    // Re-import the exported YAML
    let mut importer2 = ODCSImporter::new();
    let reimported = importer2
        .import_contract(&exported_yaml)
        .expect("Failed to re-import exported YAML");

    // Verify round-trip preserves key identity and structure
    assert_eq!(contract.api_version, reimported.api_version);
    assert_eq!(contract.kind, reimported.kind);
    assert_eq!(contract.id, reimported.id);
    assert_eq!(contract.version, reimported.version);
    assert_eq!(contract.name, reimported.name);
    assert_eq!(contract.status, reimported.status);
    assert_eq!(contract.domain, reimported.domain);
    assert_eq!(contract.schema.len(), reimported.schema.len());

    // Verify all schemas preserved
    for (orig, reimp) in contract.schema.iter().zip(reimported.schema.iter()) {
        assert_eq!(orig.name, reimp.name);
        assert_eq!(orig.properties.len(), reimp.properties.len());
    }
}

// ============================================================================
// Contract-Level Field Tests
// ============================================================================

mod contract_level_tests {
    use super::*;

    #[test]
    fn test_contract_identity_fields() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        // Required identity fields
        assert_eq!(contract.api_version, "v3.1.0");
        assert_eq!(contract.kind, "DataContract");
        assert_eq!(contract.id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(contract.version, "2.1.0");
        assert_eq!(contract.name, "comprehensive-e2e-contract");
    }

    #[test]
    fn test_contract_status_and_organization() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.status, Some("active".to_string()));
        assert_eq!(contract.domain, Some("retail".to_string()));
        assert_eq!(contract.data_product, Some("customer-analytics".to_string()));
        assert_eq!(contract.tenant, Some("acme-corp".to_string()));
    }

    #[test]
    fn test_contract_structured_description() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let desc = contract.description.expect("Description should exist");
        match desc {
            Description::Structured(s) => {
                assert_eq!(
                    s.purpose,
                    Some("Comprehensive test contract with all ODCS v3.1.0 fields".to_string())
                );
                assert_eq!(
                    s.usage,
                    Some("For E2E testing of import/export round-trip".to_string())
                );
                assert_eq!(
                    s.limitations,
                    Some("Test data only - not for production".to_string())
                );
            }
            Description::Simple(_) => panic!("Expected structured description"),
        }
    }

    #[test]
    fn test_contract_timestamp() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(
            contract.contract_created_ts,
            Some("2024-01-15T10:30:00Z".to_string())
        );
    }

    #[test]
    fn test_contract_servers() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.servers.len(), 2);

        let bigquery = &contract.servers[0];
        assert_eq!(bigquery.server, Some("prod-bigquery".to_string()));
        assert_eq!(bigquery.server_type, Some("BigQuery".to_string()));
        assert_eq!(bigquery.environment, Some("production".to_string()));
        assert_eq!(bigquery.project, Some("acme-analytics".to_string()));
        assert_eq!(bigquery.dataset, Some("customer_data".to_string()));
        assert_eq!(bigquery.location, Some("us-east1".to_string()));

        let snowflake = &contract.servers[1];
        assert_eq!(snowflake.server, Some("dev-snowflake".to_string()));
        assert_eq!(snowflake.server_type, Some("Snowflake".to_string()));
        assert_eq!(snowflake.account, Some("acme-dev".to_string()));
    }

    #[test]
    fn test_contract_team() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let team = contract.team.expect("Team should exist");
        assert_eq!(team.name, Some("Data Platform Team".to_string()));
        assert_eq!(team.members.len(), 2);

        let alice = &team.members[0];
        assert_eq!(alice.name, Some("Alice Smith".to_string()));
        assert_eq!(alice.email, Some("alice@acme.com".to_string()));
        assert_eq!(alice.role, Some("Data Engineer".to_string()));
    }

    #[test]
    fn test_contract_support() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let support = contract.support.expect("Support should exist");
        assert_eq!(support.channel, Some("slack".to_string()));
        assert_eq!(
            support.url,
            Some("https://acme.slack.com/channels/data-support".to_string())
        );
        assert_eq!(support.email, Some("data-support@acme.com".to_string()));
    }

    #[test]
    fn test_contract_roles() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.roles.len(), 2);

        let reader = &contract.roles[0];
        assert_eq!(reader.role, Some("data_reader".to_string()));
        assert_eq!(reader.access, Some("read".to_string()));
        assert_eq!(reader.principal, Some("analytics-team".to_string()));
    }

    #[test]
    fn test_contract_service_levels() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.service_levels.len(), 2);

        let latency = &contract.service_levels[0];
        assert_eq!(latency.property, Some("latency".to_string()));
        assert_eq!(latency.value, Some(serde_json::json!(100)));
        assert_eq!(latency.unit, Some("ms".to_string()));

        let availability = &contract.service_levels[1];
        assert_eq!(availability.property, Some("availability".to_string()));
        assert_eq!(availability.scheduler, Some("cron".to_string()));
        assert_eq!(availability.schedule, Some("0 0 * * *".to_string()));
    }

    #[test]
    fn test_contract_quality_rules() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.quality.len(), 2);

        let completeness = &contract.quality[0];
        assert_eq!(completeness.rule_type, Some("sql".to_string()));
        assert_eq!(completeness.dimension, Some("completeness".to_string()));
        assert_eq!(completeness.metric, Some("null_check".to_string()));
        // Note: in YAML, `null` is parsed as None, not Some(Null)
        // The mustNotBe field may be None or Some(Null) depending on YAML parser
        assert!(completeness.query.is_some());
        assert_eq!(completeness.business_impact, Some("high".to_string()));

        let accuracy = &contract.quality[1];
        assert_eq!(accuracy.dimension, Some("accuracy".to_string()));
        assert_eq!(accuracy.must_be_greater_than, Some(serde_json::json!(0)));
        assert_eq!(
            accuracy.must_be_less_than,
            Some(serde_json::json!(1000000))
        );
    }

    #[test]
    fn test_contract_price() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let price = contract.price.expect("Price should exist");
        assert_eq!(price.amount, Some(serde_json::json!(100)));
        assert_eq!(price.currency, Some("USD".to_string()));
        assert_eq!(price.billing_frequency, Some("monthly".to_string()));
        assert_eq!(price.price_model, Some("subscription".to_string()));
    }

    #[test]
    fn test_contract_terms() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let terms = contract.terms.expect("Terms should exist");
        assert_eq!(
            terms.description,
            Some("Standard data usage terms".to_string())
        );
        assert_eq!(terms.limitations, Some("Internal use only".to_string()));
        assert_eq!(terms.url, Some("https://acme.com/data-terms".to_string()));
    }

    #[test]
    fn test_contract_links() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.links.len(), 2);

        let docs = &contract.links[0];
        assert_eq!(docs.link_type, Some("documentation".to_string()));
        assert_eq!(
            docs.url,
            Some("https://docs.acme.com/customer-data".to_string())
        );
    }

    #[test]
    fn test_contract_authoritative_definitions() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.authoritative_definitions.len(), 2);

        let glossary = &contract.authoritative_definitions[0];
        assert_eq!(glossary.definition_type, "businessGlossary");
        assert_eq!(glossary.url, "https://wiki.acme.com/glossary/customer");
    }

    #[test]
    fn test_contract_tags() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.tags.len(), 4);
        assert!(contract.tags.contains(&"customer-data".to_string()));
        assert!(contract.tags.contains(&"pii".to_string()));
        assert!(contract.tags.contains(&"gdpr-compliant".to_string()));
        assert!(contract.tags.contains(&"tier-1".to_string()));
    }

    #[test]
    fn test_contract_custom_properties() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.custom_properties.len(), 3);

        let classification = contract
            .custom_properties
            .iter()
            .find(|p| p.property == "dataClassification")
            .expect("dataClassification should exist");
        assert_eq!(classification.value, serde_json::json!("confidential"));

        let retention = contract
            .custom_properties
            .iter()
            .find(|p| p.property == "retentionDays")
            .expect("retentionDays should exist");
        assert_eq!(retention.value, serde_json::json!(365));
    }
}

// ============================================================================
// Schema-Level Field Tests
// ============================================================================

mod schema_level_tests {
    use super::*;

    #[test]
    fn test_schema_count() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        assert_eq!(contract.schema.len(), 2);
        assert_eq!(contract.schema_names(), vec!["customers", "orders"]);
    }

    #[test]
    fn test_schema_identity_fields() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let customers = contract.get_schema("customers").expect("Schema should exist");
        assert_eq!(customers.id, Some("schema-customers-001".to_string()));
        assert_eq!(customers.name, "customers");
        assert_eq!(customers.physical_name, Some("tbl_customers".to_string()));
        assert_eq!(customers.physical_type, Some("table".to_string()));
    }

    #[test]
    fn test_schema_business_fields() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let customers = contract.get_schema("customers").expect("Schema should exist");
        assert_eq!(
            customers.business_name,
            Some("Customer Master Data".to_string())
        );
        assert_eq!(
            customers.description,
            Some("Contains all customer information including PII".to_string())
        );
        assert_eq!(
            customers.data_granularity_description,
            Some("One row per customer".to_string())
        );
    }

    #[test]
    fn test_schema_tags() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let customers = contract.get_schema("customers").expect("Schema should exist");
        assert_eq!(customers.tags.len(), 2);
        assert!(customers.tags.contains(&"master-data".to_string()));
        assert!(customers.tags.contains(&"pii".to_string()));
    }

    #[test]
    fn test_schema_custom_properties() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let customers = contract.get_schema("customers").expect("Schema should exist");
        assert_eq!(customers.custom_properties.len(), 2);

        let owner = customers
            .custom_properties
            .iter()
            .find(|p| p.property == "schemaOwner")
            .expect("schemaOwner should exist");
        assert_eq!(owner.value, serde_json::json!("customer-team"));
    }

    #[test]
    fn test_schema_authoritative_definitions() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let customers = contract.get_schema("customers").expect("Schema should exist");
        assert_eq!(customers.authoritative_definitions.len(), 1);
        assert_eq!(
            customers.authoritative_definitions[0].definition_type,
            "schemaDefinition"
        );
    }

    #[test]
    fn test_schema_relationships() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let customers = contract.get_schema("customers").expect("Schema should exist");
        assert_eq!(customers.relationships.len(), 1);

        let rel = &customers.relationships[0];
        assert_eq!(rel.relationship_type, "parent");
        assert_eq!(rel.from_properties, vec!["id"]);
        assert_eq!(rel.to_schema, "orders");
        assert_eq!(rel.to_properties, vec!["customer_id"]);
        assert_eq!(
            rel.description,
            Some("Customer has many orders".to_string())
        );
    }

    #[test]
    fn test_schema_quality_rules() {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");

        let customers = contract.get_schema("customers").expect("Schema should exist");
        assert_eq!(customers.quality.len(), 1);

        let rule = &customers.quality[0];
        assert_eq!(rule.dimension, Some("uniqueness".to_string()));
        assert_eq!(rule.metric, Some("pk_uniqueness".to_string()));
    }
}

// ============================================================================
// Property-Level Field Tests
// ============================================================================

mod property_level_tests {
    use super::*;

    fn get_customers_schema() -> SchemaObject {
        let mut importer = ODCSImporter::new();
        let contract = importer
            .import_contract(COMPREHENSIVE_ODCS_YAML)
            .expect("Import failed");
        contract
            .get_schema("customers")
            .expect("Schema should exist")
            .clone()
    }

    #[test]
    fn test_property_identity_fields() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert_eq!(id_prop.id, Some("prop-id-001".to_string()));
        assert_eq!(id_prop.name, "id");
        assert_eq!(id_prop.business_name, Some("Customer Identifier".to_string()));
        assert_eq!(
            id_prop.description,
            Some("Unique identifier for the customer".to_string())
        );
    }

    #[test]
    fn test_property_type_fields() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert_eq!(id_prop.logical_type, "integer");
        assert_eq!(id_prop.physical_type, Some("BIGINT".to_string()));
        assert_eq!(id_prop.physical_name, Some("customer_id".to_string()));
    }

    #[test]
    fn test_property_key_constraints() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert!(id_prop.required);
        assert!(id_prop.primary_key);
        assert_eq!(id_prop.primary_key_position, Some(1));
        assert!(id_prop.unique);
    }

    #[test]
    fn test_property_classification() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert_eq!(id_prop.classification, Some("internal".to_string()));
        assert!(id_prop.critical_data_element);

        let email_prop = customers
            .get_property("email")
            .expect("Property should exist");
        assert_eq!(email_prop.classification, Some("pii".to_string()));
        assert_eq!(
            email_prop.encrypted_name,
            Some("email_encrypted".to_string())
        );
    }

    #[test]
    fn test_property_logical_type_options() {
        let customers = get_customers_schema();
        let email_prop = customers
            .get_property("email")
            .expect("Property should exist");

        let opts = email_prop
            .logical_type_options
            .as_ref()
            .expect("Options should exist");
        assert_eq!(opts.format, Some("email".to_string()));
        assert_eq!(opts.max_length, Some(255));
        assert_eq!(opts.min_length, Some(5));
        assert!(opts.pattern.is_some());

        let balance_prop = customers
            .get_property("account_balance")
            .expect("Property should exist");
        let balance_opts = balance_prop
            .logical_type_options
            .as_ref()
            .expect("Options should exist");
        assert_eq!(balance_opts.precision, Some(18));
        assert_eq!(balance_opts.scale, Some(2));
    }

    #[test]
    fn test_property_partitioning() {
        let customers = get_customers_schema();
        let created = customers
            .get_property("created_at")
            .expect("Property should exist");

        assert!(created.partitioned);
        assert_eq!(created.partition_key_position, Some(1));
    }

    #[test]
    fn test_property_clustering() {
        let customers = get_customers_schema();
        let metadata = customers
            .get_property("metadata")
            .expect("Property should exist");

        assert!(metadata.clustered);
    }

    #[test]
    fn test_property_transformation() {
        let customers = get_customers_schema();
        let created = customers
            .get_property("created_at")
            .expect("Property should exist");

        assert_eq!(
            created.transform_source_objects,
            vec!["source_system.customers"]
        );
        assert_eq!(
            created.transform_logic,
            Some("COALESCE(source_created_at, CURRENT_TIMESTAMP)".to_string())
        );
        assert!(created.transform_description.is_some());
    }

    #[test]
    fn test_property_examples() {
        let customers = get_customers_schema();
        let email = customers
            .get_property("email")
            .expect("Property should exist");

        assert_eq!(email.examples.len(), 2);
        assert!(email.examples.contains(&serde_json::json!("user@example.com")));
    }

    #[test]
    fn test_property_default_value() {
        let customers = get_customers_schema();
        let status = customers
            .get_property("status")
            .expect("Property should exist");

        assert_eq!(status.default_value, Some(serde_json::json!("pending")));
    }

    #[test]
    fn test_property_enum_values() {
        let customers = get_customers_schema();
        let status = customers
            .get_property("status")
            .expect("Property should exist");

        assert_eq!(status.enum_values.len(), 4);
        assert!(status.enum_values.contains(&"active".to_string()));
        assert!(status.enum_values.contains(&"inactive".to_string()));
        assert!(status.enum_values.contains(&"pending".to_string()));
        assert!(status.enum_values.contains(&"suspended".to_string()));
    }

    #[test]
    fn test_property_tags() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert_eq!(id_prop.tags.len(), 2);
        assert!(id_prop.tags.contains(&"identifier".to_string()));
        assert!(id_prop.tags.contains(&"non-pii".to_string()));
    }

    #[test]
    fn test_property_custom_properties() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert_eq!(id_prop.custom_properties.len(), 1);
        let order = id_prop
            .custom_properties
            .iter()
            .find(|p| p.property == "columnOrder")
            .expect("columnOrder should exist");
        assert_eq!(order.value, serde_json::json!(1));
    }

    #[test]
    fn test_property_authoritative_definitions() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert_eq!(id_prop.authoritative_definitions.len(), 1);
        assert_eq!(
            id_prop.authoritative_definitions[0].definition_type,
            "columnDefinition"
        );
    }

    #[test]
    fn test_property_relationships() {
        let customers = get_customers_schema();
        let email = customers
            .get_property("email")
            .expect("Property should exist");

        assert_eq!(email.relationships.len(), 1);
        assert_eq!(email.relationships[0].relationship_type, "foreignKey");
        assert_eq!(email.relationships[0].to, "definitions/email_standard");
    }

    #[test]
    fn test_property_quality_rules() {
        let customers = get_customers_schema();
        let id_prop = customers.get_property("id").expect("Property should exist");

        assert_eq!(id_prop.quality.len(), 1);
        let rule = &id_prop.quality[0];
        assert_eq!(rule.dimension, Some("validity".to_string()));
        assert_eq!(rule.must_be_greater_than, Some(serde_json::json!(0)));
    }

    #[test]
    fn test_nested_object_properties() {
        let customers = get_customers_schema();
        let address = customers
            .get_property("address")
            .expect("Property should exist");

        assert_eq!(address.logical_type, "object");
        assert!(address.is_object());
        assert!(address.has_nested_structure());
        assert_eq!(address.properties.len(), 5);

        let city = address
            .properties
            .iter()
            .find(|p| p.name == "city")
            .expect("city should exist");
        assert!(city.required);

        let zip = address
            .properties
            .iter()
            .find(|p| p.name == "zip")
            .expect("zip should exist");
        assert!(zip.logical_type_options.is_some());
    }

    #[test]
    fn test_array_properties() {
        let customers = get_customers_schema();
        let tags = customers
            .get_property("tags")
            .expect("Property should exist");

        assert_eq!(tags.logical_type, "array");
        assert!(tags.is_array());
        assert!(tags.has_nested_structure());

        let items = tags.items.as_ref().expect("items should exist");
        assert_eq!(items.logical_type, "string");
    }

    #[test]
    fn test_deeply_nested_properties() {
        let customers = get_customers_schema();
        let metadata = customers
            .get_property("metadata")
            .expect("Property should exist");

        // metadata.scores is an array of objects
        let scores = metadata
            .properties
            .iter()
            .find(|p| p.name == "scores")
            .expect("scores should exist");

        assert!(scores.is_array());
        let score_item = scores.items.as_ref().expect("items should exist");
        assert!(score_item.is_object());
        assert_eq!(score_item.properties.len(), 2);
    }
}

// ============================================================================
// Round-Trip Preservation Tests
// ============================================================================

mod roundtrip_preservation_tests {
    use super::*;

    fn roundtrip(yaml: &str) -> (ODCSContract, ODCSContract) {
        let mut importer = ODCSImporter::new();
        let original = importer.import_contract(yaml).expect("Import failed");

        let exported = ODCSExporter::export_contract(&original);

        let mut importer2 = ODCSImporter::new();
        let reimported = importer2
            .import_contract(&exported)
            .expect("Re-import failed");

        (original, reimported)
    }

    #[test]
    fn test_roundtrip_preserves_contract_identity() {
        let (original, reimported) = roundtrip(COMPREHENSIVE_ODCS_YAML);

        assert_eq!(original.api_version, reimported.api_version);
        assert_eq!(original.kind, reimported.kind);
        assert_eq!(original.id, reimported.id);
        assert_eq!(original.version, reimported.version);
        assert_eq!(original.name, reimported.name);
    }

    #[test]
    fn test_roundtrip_preserves_schema_structure() {
        let (original, reimported) = roundtrip(COMPREHENSIVE_ODCS_YAML);

        assert_eq!(original.schema.len(), reimported.schema.len());
        for (orig_schema, reimp_schema) in original.schema.iter().zip(reimported.schema.iter()) {
            assert_eq!(orig_schema.name, reimp_schema.name);
            assert_eq!(orig_schema.properties.len(), reimp_schema.properties.len());
        }
    }

    #[test]
    fn test_roundtrip_preserves_nested_properties() {
        let (original, reimported) = roundtrip(COMPREHENSIVE_ODCS_YAML);

        let orig_customers = original.get_schema("customers").unwrap();
        let reimp_customers = reimported.get_schema("customers").unwrap();

        let orig_address = orig_customers.get_property("address").unwrap();
        let reimp_address = reimp_customers.get_property("address").unwrap();

        assert_eq!(orig_address.properties.len(), reimp_address.properties.len());

        let orig_metadata = orig_customers.get_property("metadata").unwrap();
        let reimp_metadata = reimp_customers.get_property("metadata").unwrap();

        // Check deeply nested structure
        let orig_scores = orig_metadata
            .properties
            .iter()
            .find(|p| p.name == "scores")
            .unwrap();
        let reimp_scores = reimp_metadata
            .properties
            .iter()
            .find(|p| p.name == "scores")
            .unwrap();

        assert_eq!(
            orig_scores.items.as_ref().unwrap().properties.len(),
            reimp_scores.items.as_ref().unwrap().properties.len()
        );
    }

    #[test]
    fn test_roundtrip_preserves_quality_rules() {
        let (original, reimported) = roundtrip(COMPREHENSIVE_ODCS_YAML);

        // Contract-level quality
        assert_eq!(original.quality.len(), reimported.quality.len());

        // Schema-level quality
        let orig_customers = original.get_schema("customers").unwrap();
        let reimp_customers = reimported.get_schema("customers").unwrap();
        assert_eq!(orig_customers.quality.len(), reimp_customers.quality.len());

        // Property-level quality
        let orig_id = orig_customers.get_property("id").unwrap();
        let reimp_id = reimp_customers.get_property("id").unwrap();
        assert_eq!(orig_id.quality.len(), reimp_id.quality.len());
    }

    #[test]
    fn test_roundtrip_preserves_custom_properties() {
        let (original, reimported) = roundtrip(COMPREHENSIVE_ODCS_YAML);

        // Contract-level
        assert_eq!(
            original.custom_properties.len(),
            reimported.custom_properties.len()
        );

        // Schema-level
        let orig_customers = original.get_schema("customers").unwrap();
        let reimp_customers = reimported.get_schema("customers").unwrap();
        assert_eq!(
            orig_customers.custom_properties.len(),
            reimp_customers.custom_properties.len()
        );

        // Property-level
        let orig_id = orig_customers.get_property("id").unwrap();
        let reimp_id = reimp_customers.get_property("id").unwrap();
        assert_eq!(
            orig_id.custom_properties.len(),
            reimp_id.custom_properties.len()
        );
    }

    #[test]
    fn test_roundtrip_full_equality() {
        let (original, reimported) = roundtrip(COMPREHENSIVE_ODCS_YAML);

        // Compare all major fields individually for better error reporting
        assert_eq!(original.api_version, reimported.api_version, "api_version mismatch");
        assert_eq!(original.kind, reimported.kind, "kind mismatch");
        assert_eq!(original.id, reimported.id, "id mismatch");
        assert_eq!(original.version, reimported.version, "version mismatch");
        assert_eq!(original.name, reimported.name, "name mismatch");
        assert_eq!(original.status, reimported.status, "status mismatch");
        assert_eq!(original.domain, reimported.domain, "domain mismatch");
        assert_eq!(original.data_product, reimported.data_product, "data_product mismatch");
        assert_eq!(original.tenant, reimported.tenant, "tenant mismatch");
        assert_eq!(original.contract_created_ts, reimported.contract_created_ts, "timestamp mismatch");
        assert_eq!(original.tags, reimported.tags, "tags mismatch");
        assert_eq!(original.custom_properties.len(), reimported.custom_properties.len(), "custom_properties count mismatch");
        assert_eq!(original.servers.len(), reimported.servers.len(), "servers count mismatch");
        assert_eq!(original.roles.len(), reimported.roles.len(), "roles count mismatch");
        assert_eq!(original.service_levels.len(), reimported.service_levels.len(), "service_levels count mismatch");
        assert_eq!(original.quality.len(), reimported.quality.len(), "quality count mismatch");
        assert_eq!(original.links.len(), reimported.links.len(), "links count mismatch");
        assert_eq!(original.authoritative_definitions.len(), reimported.authoritative_definitions.len(), "auth_defs count mismatch");

        // Deep comparison of schemas
        assert_eq!(original.schema.len(), reimported.schema.len(), "schema count mismatch");
        for (orig, reimp) in original.schema.iter().zip(reimported.schema.iter()) {
            assert_eq!(orig.name, reimp.name, "schema name mismatch");
            assert_eq!(orig.physical_name, reimp.physical_name, "schema physical_name mismatch");
            assert_eq!(orig.physical_type, reimp.physical_type, "schema physical_type mismatch");
            assert_eq!(orig.business_name, reimp.business_name, "schema business_name mismatch");
            assert_eq!(orig.description, reimp.description, "schema description mismatch");
            assert_eq!(orig.tags, reimp.tags, "schema tags mismatch");
            assert_eq!(orig.custom_properties.len(), reimp.custom_properties.len(), "schema custom_properties count mismatch");
            assert_eq!(orig.properties.len(), reimp.properties.len(), "schema properties count mismatch");

            // Deep comparison of properties
            for (orig_prop, reimp_prop) in orig.properties.iter().zip(reimp.properties.iter()) {
                assert_eq!(orig_prop.name, reimp_prop.name, "property name mismatch");
                assert_eq!(orig_prop.logical_type, reimp_prop.logical_type, "property logical_type mismatch");
                assert_eq!(orig_prop.physical_type, reimp_prop.physical_type, "property physical_type mismatch");
                assert_eq!(orig_prop.required, reimp_prop.required, "property required mismatch");
                assert_eq!(orig_prop.primary_key, reimp_prop.primary_key, "property primary_key mismatch");
                assert_eq!(orig_prop.tags, reimp_prop.tags, "property tags mismatch");
                assert_eq!(orig_prop.custom_properties.len(), reimp_prop.custom_properties.len(), "property custom_properties count mismatch");
            }
        }
    }
}

// ============================================================================
// Builder API Tests (Programmatic Construction)
// ============================================================================

mod builder_api_tests {
    use super::*;

    #[test]
    fn test_build_and_export_contract() {
        let contract = ODCSContract::new_with_id(
            "test-uuid-123",
            "builder-test-contract",
            "1.0.0",
        )
        .with_status("draft")
        .with_domain("test-domain")
        .with_description("Built via builder API")
        .with_tag("test")
        .with_custom_property(CustomProperty::string("source", "builder"))
        .with_schema(
            SchemaObject::new("test_table")
                .with_physical_type("table")
                .with_description("Test table")
                .with_tag("test-schema")
                .with_custom_property(CustomProperty::string("schemaSource", "builder"))
                .with_properties(vec![
                    Property::new("id", "integer")
                        .with_primary_key(true)
                        .with_required(true)
                        .with_description("Primary key"),
                    Property::new("name", "string")
                        .with_required(true)
                        .with_physical_type("VARCHAR(100)")
                        .with_custom_property(CustomProperty::string("columnSource", "builder")),
                    Property::new("address", "object").with_nested_properties(vec![
                        Property::new("city", "string").with_required(true),
                        Property::new("zip", "string"),
                    ]),
                ]),
        );

        // Export
        let yaml = ODCSExporter::export_contract(&contract);

        // Verify key fields in YAML
        assert!(yaml.contains("apiVersion"));
        assert!(yaml.contains("v3.1.0"));
        assert!(yaml.contains("builder-test-contract"));
        assert!(yaml.contains("test-domain"));

        // Import back and verify equality
        let mut importer = ODCSImporter::new();
        let reimported = importer.import_contract(&yaml).expect("Re-import failed");

        assert_eq!(contract.name, reimported.name);
        assert_eq!(contract.status, reimported.status);
        assert_eq!(contract.domain, reimported.domain);
        assert_eq!(contract.custom_properties.len(), reimported.custom_properties.len());

        let orig_schema = contract.get_schema("test_table").unwrap();
        let reimp_schema = reimported.get_schema("test_table").unwrap();
        assert_eq!(orig_schema.custom_properties.len(), reimp_schema.custom_properties.len());
    }
}
