//! Column model for the SDK

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKey {
    pub table_id: String, // UUID as string
    pub column_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    #[serde(default = "default_true")]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub secondary_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key: Option<ForeignKey>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub errors: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub quality: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub enum_values: Vec<String>,
    #[serde(default)]
    pub column_order: i32,
}

fn default_true() -> bool {
    true
}

impl Column {
    pub fn new(name: String, data_type: String) -> Self {
        Self {
            name,
            data_type: normalize_data_type(&data_type),
            nullable: true,
            primary_key: false,
            secondary_key: false,
            composite_key: None,
            foreign_key: None,
            constraints: Vec::new(),
            description: String::new(),
            errors: Vec::new(),
            quality: Vec::new(),
            enum_values: Vec::new(),
            column_order: 0,
        }
    }
}

fn normalize_data_type(data_type: &str) -> String {
    if data_type.is_empty() {
        return data_type.to_string();
    }

    let upper = data_type.to_uppercase();

    // Handle STRUCT<...>, ARRAY<...>, MAP<...> preserving inner content
    if upper.starts_with("STRUCT") {
        if let Some(start) = data_type.find('<') {
            if let Some(end) = data_type.rfind('>') {
                let inner = &data_type[start + 1..end];
                return format!("STRUCT<{}>", inner);
            }
        }
        return format!("STRUCT{}", &data_type[6..]);
    } else if upper.starts_with("ARRAY") {
        if let Some(start) = data_type.find('<') {
            if let Some(end) = data_type.rfind('>') {
                let inner = &data_type[start + 1..end];
                return format!("ARRAY<{}>", inner);
            }
        }
        return format!("ARRAY{}", &data_type[5..]);
    } else if upper.starts_with("MAP") {
        if let Some(start) = data_type.find('<') {
            if let Some(end) = data_type.rfind('>') {
                let inner = &data_type[start + 1..end];
                return format!("MAP<{}>", inner);
            }
        }
        return format!("MAP{}", &data_type[3..]);
    }

    upper
}



