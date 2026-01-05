//! ODPS (Open Data Product Standard) exporter
//!
//! Exports ODPSDataProduct models to ODPS YAML format.

use crate::export::ExportError;
use crate::models::odps::*;
use serde_yaml;

/// ODPS exporter for generating ODPS YAML from ODPSDataProduct models
pub struct ODPSExporter;

impl ODPSExporter {
    /// Export a Data Product to ODPS YAML format (instance method for WASM compatibility)
    ///
    /// # Arguments
    ///
    /// * `product` - The Data Product to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string in ODPS format, or an ExportError
    pub fn export(&self, product: &ODPSDataProduct) -> Result<String, ExportError> {
        let yaml = Self::export_product(product);

        // Validate exported YAML against ODPS schema (if feature enabled)
        #[cfg(feature = "odps-validation")]
        {
            #[cfg(feature = "cli")]
            {
                use crate::cli::validation::validate_odps_internal;
                validate_odps_internal(&yaml).map_err(ExportError::ValidationError)?;
            }
            #[cfg(not(feature = "cli"))]
            {
                // Inline validation when CLI feature is not enabled
                use jsonschema::Validator;
                use serde_json::Value;

                let schema_content = include_str!("../../schemas/odps-json-schema-latest.json");
                let schema: Value = serde_json::from_str(schema_content).map_err(|e| {
                    ExportError::ValidationError(format!("Failed to load ODPS schema: {}", e))
                })?;

                let validator = Validator::new(&schema).map_err(|e| {
                    ExportError::ValidationError(format!("Failed to compile ODPS schema: {}", e))
                })?;

                let data: Value = serde_yaml::from_str(&yaml).map_err(|e| {
                    ExportError::ValidationError(format!("Failed to parse YAML: {}", e))
                })?;

                if let Err(error) = validator.validate(&data) {
                    return Err(ExportError::ValidationError(format!(
                        "ODPS validation failed: {}",
                        error
                    )));
                }
            }
        }

        Ok(yaml)
    }

    /// Export a Data Product to ODPS YAML format
    ///
    /// # Arguments
    ///
    /// * `product` - The Data Product to export
    ///
    /// # Returns
    ///
    /// A YAML string in ODPS format
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::odps::ODPSExporter;
    /// use data_modelling_sdk::models::odps::*;
    ///
    /// let product = ODPSDataProduct {
    ///     api_version: "v1.0.0".to_string(),
    ///     kind: "DataProduct".to_string(),
    ///     id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    ///     name: Some("customer-data-product".to_string()),
    ///     version: Some("1.0.0".to_string()),
    ///     status: ODPSStatus::Active,
    ///     domain: None,
    ///     tenant: None,
    ///     authoritative_definitions: None,
    ///     description: None,
    ///     custom_properties: None,
    ///     tags: vec![],
    ///     input_ports: None,
    ///     output_ports: None,
    ///     management_ports: None,
    ///     support: None,
    ///     team: None,
    ///     product_created_ts: None,
    ///     created_at: None,
    ///     updated_at: None,
    /// };
    ///
    /// let yaml = ODPSExporter::export_product(&product);
    /// assert!(yaml.contains("apiVersion: v1.0.0"));
    /// assert!(yaml.contains("kind: DataProduct"));
    /// ```
    pub fn export_product(product: &ODPSDataProduct) -> String {
        let mut yaml = serde_yaml::Mapping::new();

        // Required fields
        yaml.insert(
            serde_yaml::Value::String("apiVersion".to_string()),
            serde_yaml::Value::String(product.api_version.clone()),
        );

        yaml.insert(
            serde_yaml::Value::String("kind".to_string()),
            serde_yaml::Value::String(product.kind.clone()),
        );

        yaml.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(product.id.clone()),
        );

        let status_str = match product.status {
            ODPSStatus::Proposed => "proposed",
            ODPSStatus::Draft => "draft",
            ODPSStatus::Active => "active",
            ODPSStatus::Deprecated => "deprecated",
            ODPSStatus::Retired => "retired",
        };
        yaml.insert(
            serde_yaml::Value::String("status".to_string()),
            serde_yaml::Value::String(status_str.to_string()),
        );

        // Optional fields
        if let Some(name) = &product.name {
            yaml.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String(name.clone()),
            );
        }

        if let Some(version) = &product.version {
            yaml.insert(
                serde_yaml::Value::String("version".to_string()),
                serde_yaml::Value::String(version.clone()),
            );
        }

        if let Some(domain) = &product.domain {
            yaml.insert(
                serde_yaml::Value::String("domain".to_string()),
                serde_yaml::Value::String(domain.clone()),
            );
        }

        if let Some(tenant) = &product.tenant {
            yaml.insert(
                serde_yaml::Value::String("tenant".to_string()),
                serde_yaml::Value::String(tenant.clone()),
            );
        }

        if !product.tags.is_empty() {
            let tags_yaml: Vec<serde_yaml::Value> = product
                .tags
                .iter()
                .map(|t| serde_yaml::Value::String(t.to_string()))
                .collect();
            yaml.insert(
                serde_yaml::Value::String("tags".to_string()),
                serde_yaml::Value::Sequence(tags_yaml),
            );
        }

        if let Some(description) = &product.description {
            let mut desc_map = serde_yaml::Mapping::new();
            if let Some(purpose) = &description.purpose {
                desc_map.insert(
                    serde_yaml::Value::String("purpose".to_string()),
                    serde_yaml::Value::String(purpose.clone()),
                );
            }
            if let Some(limitations) = &description.limitations {
                desc_map.insert(
                    serde_yaml::Value::String("limitations".to_string()),
                    serde_yaml::Value::String(limitations.clone()),
                );
            }
            if let Some(usage) = &description.usage {
                desc_map.insert(
                    serde_yaml::Value::String("usage".to_string()),
                    serde_yaml::Value::String(usage.clone()),
                );
            }
            if let Some(auth_defs) = &description.authoritative_definitions {
                let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
                if !defs_yaml.is_empty() {
                    desc_map.insert(
                        serde_yaml::Value::String("authoritativeDefinitions".to_string()),
                        serde_yaml::Value::Sequence(defs_yaml),
                    );
                }
            }
            if let Some(custom_props) = &description.custom_properties {
                let props_yaml = Self::serialize_custom_properties(custom_props);
                if !props_yaml.is_empty() {
                    desc_map.insert(
                        serde_yaml::Value::String("customProperties".to_string()),
                        serde_yaml::Value::Sequence(props_yaml),
                    );
                }
            }
            if !desc_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::Mapping(desc_map),
                );
            }
        }

        if let Some(auth_defs) = &product.authoritative_definitions {
            // Preserve empty arrays (include even if empty)
            let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
            yaml.insert(
                serde_yaml::Value::String("authoritativeDefinitions".to_string()),
                serde_yaml::Value::Sequence(defs_yaml),
            );
        }

        if let Some(custom_props) = &product.custom_properties {
            // Preserve empty arrays (include even if empty)
            let props_yaml = Self::serialize_custom_properties(custom_props);
            yaml.insert(
                serde_yaml::Value::String("customProperties".to_string()),
                serde_yaml::Value::Sequence(props_yaml),
            );
        }

        if let Some(input_ports) = &product.input_ports {
            // Preserve empty arrays (include even if empty)
            let ports_yaml: Vec<serde_yaml::Value> = input_ports
                .iter()
                .map(|port| {
                    let mut port_map = serde_yaml::Mapping::new();
                    port_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(port.name.clone()),
                    );
                    port_map.insert(
                        serde_yaml::Value::String("version".to_string()),
                        serde_yaml::Value::String(port.version.clone()),
                    );
                    port_map.insert(
                        serde_yaml::Value::String("contractId".to_string()),
                        serde_yaml::Value::String(port.contract_id.clone()),
                    );
                    if !port.tags.is_empty() {
                        let tags_yaml: Vec<serde_yaml::Value> = port
                            .tags
                            .iter()
                            .map(|t| serde_yaml::Value::String(t.to_string()))
                            .collect();
                        port_map.insert(
                            serde_yaml::Value::String("tags".to_string()),
                            serde_yaml::Value::Sequence(tags_yaml),
                        );
                    }
                    if let Some(custom_props) = &port.custom_properties {
                        let props_yaml = Self::serialize_custom_properties(custom_props);
                        if !props_yaml.is_empty() {
                            port_map.insert(
                                serde_yaml::Value::String("customProperties".to_string()),
                                serde_yaml::Value::Sequence(props_yaml),
                            );
                        }
                    }
                    if let Some(auth_defs) = &port.authoritative_definitions {
                        let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
                        if !defs_yaml.is_empty() {
                            port_map.insert(
                                serde_yaml::Value::String("authoritativeDefinitions".to_string()),
                                serde_yaml::Value::Sequence(defs_yaml),
                            );
                        }
                    }
                    serde_yaml::Value::Mapping(port_map)
                })
                .collect();
            // Always include inputPorts, even if empty, to preserve structure
            yaml.insert(
                serde_yaml::Value::String("inputPorts".to_string()),
                serde_yaml::Value::Sequence(ports_yaml),
            );
        }

        if let Some(output_ports) = &product.output_ports {
            let ports_yaml: Vec<serde_yaml::Value> = output_ports
                .iter()
                .map(|port| {
                    let mut port_map = serde_yaml::Mapping::new();
                    port_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(port.name.clone()),
                    );
                    port_map.insert(
                        serde_yaml::Value::String("version".to_string()),
                        serde_yaml::Value::String(port.version.clone()),
                    );
                    if let Some(description) = &port.description {
                        port_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(description.clone()),
                        );
                    }
                    if let Some(r#type) = &port.r#type {
                        port_map.insert(
                            serde_yaml::Value::String("type".to_string()),
                            serde_yaml::Value::String(r#type.clone()),
                        );
                    }
                    if let Some(contract_id) = &port.contract_id {
                        port_map.insert(
                            serde_yaml::Value::String("contractId".to_string()),
                            serde_yaml::Value::String(contract_id.clone()),
                        );
                    }
                    if let Some(sbom) = &port.sbom {
                        let sbom_yaml: Vec<serde_yaml::Value> = sbom
                            .iter()
                            .map(|s| {
                                let mut sbom_map = serde_yaml::Mapping::new();
                                sbom_map.insert(
                                    serde_yaml::Value::String("url".to_string()),
                                    serde_yaml::Value::String(s.url.clone()),
                                );
                                if let Some(r#type) = &s.r#type {
                                    sbom_map.insert(
                                        serde_yaml::Value::String("type".to_string()),
                                        serde_yaml::Value::String(r#type.clone()),
                                    );
                                }
                                serde_yaml::Value::Mapping(sbom_map)
                            })
                            .collect();
                        port_map.insert(
                            serde_yaml::Value::String("sbom".to_string()),
                            serde_yaml::Value::Sequence(sbom_yaml),
                        );
                    }
                    if let Some(input_contracts) = &port.input_contracts {
                        let contracts_yaml: Vec<serde_yaml::Value> = input_contracts
                            .iter()
                            .map(|contract| {
                                let mut contract_map = serde_yaml::Mapping::new();
                                contract_map.insert(
                                    serde_yaml::Value::String("id".to_string()),
                                    serde_yaml::Value::String(contract.id.clone()),
                                );
                                contract_map.insert(
                                    serde_yaml::Value::String("version".to_string()),
                                    serde_yaml::Value::String(contract.version.clone()),
                                );
                                serde_yaml::Value::Mapping(contract_map)
                            })
                            .collect();
                        port_map.insert(
                            serde_yaml::Value::String("inputContracts".to_string()),
                            serde_yaml::Value::Sequence(contracts_yaml),
                        );
                    }
                    if !port.tags.is_empty() {
                        let tags_yaml: Vec<serde_yaml::Value> = port
                            .tags
                            .iter()
                            .map(|t| serde_yaml::Value::String(t.to_string()))
                            .collect();
                        port_map.insert(
                            serde_yaml::Value::String("tags".to_string()),
                            serde_yaml::Value::Sequence(tags_yaml),
                        );
                    }
                    if let Some(custom_props) = &port.custom_properties {
                        let props_yaml = Self::serialize_custom_properties(custom_props);
                        if !props_yaml.is_empty() {
                            port_map.insert(
                                serde_yaml::Value::String("customProperties".to_string()),
                                serde_yaml::Value::Sequence(props_yaml),
                            );
                        }
                    }
                    if let Some(auth_defs) = &port.authoritative_definitions {
                        let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
                        if !defs_yaml.is_empty() {
                            port_map.insert(
                                serde_yaml::Value::String("authoritativeDefinitions".to_string()),
                                serde_yaml::Value::Sequence(defs_yaml),
                            );
                        }
                    }
                    serde_yaml::Value::Mapping(port_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("outputPorts".to_string()),
                serde_yaml::Value::Sequence(ports_yaml),
            );
        }

        if let Some(management_ports) = &product.management_ports {
            let ports_yaml: Vec<serde_yaml::Value> = management_ports
                .iter()
                .map(|port| {
                    let mut port_map = serde_yaml::Mapping::new();
                    port_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(port.name.clone()),
                    );
                    port_map.insert(
                        serde_yaml::Value::String("content".to_string()),
                        serde_yaml::Value::String(port.content.clone()),
                    );
                    if let Some(r#type) = &port.r#type {
                        port_map.insert(
                            serde_yaml::Value::String("type".to_string()),
                            serde_yaml::Value::String(r#type.clone()),
                        );
                    }
                    if let Some(url) = &port.url {
                        port_map.insert(
                            serde_yaml::Value::String("url".to_string()),
                            serde_yaml::Value::String(url.clone()),
                        );
                    }
                    if let Some(channel) = &port.channel {
                        port_map.insert(
                            serde_yaml::Value::String("channel".to_string()),
                            serde_yaml::Value::String(channel.clone()),
                        );
                    }
                    if let Some(description) = &port.description {
                        port_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(description.clone()),
                        );
                    }
                    if !port.tags.is_empty() {
                        let tags_yaml: Vec<serde_yaml::Value> = port
                            .tags
                            .iter()
                            .map(|t| serde_yaml::Value::String(t.to_string()))
                            .collect();
                        port_map.insert(
                            serde_yaml::Value::String("tags".to_string()),
                            serde_yaml::Value::Sequence(tags_yaml),
                        );
                    }
                    if let Some(custom_props) = &port.custom_properties {
                        let props_yaml = Self::serialize_custom_properties(custom_props);
                        if !props_yaml.is_empty() {
                            port_map.insert(
                                serde_yaml::Value::String("customProperties".to_string()),
                                serde_yaml::Value::Sequence(props_yaml),
                            );
                        }
                    }
                    if let Some(auth_defs) = &port.authoritative_definitions {
                        let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
                        if !defs_yaml.is_empty() {
                            port_map.insert(
                                serde_yaml::Value::String("authoritativeDefinitions".to_string()),
                                serde_yaml::Value::Sequence(defs_yaml),
                            );
                        }
                    }
                    serde_yaml::Value::Mapping(port_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("managementPorts".to_string()),
                serde_yaml::Value::Sequence(ports_yaml),
            );
        }

        if let Some(support) = &product.support {
            let support_yaml: Vec<serde_yaml::Value> = support
                .iter()
                .map(|s| {
                    let mut support_map = serde_yaml::Mapping::new();
                    support_map.insert(
                        serde_yaml::Value::String("channel".to_string()),
                        serde_yaml::Value::String(s.channel.clone()),
                    );
                    support_map.insert(
                        serde_yaml::Value::String("url".to_string()),
                        serde_yaml::Value::String(s.url.clone()),
                    );
                    if let Some(description) = &s.description {
                        support_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(description.clone()),
                        );
                    }
                    if let Some(tool) = &s.tool {
                        support_map.insert(
                            serde_yaml::Value::String("tool".to_string()),
                            serde_yaml::Value::String(tool.clone()),
                        );
                    }
                    if let Some(scope) = &s.scope {
                        support_map.insert(
                            serde_yaml::Value::String("scope".to_string()),
                            serde_yaml::Value::String(scope.clone()),
                        );
                    }
                    if let Some(invitation_url) = &s.invitation_url {
                        support_map.insert(
                            serde_yaml::Value::String("invitationUrl".to_string()),
                            serde_yaml::Value::String(invitation_url.clone()),
                        );
                    }
                    if !s.tags.is_empty() {
                        let tags_yaml: Vec<serde_yaml::Value> = s
                            .tags
                            .iter()
                            .map(|t| serde_yaml::Value::String(t.to_string()))
                            .collect();
                        support_map.insert(
                            serde_yaml::Value::String("tags".to_string()),
                            serde_yaml::Value::Sequence(tags_yaml),
                        );
                    }
                    if let Some(custom_props) = &s.custom_properties {
                        let props_yaml = Self::serialize_custom_properties(custom_props);
                        if !props_yaml.is_empty() {
                            support_map.insert(
                                serde_yaml::Value::String("customProperties".to_string()),
                                serde_yaml::Value::Sequence(props_yaml),
                            );
                        }
                    }
                    if let Some(auth_defs) = &s.authoritative_definitions {
                        let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
                        if !defs_yaml.is_empty() {
                            support_map.insert(
                                serde_yaml::Value::String("authoritativeDefinitions".to_string()),
                                serde_yaml::Value::Sequence(defs_yaml),
                            );
                        }
                    }
                    serde_yaml::Value::Mapping(support_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("support".to_string()),
                serde_yaml::Value::Sequence(support_yaml),
            );
        }

        if let Some(team) = &product.team {
            let mut team_map = serde_yaml::Mapping::new();
            if let Some(name) = &team.name {
                team_map.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String(name.clone()),
                );
            }
            if let Some(description) = &team.description {
                team_map.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::String(description.clone()),
                );
            }
            if let Some(members) = &team.members {
                let members_yaml: Vec<serde_yaml::Value> = members
                    .iter()
                    .map(|member| {
                        let mut member_map = serde_yaml::Mapping::new();
                        member_map.insert(
                            serde_yaml::Value::String("username".to_string()),
                            serde_yaml::Value::String(member.username.clone()),
                        );
                        if let Some(name) = &member.name {
                            member_map.insert(
                                serde_yaml::Value::String("name".to_string()),
                                serde_yaml::Value::String(name.clone()),
                            );
                        }
                        if let Some(description) = &member.description {
                            member_map.insert(
                                serde_yaml::Value::String("description".to_string()),
                                serde_yaml::Value::String(description.clone()),
                            );
                        }
                        if let Some(role) = &member.role {
                            member_map.insert(
                                serde_yaml::Value::String("role".to_string()),
                                serde_yaml::Value::String(role.clone()),
                            );
                        }
                        if let Some(date_in) = &member.date_in {
                            member_map.insert(
                                serde_yaml::Value::String("dateIn".to_string()),
                                serde_yaml::Value::String(date_in.clone()),
                            );
                        }
                        if let Some(date_out) = &member.date_out {
                            member_map.insert(
                                serde_yaml::Value::String("dateOut".to_string()),
                                serde_yaml::Value::String(date_out.clone()),
                            );
                        }
                        if let Some(replaced_by) = &member.replaced_by_username {
                            member_map.insert(
                                serde_yaml::Value::String("replacedByUsername".to_string()),
                                serde_yaml::Value::String(replaced_by.clone()),
                            );
                        }
                        if !member.tags.is_empty() {
                            let tags_yaml: Vec<serde_yaml::Value> = member
                                .tags
                                .iter()
                                .map(|t| serde_yaml::Value::String(t.to_string()))
                                .collect();
                            member_map.insert(
                                serde_yaml::Value::String("tags".to_string()),
                                serde_yaml::Value::Sequence(tags_yaml),
                            );
                        }
                        if let Some(custom_props) = &member.custom_properties {
                            let props_yaml = Self::serialize_custom_properties(custom_props);
                            if !props_yaml.is_empty() {
                                member_map.insert(
                                    serde_yaml::Value::String("customProperties".to_string()),
                                    serde_yaml::Value::Sequence(props_yaml),
                                );
                            }
                        }
                        if let Some(auth_defs) = &member.authoritative_definitions {
                            let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
                            if !defs_yaml.is_empty() {
                                member_map.insert(
                                    serde_yaml::Value::String(
                                        "authoritativeDefinitions".to_string(),
                                    ),
                                    serde_yaml::Value::Sequence(defs_yaml),
                                );
                            }
                        }
                        serde_yaml::Value::Mapping(member_map)
                    })
                    .collect();
                team_map.insert(
                    serde_yaml::Value::String("members".to_string()),
                    serde_yaml::Value::Sequence(members_yaml),
                );
            }
            if !team.tags.is_empty() {
                let tags_yaml: Vec<serde_yaml::Value> = team
                    .tags
                    .iter()
                    .map(|t| serde_yaml::Value::String(t.to_string()))
                    .collect();
                team_map.insert(
                    serde_yaml::Value::String("tags".to_string()),
                    serde_yaml::Value::Sequence(tags_yaml),
                );
            }
            if let Some(custom_props) = &team.custom_properties {
                let props_yaml = Self::serialize_custom_properties(custom_props);
                if !props_yaml.is_empty() {
                    team_map.insert(
                        serde_yaml::Value::String("customProperties".to_string()),
                        serde_yaml::Value::Sequence(props_yaml),
                    );
                }
            }
            if let Some(auth_defs) = &team.authoritative_definitions {
                let defs_yaml = Self::serialize_authoritative_definitions(auth_defs);
                if !defs_yaml.is_empty() {
                    team_map.insert(
                        serde_yaml::Value::String("authoritativeDefinitions".to_string()),
                        serde_yaml::Value::Sequence(defs_yaml),
                    );
                }
            }
            if !team_map.is_empty() {
                yaml.insert(
                    serde_yaml::Value::String("team".to_string()),
                    serde_yaml::Value::Mapping(team_map),
                );
            }
        }

        if let Some(product_created_ts) = &product.product_created_ts {
            yaml.insert(
                serde_yaml::Value::String("productCreatedTs".to_string()),
                serde_yaml::Value::String(product_created_ts.clone()),
            );
        }

        // Serialize to YAML string
        serde_yaml::to_string(&serde_yaml::Value::Mapping(yaml))
            .unwrap_or_else(|_| String::from(""))
    }

    /// Serialize authoritative definitions
    fn serialize_authoritative_definitions(
        defs: &[ODPSAuthoritativeDefinition],
    ) -> Vec<serde_yaml::Value> {
        defs.iter()
            .map(|def| {
                let mut def_map = serde_yaml::Mapping::new();
                def_map.insert(
                    serde_yaml::Value::String("type".to_string()),
                    serde_yaml::Value::String(def.r#type.clone()),
                );
                def_map.insert(
                    serde_yaml::Value::String("url".to_string()),
                    serde_yaml::Value::String(def.url.clone()),
                );
                if let Some(description) = &def.description {
                    def_map.insert(
                        serde_yaml::Value::String("description".to_string()),
                        serde_yaml::Value::String(description.clone()),
                    );
                }
                serde_yaml::Value::Mapping(def_map)
            })
            .collect()
    }

    /// Serialize custom properties
    fn serialize_custom_properties(props: &[ODPSCustomProperty]) -> Vec<serde_yaml::Value> {
        props
            .iter()
            .map(|prop| {
                let mut prop_map = serde_yaml::Mapping::new();
                prop_map.insert(
                    serde_yaml::Value::String("property".to_string()),
                    serde_yaml::Value::String(prop.property.clone()),
                );
                prop_map.insert(
                    serde_yaml::Value::String("value".to_string()),
                    Self::json_to_yaml_value(&prop.value),
                );
                if let Some(description) = &prop.description {
                    prop_map.insert(
                        serde_yaml::Value::String("description".to_string()),
                        serde_yaml::Value::String(description.clone()),
                    );
                }
                serde_yaml::Value::Mapping(prop_map)
            })
            .collect()
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
