//! ODCSContract type for ODCS native data structures
//!
//! Represents the root data contract document following the ODCS v3.1.0 specification.

use super::schema::SchemaObject;
use super::supporting::{
    AuthoritativeDefinition, CustomProperty, Description, Link, Price, QualityRule, Role, Server,
    ServiceLevel, Support, Team, Terms,
};
use serde::{Deserialize, Serialize};

/// ODCSContract - the root data contract document (ODCS v3.1.0)
///
/// This is the top-level structure that represents an entire ODCS data contract.
/// It contains all contract-level metadata plus one or more schema objects (tables).
///
/// # Example
///
/// ```rust
/// use data_modelling_core::models::odcs::{ODCSContract, SchemaObject, Property};
///
/// let contract = ODCSContract::new("customer-contract", "v1.0.0")
///     .with_domain("retail")
///     .with_status("active")
///     .with_schema(
///         SchemaObject::new("customers")
///             .with_physical_type("table")
///             .with_properties(vec![
///                 Property::new("id", "integer").with_primary_key(true),
///                 Property::new("name", "string").with_required(true),
///             ])
///     );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODCSContract {
    // === Required Identity Fields ===
    /// API version (e.g., "v3.1.0")
    pub api_version: String,
    /// Kind identifier (always "DataContract")
    pub kind: String,
    /// Unique contract ID (UUID or other identifier)
    pub id: String,
    /// Contract version (semantic versioning recommended)
    pub version: String,
    /// Contract name
    pub name: String,

    // === Status ===
    /// Contract status: "draft", "active", "deprecated", "retired"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    // === Organization ===
    /// Domain this contract belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Data product this contract belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_product: Option<String>,
    /// Tenant identifier for multi-tenant systems
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    // === Description ===
    /// Contract description (can be simple string or structured object)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,

    // === Schema (Tables) ===
    /// Schema objects (tables, views, topics) in this contract
    #[serde(default)]
    pub schema: Vec<SchemaObject>,

    // === Configuration ===
    /// Server configurations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<Server>,
    /// Team information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<Team>,
    /// Support information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support: Option<Support>,
    /// Role definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,

    // === SLA & Quality ===
    /// Service level agreements
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub service_levels: Vec<ServiceLevel>,
    /// Contract-level quality rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quality: Vec<QualityRule>,

    // === Pricing & Terms ===
    /// Price information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Price>,
    /// Terms and conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms: Option<Terms>,

    // === Links & References ===
    /// External links
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Authoritative definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authoritative_definitions: Vec<AuthoritativeDefinition>,

    // === Tags & Custom Properties ===
    /// Contract-level tags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Custom properties for format-specific metadata
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_properties: Vec<CustomProperty>,

    // === Timestamps ===
    /// Contract creation timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_created_ts: Option<String>,
}

impl Default for ODCSContract {
    fn default() -> Self {
        Self {
            api_version: "v3.1.0".to_string(),
            kind: "DataContract".to_string(),
            id: String::new(),
            version: "1.0.0".to_string(),
            name: String::new(),
            status: None,
            domain: None,
            data_product: None,
            tenant: None,
            description: None,
            schema: Vec::new(),
            servers: Vec::new(),
            team: None,
            support: None,
            roles: Vec::new(),
            service_levels: Vec::new(),
            quality: Vec::new(),
            price: None,
            terms: None,
            links: Vec::new(),
            authoritative_definitions: Vec::new(),
            tags: Vec::new(),
            custom_properties: Vec::new(),
            contract_created_ts: None,
        }
    }
}

impl ODCSContract {
    /// Create a new contract with the given name and version
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            id: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        }
    }

    /// Create a new contract with a specific ID
    pub fn new_with_id(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            ..Default::default()
        }
    }

    /// Set the API version
    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = api_version.into();
        self
    }

    /// Set the status
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Set the domain
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set the data product
    pub fn with_data_product(mut self, data_product: impl Into<String>) -> Self {
        self.data_product = Some(data_product.into());
        self
    }

    /// Set the tenant
    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    /// Set the description (simple string)
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(Description::Simple(description.into()));
        self
    }

    /// Set a structured description
    pub fn with_structured_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a schema object
    pub fn with_schema(mut self, schema: SchemaObject) -> Self {
        self.schema.push(schema);
        self
    }

    /// Set all schema objects
    pub fn with_schemas(mut self, schemas: Vec<SchemaObject>) -> Self {
        self.schema = schemas;
        self
    }

    /// Add a server configuration
    pub fn with_server(mut self, server: Server) -> Self {
        self.servers.push(server);
        self
    }

    /// Set the team information
    pub fn with_team(mut self, team: Team) -> Self {
        self.team = Some(team);
        self
    }

    /// Set the support information
    pub fn with_support(mut self, support: Support) -> Self {
        self.support = Some(support);
        self
    }

    /// Add a role
    pub fn with_role(mut self, role: Role) -> Self {
        self.roles.push(role);
        self
    }

    /// Add a service level
    pub fn with_service_level(mut self, service_level: ServiceLevel) -> Self {
        self.service_levels.push(service_level);
        self
    }

    /// Add a quality rule
    pub fn with_quality_rule(mut self, rule: QualityRule) -> Self {
        self.quality.push(rule);
        self
    }

    /// Set the price
    pub fn with_price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }

    /// Set the terms
    pub fn with_terms(mut self, terms: Terms) -> Self {
        self.terms = Some(terms);
        self
    }

    /// Add a link
    pub fn with_link(mut self, link: Link) -> Self {
        self.links.push(link);
        self
    }

    /// Add an authoritative definition
    pub fn with_authoritative_definition(mut self, definition: AuthoritativeDefinition) -> Self {
        self.authoritative_definitions.push(definition);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set all tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a custom property
    pub fn with_custom_property(mut self, custom_property: CustomProperty) -> Self {
        self.custom_properties.push(custom_property);
        self
    }

    /// Set the contract creation timestamp
    pub fn with_contract_created_ts(mut self, timestamp: impl Into<String>) -> Self {
        self.contract_created_ts = Some(timestamp.into());
        self
    }

    /// Get the number of schema objects
    pub fn schema_count(&self) -> usize {
        self.schema.len()
    }

    /// Get a schema object by name
    pub fn get_schema(&self, name: &str) -> Option<&SchemaObject> {
        self.schema.iter().find(|s| s.name == name)
    }

    /// Get a mutable schema object by name
    pub fn get_schema_mut(&mut self, name: &str) -> Option<&mut SchemaObject> {
        self.schema.iter_mut().find(|s| s.name == name)
    }

    /// Get all schema names
    pub fn schema_names(&self) -> Vec<&str> {
        self.schema.iter().map(|s| s.name.as_str()).collect()
    }

    /// Check if this is a multi-table contract
    pub fn is_multi_table(&self) -> bool {
        self.schema.len() > 1
    }

    /// Get the first schema (for single-table contracts)
    pub fn first_schema(&self) -> Option<&SchemaObject> {
        self.schema.first()
    }

    /// Get the description as a simple string
    pub fn description_string(&self) -> Option<String> {
        self.description.as_ref().map(|d| d.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::odcs::Property;

    #[test]
    fn test_contract_creation() {
        let contract = ODCSContract::new("my-contract", "1.0.0")
            .with_domain("retail")
            .with_status("active");

        assert_eq!(contract.name, "my-contract");
        assert_eq!(contract.version, "1.0.0");
        assert_eq!(contract.domain, Some("retail".to_string()));
        assert_eq!(contract.status, Some("active".to_string()));
        assert_eq!(contract.api_version, "v3.1.0");
        assert_eq!(contract.kind, "DataContract");
        assert!(!contract.id.is_empty()); // UUID was generated
    }

    #[test]
    fn test_contract_with_schema() {
        let contract = ODCSContract::new("order-contract", "2.0.0")
            .with_schema(
                SchemaObject::new("orders")
                    .with_physical_type("table")
                    .with_properties(vec![
                        Property::new("id", "integer").with_primary_key(true),
                        Property::new("customer_id", "integer"),
                        Property::new("total", "number"),
                    ]),
            )
            .with_schema(
                SchemaObject::new("order_items")
                    .with_physical_type("table")
                    .with_properties(vec![
                        Property::new("id", "integer").with_primary_key(true),
                        Property::new("order_id", "integer"),
                        Property::new("product_id", "integer"),
                    ]),
            );

        assert_eq!(contract.schema_count(), 2);
        assert!(contract.is_multi_table());
        assert_eq!(contract.schema_names(), vec!["orders", "order_items"]);

        let orders = contract.get_schema("orders");
        assert!(orders.is_some());
        assert_eq!(orders.unwrap().property_count(), 3);
    }

    #[test]
    fn test_contract_serialization() {
        let contract = ODCSContract::new_with_id(
            "550e8400-e29b-41d4-a716-446655440000",
            "test-contract",
            "1.0.0",
        )
        .with_domain("test")
        .with_status("draft")
        .with_description("A test contract")
        .with_tag("test")
        .with_schema(SchemaObject::new("test_table").with_property(Property::new("id", "string")));

        let json = serde_json::to_string_pretty(&contract).unwrap();

        assert!(json.contains("\"apiVersion\": \"v3.1.0\""));
        assert!(json.contains("\"kind\": \"DataContract\""));
        assert!(json.contains("\"id\": \"550e8400-e29b-41d4-a716-446655440000\""));
        assert!(json.contains("\"name\": \"test-contract\""));
        assert!(json.contains("\"domain\": \"test\""));
        assert!(json.contains("\"status\": \"draft\""));

        // Verify camelCase
        assert!(json.contains("apiVersion"));
        assert!(!json.contains("api_version"));
    }

    #[test]
    fn test_contract_deserialization() {
        let json = r#"{
            "apiVersion": "v3.1.0",
            "kind": "DataContract",
            "id": "test-id-123",
            "version": "2.0.0",
            "name": "customer-contract",
            "status": "active",
            "domain": "customers",
            "description": "Customer data contract",
            "schema": [
                {
                    "name": "customers",
                    "physicalType": "table",
                    "properties": [
                        {
                            "name": "id",
                            "logicalType": "integer",
                            "primaryKey": true
                        },
                        {
                            "name": "name",
                            "logicalType": "string",
                            "required": true
                        }
                    ]
                }
            ],
            "tags": ["customer", "pii"]
        }"#;

        let contract: ODCSContract = serde_json::from_str(json).unwrap();
        assert_eq!(contract.api_version, "v3.1.0");
        assert_eq!(contract.kind, "DataContract");
        assert_eq!(contract.id, "test-id-123");
        assert_eq!(contract.version, "2.0.0");
        assert_eq!(contract.name, "customer-contract");
        assert_eq!(contract.status, Some("active".to_string()));
        assert_eq!(contract.domain, Some("customers".to_string()));
        assert_eq!(contract.schema_count(), 1);
        assert_eq!(contract.tags, vec!["customer", "pii"]);

        let customers = contract.get_schema("customers").unwrap();
        assert_eq!(customers.property_count(), 2);
    }

    #[test]
    fn test_structured_description() {
        let json = r#"{
            "apiVersion": "v3.1.0",
            "kind": "DataContract",
            "id": "test",
            "version": "1.0.0",
            "name": "test",
            "description": {
                "purpose": "Store customer information",
                "usage": "Read-only access for analytics"
            }
        }"#;

        let contract: ODCSContract = serde_json::from_str(json).unwrap();
        assert_eq!(
            contract.description_string(),
            Some("Store customer information".to_string())
        );
    }
}
