//! CADS (Compute Asset Description Specification) importer
//!
//! Parses CADS v1.0 YAML files and converts them to CADSAsset models.

use super::ImportError;
use crate::models::Tag;
use crate::models::cads::*;
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::str::FromStr;

/// CADS importer for parsing CADS v1.0 YAML files
pub struct CADSImporter;

impl CADSImporter {
    /// Create a new CADS importer instance
    pub fn new() -> Self {
        Self
    }

    /// Import CADS YAML content and create CADSAsset
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - CADS YAML content as a string
    ///
    /// # Returns
    ///
    /// A `CADSAsset` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::cads::CADSImporter;
    ///
    /// let importer = CADSImporter::new();
    /// let yaml = r#"
    /// apiVersion: v1.0
    /// kind: AIModel
    /// id: 550e8400-e29b-41d4-a716-446655440000
    /// name: sentiment-analysis-model
    /// version: 1.0.0
    /// status: production
    /// "#;
    /// let asset = importer.import(yaml).unwrap();
    /// assert_eq!(asset.name, "sentiment-analysis-model");
    /// ```
    pub fn import(&self, yaml_content: &str) -> Result<CADSAsset, ImportError> {
        let yaml_value: YamlValue = serde_yaml::from_str(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse YAML: {}", e)))?;

        self.parse_cads_asset(&yaml_value)
            .map_err(|e| ImportError::ParseError(e.to_string()))
    }

    /// Parse CADS asset from YAML value
    fn parse_cads_asset(&self, yaml: &YamlValue) -> Result<CADSAsset> {
        let _obj = yaml
            .as_mapping()
            .ok_or_else(|| anyhow::anyhow!("CADS YAML must be a mapping"))?;

        // Convert YAML to JSON for easier parsing
        let json_value: JsonValue =
            serde_json::to_value(yaml).context("Failed to convert YAML to JSON")?;

        // Parse required fields
        let api_version = json_value
            .get("apiVersion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: apiVersion"))?
            .to_string();

        let kind_str = json_value
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: kind"))?;

        let kind = match kind_str {
            "AIModel" => CADSKind::AIModel,
            "MLPipeline" => CADSKind::MLPipeline,
            "Application" => CADSKind::Application,
            "ETLPipeline" => CADSKind::ETLPipeline,
            "SourceSystem" => CADSKind::SourceSystem,
            "DestinationSystem" => CADSKind::DestinationSystem,
            _ => return Err(anyhow::anyhow!("Invalid kind: {}", kind_str)),
        };

        let id = json_value
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: id"))?
            .to_string();

        let name = json_value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: name"))?
            .to_string();

        let version = json_value
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: version"))?
            .to_string();

        let status_str = json_value
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: status"))?;

        let status = match status_str {
            "draft" => CADSStatus::Draft,
            "validated" => CADSStatus::Validated,
            "production" => CADSStatus::Production,
            "deprecated" => CADSStatus::Deprecated,
            _ => return Err(anyhow::anyhow!("Invalid status: {}", status_str)),
        };

        // Parse optional fields
        let domain = json_value
            .get("domain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = self.parse_tags(&json_value)?;
        let description = self.parse_description(&json_value)?;
        let runtime = self.parse_runtime(&json_value)?;
        let sla = self.parse_sla(&json_value)?;
        let pricing = self.parse_pricing(&json_value)?;
        let team = self.parse_team(&json_value)?;
        let risk = self.parse_risk(&json_value)?;
        let compliance = self.parse_compliance(&json_value)?;
        let validation_profiles = self.parse_validation_profiles(&json_value)?;
        let bpmn_models = self.parse_bpmn_models(&json_value)?;
        let dmn_models = self.parse_dmn_models(&json_value)?;
        let openapi_specs = self.parse_openapi_specs(&json_value)?;
        let custom_properties = self.parse_custom_properties(&json_value)?;

        Ok(CADSAsset {
            api_version,
            kind,
            id,
            name,
            version,
            status,
            domain,
            tags,
            description,
            runtime,
            sla,
            pricing,
            team,
            risk,
            compliance,
            validation_profiles,
            bpmn_models,
            dmn_models,
            openapi_specs,
            custom_properties,
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        })
    }

    /// Parse tags array
    fn parse_tags(&self, json: &JsonValue) -> Result<Vec<Tag>> {
        let mut tags = Vec::new();
        if let Some(tags_arr) = json.get("tags").and_then(|v| v.as_array()) {
            for item in tags_arr {
                if let Some(s) = item.as_str() {
                    if let Ok(tag) = Tag::from_str(s) {
                        tags.push(tag);
                    } else {
                        tags.push(Tag::Simple(s.to_string()));
                    }
                }
            }
        }
        Ok(tags)
    }

    /// Parse description object
    fn parse_description(&self, json: &JsonValue) -> Result<Option<CADSDescription>> {
        if let Some(desc_obj) = json.get("description").and_then(|v| v.as_object()) {
            let purpose = desc_obj
                .get("purpose")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let usage = desc_obj
                .get("usage")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let limitations = desc_obj
                .get("limitations")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let external_links =
                if let Some(links_arr) = desc_obj.get("externalLinks").and_then(|v| v.as_array()) {
                    let mut links = Vec::new();
                    for link_item in links_arr {
                        if let Some(link_obj) = link_item.as_object()
                            && let Some(url) = link_obj.get("url").and_then(|v| v.as_str())
                        {
                            links.push(CADSExternalLink {
                                url: url.to_string(),
                                description: link_obj
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            });
                        }
                    }
                    if !links.is_empty() { Some(links) } else { None }
                } else {
                    None
                };

            Ok(Some(CADSDescription {
                purpose,
                usage,
                limitations,
                external_links,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse runtime object
    fn parse_runtime(&self, json: &JsonValue) -> Result<Option<CADSRuntime>> {
        if let Some(runtime_obj) = json.get("runtime").and_then(|v| v.as_object()) {
            let environment = runtime_obj
                .get("environment")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let endpoints = if let Some(endpoints_arr) =
                runtime_obj.get("endpoints").and_then(|v| v.as_array())
            {
                let mut eps = Vec::new();
                for ep in endpoints_arr {
                    if let Some(s) = ep.as_str() {
                        eps.push(s.to_string());
                    }
                }
                if !eps.is_empty() { Some(eps) } else { None }
            } else {
                None
            };

            let container = runtime_obj
                .get("container")
                .and_then(|v| v.as_object())
                .map(|container_obj| CADSRuntimeContainer {
                    image: container_obj
                        .get("image")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });

            let resources = runtime_obj
                .get("resources")
                .and_then(|v| v.as_object())
                .map(|resources_obj| CADSRuntimeResources {
                    cpu: resources_obj
                        .get("cpu")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    memory: resources_obj
                        .get("memory")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    gpu: resources_obj
                        .get("gpu")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });

            Ok(Some(CADSRuntime {
                environment,
                endpoints,
                container,
                resources,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse SLA object
    fn parse_sla(&self, json: &JsonValue) -> Result<Option<CADSSLA>> {
        if let Some(sla_obj) = json.get("sla").and_then(|v| v.as_object()) {
            let properties =
                if let Some(props_arr) = sla_obj.get("properties").and_then(|v| v.as_array()) {
                    let mut props = Vec::new();
                    for prop_item in props_arr {
                        if let Some(prop_obj) = prop_item.as_object()
                            && let (Some(element), Some(value), Some(unit)) = (
                                prop_obj.get("element").and_then(|v| v.as_str()),
                                prop_obj.get("value"),
                                prop_obj.get("unit").and_then(|v| v.as_str()),
                            )
                        {
                            props.push(CADSSLAProperty {
                                element: element.to_string(),
                                value: value.clone(),
                                unit: unit.to_string(),
                                driver: prop_obj
                                    .get("driver")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            });
                        }
                    }
                    if !props.is_empty() { Some(props) } else { None }
                } else {
                    None
                };

            Ok(Some(CADSSLA { properties }))
        } else {
            Ok(None)
        }
    }

    /// Parse pricing object
    fn parse_pricing(&self, json: &JsonValue) -> Result<Option<CADSPricing>> {
        if let Some(pricing_obj) = json.get("pricing").and_then(|v| v.as_object()) {
            let model = pricing_obj
                .get("model")
                .and_then(|v| v.as_str())
                .and_then(|s| match s {
                    "per_request" => Some(CADSPricingModel::PerRequest),
                    "per_hour" => Some(CADSPricingModel::PerHour),
                    "per_batch" => Some(CADSPricingModel::PerBatch),
                    "subscription" => Some(CADSPricingModel::Subscription),
                    "internal" => Some(CADSPricingModel::Internal),
                    _ => None,
                });

            Ok(Some(CADSPricing {
                model,
                currency: pricing_obj
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                unit_cost: pricing_obj.get("unitCost").and_then(|v| v.as_f64()),
                billing_unit: pricing_obj
                    .get("billingUnit")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                notes: pricing_obj
                    .get("notes")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse team array
    fn parse_team(&self, json: &JsonValue) -> Result<Option<Vec<CADSTeamMember>>> {
        if let Some(team_arr) = json.get("team").and_then(|v| v.as_array()) {
            let mut team = Vec::new();
            for member_item in team_arr {
                if let Some(member_obj) = member_item.as_object()
                    && let (Some(role), Some(name)) = (
                        member_obj.get("role").and_then(|v| v.as_str()),
                        member_obj.get("name").and_then(|v| v.as_str()),
                    )
                {
                    team.push(CADSTeamMember {
                        role: role.to_string(),
                        name: name.to_string(),
                        contact: member_obj
                            .get("contact")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
            if !team.is_empty() {
                Ok(Some(team))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Parse risk object
    fn parse_risk(&self, json: &JsonValue) -> Result<Option<CADSRisk>> {
        if let Some(risk_obj) = json.get("risk").and_then(|v| v.as_object()) {
            let classification = risk_obj
                .get("classification")
                .and_then(|v| v.as_str())
                .and_then(|s| match s {
                    "minimal" => Some(CADSRiskClassification::Minimal),
                    "low" => Some(CADSRiskClassification::Low),
                    "medium" => Some(CADSRiskClassification::Medium),
                    "high" => Some(CADSRiskClassification::High),
                    _ => None,
                });

            let impact_areas =
                if let Some(areas_arr) = risk_obj.get("impactAreas").and_then(|v| v.as_array()) {
                    let mut areas = Vec::new();
                    for area_item in areas_arr {
                        if let Some(s) = area_item.as_str()
                            && let Ok(area) =
                                serde_json::from_str::<CADSImpactArea>(&format!("\"{}\"", s))
                        {
                            areas.push(area);
                        }
                    }
                    if !areas.is_empty() { Some(areas) } else { None }
                } else {
                    None
                };

            let assessment =
                risk_obj
                    .get("assessment")
                    .and_then(|v| v.as_object())
                    .map(|assess_obj| CADSRiskAssessment {
                        methodology: assess_obj
                            .get("methodology")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        date: assess_obj
                            .get("date")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        assessor: assess_obj
                            .get("assessor")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });

            let mitigations =
                if let Some(mit_arr) = risk_obj.get("mitigations").and_then(|v| v.as_array()) {
                    let mut mitigations = Vec::new();
                    for mit_item in mit_arr {
                        if let Some(mit_obj) = mit_item.as_object()
                            && let (Some(description), Some(status_str)) = (
                                mit_obj.get("description").and_then(|v| v.as_str()),
                                mit_obj.get("status").and_then(|v| v.as_str()),
                            )
                        {
                            let status = match status_str {
                                "planned" => CADSMitigationStatus::Planned,
                                "implemented" => CADSMitigationStatus::Implemented,
                                "verified" => CADSMitigationStatus::Verified,
                                _ => continue,
                            };
                            mitigations.push(CADSRiskMitigation {
                                description: description.to_string(),
                                status,
                            });
                        }
                    }
                    if !mitigations.is_empty() {
                        Some(mitigations)
                    } else {
                        None
                    }
                } else {
                    None
                };

            Ok(Some(CADSRisk {
                classification,
                impact_areas,
                intended_use: risk_obj
                    .get("intendedUse")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                out_of_scope_use: risk_obj
                    .get("outOfScopeUse")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                assessment,
                mitigations,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse compliance object
    fn parse_compliance(&self, json: &JsonValue) -> Result<Option<CADSCompliance>> {
        if let Some(comp_obj) = json.get("compliance").and_then(|v| v.as_object()) {
            let frameworks = if let Some(frameworks_arr) =
                comp_obj.get("frameworks").and_then(|v| v.as_array())
            {
                let mut frameworks = Vec::new();
                for fw_item in frameworks_arr {
                    if let Some(fw_obj) = fw_item.as_object()
                        && let (Some(name), Some(status_str)) = (
                            fw_obj.get("name").and_then(|v| v.as_str()),
                            fw_obj.get("status").and_then(|v| v.as_str()),
                        )
                    {
                        let status = match status_str {
                            "not_applicable" => CADSComplianceStatus::NotApplicable,
                            "assessed" => CADSComplianceStatus::Assessed,
                            "compliant" => CADSComplianceStatus::Compliant,
                            "non_compliant" => CADSComplianceStatus::NonCompliant,
                            _ => continue,
                        };
                        frameworks.push(CADSComplianceFramework {
                            name: name.to_string(),
                            category: fw_obj
                                .get("category")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            status,
                        });
                    }
                }
                if !frameworks.is_empty() {
                    Some(frameworks)
                } else {
                    None
                }
            } else {
                None
            };

            let controls =
                if let Some(controls_arr) = comp_obj.get("controls").and_then(|v| v.as_array()) {
                    let mut controls = Vec::new();
                    for ctrl_item in controls_arr {
                        if let Some(ctrl_obj) = ctrl_item.as_object()
                            && let (Some(id), Some(description)) = (
                                ctrl_obj.get("id").and_then(|v| v.as_str()),
                                ctrl_obj.get("description").and_then(|v| v.as_str()),
                            )
                        {
                            controls.push(CADSComplianceControl {
                                id: id.to_string(),
                                description: description.to_string(),
                                evidence: ctrl_obj
                                    .get("evidence")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            });
                        }
                    }
                    if !controls.is_empty() {
                        Some(controls)
                    } else {
                        None
                    }
                } else {
                    None
                };

            Ok(Some(CADSCompliance {
                frameworks,
                controls,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse validation profiles array
    fn parse_validation_profiles(
        &self,
        json: &JsonValue,
    ) -> Result<Option<Vec<CADSValidationProfile>>> {
        if let Some(profiles_arr) = json.get("validationProfiles").and_then(|v| v.as_array()) {
            let mut profiles = Vec::new();
            for profile_item in profiles_arr {
                if let Some(profile_obj) = profile_item.as_object()
                    && let (Some(name), Some(checks_arr)) = (
                        profile_obj.get("name").and_then(|v| v.as_str()),
                        profile_obj.get("requiredChecks").and_then(|v| v.as_array()),
                    )
                {
                    let mut required_checks = Vec::new();
                    for check_item in checks_arr {
                        if let Some(s) = check_item.as_str() {
                            required_checks.push(s.to_string());
                        }
                    }

                    let applies_to = profile_obj
                        .get("appliesTo")
                        .and_then(|v| v.as_object())
                        .map(|applies_obj| CADSValidationProfileAppliesTo {
                            kind: applies_obj
                                .get("kind")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            risk_classification: applies_obj
                                .get("riskClassification")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        });

                    profiles.push(CADSValidationProfile {
                        name: name.to_string(),
                        applies_to,
                        required_checks,
                    });
                }
            }
            if !profiles.is_empty() {
                Ok(Some(profiles))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Parse BPMN models array
    fn parse_bpmn_models(&self, json: &JsonValue) -> Result<Option<Vec<CADSBPMNModel>>> {
        if let Some(models_arr) = json.get("bpmnModels").and_then(|v| v.as_array()) {
            let mut models = Vec::new();
            for model_item in models_arr {
                if let Some(model_obj) = model_item.as_object()
                    && let (Some(name), Some(reference), Some(format_str)) = (
                        model_obj.get("name").and_then(|v| v.as_str()),
                        model_obj.get("reference").and_then(|v| v.as_str()),
                        model_obj.get("format").and_then(|v| v.as_str()),
                    )
                {
                    let format = match format_str {
                        "bpmn20-xml" => CADSBPMNFormat::Bpmn20Xml,
                        "json" => CADSBPMNFormat::Json,
                        _ => continue,
                    };

                    models.push(CADSBPMNModel {
                        name: name.to_string(),
                        reference: reference.to_string(),
                        format,
                        description: model_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
            if !models.is_empty() {
                Ok(Some(models))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Parse DMN models array
    fn parse_dmn_models(&self, json: &JsonValue) -> Result<Option<Vec<CADSDMNModel>>> {
        if let Some(models_arr) = json.get("dmnModels").and_then(|v| v.as_array()) {
            let mut models = Vec::new();
            for model_item in models_arr {
                if let Some(model_obj) = model_item.as_object()
                    && let (Some(name), Some(reference), Some(format_str)) = (
                        model_obj.get("name").and_then(|v| v.as_str()),
                        model_obj.get("reference").and_then(|v| v.as_str()),
                        model_obj.get("format").and_then(|v| v.as_str()),
                    )
                {
                    let format = match format_str {
                        "dmn13-xml" => CADSDMNFormat::Dmn13Xml,
                        _ => continue,
                    };

                    models.push(CADSDMNModel {
                        name: name.to_string(),
                        reference: reference.to_string(),
                        format,
                        description: model_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
            if !models.is_empty() {
                Ok(Some(models))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Parse OpenAPI specs array
    fn parse_openapi_specs(&self, json: &JsonValue) -> Result<Option<Vec<CADSOpenAPISpec>>> {
        if let Some(specs_arr) = json.get("openapiSpecs").and_then(|v| v.as_array()) {
            let mut specs = Vec::new();
            for spec_item in specs_arr {
                if let Some(spec_obj) = spec_item.as_object()
                    && let (Some(name), Some(reference), Some(format_str)) = (
                        spec_obj.get("name").and_then(|v| v.as_str()),
                        spec_obj.get("reference").and_then(|v| v.as_str()),
                        spec_obj.get("format").and_then(|v| v.as_str()),
                    )
                {
                    let format = match format_str {
                        "openapi-311-yaml" => CADSOpenAPIFormat::Openapi311Yaml,
                        "openapi-311-json" => CADSOpenAPIFormat::Openapi311Json,
                        _ => continue,
                    };

                    specs.push(CADSOpenAPISpec {
                        name: name.to_string(),
                        reference: reference.to_string(),
                        format,
                        description: spec_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
            if !specs.is_empty() {
                Ok(Some(specs))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Parse custom properties object
    fn parse_custom_properties(
        &self,
        json: &JsonValue,
    ) -> Result<Option<HashMap<String, serde_json::Value>>> {
        if let Some(custom_obj) = json.get("customProperties").and_then(|v| v.as_object()) {
            let mut props = HashMap::new();
            for (key, value) in custom_obj {
                props.insert(key.clone(), value.clone());
            }
            if !props.is_empty() {
                Ok(Some(props))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl Default for CADSImporter {
    fn default() -> Self {
        Self::new()
    }
}
