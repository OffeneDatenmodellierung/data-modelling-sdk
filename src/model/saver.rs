//! Model saving functionality
//!
//! Saves models to storage backends, handling YAML serialization.
//!
//! File structure:
//! - Base directory (workspace_path)
//!   - Domain directories (e.g., `domain1/`, `domain2/`)
//!     - `domain.yaml` - Domain definition
//!     - `{name}.odcs.yaml` - ODCS table files
//!     - `{name}.odps.yaml` - ODPS product files
//!     - `{name}.cads.yaml` - CADS asset files
//!   - `tables/` - Legacy: tables not in any domain (backward compatibility)

use crate::export::{cads::CADSExporter, odcs::ODCSExporter, odps::ODPSExporter};
#[cfg(feature = "bpmn")]
use crate::models::bpmn::BPMNModel;
#[cfg(feature = "dmn")]
use crate::models::dmn::DMNModel;
#[cfg(feature = "openapi")]
use crate::models::openapi::{OpenAPIFormat, OpenAPIModel};
use crate::models::{cads::CADSAsset, domain::Domain, odps::ODPSDataProduct, table::Table};
use crate::storage::{StorageBackend, StorageError};
use anyhow::Result;
use serde_yaml;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Model saver that uses a storage backend
pub struct ModelSaver<B: StorageBackend> {
    storage: B,
}

impl<B: StorageBackend> ModelSaver<B> {
    /// Create a new model saver with the given storage backend
    pub fn new(storage: B) -> Self {
        Self { storage }
    }

    /// Save a table to storage
    ///
    /// Saves the table as a YAML file in the workspace's `tables/` directory.
    /// The filename will be based on the table name if yaml_file_path is not provided.
    pub async fn save_table(
        &self,
        workspace_path: &str,
        table: &TableData,
    ) -> Result<(), StorageError> {
        let tables_dir = format!("{}/tables", workspace_path);

        // Ensure tables directory exists
        if !self.storage.dir_exists(&tables_dir).await? {
            self.storage.create_dir(&tables_dir).await?;
        }

        // Determine file path
        let file_path = if let Some(ref yaml_path) = table.yaml_file_path {
            format!(
                "{}/{}",
                workspace_path,
                yaml_path.strip_prefix('/').unwrap_or(yaml_path)
            )
        } else {
            // Generate filename from table name
            let sanitized_name = sanitize_filename(&table.name);
            format!("{}/tables/{}.yaml", workspace_path, sanitized_name)
        };

        // Serialize table to YAML
        let yaml_content = serde_yaml::to_string(&table.yaml_value).map_err(|e| {
            StorageError::SerializationError(format!("Failed to serialize table: {}", e))
        })?;

        // Write to storage
        self.storage
            .write_file(&file_path, yaml_content.as_bytes())
            .await?;

        info!("Saved table '{}' to {}", table.name, file_path);
        Ok(())
    }

    /// Save relationships to storage
    ///
    /// Saves relationships to `relationships.yaml` in the workspace directory.
    /// Note: Relationships are now stored within domain.yaml files, but this method
    /// is kept for backward compatibility.
    pub async fn save_relationships(
        &self,
        workspace_path: &str,
        relationships: &[RelationshipData],
    ) -> Result<(), StorageError> {
        let file_path = format!("{}/relationships.yaml", workspace_path);

        // Serialize relationships to YAML
        let mut yaml_map = serde_yaml::Mapping::new();
        let mut rels_array = serde_yaml::Sequence::new();
        for rel in relationships {
            rels_array.push(rel.yaml_value.clone());
        }
        yaml_map.insert(
            serde_yaml::Value::String("relationships".to_string()),
            serde_yaml::Value::Sequence(rels_array),
        );
        let yaml_value = serde_yaml::Value::Mapping(yaml_map);

        let yaml_content = serde_yaml::to_string(&yaml_value).map_err(|e| {
            StorageError::SerializationError(format!("Failed to write YAML: {}", e))
        })?;

        // Write to storage
        self.storage
            .write_file(&file_path, yaml_content.as_bytes())
            .await?;

        info!(
            "Saved {} relationships to {}",
            relationships.len(),
            file_path
        );
        Ok(())
    }

    /// Save a domain to storage
    ///
    /// Saves the domain as `domain.yaml` in a domain directory named after the domain.
    /// Also saves all associated ODCS tables, ODPS products, and CADS assets within the domain directory.
    pub async fn save_domain(
        &self,
        workspace_path: &str,
        domain: &Domain,
        tables: &HashMap<Uuid, Table>,
        odps_products: &HashMap<Uuid, ODPSDataProduct>,
        cads_assets: &HashMap<Uuid, CADSAsset>,
    ) -> Result<(), StorageError> {
        let sanitized_domain_name = sanitize_filename(&domain.name);
        let domain_dir = format!("{}/{}", workspace_path, sanitized_domain_name);

        // Ensure domain directory exists
        if !self.storage.dir_exists(&domain_dir).await? {
            self.storage.create_dir(&domain_dir).await?;
        }

        // Save domain.yaml
        let domain_yaml = domain.to_yaml().map_err(|e| {
            StorageError::SerializationError(format!("Failed to serialize domain: {}", e))
        })?;
        let domain_file_path = format!("{}/domain.yaml", domain_dir);
        self.storage
            .write_file(&domain_file_path, domain_yaml.as_bytes())
            .await?;
        info!("Saved domain '{}' to {}", domain.name, domain_file_path);

        // Save ODCS tables referenced by ODCSNodes
        for odcs_node in &domain.odcs_nodes {
            if let Some(table_id) = odcs_node.table_id
                && let Some(table) = tables.get(&table_id)
            {
                let sanitized_table_name = sanitize_filename(&table.name);
                let table_file_path = format!("{}/{}.odcs.yaml", domain_dir, sanitized_table_name);
                let odcs_yaml = ODCSExporter::export_table(table, "odcs_v3_1_0");
                self.storage
                    .write_file(&table_file_path, odcs_yaml.as_bytes())
                    .await?;
                info!("Saved ODCS table '{}' to {}", table.name, table_file_path);
            }
        }

        // Save ODPS products (if we have a way to identify which products belong to this domain)
        // For now, we'll save all products that have a matching domain field
        for product in odps_products.values() {
            if let Some(product_domain) = &product.domain
                && product_domain == &domain.name
            {
                let sanitized_product_name =
                    sanitize_filename(product.name.as_ref().unwrap_or(&product.id));
                let product_file_path =
                    format!("{}/{}.odps.yaml", domain_dir, sanitized_product_name);
                let odps_yaml = ODPSExporter::export_product(product);
                self.storage
                    .write_file(&product_file_path, odps_yaml.as_bytes())
                    .await?;
                info!(
                    "Saved ODPS product '{}' to {}",
                    product.id, product_file_path
                );
            }
        }

        // Save CADS assets referenced by CADSNodes
        for cads_node in &domain.cads_nodes {
            if let Some(cads_asset_id) = cads_node.cads_asset_id
                && let Some(asset) = cads_assets.get(&cads_asset_id)
            {
                let sanitized_asset_name = sanitize_filename(&asset.name);
                let asset_file_path = format!("{}/{}.cads.yaml", domain_dir, sanitized_asset_name);
                let cads_yaml = CADSExporter::export_asset(asset);
                self.storage
                    .write_file(&asset_file_path, cads_yaml.as_bytes())
                    .await?;
                info!("Saved CADS asset '{}' to {}", asset.name, asset_file_path);
            }
        }

        Ok(())
    }

    /// Save an ODPS product to a domain directory
    ///
    /// Saves the product as `{product_name}.odps.yaml` in the specified domain directory.
    pub async fn save_odps_product(
        &self,
        workspace_path: &str,
        domain_name: &str,
        product: &ODPSDataProduct,
    ) -> Result<(), StorageError> {
        let sanitized_domain_name = sanitize_filename(domain_name);
        let domain_dir = format!("{}/{}", workspace_path, sanitized_domain_name);

        // Ensure domain directory exists
        if !self.storage.dir_exists(&domain_dir).await? {
            self.storage.create_dir(&domain_dir).await?;
        }

        let sanitized_product_name =
            sanitize_filename(product.name.as_ref().unwrap_or(&product.id));
        let product_file_path = format!("{}/{}.odps.yaml", domain_dir, sanitized_product_name);
        let odps_yaml = ODPSExporter::export_product(product);
        self.storage
            .write_file(&product_file_path, odps_yaml.as_bytes())
            .await?;

        info!(
            "Saved ODPS product '{}' to {}",
            product.id, product_file_path
        );
        Ok(())
    }

    /// Save a CADS asset to a domain directory
    ///
    /// Saves the asset as `{asset_name}.cads.yaml` in the specified domain directory.
    pub async fn save_cads_asset(
        &self,
        workspace_path: &str,
        domain_name: &str,
        asset: &CADSAsset,
    ) -> Result<(), StorageError> {
        let sanitized_domain_name = sanitize_filename(domain_name);
        let domain_dir = format!("{}/{}", workspace_path, sanitized_domain_name);

        // Ensure domain directory exists
        if !self.storage.dir_exists(&domain_dir).await? {
            self.storage.create_dir(&domain_dir).await?;
        }

        let sanitized_asset_name = sanitize_filename(&asset.name);
        let asset_file_path = format!("{}/{}.cads.yaml", domain_dir, sanitized_asset_name);
        let cads_yaml = CADSExporter::export_asset(asset);
        self.storage
            .write_file(&asset_file_path, cads_yaml.as_bytes())
            .await?;

        info!("Saved CADS asset '{}' to {}", asset.name, asset_file_path);
        Ok(())
    }

    /// Save a BPMN model to a domain directory
    ///
    /// Saves the model as `{model_name}.bpmn.xml` in the specified domain directory.
    #[cfg(feature = "bpmn")]
    pub async fn save_bpmn_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model: &BPMNModel,
        xml_content: &str,
    ) -> Result<(), StorageError> {
        let sanitized_domain_name = sanitize_filename(domain_name);
        let domain_dir = format!("{}/{}", workspace_path, sanitized_domain_name);

        // Ensure domain directory exists
        if !self.storage.dir_exists(&domain_dir).await? {
            self.storage.create_dir(&domain_dir).await?;
        }

        let sanitized_model_name = sanitize_filename(&model.name);
        let model_file_path = format!("{}/{}.bpmn.xml", domain_dir, sanitized_model_name);
        self.storage
            .write_file(&model_file_path, xml_content.as_bytes())
            .await?;

        info!("Saved BPMN model '{}' to {}", model.name, model_file_path);
        Ok(())
    }

    /// Save a DMN model to a domain directory
    ///
    /// Saves the model as `{model_name}.dmn.xml` in the specified domain directory.
    #[cfg(feature = "dmn")]
    pub async fn save_dmn_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model: &DMNModel,
        xml_content: &str,
    ) -> Result<(), StorageError> {
        let sanitized_domain_name = sanitize_filename(domain_name);
        let domain_dir = format!("{}/{}", workspace_path, sanitized_domain_name);

        // Ensure domain directory exists
        if !self.storage.dir_exists(&domain_dir).await? {
            self.storage.create_dir(&domain_dir).await?;
        }

        let sanitized_model_name = sanitize_filename(&model.name);
        let model_file_path = format!("{}/{}.dmn.xml", domain_dir, sanitized_model_name);
        self.storage
            .write_file(&model_file_path, xml_content.as_bytes())
            .await?;

        info!("Saved DMN model '{}' to {}", model.name, model_file_path);
        Ok(())
    }

    /// Save an OpenAPI specification to a domain directory
    ///
    /// Saves the specification as `{api_name}.openapi.yaml` or `.openapi.json` in the specified domain directory.
    #[cfg(feature = "openapi")]
    pub async fn save_openapi_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model: &OpenAPIModel,
        content: &str,
    ) -> Result<(), StorageError> {
        let sanitized_domain_name = sanitize_filename(domain_name);
        let domain_dir = format!("{}/{}", workspace_path, sanitized_domain_name);

        // Ensure domain directory exists
        if !self.storage.dir_exists(&domain_dir).await? {
            self.storage.create_dir(&domain_dir).await?;
        }

        let sanitized_api_name = sanitize_filename(&model.name);
        let extension = match model.format {
            OpenAPIFormat::Yaml => "yaml",
            OpenAPIFormat::Json => "json",
        };
        let model_file_path = format!(
            "{}/{}.openapi.{}",
            domain_dir, sanitized_api_name, extension
        );
        self.storage
            .write_file(&model_file_path, content.as_bytes())
            .await?;

        info!("Saved OpenAPI spec '{}' to {}", model.name, model_file_path);
        Ok(())
    }
}

/// Table data to save
#[derive(Debug, Clone)]
pub struct TableData {
    pub id: Uuid,
    pub name: String,
    pub yaml_file_path: Option<String>,
    pub yaml_value: serde_yaml::Value,
}

/// Relationship data to save
#[derive(Debug, Clone)]
pub struct RelationshipData {
    pub id: Uuid,
    pub source_table_id: Uuid,
    pub target_table_id: Uuid,
    pub yaml_value: serde_yaml::Value,
}

/// Sanitize a filename by removing invalid characters
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
