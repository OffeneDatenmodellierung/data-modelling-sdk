//! Catalog abstraction for Apache Iceberg
//!
//! This module provides a unified interface for different Iceberg catalog types:
//! - REST catalog (Lakekeeper, Nessie, Polaris)
//! - AWS S3 Tables
//! - Databricks Unity Catalog
//! - AWS Glue
//!
//! # Example
//!
//! ```ignore
//! use data_modelling_core::staging::catalog::{CatalogConfig, IcebergCatalog};
//!
//! // Create a REST catalog (Lakekeeper)
//! let config = CatalogConfig::Rest {
//!     endpoint: "http://localhost:8181".to_string(),
//!     warehouse: "./local-warehouse".to_string(),
//!     token: None,
//! };
//!
//! let catalog = IcebergCatalog::new(config).await?;
//! let tables = catalog.list_tables("staging").await?;
//! ```

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Catalog configuration for different Iceberg catalog types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CatalogConfig {
    /// REST catalog (Lakekeeper, Nessie, Polaris, etc.)
    Rest {
        /// Catalog endpoint URL
        endpoint: String,
        /// Warehouse location (S3, local path, etc.)
        warehouse: String,
        /// Optional bearer token for authentication
        token: Option<String>,
        /// Additional properties
        #[serde(default)]
        properties: HashMap<String, String>,
    },

    /// AWS S3 Tables catalog
    S3Tables {
        /// S3 Tables ARN
        arn: String,
        /// AWS region
        region: String,
        /// Optional AWS credentials profile
        profile: Option<String>,
    },

    /// Databricks Unity Catalog
    Unity {
        /// Unity Catalog endpoint
        endpoint: String,
        /// Catalog name
        catalog: String,
        /// OAuth token
        token: String,
    },

    /// AWS Glue catalog
    Glue {
        /// AWS region
        region: String,
        /// Glue database name
        database: String,
        /// Optional AWS credentials profile
        profile: Option<String>,
    },
}

/// Errors that can occur during catalog operations
#[derive(Error, Debug)]
pub enum CatalogError {
    /// Failed to connect to catalog
    #[error("Failed to connect to catalog: {0}")]
    ConnectionError(String),

    /// Table not found
    #[error("Table not found: {0}")]
    TableNotFound(String),

    /// Namespace not found
    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),

    /// Table already exists
    #[error("Table already exists: {0}")]
    TableExists(String),

    /// Schema error
    #[error("Schema error: {0}")]
    SchemaError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Iceberg error
    #[error("Iceberg error: {0}")]
    IcebergError(String),
}

/// Result type for catalog operations
pub type CatalogResult<T> = Result<T, CatalogError>;

/// Table identifier with namespace and name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableIdentifier {
    /// Namespace (database/schema)
    pub namespace: String,
    /// Table name
    pub name: String,
}

impl TableIdentifier {
    /// Create a new table identifier
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }

    /// Parse from a dot-separated string (e.g., "staging.raw_json")
    pub fn parse(s: &str) -> CatalogResult<Self> {
        let parts: Vec<&str> = s.splitn(2, '.').collect();
        if parts.len() != 2 {
            return Err(CatalogError::ConfigError(format!(
                "Invalid table identifier: {}. Expected format: namespace.table",
                s
            )));
        }
        Ok(Self::new(parts[0], parts[1]))
    }

    /// Convert to dot-separated string
    pub fn to_string(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }
}

/// Table metadata summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table identifier
    pub identifier: TableIdentifier,
    /// Table location (storage path)
    pub location: String,
    /// Current snapshot ID
    pub current_snapshot_id: Option<i64>,
    /// Table properties
    pub properties: HashMap<String, String>,
}

/// Trait for Iceberg catalog operations
#[async_trait]
pub trait CatalogOperations: Send + Sync {
    /// List all namespaces in the catalog
    async fn list_namespaces(&self) -> CatalogResult<Vec<String>>;

    /// Create a new namespace
    async fn create_namespace(&self, namespace: &str) -> CatalogResult<()>;

    /// List all tables in a namespace
    async fn list_tables(&self, namespace: &str) -> CatalogResult<Vec<TableIdentifier>>;

    /// Check if a table exists
    async fn table_exists(&self, identifier: &TableIdentifier) -> CatalogResult<bool>;

    /// Get table info
    async fn get_table_info(&self, identifier: &TableIdentifier) -> CatalogResult<TableInfo>;

    /// Drop a table
    async fn drop_table(&self, identifier: &TableIdentifier) -> CatalogResult<()>;
}

/// Iceberg catalog wrapper
pub struct IcebergCatalog {
    config: CatalogConfig,
    #[cfg(feature = "iceberg")]
    pub(crate) inner: Arc<dyn iceberg::Catalog>,
}

impl IcebergCatalog {
    /// Create a new Iceberg catalog from configuration
    #[cfg(feature = "iceberg")]
    pub async fn new(config: CatalogConfig) -> CatalogResult<Self> {
        use iceberg::CatalogBuilder;
        use iceberg_catalog_rest::{
            REST_CATALOG_PROP_URI, REST_CATALOG_PROP_WAREHOUSE, RestCatalogBuilder,
        };

        match &config {
            CatalogConfig::Rest {
                endpoint,
                warehouse,
                token,
                properties,
            } => {
                let mut props = HashMap::new();
                props.insert(REST_CATALOG_PROP_URI.to_string(), endpoint.clone());
                props.insert(REST_CATALOG_PROP_WAREHOUSE.to_string(), warehouse.clone());

                if let Some(t) = token {
                    props.insert("token".to_string(), t.clone());
                }

                // Add additional properties
                for (k, v) in properties {
                    props.insert(k.clone(), v.clone());
                }

                let catalog = RestCatalogBuilder::default()
                    .load("rest", props)
                    .await
                    .map_err(|e| CatalogError::ConnectionError(e.to_string()))?;

                Ok(Self {
                    config,
                    inner: Arc::new(catalog),
                })
            }
            CatalogConfig::S3Tables { arn, region, .. } => {
                // S3 Tables uses the REST API protocol, similar to Unity Catalog
                // The S3 Tables service exposes an Iceberg REST endpoint
                let mut props = HashMap::new();
                // S3 Tables endpoint format: https://s3tables.<region>.amazonaws.com
                props.insert(
                    iceberg_catalog_rest::REST_CATALOG_PROP_URI.to_string(),
                    format!("https://s3tables.{}.amazonaws.com", region),
                );
                props.insert(
                    iceberg_catalog_rest::REST_CATALOG_PROP_WAREHOUSE.to_string(),
                    arn.clone(),
                );
                // AWS credentials are picked up from environment/profile automatically

                let catalog = RestCatalogBuilder::default()
                    .load("s3tables", props)
                    .await
                    .map_err(|e| CatalogError::ConnectionError(e.to_string()))?;

                Ok(Self {
                    config,
                    inner: Arc::new(catalog),
                })
            }

            CatalogConfig::Unity {
                endpoint,
                catalog: catalog_name,
                token,
            } => {
                // Unity Catalog exposes an Iceberg REST API, so we can use the REST catalog
                // See: https://docs.databricks.com/en/data-governance/unity-catalog/index.html
                let mut props = HashMap::new();
                props.insert(
                    iceberg_catalog_rest::REST_CATALOG_PROP_URI.to_string(),
                    format!("{}/api/2.1/unity-catalog/iceberg", endpoint),
                );
                props.insert(
                    iceberg_catalog_rest::REST_CATALOG_PROP_WAREHOUSE.to_string(),
                    catalog_name.clone(),
                );
                props.insert("token".to_string(), token.clone());
                props.insert("credential".to_string(), format!("Bearer {}", token));

                let catalog = RestCatalogBuilder::default()
                    .load("unity", props)
                    .await
                    .map_err(|e| CatalogError::ConnectionError(e.to_string()))?;

                Ok(Self {
                    config,
                    inner: Arc::new(catalog),
                })
            }

            #[cfg(feature = "iceberg-glue")]
            CatalogConfig::Glue {
                region,
                database,
                profile,
            } => {
                use iceberg::CatalogBuilder;
                use iceberg_catalog_glue::{GLUE_CATALOG_PROP_WAREHOUSE, GlueCatalogBuilder};

                let mut props = HashMap::new();
                props.insert(GLUE_CATALOG_PROP_WAREHOUSE.to_string(), database.clone());
                props.insert("aws.region".to_string(), region.clone());

                if let Some(p) = profile {
                    props.insert("aws.profile".to_string(), p.clone());
                }

                let catalog = GlueCatalogBuilder::default()
                    .load("glue", props)
                    .await
                    .map_err(|e| CatalogError::ConnectionError(e.to_string()))?;

                Ok(Self {
                    config,
                    inner: Arc::new(catalog),
                })
            }
            #[cfg(not(feature = "iceberg-glue"))]
            CatalogConfig::Glue { .. } => Err(CatalogError::ConfigError(
                "Glue catalog requires the 'iceberg-glue' feature. \
                 Enable it with: --features iceberg-glue"
                    .to_string(),
            )),
        }
    }

    /// Get the catalog configuration
    pub fn config(&self) -> &CatalogConfig {
        &self.config
    }
}

#[cfg(feature = "iceberg")]
#[async_trait]
impl CatalogOperations for IcebergCatalog {
    async fn list_namespaces(&self) -> CatalogResult<Vec<String>> {
        let namespaces = self
            .inner
            .list_namespaces(None)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

        Ok(namespaces.into_iter().map(|ns| ns.to_string()).collect())
    }

    async fn create_namespace(&self, namespace: &str) -> CatalogResult<()> {
        use iceberg::NamespaceIdent;

        let ns_ident = NamespaceIdent::new(namespace.to_string());
        self.inner
            .create_namespace(&ns_ident, HashMap::new())
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

        Ok(())
    }

    async fn list_tables(&self, namespace: &str) -> CatalogResult<Vec<TableIdentifier>> {
        use iceberg::NamespaceIdent;

        let ns_ident = NamespaceIdent::new(namespace.to_string());
        let tables = self
            .inner
            .list_tables(&ns_ident)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

        Ok(tables
            .into_iter()
            .map(|t| TableIdentifier::new(namespace, t.name()))
            .collect())
    }

    async fn table_exists(&self, identifier: &TableIdentifier) -> CatalogResult<bool> {
        use iceberg::{NamespaceIdent, TableIdent};

        let ns_ident = NamespaceIdent::new(identifier.namespace.clone());
        let table_ident = TableIdent::new(ns_ident, identifier.name.clone());

        Ok(self
            .inner
            .table_exists(&table_ident)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?)
    }

    async fn get_table_info(&self, identifier: &TableIdentifier) -> CatalogResult<TableInfo> {
        use iceberg::{NamespaceIdent, TableIdent};

        let ns_ident = NamespaceIdent::new(identifier.namespace.clone());
        let table_ident = TableIdent::new(ns_ident, identifier.name.clone());

        let table = self
            .inner
            .load_table(&table_ident)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

        let metadata = table.metadata();

        Ok(TableInfo {
            identifier: identifier.clone(),
            location: metadata.location().to_string(),
            current_snapshot_id: metadata.current_snapshot_id(),
            properties: metadata.properties().clone(),
        })
    }

    async fn drop_table(&self, identifier: &TableIdentifier) -> CatalogResult<()> {
        use iceberg::{NamespaceIdent, TableIdent};

        let ns_ident = NamespaceIdent::new(identifier.namespace.clone());
        let table_ident = TableIdent::new(ns_ident, identifier.name.clone());

        self.inner
            .drop_table(&table_ident)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_identifier_parse() {
        let id = TableIdentifier::parse("staging.raw_json").unwrap();
        assert_eq!(id.namespace, "staging");
        assert_eq!(id.name, "raw_json");
    }

    #[test]
    fn test_table_identifier_parse_error() {
        let result = TableIdentifier::parse("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_table_identifier_to_string() {
        let id = TableIdentifier::new("staging", "raw_json");
        assert_eq!(id.to_string(), "staging.raw_json");
    }

    #[test]
    fn test_table_identifier_with_dots_in_name() {
        // Should only split on first dot
        let id = TableIdentifier::parse("staging.my.table.name").unwrap();
        assert_eq!(id.namespace, "staging");
        assert_eq!(id.name, "my.table.name");
    }

    #[test]
    fn test_catalog_config_serialize_rest() {
        let config = CatalogConfig::Rest {
            endpoint: "http://localhost:8181".to_string(),
            warehouse: "./warehouse".to_string(),
            token: None,
            properties: HashMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("rest"));
        assert!(json.contains("localhost:8181"));
    }

    #[test]
    fn test_catalog_config_serialize_s3_tables() {
        let config = CatalogConfig::S3Tables {
            arn: "arn:aws:s3tables:us-east-1:123456789:bucket/my-bucket".to_string(),
            region: "us-east-1".to_string(),
            profile: Some("default".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("s3_tables"));
        assert!(json.contains("arn:aws:s3tables"));
        assert!(json.contains("us-east-1"));
    }

    #[test]
    fn test_catalog_config_serialize_unity() {
        let config = CatalogConfig::Unity {
            endpoint: "https://workspace.cloud.databricks.com".to_string(),
            catalog: "main".to_string(),
            token: "dapi123456".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("unity"));
        assert!(json.contains("databricks.com"));
        assert!(json.contains("main"));
    }

    #[test]
    fn test_catalog_config_serialize_glue() {
        let config = CatalogConfig::Glue {
            region: "eu-west-1".to_string(),
            database: "staging_db".to_string(),
            profile: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("glue"));
        assert!(json.contains("eu-west-1"));
        assert!(json.contains("staging_db"));
    }

    #[test]
    fn test_catalog_config_deserialize_rest() {
        let json = r#"{
            "type": "rest",
            "endpoint": "http://localhost:8181",
            "warehouse": "./warehouse",
            "token": null,
            "properties": {}
        }"#;

        let config: CatalogConfig = serde_json::from_str(json).unwrap();
        match config {
            CatalogConfig::Rest {
                endpoint,
                warehouse,
                token,
                ..
            } => {
                assert_eq!(endpoint, "http://localhost:8181");
                assert_eq!(warehouse, "./warehouse");
                assert!(token.is_none());
            }
            _ => panic!("Expected REST config"),
        }
    }

    #[test]
    fn test_catalog_config_with_properties() {
        let mut props = HashMap::new();
        props.insert(
            "oauth2-server-uri".to_string(),
            "https://auth.example.com".to_string(),
        );
        props.insert("credential".to_string(), "client_credentials".to_string());

        let config = CatalogConfig::Rest {
            endpoint: "http://localhost:8181".to_string(),
            warehouse: "./warehouse".to_string(),
            token: None,
            properties: props,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("oauth2-server-uri"));
        assert!(json.contains("client_credentials"));
    }

    #[test]
    fn test_table_info_serialization() {
        let mut props = HashMap::new();
        props.insert("batch.123".to_string(), "metadata".to_string());

        let info = TableInfo {
            identifier: TableIdentifier::new("staging", "raw_json"),
            location: "s3://bucket/warehouse/staging/raw_json".to_string(),
            current_snapshot_id: Some(12345),
            properties: props,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("staging"));
        assert!(json.contains("raw_json"));
        assert!(json.contains("12345"));
        assert!(json.contains("batch.123"));
    }

    #[test]
    fn test_catalog_error_display() {
        let err = CatalogError::TableNotFound("staging.raw_json".to_string());
        assert_eq!(err.to_string(), "Table not found: staging.raw_json");

        let err = CatalogError::NamespaceNotFound("staging".to_string());
        assert_eq!(err.to_string(), "Namespace not found: staging");

        let err = CatalogError::ConnectionError("Connection refused".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to connect to catalog: Connection refused"
        );
    }
}
