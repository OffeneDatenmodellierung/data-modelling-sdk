//! OpenAPI model structures
//!
//! Defines structures for representing OpenAPI 3.1.1 specifications stored in native YAML or JSON format.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// OpenAPI format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAPIFormat {
    /// YAML format
    Yaml,
    /// JSON format
    Json,
}

/// OpenAPI Model
///
/// Represents an OpenAPI 3.1.1 specification stored in native YAML or JSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenAPIModel {
    /// Unique identifier
    pub id: Uuid,
    /// Domain this model belongs to
    pub domain_id: Uuid,
    /// API name (extracted from `info.title` or provided)
    pub name: String,
    /// Relative path within domain directory (e.g., `{domain_name}/{name}.openapi.yaml`)
    pub file_path: String,
    /// Format (YAML or JSON)
    pub format: OpenAPIFormat,
    /// File size in bytes
    pub file_size: u64,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Extracted metadata (version, description, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl OpenAPIModel {
    /// Create a new OpenAPI model
    pub fn new(
        domain_id: Uuid,
        name: String,
        file_path: String,
        format: OpenAPIFormat,
        file_size: u64,
    ) -> Self {
        OpenAPIModel {
            id: Uuid::new_v4(),
            domain_id,
            name,
            file_path,
            format,
            file_size,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            metadata: HashMap::new(),
        }
    }
}
