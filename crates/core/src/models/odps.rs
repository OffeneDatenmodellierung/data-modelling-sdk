//! ODPS (Open Data Product Standard) models
//!
//! Defines structures for ODPS Data Products that link to ODCS Tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::tag::Tag;

/// ODPS API version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ODPSApiVersion {
    #[serde(rename = "v0.9.0")]
    V0_9_0,
    #[serde(rename = "v1.0.0")]
    V1_0_0,
}

/// ODPS status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ODPSStatus {
    #[serde(rename = "proposed")]
    Proposed,
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "deprecated")]
    Deprecated,
    #[serde(rename = "retired")]
    Retired,
}

/// Authoritative definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSAuthoritativeDefinition {
    /// Type of definition
    pub r#type: String,
    /// URL to the authority
    pub url: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Custom property
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSCustomProperty {
    /// Property name
    pub property: String,
    /// Property value
    pub value: serde_json::Value,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// ODPS description
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSDescription {
    /// Intended purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Limitations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limitations: Option<String>,
    /// Recommended usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
}

/// Input port
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSInputPort {
    /// Port name
    pub name: String,
    /// Port version
    pub version: String,
    /// Contract ID (links to ODCS Table)
    pub contract_id: String,
    /// Tags
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
}

/// SBOM (Software Bill of Materials)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSSBOM {
    /// SBOM type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// URL to SBOM
    pub url: String,
}

/// Input contract dependency
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSInputContract {
    /// Contract ID
    pub id: String,
    /// Contract version
    pub version: String,
}

/// Output port
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSOutputPort {
    /// Port name
    pub name: String,
    /// Port description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Port type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Port version
    pub version: String,
    /// Contract ID (links to ODCS Table)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    /// SBOM array
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sbom: Option<Vec<ODPSSBOM>>,
    /// Input contracts (dependencies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_contracts: Option<Vec<ODPSInputContract>>,
    /// Tags
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
}

/// Management port
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSManagementPort {
    /// Port name
    pub name: String,
    /// Content type
    pub content: String,
    /// Port type (rest or topic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// URL to access endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Channel name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
}

/// Support channel
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSSupport {
    /// Channel name
    pub channel: String,
    /// Access URL
    pub url: String,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tool name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Scope
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Invitation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitation_url: Option<String>,
    /// Tags
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
}

/// Team member
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSTeamMember {
    /// Username or email
    pub username: String,
    /// Member name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Date joined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_in: Option<String>,
    /// Date left
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_out: Option<String>,
    /// Replaced by username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_by_username: Option<String>,
    /// Tags
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
}

/// Team
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSTeam {
    /// Team name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Team description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Team members
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<ODPSTeamMember>>,
    /// Tags
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
}

/// Data Product - main structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ODPSDataProduct {
    /// API version
    pub api_version: String,
    /// Kind (always "DataProduct")
    pub kind: String,
    /// Unique identifier
    pub id: String,
    /// Product name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Product version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Status
    pub status: ODPSStatus,
    /// Business domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Tenant/organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Authoritative definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<ODPSDescription>,
    /// Custom properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_properties: Option<Vec<ODPSCustomProperty>>,
    /// Tags
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_tags"
    )]
    pub tags: Vec<Tag>,
    /// Input ports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_ports: Option<Vec<ODPSInputPort>>,
    /// Output ports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_ports: Option<Vec<ODPSOutputPort>>,
    /// Management ports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub management_ports: Option<Vec<ODPSManagementPort>>,
    /// Support channels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support: Option<Vec<ODPSSupport>>,
    /// Team
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<ODPSTeam>,
    /// Product creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_created_ts: Option<String>,
    /// Creation timestamp (internal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp (internal)
    #[serde(skip_serializing_if = "Option::is_none")]
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
