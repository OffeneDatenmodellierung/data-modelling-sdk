//! Universal format converter module
//!
//! Provides functionality to convert any import format to ODCS v3.1.0 format.

pub mod converter;
pub mod migrate_dataflow;
pub mod openapi_to_odcs;

pub use converter::{ConversionError, convert_to_odcs};
pub use migrate_dataflow::{MigrationError, migrate_dataflow_to_domain};
pub use openapi_to_odcs::{
    ConversionReport, NestedObjectStrategy, OpenAPIToODCSConverter, TypeMappingRule,
};
