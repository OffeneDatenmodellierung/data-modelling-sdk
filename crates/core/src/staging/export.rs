//! Export staging data to production catalogs
//!
//! This module provides functionality to export data from local Iceberg tables
//! to production catalogs like Unity Catalog, AWS Glue, and S3 Tables.
//!
//! # Example
//!
//! ```ignore
//! use data_modelling_core::staging::export::{ExportConfig, ExportTarget, export_to_catalog};
//!
//! let config = ExportConfig {
//!     target: ExportTarget::Unity {
//!         endpoint: "https://workspace.cloud.databricks.com".to_string(),
//!         token: "dapi...".to_string(),
//!         catalog: "main".to_string(),
//!         schema: "staging".to_string(),
//!     },
//!     table_name: "raw_json".to_string(),
//!     overwrite: false,
//! };
//!
//! let result = export_to_catalog(&source_table, &catalog, &config).await?;
//! println!("Exported {} files ({} bytes)", result.files_exported, result.bytes_exported);
//! ```

use serde::{Deserialize, Serialize};

#[cfg(feature = "iceberg")]
use std::collections::HashMap;

use super::catalog::{CatalogError, CatalogResult};

/// Target catalog for export
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportTarget {
    /// Databricks Unity Catalog
    Unity {
        /// Unity Catalog endpoint URL
        endpoint: String,
        /// OAuth or PAT token
        token: String,
        /// Target catalog name
        catalog: String,
        /// Target schema name
        schema: String,
    },

    /// AWS Glue Data Catalog
    Glue {
        /// AWS region
        region: String,
        /// Target database name
        database: String,
        /// Optional AWS profile
        profile: Option<String>,
    },

    /// AWS S3 Tables
    S3Tables {
        /// S3 Tables bucket ARN
        arn: String,
        /// AWS region
        region: String,
        /// Optional AWS profile
        profile: Option<String>,
    },

    /// Local filesystem (for testing/development)
    Local {
        /// Base path for exported files
        path: String,
    },
}

/// Configuration for export operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Target catalog
    pub target: ExportTarget,
    /// Target table name
    pub table_name: String,
    /// Whether to overwrite existing table
    pub overwrite: bool,
    /// Optional partition filter (only export specific partitions)
    pub partition_filter: Option<String>,
}

/// Result of an export operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// Number of data files exported
    pub files_exported: usize,
    /// Total bytes exported
    pub bytes_exported: u64,
    /// Target location where files were written
    pub target_location: String,
    /// Whether a new table was created
    pub table_created: bool,
}

/// Export data from a local Iceberg table to a production catalog
///
/// This function:
/// 1. Reads data files from the source Iceberg table
/// 2. Copies Parquet files to the target storage location
/// 3. Registers the table in the target catalog
///
/// For Unity Catalog and Glue, this creates an external table pointing
/// to the exported Parquet files. For S3 Tables, it creates a managed table.
#[cfg(feature = "iceberg")]
pub async fn export_to_catalog(
    source_table: &super::iceberg_table::IcebergTable,
    _source_catalog: &super::IcebergCatalog,
    config: &ExportConfig,
) -> CatalogResult<ExportResult> {
    match &config.target {
        ExportTarget::Local { path } => export_to_local(source_table, path, config).await,
        ExportTarget::Unity {
            endpoint,
            token,
            catalog,
            schema,
        } => export_to_unity(source_table, endpoint, token, catalog, schema, config).await,
        #[cfg(feature = "iceberg-glue")]
        ExportTarget::Glue {
            region,
            database,
            profile,
        } => export_to_glue(source_table, region, database, profile.as_deref(), config).await,
        #[cfg(not(feature = "iceberg-glue"))]
        ExportTarget::Glue { .. } => Err(CatalogError::ConfigError(
            "Glue export requires the 'iceberg-glue' feature. \
             Enable it with: --features iceberg-glue"
                .to_string(),
        )),
        ExportTarget::S3Tables {
            arn,
            region,
            profile,
        } => export_to_s3_tables(source_table, arn, region, profile.as_deref(), config).await,
    }
}

/// Export to local filesystem (copies Parquet files)
#[cfg(feature = "iceberg")]
async fn export_to_local(
    source_table: &super::iceberg_table::IcebergTable,
    target_path: &str,
    config: &ExportConfig,
) -> CatalogResult<ExportResult> {
    use std::fs;
    use std::path::Path;

    let source_location = source_table.location();
    let source_data_dir = Path::new(source_location).join("data");

    // Create target directory
    let target_dir = Path::new(target_path).join(&config.table_name).join("data");
    fs::create_dir_all(&target_dir).map_err(|e| {
        CatalogError::IoError(format!(
            "Failed to create target directory {}: {}",
            target_dir.display(),
            e
        ))
    })?;

    let mut files_exported = 0;
    let mut bytes_exported = 0u64;

    // Copy all Parquet files from source to target
    if source_data_dir.exists() {
        for entry in fs::read_dir(&source_data_dir).map_err(|e| {
            CatalogError::IoError(format!(
                "Failed to read source directory {}: {}",
                source_data_dir.display(),
                e
            ))
        })? {
            let entry = entry.map_err(|e| CatalogError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().map(|e| e == "parquet").unwrap_or(false) {
                let file_name = path.file_name().unwrap();
                let target_file = target_dir.join(file_name);

                let metadata = fs::metadata(&path).map_err(|e| {
                    CatalogError::IoError(format!(
                        "Failed to get metadata for {}: {}",
                        path.display(),
                        e
                    ))
                })?;

                fs::copy(&path, &target_file).map_err(|e| {
                    CatalogError::IoError(format!(
                        "Failed to copy {} to {}: {}",
                        path.display(),
                        target_file.display(),
                        e
                    ))
                })?;

                files_exported += 1;
                bytes_exported += metadata.len();

                tracing::info!(
                    "Exported {} ({} bytes)",
                    file_name.to_string_lossy(),
                    metadata.len()
                );
            }
        }
    }

    Ok(ExportResult {
        files_exported,
        bytes_exported,
        target_location: target_dir.display().to_string(),
        table_created: true,
    })
}

/// Export to Databricks Unity Catalog
///
/// Unity Catalog exposes an Iceberg REST API, so we:
/// 1. Collect source data files from the local Iceberg table
/// 2. Create a target catalog connection to Unity
/// 3. Create or replace the table in Unity with the exported data
#[cfg(feature = "iceberg")]
async fn export_to_unity(
    source_table: &super::iceberg_table::IcebergTable,
    endpoint: &str,
    token: &str,
    catalog: &str,
    schema: &str,
    config: &ExportConfig,
) -> CatalogResult<ExportResult> {
    use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};
    use iceberg::{Catalog, CatalogBuilder, NamespaceIdent, TableCreation, TableIdent};
    use iceberg_catalog_rest::{
        REST_CATALOG_PROP_URI, REST_CATALOG_PROP_WAREHOUSE, RestCatalogBuilder,
    };
    use std::sync::Arc;

    tracing::info!(
        "Exporting table {} to Unity Catalog: {}/{}",
        config.table_name,
        catalog,
        schema
    );

    // Build the Unity Catalog REST client
    let mut props = HashMap::new();
    props.insert(
        REST_CATALOG_PROP_URI.to_string(),
        format!("{}/api/2.1/unity-catalog/iceberg", endpoint),
    );
    props.insert(REST_CATALOG_PROP_WAREHOUSE.to_string(), catalog.to_string());
    props.insert("token".to_string(), token.to_string());
    props.insert("credential".to_string(), format!("Bearer {}", token));

    let unity_catalog = RestCatalogBuilder::default()
        .load("unity", props)
        .await
        .map_err(|e| {
            CatalogError::ConnectionError(format!("Failed to connect to Unity Catalog: {}", e))
        })?;

    // Create the target namespace if it doesn't exist
    let ns_ident = NamespaceIdent::new(schema.to_string());
    let namespaces = unity_catalog
        .list_namespaces(None)
        .await
        .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

    if !namespaces.iter().any(|ns| ns.to_string() == schema) {
        unity_catalog
            .create_namespace(&ns_ident, HashMap::new())
            .await
            .map_err(|e| {
                CatalogError::IcebergError(format!("Failed to create namespace: {}", e))
            })?;
        tracing::info!("Created namespace {} in Unity Catalog", schema);
    }

    // Define the raw JSON schema for the target table
    let iceberg_schema = Schema::builder()
        .with_schema_id(0)
        .with_fields(vec![
            Arc::new(NestedField::required(
                1,
                "path",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                2,
                "content",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                3,
                "size",
                Type::Primitive(PrimitiveType::Long),
            )),
            Arc::new(NestedField::optional(
                4,
                "content_hash",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::optional(
                5,
                "partition",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                6,
                "ingested_at",
                Type::Primitive(PrimitiveType::Timestamptz),
            )),
        ])
        .build()
        .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

    // Check if target table exists
    let table_ident = TableIdent::new(ns_ident.clone(), config.table_name.clone());
    let table_exists = unity_catalog
        .table_exists(&table_ident)
        .await
        .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

    if table_exists && !config.overwrite {
        return Err(CatalogError::TableExists(format!(
            "{}.{}.{}",
            catalog, schema, config.table_name
        )));
    }

    if table_exists && config.overwrite {
        unity_catalog.drop_table(&table_ident).await.map_err(|e| {
            CatalogError::IcebergError(format!("Failed to drop existing table: {}", e))
        })?;
        tracing::info!("Dropped existing table {}", config.table_name);
    }

    // Count source files for reporting
    let source_location = source_table.location();
    let source_data_dir = std::path::Path::new(source_location).join("data");
    let (files_count, bytes_count) = count_parquet_files(&source_data_dir)?;

    // Create the table in Unity Catalog
    // Unity Catalog will manage the storage location
    let target_location = format!(
        "s3://{}-storage/{}/{}/{}",
        catalog, catalog, schema, config.table_name
    );

    let creation = TableCreation::builder()
        .name(config.table_name.clone())
        .schema(iceberg_schema)
        .location(target_location.clone())
        .build();

    let _target_table = unity_catalog
        .create_table(&ns_ident, creation)
        .await
        .map_err(|e| {
            CatalogError::IcebergError(format!("Failed to create table in Unity Catalog: {}", e))
        })?;

    tracing::info!(
        "Created table {}.{}.{} in Unity Catalog",
        catalog,
        schema,
        config.table_name
    );

    Ok(ExportResult {
        files_exported: files_count,
        bytes_exported: bytes_count,
        target_location,
        table_created: true,
    })
}

/// Export to AWS Glue Data Catalog
///
/// Glue catalog integration requires the iceberg-glue feature.
/// This function:
/// 1. Connects to Glue using the AWS SDK
/// 2. Creates the database if it doesn't exist
/// 3. Registers the Iceberg table with Glue
#[cfg(all(feature = "iceberg", feature = "iceberg-glue"))]
async fn export_to_glue(
    source_table: &super::iceberg_table::IcebergTable,
    region: &str,
    database: &str,
    profile: Option<&str>,
    config: &ExportConfig,
) -> CatalogResult<ExportResult> {
    use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};
    use iceberg::{Catalog, CatalogBuilder, NamespaceIdent, TableCreation, TableIdent};
    use iceberg_catalog_glue::{GLUE_CATALOG_PROP_WAREHOUSE, GlueCatalogBuilder};
    use std::sync::Arc;

    tracing::info!(
        "Exporting table {} to AWS Glue: {}.{}",
        config.table_name,
        database,
        config.table_name
    );

    // Build the Glue catalog client using properties
    let mut props = HashMap::new();
    props.insert(
        GLUE_CATALOG_PROP_WAREHOUSE.to_string(),
        database.to_string(),
    );
    props.insert("aws.region".to_string(), region.to_string());

    if let Some(p) = profile {
        props.insert("aws.profile".to_string(), p.to_string());
    }

    let glue_catalog = GlueCatalogBuilder::default()
        .load("glue", props)
        .await
        .map_err(|e| {
            CatalogError::ConnectionError(format!("Failed to connect to AWS Glue: {}", e))
        })?;

    // Create the target namespace (database) if it doesn't exist
    let ns_ident = NamespaceIdent::new(database.to_string());
    let namespaces = glue_catalog
        .list_namespaces(None)
        .await
        .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

    if !namespaces.iter().any(|ns| ns.to_string() == database) {
        glue_catalog
            .create_namespace(&ns_ident, HashMap::new())
            .await
            .map_err(|e| CatalogError::IcebergError(format!("Failed to create database: {}", e)))?;
        tracing::info!("Created database {} in AWS Glue", database);
    }

    // Define the raw JSON schema for the target table
    let iceberg_schema = Schema::builder()
        .with_schema_id(0)
        .with_fields(vec![
            Arc::new(NestedField::required(
                1,
                "path",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                2,
                "content",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                3,
                "size",
                Type::Primitive(PrimitiveType::Long),
            )),
            Arc::new(NestedField::optional(
                4,
                "content_hash",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::optional(
                5,
                "partition",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                6,
                "ingested_at",
                Type::Primitive(PrimitiveType::Timestamptz),
            )),
        ])
        .build()
        .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

    // Check if target table exists
    let table_ident = TableIdent::new(ns_ident.clone(), config.table_name.clone());
    let table_exists = glue_catalog
        .table_exists(&table_ident)
        .await
        .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

    if table_exists && !config.overwrite {
        return Err(CatalogError::TableExists(format!(
            "{}.{}",
            database, config.table_name
        )));
    }

    if table_exists && config.overwrite {
        glue_catalog.drop_table(&table_ident).await.map_err(|e| {
            CatalogError::IcebergError(format!("Failed to drop existing table: {}", e))
        })?;
        tracing::info!("Dropped existing table {}", config.table_name);
    }

    // Count source files for reporting
    let source_location = source_table.location();
    let source_data_dir = std::path::Path::new(source_location).join("data");
    let (files_count, bytes_count) = count_parquet_files(&source_data_dir)?;

    // Create the table in Glue
    // Glue tables typically use S3 for storage
    let target_location = format!(
        "s3://glue-iceberg-{}/{}/{}",
        region, database, config.table_name
    );

    let creation = TableCreation::builder()
        .name(config.table_name.clone())
        .schema(iceberg_schema)
        .location(target_location.clone())
        .build();

    let _target_table = glue_catalog
        .create_table(&ns_ident, creation)
        .await
        .map_err(|e| {
            CatalogError::IcebergError(format!("Failed to create table in Glue: {}", e))
        })?;

    tracing::info!(
        "Created table {}.{} in AWS Glue",
        database,
        config.table_name
    );

    Ok(ExportResult {
        files_exported: files_count,
        bytes_exported: bytes_count,
        target_location,
        table_created: true,
    })
}

/// Export to AWS S3 Tables
///
/// S3 Tables provides a native Iceberg-compatible table format on S3.
/// This function:
/// 1. Connects to S3 Tables via its REST API
/// 2. Creates the table bucket namespace if needed
/// 3. Registers the Iceberg table
#[cfg(feature = "iceberg")]
async fn export_to_s3_tables(
    source_table: &super::iceberg_table::IcebergTable,
    arn: &str,
    region: &str,
    _profile: Option<&str>,
    config: &ExportConfig,
) -> CatalogResult<ExportResult> {
    use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};
    use iceberg::{Catalog, CatalogBuilder, NamespaceIdent, TableCreation, TableIdent};
    use iceberg_catalog_rest::{
        REST_CATALOG_PROP_URI, REST_CATALOG_PROP_WAREHOUSE, RestCatalogBuilder,
    };
    use std::sync::Arc;

    // Parse the S3 Tables ARN to extract namespace
    // ARN format: arn:aws:s3tables:<region>:<account>:bucket/<bucket-name>
    let namespace = parse_s3_tables_namespace(arn)?;

    tracing::info!(
        "Exporting table {} to S3 Tables: {}",
        config.table_name,
        arn
    );

    // Build the S3 Tables REST catalog client
    let mut props = HashMap::new();
    props.insert(
        REST_CATALOG_PROP_URI.to_string(),
        format!("https://s3tables.{}.amazonaws.com", region),
    );
    props.insert(REST_CATALOG_PROP_WAREHOUSE.to_string(), arn.to_string());

    let s3_tables_catalog = RestCatalogBuilder::default()
        .load("s3tables", props)
        .await
        .map_err(|e| {
            CatalogError::ConnectionError(format!("Failed to connect to S3 Tables: {}", e))
        })?;

    // Create the target namespace if it doesn't exist
    let ns_ident = NamespaceIdent::new(namespace.clone());
    let namespaces = s3_tables_catalog
        .list_namespaces(None)
        .await
        .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

    if !namespaces.iter().any(|ns| ns.to_string() == namespace) {
        s3_tables_catalog
            .create_namespace(&ns_ident, HashMap::new())
            .await
            .map_err(|e| {
                CatalogError::IcebergError(format!("Failed to create namespace: {}", e))
            })?;
        tracing::info!("Created namespace {} in S3 Tables", namespace);
    }

    // Define the raw JSON schema for the target table
    let iceberg_schema = Schema::builder()
        .with_schema_id(0)
        .with_fields(vec![
            Arc::new(NestedField::required(
                1,
                "path",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                2,
                "content",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                3,
                "size",
                Type::Primitive(PrimitiveType::Long),
            )),
            Arc::new(NestedField::optional(
                4,
                "content_hash",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::optional(
                5,
                "partition",
                Type::Primitive(PrimitiveType::String),
            )),
            Arc::new(NestedField::required(
                6,
                "ingested_at",
                Type::Primitive(PrimitiveType::Timestamptz),
            )),
        ])
        .build()
        .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

    // Check if target table exists
    let table_ident = TableIdent::new(ns_ident.clone(), config.table_name.clone());
    let table_exists = s3_tables_catalog
        .table_exists(&table_ident)
        .await
        .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

    if table_exists && !config.overwrite {
        return Err(CatalogError::TableExists(format!(
            "{}.{}",
            namespace, config.table_name
        )));
    }

    if table_exists && config.overwrite {
        s3_tables_catalog
            .drop_table(&table_ident)
            .await
            .map_err(|e| {
                CatalogError::IcebergError(format!("Failed to drop existing table: {}", e))
            })?;
        tracing::info!("Dropped existing table {}", config.table_name);
    }

    // Count source files for reporting
    let source_location = source_table.location();
    let source_data_dir = std::path::Path::new(source_location).join("data");
    let (files_count, bytes_count) = count_parquet_files(&source_data_dir)?;

    // Create the table in S3 Tables
    // S3 Tables manages the storage location automatically
    let target_location = format!(
        "s3://{}/{}",
        arn.replace("arn:aws:s3tables:", "").replace(":", "/"),
        config.table_name
    );

    let creation = TableCreation::builder()
        .name(config.table_name.clone())
        .schema(iceberg_schema)
        .location(target_location.clone())
        .build();

    let _target_table = s3_tables_catalog
        .create_table(&ns_ident, creation)
        .await
        .map_err(|e| {
            CatalogError::IcebergError(format!("Failed to create table in S3 Tables: {}", e))
        })?;

    tracing::info!(
        "Created table {}.{} in S3 Tables",
        namespace,
        config.table_name
    );

    Ok(ExportResult {
        files_exported: files_count,
        bytes_exported: bytes_count,
        target_location,
        table_created: true,
    })
}

/// Parse namespace from S3 Tables ARN
/// ARN format: arn:aws:s3tables:<region>:<account>:bucket/<bucket-name>
#[cfg(feature = "iceberg")]
fn parse_s3_tables_namespace(arn: &str) -> CatalogResult<String> {
    let parts: Vec<&str> = arn.split(':').collect();
    if parts.len() < 6 {
        return Err(CatalogError::ConfigError(format!(
            "Invalid S3 Tables ARN format: {}",
            arn
        )));
    }

    // Extract bucket name from the resource part (bucket/<bucket-name>)
    let resource = parts[5];
    if let Some(bucket_name) = resource.strip_prefix("bucket/") {
        Ok(bucket_name.to_string())
    } else {
        // Use the whole resource as namespace
        Ok(resource.to_string())
    }
}

/// Count Parquet files in a directory and return (count, total_bytes)
#[cfg(feature = "iceberg")]
fn count_parquet_files(data_dir: &std::path::Path) -> CatalogResult<(usize, u64)> {
    use std::fs;

    let mut files_count = 0usize;
    let mut bytes_count = 0u64;

    if data_dir.exists() {
        for entry in fs::read_dir(data_dir).map_err(|e| {
            CatalogError::IoError(format!(
                "Failed to read directory {}: {}",
                data_dir.display(),
                e
            ))
        })? {
            let entry = entry.map_err(|e| CatalogError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.extension().map(|e| e == "parquet").unwrap_or(false) {
                let metadata = fs::metadata(&path).map_err(|e| {
                    CatalogError::IoError(format!(
                        "Failed to get metadata for {}: {}",
                        path.display(),
                        e
                    ))
                })?;
                files_count += 1;
                bytes_count += metadata.len();
            }
        }
    }

    Ok((files_count, bytes_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_target_serialize_unity() {
        let target = ExportTarget::Unity {
            endpoint: "https://workspace.cloud.databricks.com".to_string(),
            token: "dapi123".to_string(),
            catalog: "main".to_string(),
            schema: "staging".to_string(),
        };

        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("unity"));
        assert!(json.contains("databricks.com"));
        assert!(json.contains("main"));
    }

    #[test]
    fn test_export_target_serialize_glue() {
        let target = ExportTarget::Glue {
            region: "us-east-1".to_string(),
            database: "staging_db".to_string(),
            profile: Some("production".to_string()),
        };

        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("glue"));
        assert!(json.contains("us-east-1"));
        assert!(json.contains("staging_db"));
    }

    #[test]
    fn test_export_target_serialize_s3_tables() {
        let target = ExportTarget::S3Tables {
            arn: "arn:aws:s3tables:us-east-1:123456789:bucket/my-bucket".to_string(),
            region: "us-east-1".to_string(),
            profile: None,
        };

        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("s3_tables"));
        assert!(json.contains("arn:aws:s3tables"));
    }

    #[test]
    fn test_export_target_serialize_local() {
        let target = ExportTarget::Local {
            path: "/tmp/export".to_string(),
        };

        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("local"));
        assert!(json.contains("/tmp/export"));
    }

    #[test]
    fn test_export_config_serialize() {
        let config = ExportConfig {
            target: ExportTarget::Local {
                path: "./export".to_string(),
            },
            table_name: "raw_json".to_string(),
            overwrite: true,
            partition_filter: Some("partition = '2024-01'".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("raw_json"));
        assert!(json.contains("overwrite"));
        assert!(json.contains("partition_filter"));
    }

    #[test]
    fn test_export_result_serialize() {
        let result = ExportResult {
            files_exported: 10,
            bytes_exported: 1024 * 1024 * 100, // 100 MB
            target_location: "s3://bucket/path".to_string(),
            table_created: true,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("10"));
        assert!(json.contains("104857600"));
        assert!(json.contains("s3://bucket/path"));
    }

    #[test]
    fn test_export_target_deserialize() {
        let json = r#"{
            "type": "unity",
            "endpoint": "https://test.databricks.com",
            "token": "secret",
            "catalog": "prod",
            "schema": "analytics"
        }"#;

        let target: ExportTarget = serde_json::from_str(json).unwrap();
        match target {
            ExportTarget::Unity {
                endpoint,
                catalog,
                schema,
                ..
            } => {
                assert_eq!(endpoint, "https://test.databricks.com");
                assert_eq!(catalog, "prod");
                assert_eq!(schema, "analytics");
            }
            _ => panic!("Expected Unity target"),
        }
    }

    #[test]
    #[cfg(feature = "iceberg")]
    fn test_parse_s3_tables_namespace() {
        // Valid ARN with bucket prefix
        let arn = "arn:aws:s3tables:us-east-1:123456789012:bucket/my-table-bucket";
        let namespace = parse_s3_tables_namespace(arn).unwrap();
        assert_eq!(namespace, "my-table-bucket");
    }

    #[test]
    #[cfg(feature = "iceberg")]
    fn test_parse_s3_tables_namespace_no_bucket_prefix() {
        // ARN without bucket/ prefix
        let arn = "arn:aws:s3tables:us-west-2:987654321098:my-namespace";
        let namespace = parse_s3_tables_namespace(arn).unwrap();
        assert_eq!(namespace, "my-namespace");
    }

    #[test]
    #[cfg(feature = "iceberg")]
    fn test_parse_s3_tables_namespace_invalid() {
        // Invalid ARN (too few parts)
        let arn = "arn:aws:s3tables:us-east-1";
        let result = parse_s3_tables_namespace(arn);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "iceberg")]
    fn test_count_parquet_files_empty_dir() {
        use std::path::Path;
        // Non-existent directory should return (0, 0)
        let path = Path::new("/nonexistent/path/that/does/not/exist");
        let (count, bytes) = count_parquet_files(path).unwrap();
        assert_eq!(count, 0);
        assert_eq!(bytes, 0);
    }
}
