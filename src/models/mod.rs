//! Models module for the SDK
//!
//! Defines core data structures used by the SDK for import/export operations.
//! These models are simplified versions focused on the SDK's needs.

pub mod cads;
pub mod column;
pub mod cross_domain;
pub mod data_model;
pub mod domain;
pub mod enums;
pub mod odps;
pub mod relationship;
pub mod table;
pub mod tag;

pub use cads::{
    CADSAsset, CADSBPMNFormat, CADSBPMNModel, CADSCompliance, CADSComplianceControl,
    CADSComplianceFramework, CADSComplianceStatus, CADSDescription, CADSExternalLink,
    CADSImpactArea, CADSKind, CADSMitigationStatus, CADSPricing, CADSPricingModel, CADSRisk,
    CADSRiskAssessment, CADSRiskClassification, CADSRiskMitigation, CADSRuntime,
    CADSRuntimeContainer, CADSRuntimeResources, CADSSLA, CADSSLAProperty, CADSStatus,
    CADSTeamMember, CADSValidationProfile, CADSValidationProfileAppliesTo,
};
pub use column::{Column, ForeignKey};
pub use cross_domain::{CrossDomainConfig, CrossDomainRelationshipRef, CrossDomainTableRef};
pub use data_model::DataModel;
pub use domain::{
    CADSNode, CrowsfeetCardinality, Domain, NodeConnection, ODCSNode, SharedNodeReference, System,
    SystemConnection,
};
pub use enums::*;
pub use odps::{
    ODPSApiVersion, ODPSAuthoritativeDefinition, ODPSCustomProperty, ODPSDataProduct,
    ODPSDescription, ODPSInputContract, ODPSInputPort, ODPSManagementPort, ODPSOutputPort,
    ODPSSBOM, ODPSStatus, ODPSSupport, ODPSTeam, ODPSTeamMember,
};
pub use relationship::{
    ConnectionPoint, ETLJobMetadata, ForeignKeyDetails, Relationship, VisualMetadata,
};
pub use table::{ContactDetails, Position, SlaProperty, Table};
pub use tag::Tag;
