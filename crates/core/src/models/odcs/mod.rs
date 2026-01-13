//! ODCS Native Data Structures
//!
//! This module provides native Rust types that model the ODCS (Open Data Contract Standard)
//! v3.1.0 specification accurately. These types preserve the three-level hierarchy:
//!
//! 1. **Contract Level** ([`ODCSContract`]) - Root document with metadata
//! 2. **Schema Level** ([`SchemaObject`]) - Tables/views/topics within a contract
//! 3. **Property Level** ([`Property`]) - Columns/fields within a schema
//!
//! ## Design Goals
//!
//! - **Zero data loss**: Full round-trip import/export without losing metadata
//! - **Multi-table support**: Native support for contracts with multiple schema objects
//! - **Nested properties**: Proper hierarchical representation for OBJECT and ARRAY types
//! - **Format mapping**: Clean mapping from Avro, Protobuf, JSON Schema, OpenAPI via custom properties
//! - **Backwards compatibility**: Converters to/from existing `Table`/`Column` types
//!
//! ## Example
//!
//! ```rust
//! use data_modelling_core::models::odcs::{ODCSContract, SchemaObject, Property};
//!
//! // Create a contract with two tables
//! let contract = ODCSContract::new("ecommerce", "1.0.0")
//!     .with_domain("retail")
//!     .with_status("active")
//!     .with_schema(
//!         SchemaObject::new("orders")
//!             .with_physical_type("table")
//!             .with_properties(vec![
//!                 Property::new("id", "integer").with_primary_key(true),
//!                 Property::new("customer_id", "integer").with_required(true),
//!                 Property::new("total", "number"),
//!             ])
//!     )
//!     .with_schema(
//!         SchemaObject::new("order_items")
//!             .with_physical_type("table")
//!             .with_properties(vec![
//!                 Property::new("id", "integer").with_primary_key(true),
//!                 Property::new("order_id", "integer").with_required(true),
//!                 Property::new("product_name", "string"),
//!             ])
//!     );
//!
//! assert_eq!(contract.schema_count(), 2);
//! ```

pub mod contract;
pub mod converters;
pub mod property;
pub mod schema;
pub mod supporting;

// Re-export main types for convenience
pub use contract::ODCSContract;
pub use property::Property;
pub use schema::SchemaObject;

// Re-export supporting types
pub use supporting::{
    AuthoritativeDefinition, CustomProperty, Description, Link, LogicalTypeOptions, Price,
    PropertyRelationship, QualityRule, Role, SchemaRelationship, Server, ServiceLevel,
    StructuredDescription, Support, Team, TeamMember, Terms,
};
