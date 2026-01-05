//! ODPS (Open Data Product Standard) importer
//!
//! Parses ODPS YAML files and converts them to ODPSDataProduct models.

use super::ImportError;
use crate::models::Tag;
use crate::models::odps::*;
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::str::FromStr;

/// ODPS importer for parsing ODPS YAML files
pub struct ODPSImporter {
    /// Optional: Known ODCS Table IDs for contractId validation
    known_table_ids: Option<Vec<String>>,
}

impl ODPSImporter {
    /// Create a new ODPS importer instance
    pub fn new() -> Self {
        Self {
            known_table_ids: None,
        }
    }

    /// Create a new ODPS importer with known table IDs for contractId validation
    pub fn with_table_ids(table_ids: Vec<String>) -> Self {
        Self {
            known_table_ids: Some(table_ids),
        }
    }

    /// Import ODPS YAML content and create ODPSDataProduct
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - ODPS YAML content as a string
    ///
    /// # Returns
    ///
    /// A `ODPSDataProduct` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::odps::ODPSImporter;
    ///
    /// let importer = ODPSImporter::new();
    /// let yaml = r#"
    /// apiVersion: v1.0.0
    /// kind: DataProduct
    /// id: 550e8400-e29b-41d4-a716-446655440000
    /// name: customer-data-product
    /// version: 1.0.0
    /// status: active
    /// "#;
    /// let product = importer.import(yaml).unwrap();
    /// assert_eq!(product.name, Some("customer-data-product".to_string()));
    /// ```
    pub fn import(&self, yaml_content: &str) -> Result<ODPSDataProduct, ImportError> {
        // Validate against ODPS schema before parsing (if feature enabled)
        #[cfg(feature = "odps-validation")]
        {
            #[cfg(feature = "cli")]
            {
                use crate::cli::validation::validate_odps_internal;
                validate_odps_internal(yaml_content).map_err(ImportError::ValidationError)?;
            }
            #[cfg(not(feature = "cli"))]
            {
                // Inline validation when CLI feature is not enabled
                use jsonschema::Validator;
                use serde_json::Value;

                let schema_content = include_str!("../../schemas/odps-json-schema-latest.json");
                let schema: Value = serde_json::from_str(schema_content).map_err(|e| {
                    ImportError::ValidationError(format!("Failed to load ODPS schema: {}", e))
                })?;

                let validator = Validator::new(&schema).map_err(|e| {
                    ImportError::ValidationError(format!("Failed to compile ODPS schema: {}", e))
                })?;

                let data: Value = serde_yaml::from_str(yaml_content).map_err(|e| {
                    ImportError::ValidationError(format!("Failed to parse YAML: {}", e))
                })?;

                if let Err(errors) = validator.validate(&data) {
                    let error_messages: Vec<String> = errors
                        .map(|e| format!("{}: {}", e.instance_path, e))
                        .collect();
                    return Err(ImportError::ValidationError(format!(
                        "ODPS validation failed:\n{}",
                        error_messages.join("\n")
                    )));
                }
            }
        }

        let yaml_value: YamlValue = serde_yaml::from_str(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse YAML: {}", e)))?;

        self.parse_odps_product(&yaml_value)
            .map_err(|e| ImportError::ParseError(e.to_string()))
    }

    /// Parse ODPS data product from YAML value
    fn parse_odps_product(&self, yaml: &YamlValue) -> Result<ODPSDataProduct> {
        let _obj = yaml
            .as_mapping()
            .ok_or_else(|| anyhow::anyhow!("ODPS YAML must be a mapping"))?;

        // Convert YAML to JSON for easier parsing
        let json_value: JsonValue =
            serde_json::to_value(yaml).context("Failed to convert YAML to JSON")?;

        // Parse required fields
        let api_version = json_value
            .get("apiVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("v1.0.0")
            .to_string();

        let kind = json_value
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("DataProduct")
            .to_string();

        let id = json_value
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: id"))?
            .to_string();

        let status_str = json_value
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: status"))?;

        let status = match status_str {
            "proposed" => ODPSStatus::Proposed,
            "draft" => ODPSStatus::Draft,
            "active" => ODPSStatus::Active,
            "deprecated" => ODPSStatus::Deprecated,
            "retired" => ODPSStatus::Retired,
            _ => return Err(anyhow::anyhow!("Invalid status: {}", status_str)),
        };

        // Parse optional fields
        let name = json_value
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let version = json_value
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let domain = json_value
            .get("domain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tenant = json_value
            .get("tenant")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = self.parse_tags(&json_value)?;
        let description = self.parse_description(&json_value)?;
        let authoritative_definitions = self.parse_authoritative_definitions(&json_value)?;
        let custom_properties = self.parse_custom_properties(&json_value)?;
        let input_ports = self.parse_input_ports(&json_value)?;
        let output_ports = self.parse_output_ports(&json_value)?;
        let management_ports = self.parse_management_ports(&json_value)?;
        let support = self.parse_support(&json_value)?;
        let team = self.parse_team(&json_value)?;
        let product_created_ts = json_value
            .get("productCreatedTs")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ODPSDataProduct {
            api_version,
            kind,
            id,
            name,
            version,
            status,
            domain,
            tenant,
            authoritative_definitions,
            description,
            custom_properties,
            tags,
            input_ports,
            output_ports,
            management_ports,
            support,
            team,
            product_created_ts,
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
    fn parse_description(&self, json: &JsonValue) -> Result<Option<ODPSDescription>> {
        if let Some(desc_val) = json.get("description") {
            let desc_obj = desc_val
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Description must be an object"))?;
            Ok(Some(ODPSDescription {
                purpose: desc_obj
                    .get("purpose")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                limitations: desc_obj
                    .get("limitations")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                usage: desc_obj
                    .get("usage")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                authoritative_definitions: self.parse_authoritative_definitions(desc_val)?,
                custom_properties: self.parse_custom_properties(desc_val)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse authoritative definitions array
    fn parse_authoritative_definitions(
        &self,
        json: &JsonValue,
    ) -> Result<Option<Vec<ODPSAuthoritativeDefinition>>> {
        if let Some(defs_arr) = json
            .get("authoritativeDefinitions")
            .and_then(|v| v.as_array())
        {
            let mut defs = Vec::new();
            for def_item in defs_arr {
                if let Some(def_obj) = def_item.as_object()
                    && let (Some(r#type), Some(url)) = (
                        def_obj.get("type").and_then(|v| v.as_str()),
                        def_obj.get("url").and_then(|v| v.as_str()),
                    )
                {
                    defs.push(ODPSAuthoritativeDefinition {
                        r#type: r#type.to_string(),
                        url: url.to_string(),
                        description: def_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
            // Preserve empty arrays as Some(vec![]) to maintain structure
            Ok(Some(defs))
        } else {
            Ok(None)
        }
    }

    /// Parse custom properties array
    fn parse_custom_properties(&self, json: &JsonValue) -> Result<Option<Vec<ODPSCustomProperty>>> {
        if let Some(props_arr) = json.get("customProperties").and_then(|v| v.as_array()) {
            let mut props = Vec::new();
            for prop_item in props_arr {
                if let Some(prop_obj) = prop_item.as_object()
                    && let (Some(property), Some(value)) = (
                        prop_obj.get("property").and_then(|v| v.as_str()),
                        prop_obj.get("value"),
                    )
                {
                    props.push(ODPSCustomProperty {
                        property: property.to_string(),
                        value: value.clone(),
                        description: prop_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
            // Preserve empty arrays as Some(vec![]) to maintain structure
            Ok(Some(props))
        } else {
            Ok(None)
        }
    }

    /// Parse input ports array
    fn parse_input_ports(&self, json: &JsonValue) -> Result<Option<Vec<ODPSInputPort>>> {
        if let Some(ports_arr) = json.get("inputPorts").and_then(|v| v.as_array()) {
            let mut ports = Vec::new();
            for port_item in ports_arr {
                if let Some(port_obj) = port_item.as_object()
                    && let (Some(name), Some(version), Some(contract_id)) = (
                        port_obj.get("name").and_then(|v| v.as_str()),
                        port_obj.get("version").and_then(|v| v.as_str()),
                        port_obj.get("contractId").and_then(|v| v.as_str()),
                    )
                {
                    // Validate contractId if known table IDs provided
                    if let Some(ref table_ids) = self.known_table_ids
                        && !table_ids.contains(&contract_id.to_string())
                    {
                        return Err(anyhow::anyhow!(
                            "Input port '{}' references unknown contractId: {}",
                            name,
                            contract_id
                        ));
                    }

                    let port_json = JsonValue::Object(port_obj.clone());
                    ports.push(ODPSInputPort {
                        name: name.to_string(),
                        version: version.to_string(),
                        contract_id: contract_id.to_string(),
                        tags: self.parse_tags(&port_json)?,
                        custom_properties: self.parse_custom_properties(&port_json)?,
                        authoritative_definitions: self
                            .parse_authoritative_definitions(&port_json)?,
                    });
                }
            }
            // Preserve empty arrays as Some(vec![]) to maintain structure
            Ok(Some(ports))
        } else {
            Ok(None)
        }
    }

    /// Parse output ports array
    fn parse_output_ports(&self, json: &JsonValue) -> Result<Option<Vec<ODPSOutputPort>>> {
        if let Some(ports_arr) = json.get("outputPorts").and_then(|v| v.as_array()) {
            let mut ports = Vec::new();
            for port_item in ports_arr {
                if let Some(port_obj) = port_item.as_object()
                    && let (Some(name), Some(version)) = (
                        port_obj.get("name").and_then(|v| v.as_str()),
                        port_obj.get("version").and_then(|v| v.as_str()),
                    )
                {
                    let contract_id = port_obj
                        .get("contractId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Validate contractId if known table IDs provided
                    if let Some(ref contract_id_str) = contract_id
                        && let Some(ref table_ids) = self.known_table_ids
                        && !table_ids.contains(contract_id_str)
                    {
                        return Err(anyhow::anyhow!(
                            "Output port '{}' references unknown contractId: {}",
                            name,
                            contract_id_str
                        ));
                    }

                    // Parse input contracts
                    let input_contracts = if let Some(contracts_arr) =
                        port_obj.get("inputContracts").and_then(|v| v.as_array())
                    {
                        let mut contracts = Vec::new();
                        for contract_item in contracts_arr {
                            if let Some(contract_obj) = contract_item.as_object()
                                && let (Some(id), Some(version)) = (
                                    contract_obj.get("id").and_then(|v| v.as_str()),
                                    contract_obj.get("version").and_then(|v| v.as_str()),
                                )
                            {
                                contracts.push(ODPSInputContract {
                                    id: id.to_string(),
                                    version: version.to_string(),
                                });
                            }
                        }
                        if !contracts.is_empty() {
                            Some(contracts)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Parse SBOM
                    let sbom =
                        if let Some(sbom_arr) = port_obj.get("sbom").and_then(|v| v.as_array()) {
                            let mut sboms = Vec::new();
                            for sbom_item in sbom_arr {
                                if let Some(sbom_obj) = sbom_item.as_object()
                                    && let Some(url) = sbom_obj.get("url").and_then(|v| v.as_str())
                                {
                                    sboms.push(ODPSSBOM {
                                        r#type: sbom_obj
                                            .get("type")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        url: url.to_string(),
                                    });
                                }
                            }
                            if !sboms.is_empty() { Some(sboms) } else { None }
                        } else {
                            None
                        };

                    let port_json = JsonValue::Object(port_obj.clone());
                    ports.push(ODPSOutputPort {
                        name: name.to_string(),
                        description: port_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        r#type: port_obj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        version: version.to_string(),
                        contract_id,
                        sbom,
                        input_contracts,
                        tags: self.parse_tags(&port_json)?,
                        custom_properties: self.parse_custom_properties(&port_json)?,
                        authoritative_definitions: self
                            .parse_authoritative_definitions(&port_json)?,
                    });
                }
            }
            // Preserve empty arrays as Some(vec![]) to maintain structure
            Ok(Some(ports))
        } else {
            Ok(None)
        }
    }

    /// Parse management ports array
    fn parse_management_ports(&self, json: &JsonValue) -> Result<Option<Vec<ODPSManagementPort>>> {
        if let Some(ports_arr) = json.get("managementPorts").and_then(|v| v.as_array()) {
            let mut ports = Vec::new();
            for port_item in ports_arr {
                if let Some(port_obj) = port_item.as_object()
                    && let (Some(name), Some(content)) = (
                        port_obj.get("name").and_then(|v| v.as_str()),
                        port_obj.get("content").and_then(|v| v.as_str()),
                    )
                {
                    let port_json = JsonValue::Object(port_obj.clone());
                    ports.push(ODPSManagementPort {
                        name: name.to_string(),
                        content: content.to_string(),
                        r#type: port_obj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        url: port_obj
                            .get("url")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        channel: port_obj
                            .get("channel")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        description: port_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        tags: self.parse_tags(&port_json)?,
                        custom_properties: self.parse_custom_properties(&port_json)?,
                        authoritative_definitions: self
                            .parse_authoritative_definitions(&port_json)?,
                    });
                }
            }
            // Preserve empty arrays as Some(vec![]) to maintain structure
            Ok(Some(ports))
        } else {
            Ok(None)
        }
    }

    /// Parse support array
    fn parse_support(&self, json: &JsonValue) -> Result<Option<Vec<ODPSSupport>>> {
        if let Some(support_arr) = json.get("support").and_then(|v| v.as_array()) {
            let mut supports = Vec::new();
            for support_item in support_arr {
                if let Some(support_obj) = support_item.as_object()
                    && let (Some(channel), Some(url)) = (
                        support_obj.get("channel").and_then(|v| v.as_str()),
                        support_obj.get("url").and_then(|v| v.as_str()),
                    )
                {
                    let support_json = JsonValue::Object(support_obj.clone());
                    supports.push(ODPSSupport {
                        channel: channel.to_string(),
                        url: url.to_string(),
                        description: support_obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        tool: support_obj
                            .get("tool")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        scope: support_obj
                            .get("scope")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        invitation_url: support_obj
                            .get("invitationUrl")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        tags: self.parse_tags(&support_json)?,
                        custom_properties: self.parse_custom_properties(&support_json)?,
                        authoritative_definitions: self
                            .parse_authoritative_definitions(&support_json)?,
                    });
                }
            }
            // Preserve empty arrays as Some(vec![]) to maintain structure
            Ok(Some(supports))
        } else {
            Ok(None)
        }
    }

    /// Parse team object
    fn parse_team(&self, json: &JsonValue) -> Result<Option<ODPSTeam>> {
        if let Some(team_obj) = json.get("team").and_then(|v| v.as_object()) {
            let members = if let Some(members_arr) =
                team_obj.get("members").and_then(|v| v.as_array())
            {
                let mut team_members = Vec::new();
                for member_item in members_arr {
                    if let Some(member_obj) = member_item.as_object()
                        && let Some(username) = member_obj.get("username").and_then(|v| v.as_str())
                    {
                        let member_json = JsonValue::Object(member_obj.clone());
                        team_members.push(ODPSTeamMember {
                            username: username.to_string(),
                            name: member_obj
                                .get("name")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            description: member_obj
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            role: member_obj
                                .get("role")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            date_in: member_obj
                                .get("dateIn")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            date_out: member_obj
                                .get("dateOut")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            replaced_by_username: member_obj
                                .get("replacedByUsername")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            tags: self.parse_tags(&member_json)?,
                            custom_properties: self.parse_custom_properties(&member_json)?,
                            authoritative_definitions: self
                                .parse_authoritative_definitions(&member_json)?,
                        });
                    }
                }
                if !team_members.is_empty() {
                    Some(team_members)
                } else {
                    None
                }
            } else {
                None
            };

            let team_json = JsonValue::Object(team_obj.clone());
            Ok(Some(ODPSTeam {
                name: team_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                description: team_obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                members,
                tags: self.parse_tags(&team_json)?,
                custom_properties: self.parse_custom_properties(&team_json)?,
                authoritative_definitions: self.parse_authoritative_definitions(&team_json)?,
            }))
        } else {
            Ok(None)
        }
    }
}

impl Default for ODPSImporter {
    fn default() -> Self {
        Self::new()
    }
}
