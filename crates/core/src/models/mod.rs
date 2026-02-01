//! Models module for the SDK
//!
//! Defines core data structures used by the SDK for import/export operations.
//! These models are simplified versions focused on the SDK's needs.

#[cfg(feature = "bpmn")]
pub mod bpmn;
pub mod cads;
pub mod column;
pub mod cross_domain;
pub mod data_model;
pub mod decision;
#[cfg(feature = "dmn")]
pub mod dmn;
pub mod domain;
pub mod domain_config;
pub mod enums;
pub mod knowledge;
pub mod odcs;
pub mod odps;
#[cfg(feature = "openapi")]
pub mod openapi;
pub mod relationship;
pub mod sketch;
pub mod table;
pub mod tag;
pub mod workspace;

#[cfg(feature = "bpmn")]
pub use bpmn::{BPMNModel, BPMNModelFormat};
pub use cads::{
    CADSAsset, CADSBPMNFormat, CADSBPMNModel, CADSCompliance, CADSComplianceControl,
    CADSComplianceFramework, CADSComplianceStatus, CADSDMNFormat, CADSDMNModel, CADSDescription,
    CADSExternalLink, CADSImpactArea, CADSKind, CADSMitigationStatus, CADSOpenAPIFormat,
    CADSOpenAPISpec, CADSPricing, CADSPricingModel, CADSRisk, CADSRiskAssessment,
    CADSRiskClassification, CADSRiskMitigation, CADSRuntime, CADSRuntimeContainer,
    CADSRuntimeResources, CADSSLA, CADSSLAProperty, CADSStatus, CADSTeamMember,
    CADSValidationProfile, CADSValidationProfileAppliesTo,
};
pub use column::{
    AuthoritativeDefinition, Column, ForeignKey, LogicalTypeOptions, PropertyRelationship,
};
pub use cross_domain::{CrossDomainConfig, CrossDomainRelationshipRef, CrossDomainTableRef};
pub use data_model::DataModel;
#[cfg(feature = "dmn")]
pub use dmn::{DMNModel, DMNModelFormat};
pub use domain::{
    CADSNode, CrowsfeetCardinality, Domain, NodeConnection, ODCSNode, SharedNodeReference, System,
    SystemConnection,
};
pub use domain_config::{DomainConfig, DomainOwner, ViewPosition};
pub use enums::*;
pub use odps::{
    ODPSApiVersion, ODPSAuthoritativeDefinition, ODPSCustomProperty, ODPSDataProduct,
    ODPSDescription, ODPSInputContract, ODPSInputPort, ODPSManagementPort, ODPSOutputPort,
    ODPSSBOM, ODPSStatus, ODPSSupport, ODPSTeam, ODPSTeamMember,
};
#[cfg(feature = "openapi")]
pub use openapi::{OpenAPIFormat, OpenAPIModel};
pub use relationship::{
    ConnectionHandle, ConnectionPoint, ETLJobMetadata, ForeignKeyDetails, Relationship,
    VisualMetadata,
};
pub use table::{ContactDetails, Position, SlaProperty, Table};
pub use tag::Tag;
pub use workspace::{DomainReference, Workspace};

// Decision and Knowledge models
pub use decision::{
    AssetLink, AssetRelationship, ComplianceAssessment, Decision, DecisionCategory, DecisionDriver,
    DecisionIndex, DecisionIndexEntry, DecisionOption, DecisionStatus, DriverPriority,
};
pub use knowledge::{
    ArticleRelationship, KnowledgeArticle, KnowledgeIndex, KnowledgeIndexEntry, KnowledgeStatus,
    KnowledgeType, RelatedArticle, ReviewFrequency, SkillLevel,
};
pub use sketch::{Sketch, SketchIndex, SketchIndexEntry, SketchStatus, SketchType};

use serde::{Deserialize, Serialize};

/// Model type for references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ModelType {
    /// BPMN model
    Bpmn,
    /// DMN model
    Dmn,
    /// OpenAPI specification
    OpenApi,
}

/// Model reference for CADS assets
///
/// Represents a reference from a CADS asset to a BPMN, DMN, or OpenAPI model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelReference {
    /// Type of model being referenced
    pub model_type: ModelType,
    /// Target domain ID (None for same domain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<uuid::Uuid>,
    /// Name of the referenced model
    pub model_name: String,
    /// Optional description of the reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ModelReference {
    /// Create a new model reference
    pub fn new(model_type: ModelType, model_name: String) -> Self {
        ModelReference {
            model_type,
            domain_id: None,
            model_name,
            description: None,
        }
    }

    /// Create a cross-domain model reference
    pub fn new_cross_domain(
        model_type: ModelType,
        domain_id: uuid::Uuid,
        model_name: String,
    ) -> Self {
        ModelReference {
            model_type,
            domain_id: Some(domain_id),
            model_name,
            description: None,
        }
    }
}
