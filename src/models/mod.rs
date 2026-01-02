//! Models module for the SDK
//!
//! Defines core data structures used by the SDK for import/export operations.
//! These models are simplified versions focused on the SDK's needs.

pub mod column;
pub mod cross_domain;
pub mod data_model;
pub mod enums;
pub mod relationship;
pub mod table;

pub use column::{Column, ForeignKey};
pub use cross_domain::{CrossDomainConfig, CrossDomainRelationshipRef, CrossDomainTableRef};
pub use data_model::DataModel;
pub use enums::*;
pub use relationship::{
    ConnectionPoint, ETLJobMetadata, ForeignKeyDetails, Relationship, VisualMetadata,
};
pub use table::{ContactDetails, Position, SlaProperty, Table};
