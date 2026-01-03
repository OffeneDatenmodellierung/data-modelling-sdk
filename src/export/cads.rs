//! CADS (Compute Asset Description Specification) exporter
//!
//! Exports CADSAsset models to CADS v1.0 YAML format.

use crate::models::cads::*;
use serde_yaml;

/// CADS exporter for generating CADS v1.0 YAML from CADSAsset models
pub struct CADSExporter;

impl CADSExporter {
    /// Export a CADS asset to CADS v1.0 YAML format
    ///
    /// # Arguments
    ///
    /// * `asset` - The CADS asset to export
    ///
    /// # Returns
    ///
    /// A YAML string in CADS v1.0 format
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::cads::CADSExporter;
    /// use data_modelling_sdk::models::cads::*;
    ///
    /// let asset = CADSAsset {
    ///     api_version: "v1.0".to_string(),
    ///     kind: CADSKind::AIModel,
    ///     id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    ///     name: "sentiment-analysis-model".to_string(),
    ///     version: "1.0.0".to_string(),
    ///     status: CADSStatus::Production,
    ///     domain: None,
    ///     tags: vec![],
    ///     description: None,
    ///     runtime: None,
    ///     sla: None,
    ///     pricing: None,
    ///     team: None,
    ///     risk: None,
    ///     compliance: None,
    ///     validation_profiles: None,
    ///     bpmn_models: None,
    ///     dmn_models: None,
    ///     openapi_specs: None,
    ///     custom_properties: None,
    ///     created_at: None,
    ///     updated_at: None,
    /// };
    ///
    /// let yaml = CADSExporter::export_asset(&asset);
    /// assert!(yaml.contains("apiVersion: v1.0"));
    /// assert!(yaml.contains("kind: AIModel"));
    /// ```
    pub fn export_asset(asset: &CADSAsset) -> String {
        let mut yaml = serde_yaml::Mapping::new();

        // Required fields
        yaml.insert(
            serde_yaml::Value::String("apiVersion".to_string()),
            serde_yaml::Value::String(asset.api_version.clone()),
        );

        let kind_str = match asset.kind {
            CADSKind::AIModel => "AIModel",
            CADSKind::MLPipeline => "MLPipeline",
            CADSKind::Application => "Application",
            CADSKind::ETLPipeline => "ETLPipeline",
            CADSKind::SourceSystem => "SourceSystem",
            CADSKind::DestinationSystem => "DestinationSystem",
        };
        yaml.insert(
            serde_yaml::Value::String("kind".to_string()),
            serde_yaml::Value::String(kind_str.to_string()),
        );

        yaml.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(asset.id.clone()),
        );

        yaml.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(asset.name.clone()),
        );

        yaml.insert(
            serde_yaml::Value::String("version".to_string()),
            serde_yaml::Value::String(asset.version.clone()),
        );

        let status_str = match asset.status {
            CADSStatus::Draft => "draft",
            CADSStatus::Validated => "validated",
            CADSStatus::Production => "production",
            CADSStatus::Deprecated => "deprecated",
        };
        yaml.insert(
            serde_yaml::Value::String("status".to_string()),
            serde_yaml::Value::String(status_str.to_string()),
        );

        // Optional fields
        if let Some(domain) = &asset.domain {
            yaml.insert(
                serde_yaml::Value::String("domain".to_string()),
                serde_yaml::Value::String(domain.clone()),
            );
        }

        if !asset.tags.is_empty() {
            let tags_yaml: Vec<serde_yaml::Value> = asset
                .tags
                .iter()
                .map(|t| serde_yaml::Value::String(t.to_string()))
                .collect();
            yaml.insert(
                serde_yaml::Value::String("tags".to_string()),
                serde_yaml::Value::Sequence(tags_yaml),
            );
        }

        if let Some(description) = &asset.description {
            let mut desc_map = serde_yaml::Mapping::new();
            if let Some(purpose) = &description.purpose {
                desc_map.insert(
                    serde_yaml::Value::String("purpose".to_string()),
                    serde_yaml::Value::String(purpose.clone()),
                );
            }
            if let Some(usage) = &description.usage {
                desc_map.insert(
                    serde_yaml::Value::String("usage".to_string()),
                    serde_yaml::Value::String(usage.clone()),
                );
            }
            if let Some(limitations) = &description.limitations {
                desc_map.insert(
                    serde_yaml::Value::String("limitations".to_string()),
                    serde_yaml::Value::String(limitations.clone()),
                );
            }
            if let Some(external_links) = &description.external_links {
                let links_yaml: Vec<serde_yaml::Value> = external_links
                    .iter()
                    .map(|link| {
                        let mut link_map = serde_yaml::Mapping::new();
                        link_map.insert(
                            serde_yaml::Value::String("url".to_string()),
                            serde_yaml::Value::String(link.url.clone()),
                        );
                        if let Some(desc) = &link.description {
                            link_map.insert(
                                serde_yaml::Value::String("description".to_string()),
                                serde_yaml::Value::String(desc.clone()),
                            );
                        }
                        serde_yaml::Value::Mapping(link_map)
                    })
                    .collect();
                desc_map.insert(
                    serde_yaml::Value::String("externalLinks".to_string()),
                    serde_yaml::Value::Sequence(links_yaml),
                );
            }
            if !desc_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::Mapping(desc_map),
                );
            }
        }

        if let Some(runtime) = &asset.runtime {
            let mut runtime_map = serde_yaml::Mapping::new();
            if let Some(environment) = &runtime.environment {
                runtime_map.insert(
                    serde_yaml::Value::String("environment".to_string()),
                    serde_yaml::Value::String(environment.clone()),
                );
            }
            if let Some(endpoints) = &runtime.endpoints {
                let endpoints_yaml: Vec<serde_yaml::Value> = endpoints
                    .iter()
                    .map(|e| serde_yaml::Value::String(e.clone()))
                    .collect();
                runtime_map.insert(
                    serde_yaml::Value::String("endpoints".to_string()),
                    serde_yaml::Value::Sequence(endpoints_yaml),
                );
            }
            if let Some(container) = &runtime.container {
                let mut container_map = serde_yaml::Mapping::new();
                if let Some(image) = &container.image {
                    container_map.insert(
                        serde_yaml::Value::String("image".to_string()),
                        serde_yaml::Value::String(image.clone()),
                    );
                }
                if !container_map.is_empty() {
                    runtime_map.insert(
                        serde_yaml::Value::String("container".to_string()),
                        serde_yaml::Value::Mapping(container_map),
                    );
                }
            }
            if let Some(resources) = &runtime.resources {
                let mut resources_map = serde_yaml::Mapping::new();
                if let Some(cpu) = &resources.cpu {
                    resources_map.insert(
                        serde_yaml::Value::String("cpu".to_string()),
                        serde_yaml::Value::String(cpu.clone()),
                    );
                }
                if let Some(memory) = &resources.memory {
                    resources_map.insert(
                        serde_yaml::Value::String("memory".to_string()),
                        serde_yaml::Value::String(memory.clone()),
                    );
                }
                if let Some(gpu) = &resources.gpu {
                    resources_map.insert(
                        serde_yaml::Value::String("gpu".to_string()),
                        serde_yaml::Value::String(gpu.clone()),
                    );
                }
                if !resources_map.is_empty() {
                    runtime_map.insert(
                        serde_yaml::Value::String("resources".to_string()),
                        serde_yaml::Value::Mapping(resources_map),
                    );
                }
            }
            if !runtime_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("runtime".to_string()),
                    serde_yaml::Value::Mapping(runtime_map),
                );
            }
        }

        if let Some(sla) = &asset.sla
            && let Some(properties) = &sla.properties
        {
            let mut sla_map = serde_yaml::Mapping::new();
            let props_yaml: Vec<serde_yaml::Value> = properties
                .iter()
                .map(|prop| {
                    let mut prop_map = serde_yaml::Mapping::new();
                    prop_map.insert(
                        serde_yaml::Value::String("element".to_string()),
                        serde_yaml::Value::String(prop.element.clone()),
                    );
                    prop_map.insert(
                        serde_yaml::Value::String("value".to_string()),
                        Self::json_to_yaml_value(&prop.value),
                    );
                    prop_map.insert(
                        serde_yaml::Value::String("unit".to_string()),
                        serde_yaml::Value::String(prop.unit.clone()),
                    );
                    if let Some(driver) = &prop.driver {
                        prop_map.insert(
                            serde_yaml::Value::String("driver".to_string()),
                            serde_yaml::Value::String(driver.clone()),
                        );
                    }
                    serde_yaml::Value::Mapping(prop_map)
                })
                .collect();
            sla_map.insert(
                serde_yaml::Value::String("properties".to_string()),
                serde_yaml::Value::Sequence(props_yaml),
            );
            yaml.insert(
                serde_yaml::Value::String("sla".to_string()),
                serde_yaml::Value::Mapping(sla_map),
            );
        }

        if let Some(pricing) = &asset.pricing {
            let mut pricing_map = serde_yaml::Mapping::new();
            if let Some(model) = &pricing.model {
                let model_str = match model {
                    CADSPricingModel::PerRequest => "per_request",
                    CADSPricingModel::PerHour => "per_hour",
                    CADSPricingModel::PerBatch => "per_batch",
                    CADSPricingModel::Subscription => "subscription",
                    CADSPricingModel::Internal => "internal",
                };
                pricing_map.insert(
                    serde_yaml::Value::String("model".to_string()),
                    serde_yaml::Value::String(model_str.to_string()),
                );
            }
            if let Some(currency) = &pricing.currency {
                pricing_map.insert(
                    serde_yaml::Value::String("currency".to_string()),
                    serde_yaml::Value::String(currency.clone()),
                );
            }
            if let Some(unit_cost) = pricing.unit_cost {
                pricing_map.insert(
                    serde_yaml::Value::String("unitCost".to_string()),
                    serde_yaml::Value::Number(serde_yaml::Number::from(unit_cost)),
                );
            }
            if let Some(billing_unit) = &pricing.billing_unit {
                pricing_map.insert(
                    serde_yaml::Value::String("billingUnit".to_string()),
                    serde_yaml::Value::String(billing_unit.clone()),
                );
            }
            if let Some(notes) = &pricing.notes {
                pricing_map.insert(
                    serde_yaml::Value::String("notes".to_string()),
                    serde_yaml::Value::String(notes.clone()),
                );
            }
            if !pricing_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("pricing".to_string()),
                    serde_yaml::Value::Mapping(pricing_map),
                );
            }
        }

        if let Some(team) = &asset.team {
            let team_yaml: Vec<serde_yaml::Value> = team
                .iter()
                .map(|member| {
                    let mut member_map = serde_yaml::Mapping::new();
                    member_map.insert(
                        serde_yaml::Value::String("role".to_string()),
                        serde_yaml::Value::String(member.role.clone()),
                    );
                    member_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(member.name.clone()),
                    );
                    if let Some(contact) = &member.contact {
                        member_map.insert(
                            serde_yaml::Value::String("contact".to_string()),
                            serde_yaml::Value::String(contact.clone()),
                        );
                    }
                    serde_yaml::Value::Mapping(member_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("team".to_string()),
                serde_yaml::Value::Sequence(team_yaml),
            );
        }

        if let Some(risk) = &asset.risk {
            let mut risk_map = serde_yaml::Mapping::new();
            if let Some(classification) = &risk.classification {
                let class_str = match classification {
                    CADSRiskClassification::Minimal => "minimal",
                    CADSRiskClassification::Low => "low",
                    CADSRiskClassification::Medium => "medium",
                    CADSRiskClassification::High => "high",
                };
                risk_map.insert(
                    serde_yaml::Value::String("classification".to_string()),
                    serde_yaml::Value::String(class_str.to_string()),
                );
            }
            if let Some(impact_areas) = &risk.impact_areas {
                let areas_yaml: Vec<serde_yaml::Value> = impact_areas
                    .iter()
                    .map(|area| {
                        let area_str = match area {
                            CADSImpactArea::Fairness => "fairness",
                            CADSImpactArea::Privacy => "privacy",
                            CADSImpactArea::Safety => "safety",
                            CADSImpactArea::Security => "security",
                            CADSImpactArea::Financial => "financial",
                            CADSImpactArea::Operational => "operational",
                            CADSImpactArea::Reputational => "reputational",
                        };
                        serde_yaml::Value::String(area_str.to_string())
                    })
                    .collect();
                risk_map.insert(
                    serde_yaml::Value::String("impactAreas".to_string()),
                    serde_yaml::Value::Sequence(areas_yaml),
                );
            }
            if let Some(intended_use) = &risk.intended_use {
                risk_map.insert(
                    serde_yaml::Value::String("intendedUse".to_string()),
                    serde_yaml::Value::String(intended_use.clone()),
                );
            }
            if let Some(out_of_scope_use) = &risk.out_of_scope_use {
                risk_map.insert(
                    serde_yaml::Value::String("outOfScopeUse".to_string()),
                    serde_yaml::Value::String(out_of_scope_use.clone()),
                );
            }
            if let Some(assessment) = &risk.assessment {
                let mut assess_map = serde_yaml::Mapping::new();
                if let Some(methodology) = &assessment.methodology {
                    assess_map.insert(
                        serde_yaml::Value::String("methodology".to_string()),
                        serde_yaml::Value::String(methodology.clone()),
                    );
                }
                if let Some(date) = &assessment.date {
                    assess_map.insert(
                        serde_yaml::Value::String("date".to_string()),
                        serde_yaml::Value::String(date.clone()),
                    );
                }
                if let Some(assessor) = &assessment.assessor {
                    assess_map.insert(
                        serde_yaml::Value::String("assessor".to_string()),
                        serde_yaml::Value::String(assessor.clone()),
                    );
                }
                if !assess_map.is_empty() {
                    risk_map.insert(
                        serde_yaml::Value::String("assessment".to_string()),
                        serde_yaml::Value::Mapping(assess_map),
                    );
                }
            }
            if let Some(mitigations) = &risk.mitigations {
                let mitigations_yaml: Vec<serde_yaml::Value> = mitigations
                    .iter()
                    .map(|mit| {
                        let mut mit_map = serde_yaml::Mapping::new();
                        mit_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(mit.description.clone()),
                        );
                        let status_str = match mit.status {
                            CADSMitigationStatus::Planned => "planned",
                            CADSMitigationStatus::Implemented => "implemented",
                            CADSMitigationStatus::Verified => "verified",
                        };
                        mit_map.insert(
                            serde_yaml::Value::String("status".to_string()),
                            serde_yaml::Value::String(status_str.to_string()),
                        );
                        serde_yaml::Value::Mapping(mit_map)
                    })
                    .collect();
                risk_map.insert(
                    serde_yaml::Value::String("mitigations".to_string()),
                    serde_yaml::Value::Sequence(mitigations_yaml),
                );
            }
            if !risk_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("risk".to_string()),
                    serde_yaml::Value::Mapping(risk_map),
                );
            }
        }

        if let Some(compliance) = &asset.compliance {
            let mut comp_map = serde_yaml::Mapping::new();
            if let Some(frameworks) = &compliance.frameworks {
                let frameworks_yaml: Vec<serde_yaml::Value> = frameworks
                    .iter()
                    .map(|fw| {
                        let mut fw_map = serde_yaml::Mapping::new();
                        fw_map.insert(
                            serde_yaml::Value::String("name".to_string()),
                            serde_yaml::Value::String(fw.name.clone()),
                        );
                        if let Some(category) = &fw.category {
                            fw_map.insert(
                                serde_yaml::Value::String("category".to_string()),
                                serde_yaml::Value::String(category.clone()),
                            );
                        }
                        let status_str = match fw.status {
                            CADSComplianceStatus::NotApplicable => "not_applicable",
                            CADSComplianceStatus::Assessed => "assessed",
                            CADSComplianceStatus::Compliant => "compliant",
                            CADSComplianceStatus::NonCompliant => "non_compliant",
                        };
                        fw_map.insert(
                            serde_yaml::Value::String("status".to_string()),
                            serde_yaml::Value::String(status_str.to_string()),
                        );
                        serde_yaml::Value::Mapping(fw_map)
                    })
                    .collect();
                comp_map.insert(
                    serde_yaml::Value::String("frameworks".to_string()),
                    serde_yaml::Value::Sequence(frameworks_yaml),
                );
            }
            if let Some(controls) = &compliance.controls {
                let controls_yaml: Vec<serde_yaml::Value> = controls
                    .iter()
                    .map(|ctrl| {
                        let mut ctrl_map = serde_yaml::Mapping::new();
                        ctrl_map.insert(
                            serde_yaml::Value::String("id".to_string()),
                            serde_yaml::Value::String(ctrl.id.clone()),
                        );
                        ctrl_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(ctrl.description.clone()),
                        );
                        if let Some(evidence) = &ctrl.evidence {
                            ctrl_map.insert(
                                serde_yaml::Value::String("evidence".to_string()),
                                serde_yaml::Value::String(evidence.clone()),
                            );
                        }
                        serde_yaml::Value::Mapping(ctrl_map)
                    })
                    .collect();
                comp_map.insert(
                    serde_yaml::Value::String("controls".to_string()),
                    serde_yaml::Value::Sequence(controls_yaml),
                );
            }
            if !comp_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("compliance".to_string()),
                    serde_yaml::Value::Mapping(comp_map),
                );
            }
        }

        if let Some(validation_profiles) = &asset.validation_profiles {
            let profiles_yaml: Vec<serde_yaml::Value> = validation_profiles
                .iter()
                .map(|profile| {
                    let mut profile_map = serde_yaml::Mapping::new();
                    profile_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(profile.name.clone()),
                    );
                    if let Some(applies_to) = &profile.applies_to {
                        let mut applies_map = serde_yaml::Mapping::new();
                        if let Some(kind) = &applies_to.kind {
                            applies_map.insert(
                                serde_yaml::Value::String("kind".to_string()),
                                serde_yaml::Value::String(kind.clone()),
                            );
                        }
                        if let Some(risk_classification) = &applies_to.risk_classification {
                            applies_map.insert(
                                serde_yaml::Value::String("riskClassification".to_string()),
                                serde_yaml::Value::String(risk_classification.clone()),
                            );
                        }
                        if !applies_map.is_empty() {
                            profile_map.insert(
                                serde_yaml::Value::String("appliesTo".to_string()),
                                serde_yaml::Value::Mapping(applies_map),
                            );
                        }
                    }
                    let checks_yaml: Vec<serde_yaml::Value> = profile
                        .required_checks
                        .iter()
                        .map(|c| serde_yaml::Value::String(c.clone()))
                        .collect();
                    profile_map.insert(
                        serde_yaml::Value::String("requiredChecks".to_string()),
                        serde_yaml::Value::Sequence(checks_yaml),
                    );
                    serde_yaml::Value::Mapping(profile_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("validationProfiles".to_string()),
                serde_yaml::Value::Sequence(profiles_yaml),
            );
        }

        if let Some(bpmn_models) = &asset.bpmn_models {
            let models_yaml: Vec<serde_yaml::Value> = bpmn_models
                .iter()
                .map(|model| {
                    let mut model_map = serde_yaml::Mapping::new();
                    model_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(model.name.clone()),
                    );
                    model_map.insert(
                        serde_yaml::Value::String("reference".to_string()),
                        serde_yaml::Value::String(model.reference.clone()),
                    );
                    let format_str = match model.format {
                        CADSBPMNFormat::Bpmn20Xml => "bpmn20-xml",
                        CADSBPMNFormat::Json => "json",
                    };
                    model_map.insert(
                        serde_yaml::Value::String("format".to_string()),
                        serde_yaml::Value::String(format_str.to_string()),
                    );
                    if let Some(description) = &model.description {
                        model_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(description.clone()),
                        );
                    }
                    serde_yaml::Value::Mapping(model_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("bpmnModels".to_string()),
                serde_yaml::Value::Sequence(models_yaml),
            );
        }

        if let Some(dmn_models) = &asset.dmn_models {
            let models_yaml: Vec<serde_yaml::Value> = dmn_models
                .iter()
                .map(|model| {
                    let mut model_map = serde_yaml::Mapping::new();
                    model_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(model.name.clone()),
                    );
                    model_map.insert(
                        serde_yaml::Value::String("reference".to_string()),
                        serde_yaml::Value::String(model.reference.clone()),
                    );
                    let format_str = match model.format {
                        CADSDMNFormat::Dmn13Xml => "dmn13-xml",
                    };
                    model_map.insert(
                        serde_yaml::Value::String("format".to_string()),
                        serde_yaml::Value::String(format_str.to_string()),
                    );
                    if let Some(description) = &model.description {
                        model_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(description.clone()),
                        );
                    }
                    serde_yaml::Value::Mapping(model_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("dmnModels".to_string()),
                serde_yaml::Value::Sequence(models_yaml),
            );
        }

        if let Some(openapi_specs) = &asset.openapi_specs {
            let specs_yaml: Vec<serde_yaml::Value> = openapi_specs
                .iter()
                .map(|spec| {
                    let mut spec_map = serde_yaml::Mapping::new();
                    spec_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(spec.name.clone()),
                    );
                    spec_map.insert(
                        serde_yaml::Value::String("reference".to_string()),
                        serde_yaml::Value::String(spec.reference.clone()),
                    );
                    let format_str = match spec.format {
                        CADSOpenAPIFormat::Openapi311Yaml => "openapi-311-yaml",
                        CADSOpenAPIFormat::Openapi311Json => "openapi-311-json",
                    };
                    spec_map.insert(
                        serde_yaml::Value::String("format".to_string()),
                        serde_yaml::Value::String(format_str.to_string()),
                    );
                    if let Some(description) = &spec.description {
                        spec_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(description.clone()),
                        );
                    }
                    serde_yaml::Value::Mapping(spec_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("openapiSpecs".to_string()),
                serde_yaml::Value::Sequence(specs_yaml),
            );
        }

        if let Some(custom_properties) = &asset.custom_properties {
            let mut custom_map = serde_yaml::Mapping::new();
            for (key, value) in custom_properties {
                custom_map.insert(
                    serde_yaml::Value::String(key.clone()),
                    Self::json_to_yaml_value(value),
                );
            }
            if !custom_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("customProperties".to_string()),
                    serde_yaml::Value::Mapping(custom_map),
                );
            }
        }

        // Serialize to YAML string
        serde_yaml::to_string(&serde_yaml::Value::Mapping(yaml))
            .unwrap_or_else(|_| String::from(""))
    }

    /// Helper to convert serde_json::Value to serde_yaml::Value
    fn json_to_yaml_value(json: &serde_json::Value) -> serde_yaml::Value {
        match json {
            serde_json::Value::Null => serde_yaml::Value::Null,
            serde_json::Value::Bool(b) => serde_yaml::Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    serde_yaml::Value::Number(serde_yaml::Number::from(i))
                } else if let Some(f) = n.as_f64() {
                    serde_yaml::Value::Number(serde_yaml::Number::from(f))
                } else {
                    serde_yaml::Value::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => serde_yaml::Value::String(s.clone()),
            serde_json::Value::Array(arr) => {
                let yaml_arr: Vec<serde_yaml::Value> =
                    arr.iter().map(Self::json_to_yaml_value).collect();
                serde_yaml::Value::Sequence(yaml_arr)
            }
            serde_json::Value::Object(obj) => {
                let mut yaml_map = serde_yaml::Mapping::new();
                for (k, v) in obj {
                    yaml_map.insert(
                        serde_yaml::Value::String(k.clone()),
                        Self::json_to_yaml_value(v),
                    );
                }
                serde_yaml::Value::Mapping(yaml_map)
            }
        }
    }
}
