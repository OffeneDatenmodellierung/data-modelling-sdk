//! DMN (Decision Model and Notation) model structures
//!
//! Defines structures for representing DMN 1.3 models stored in native XML format.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// DMN model format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DMNModelFormat {
    #[serde(rename = "dmn13-xml")]
    Dmn13Xml,
}

/// DMN Model
///
/// Represents a DMN 1.3 decision model stored in native XML format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DMNModel {
    /// Unique identifier
    pub id: Uuid,
    /// Domain this model belongs to
    pub domain_id: Uuid,
    /// Model name (extracted from XML or provided)
    pub name: String,
    /// Relative path within domain directory (e.g., `{domain_name}/{name}.dmn.xml`)
    pub file_path: String,
    /// File size in bytes
    pub file_size: u64,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Extracted metadata (namespace, version, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DMNModel {
    /// Create a new DMN model
    pub fn new(domain_id: Uuid, name: String, file_path: String, file_size: u64) -> Self {
        DMNModel {
            id: Uuid::new_v4(),
            domain_id,
            name,
            file_path,
            file_size,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            metadata: HashMap::new(),
        }
    }
}
