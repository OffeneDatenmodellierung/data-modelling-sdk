//! Models module for the SDK
//! 
//! Defines core data structures used by the SDK for import/export operations.
//! These models are simplified versions focused on the SDK's needs.

pub mod column;
pub mod enums;
pub mod table;
pub mod relationship;
pub mod data_model;
pub mod cross_domain;

pub use column::{Column, ForeignKey};
pub use enums::*;
pub use table::{Table, Position};
pub use relationship::{Relationship, VisualMetadata, ETLJobMetadata, ForeignKeyDetails, ConnectionPoint};
pub use data_model::DataModel;
pub use cross_domain::{CrossDomainConfig, CrossDomainTableRef, CrossDomainRelationshipRef};



