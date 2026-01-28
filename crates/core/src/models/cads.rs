//! CADS (Compute Asset Description Specification) models
//!
//! Defines structures for CADS v1.0 assets including AI/ML models, applications, pipelines, and systems.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

use super::tag::Tag;

/// CADS asset kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CADSKind {
    AIModel,
    MLPipeline,
    Application,
    DataPipeline,
    ETLProcess,
    ETLPipeline,
    SourceSystem,
    DestinationSystem,
}

/// CADS asset status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CADSStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "validated")]
    Validated,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "deprecated")]
    Deprecated,
}

/// CADS description object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CADSDescription {
    /// Purpose of the asset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Usage instructions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    /// Limitations and constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limitations: Option<String>,
    /// External links and references
    #[serde(skip_serializing_if = "Option::is_none", alias = "external_links")]
    pub external_links: Option<Vec<CADSExternalLink>>,
}

/// External link in CADS description
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSExternalLink {
    /// URL of the external link
    pub url: String,
    /// Description of the link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// CADS runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSRuntime {
    /// Runtime environment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    /// Service endpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<String>>,
    /// Container configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<CADSRuntimeContainer>,
    /// Resource requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<CADSRuntimeResources>,
}

/// Container configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSRuntimeContainer {
    /// Container image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Resource requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSRuntimeResources {
    /// CPU requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    /// Memory requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    /// GPU requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<String>,
}

/// CADS SLA properties
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSSLA {
    /// SLA properties array
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<CADSSLAProperty>>,
}

/// SLA property
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSSLAProperty {
    /// SLA element name
    pub element: String,
    /// Value (number or string)
    pub value: serde_json::Value,
    /// Unit of measurement
    pub unit: String,
    /// Driver (e.g., "operational", "compliance")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
}

/// CADS pricing model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CADSPricing {
    /// Pricing model type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<CADSPricingModel>,
    /// Currency code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Unit cost
    #[serde(skip_serializing_if = "Option::is_none", alias = "unit_cost")]
    pub unit_cost: Option<f64>,
    /// Billing unit
    #[serde(skip_serializing_if = "Option::is_none", alias = "billing_unit")]
    pub billing_unit: Option<String>,
    /// Additional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Pricing model enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CADSPricingModel {
    PerRequest,
    PerHour,
    PerBatch,
    Subscription,
    Internal,
}

/// Team member
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSTeamMember {
    /// Role of the team member
    pub role: String,
    /// Name of the team member
    pub name: String,
    /// Contact information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

/// Risk classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CADSRiskClassification {
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

/// Impact area
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CADSImpactArea {
    Fairness,
    Privacy,
    Safety,
    Security,
    Financial,
    Operational,
    Reputational,
}

/// Risk assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSRiskAssessment {
    /// Assessment methodology
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methodology: Option<String>,
    /// Assessment date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Assessor name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessor: Option<String>,
}

/// Risk mitigation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSRiskMitigation {
    /// Mitigation description
    pub description: String,
    /// Mitigation status
    pub status: CADSMitigationStatus,
}

/// Mitigation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CADSMitigationStatus {
    Planned,
    Implemented,
    Verified,
}

/// CADS risk management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CADSRisk {
    /// Risk classification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<CADSRiskClassification>,
    /// Impact areas
    #[serde(skip_serializing_if = "Option::is_none", alias = "impact_areas")]
    pub impact_areas: Option<Vec<CADSImpactArea>>,
    /// Intended use
    #[serde(skip_serializing_if = "Option::is_none", alias = "intended_use")]
    pub intended_use: Option<String>,
    /// Out of scope use
    #[serde(skip_serializing_if = "Option::is_none", alias = "out_of_scope_use")]
    pub out_of_scope_use: Option<String>,
    /// Risk assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment: Option<CADSRiskAssessment>,
    /// Risk mitigations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mitigations: Option<Vec<CADSRiskMitigation>>,
}

/// Compliance framework
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSComplianceFramework {
    /// Framework name
    pub name: String,
    /// Framework category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Compliance status
    pub status: CADSComplianceStatus,
}

/// Compliance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CADSComplianceStatus {
    NotApplicable,
    Assessed,
    Compliant,
    NonCompliant,
}

/// Compliance control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSComplianceControl {
    /// Control ID
    pub id: String,
    /// Control description
    pub description: String,
    /// Evidence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// CADS compliance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSCompliance {
    /// Compliance frameworks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frameworks: Option<Vec<CADSComplianceFramework>>,
    /// Compliance controls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controls: Option<Vec<CADSComplianceControl>>,
}

/// Validation profile applies to
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CADSValidationProfileAppliesTo {
    /// Asset kind
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Risk classification
    #[serde(skip_serializing_if = "Option::is_none", alias = "risk_classification")]
    pub risk_classification: Option<String>,
}

/// Validation profile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CADSValidationProfile {
    /// Profile name
    pub name: String,
    /// Applies to criteria
    #[serde(skip_serializing_if = "Option::is_none", alias = "applies_to")]
    pub applies_to: Option<CADSValidationProfileAppliesTo>,
    /// Required checks
    #[serde(alias = "required_checks")]
    pub required_checks: Vec<String>,
}

/// BPMN model format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CADSBPMNFormat {
    #[serde(rename = "bpmn20-xml")]
    Bpmn20Xml,
    #[serde(rename = "json")]
    Json,
}

/// BPMN model reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSBPMNModel {
    /// Model name
    pub name: String,
    /// Reference to BPMN model
    pub reference: String,
    /// Format
    pub format: CADSBPMNFormat,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// DMN model format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CADSDMNFormat {
    #[serde(rename = "dmn13-xml")]
    Dmn13Xml,
    #[serde(rename = "json")]
    Json,
}

/// DMN model reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSDMNModel {
    /// Model name
    pub name: String,
    /// Reference to DMN model
    pub reference: String,
    /// Format
    pub format: CADSDMNFormat,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OpenAPI spec format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CADSOpenAPIFormat {
    #[serde(rename = "openapi-3.0")]
    Openapi30,
    #[serde(rename = "openapi-3.1")]
    Openapi31,
    #[serde(rename = "swagger-2.0")]
    Swagger20,
}

/// OpenAPI spec reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CADSOpenAPISpec {
    /// Spec name
    pub name: String,
    /// Reference to OpenAPI spec
    pub reference: String,
    /// Format
    pub format: CADSOpenAPIFormat,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// CADS Asset - main structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CADSAsset {
    /// API version
    #[serde(alias = "api_version")]
    pub api_version: String,
    /// Asset kind
    pub kind: CADSKind,
    /// Unique identifier (UUID or URN)
    pub id: String,
    /// Asset name
    pub name: String,
    /// Version
    pub version: String,
    /// Status
    pub status: CADSStatus,
    /// Domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Tags
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "deserialize_tags")]
    pub tags: Vec<Tag>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CADSDescription>,
    /// Runtime configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<CADSRuntime>,
    /// SLA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<CADSSLA>,
    /// Pricing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<CADSPricing>,
    /// Team
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<Vec<CADSTeamMember>>,
    /// Risk management
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<CADSRisk>,
    /// Compliance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance: Option<CADSCompliance>,
    /// Validation profiles
    #[serde(skip_serializing_if = "Option::is_none", alias = "validation_profiles")]
    pub validation_profiles: Option<Vec<CADSValidationProfile>>,
    /// BPMN models
    #[serde(skip_serializing_if = "Option::is_none", alias = "bpmn_models")]
    pub bpmn_models: Option<Vec<CADSBPMNModel>>,
    /// DMN models
    #[serde(skip_serializing_if = "Option::is_none", alias = "dmn_models")]
    pub dmn_models: Option<Vec<CADSDMNModel>>,
    /// OpenAPI specifications
    #[serde(skip_serializing_if = "Option::is_none", alias = "openapi_specs")]
    pub openapi_specs: Option<Vec<CADSOpenAPISpec>>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none", alias = "custom_properties")]
    pub custom_properties: Option<HashMap<String, serde_json::Value>>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none", alias = "created_at")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none", alias = "updated_at")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Deserialize tags with backward compatibility (supports Vec<String> and Vec<Tag>)
fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<Tag>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Accept either Vec<String> (backward compatibility) or Vec<Tag>
    struct TagVisitor;

    impl<'de> serde::de::Visitor<'de> for TagVisitor {
        type Value = Vec<Tag>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a vector of tags (strings or Tag objects)")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut tags = Vec::new();
            while let Some(item) = seq.next_element::<serde_json::Value>()? {
                match item {
                    serde_json::Value::String(s) => {
                        // Backward compatibility: parse string as Tag
                        if let Ok(tag) = Tag::from_str(&s) {
                            tags.push(tag);
                        }
                    }
                    _ => {
                        // Try to deserialize as Tag directly (if it's a string in JSON)
                        if let serde_json::Value::String(s) = item
                            && let Ok(tag) = Tag::from_str(&s)
                        {
                            tags.push(tag);
                        }
                    }
                }
            }
            Ok(tags)
        }
    }

    deserializer.deserialize_seq(TagVisitor)
}
