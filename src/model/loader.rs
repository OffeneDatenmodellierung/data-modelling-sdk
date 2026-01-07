//! Model loading functionality
//!
//! Loads models from storage backends, handling YAML parsing and validation.
//!
//! Supports both file-based loading (FileSystemStorageBackend, BrowserStorageBackend)
//! and API-based loading (ApiStorageBackend).
//!
//! ## File Naming Convention
//!
//! All files use a flat naming pattern in the workspace root directory:
//! - `workspace.yaml` - workspace metadata with references to all assets
//! - `{workspace}_{domain}_{system}_{resource}.odcs.yaml` - ODCS table files
//! - `{workspace}_{domain}_{system}_{resource}.odps.yaml` - ODPS product files
//! - `{workspace}_{domain}_{system}_{resource}.cads.yaml` - CADS asset files
//! - `relationships.yaml` - relationship definitions
//!
//! Where `{system}` is optional if the resource is at the domain level.

#[cfg(feature = "bpmn")]
use crate::import::bpmn::BPMNImporter;
#[cfg(feature = "dmn")]
use crate::import::dmn::DMNImporter;
#[cfg(feature = "openapi")]
use crate::import::openapi::OpenAPIImporter;
use crate::import::{cads::CADSImporter, odcs::ODCSImporter, odps::ODPSImporter};
#[cfg(feature = "bpmn")]
use crate::models::bpmn::BPMNModel;
#[cfg(feature = "dmn")]
use crate::models::dmn::DMNModel;
use crate::models::domain_config::DomainConfig;
#[cfg(feature = "openapi")]
use crate::models::openapi::{OpenAPIFormat, OpenAPIModel};
use crate::models::workspace::{AssetType, Workspace};
use crate::models::{cads::CADSAsset, domain::Domain, odps::ODPSDataProduct, table::Table};
use crate::storage::{StorageBackend, StorageError};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

/// Model loader that uses a storage backend
pub struct ModelLoader<B: StorageBackend> {
    storage: B,
}

impl<B: StorageBackend> ModelLoader<B> {
    /// Create a new model loader with the given storage backend
    pub fn new(storage: B) -> Self {
        Self { storage }
    }

    /// Load a model from storage
    ///
    /// For file-based backends (FileSystemStorageBackend, BrowserStorageBackend):
    /// - Loads from flat files in workspace root using naming convention
    /// - Loads from `relationships.yaml` file
    ///
    /// For API backend (ApiStorageBackend), use `load_model_from_api()` instead.
    ///
    /// Returns the loaded model data and a list of orphaned relationships
    /// (relationships that reference non-existent tables).
    pub async fn load_model(&self, workspace_path: &str) -> Result<ModelLoadResult, StorageError> {
        // File-based loading implementation
        self.load_model_from_files(workspace_path).await
    }

    /// Load model from file-based storage using flat file naming convention
    async fn load_model_from_files(
        &self,
        workspace_path: &str,
    ) -> Result<ModelLoadResult, StorageError> {
        // Load tables from flat YAML files in workspace root
        let mut tables = Vec::new();
        let mut table_ids: HashMap<Uuid, String> = HashMap::new();

        let files = self.storage.list_files(workspace_path).await?;
        for file_name in files {
            // Only load supported asset files (skip workspace.yaml, relationships.yaml, etc.)
            if let Some(asset_type) = AssetType::from_filename(&file_name) {
                // Skip workspace-level files and non-ODCS files for table loading
                if asset_type == AssetType::Odcs {
                    let file_path = format!("{}/{}", workspace_path, file_name);
                    match self.load_table_from_yaml(&file_path, workspace_path).await {
                        Ok(table_data) => {
                            table_ids.insert(table_data.id, table_data.name.clone());
                            tables.push(table_data);
                        }
                        Err(e) => {
                            warn!("Failed to load table from {}: {}", file_path, e);
                        }
                    }
                }
            }
        }

        info!(
            "Loaded {} tables from workspace {}",
            tables.len(),
            workspace_path
        );

        // Load relationships from control file
        let relationships_file = format!("{}/relationships.yaml", workspace_path);
        let mut relationships = Vec::new();
        let mut orphaned_relationships = Vec::new();

        if self.storage.file_exists(&relationships_file).await? {
            match self.load_relationships_from_yaml(&relationships_file).await {
                Ok(loaded_rels) => {
                    // Separate valid and orphaned relationships
                    for rel in loaded_rels {
                        let source_exists = table_ids.contains_key(&rel.source_table_id);
                        let target_exists = table_ids.contains_key(&rel.target_table_id);

                        if source_exists && target_exists {
                            relationships.push(rel.clone());
                        } else {
                            orphaned_relationships.push(rel.clone());
                            warn!(
                                "Orphaned relationship {}: source={} (exists: {}), target={} (exists: {})",
                                rel.id,
                                rel.source_table_id,
                                source_exists,
                                rel.target_table_id,
                                target_exists
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to load relationships from {}: {}",
                        relationships_file, e
                    );
                }
            }
        }

        info!(
            "Loaded {} relationships ({} orphaned) from workspace {}",
            relationships.len(),
            orphaned_relationships.len(),
            workspace_path
        );

        Ok(ModelLoadResult {
            tables,
            relationships,
            orphaned_relationships,
        })
    }

    /// Load a table from a YAML file
    ///
    /// Uses ODCSImporter to fully parse the table structure, including all columns,
    /// metadata, and nested properties. This ensures complete table data is loaded.
    async fn load_table_from_yaml(
        &self,
        yaml_path: &str,
        workspace_path: &str,
    ) -> Result<TableData, StorageError> {
        let content = self.storage.read_file(yaml_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        // Use ODCSImporter to fully parse the table structure
        let mut importer = crate::import::odcs::ODCSImporter::new();
        let (table, parse_errors) = importer.parse_table(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse ODCS YAML: {}", e))
        })?;

        // Log any parse warnings/errors but don't fail if table was successfully parsed
        if !parse_errors.is_empty() {
            warn!(
                "Table '{}' parsed with {} warnings/errors",
                table.name,
                parse_errors.len()
            );
        }

        // Calculate relative path
        let relative_path = yaml_path
            .strip_prefix(workspace_path)
            .map(|s| s.strip_prefix('/').unwrap_or(s).to_string())
            .unwrap_or_else(|| yaml_path.to_string());

        Ok(TableData {
            id: table.id,
            name: table.name,
            yaml_file_path: Some(relative_path),
            yaml_content,
        })
    }

    /// Load relationships from YAML file
    async fn load_relationships_from_yaml(
        &self,
        yaml_path: &str,
    ) -> Result<Vec<RelationshipData>, StorageError> {
        let content = self.storage.read_file(yaml_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        let data: serde_yaml::Value = serde_yaml::from_str(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse YAML: {}", e))
        })?;

        let mut relationships = Vec::new();

        // Handle both formats: direct array or object with "relationships" key
        let rels_array = data
            .get("relationships")
            .and_then(|v| v.as_sequence())
            .or_else(|| data.as_sequence());

        if let Some(rels_array) = rels_array {
            for rel_data in rels_array {
                match self.parse_relationship(rel_data) {
                    Ok(rel) => relationships.push(rel),
                    Err(e) => {
                        warn!("Failed to parse relationship: {}", e);
                    }
                }
            }
        }

        Ok(relationships)
    }

    /// Parse a relationship from YAML value
    fn parse_relationship(
        &self,
        data: &serde_yaml::Value,
    ) -> Result<RelationshipData, StorageError> {
        let source_table_id = data
            .get("source_table_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                StorageError::SerializationError("Missing source_table_id".to_string())
            })?;

        let target_table_id = data
            .get("target_table_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                StorageError::SerializationError("Missing target_table_id".to_string())
            })?;

        // Parse existing UUID or generate deterministic one based on source and target table IDs
        let id = data
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(|| {
                crate::models::relationship::Relationship::generate_id(
                    source_table_id,
                    target_table_id,
                )
            });

        Ok(RelationshipData {
            id,
            source_table_id,
            target_table_id,
        })
    }

    /// Load all domains from storage
    ///
    /// Loads domains and assets from flat files in the workspace root directory.
    /// Uses the file naming convention: {workspace}_{domain}_{system}_{resource}.xxx.yaml
    ///
    /// Domain and system information is extracted from filenames and the workspace.yaml file.
    pub async fn load_domains(
        &self,
        workspace_path: &str,
    ) -> Result<DomainLoadResult, StorageError> {
        let mut domains = Vec::new();
        let mut tables = HashMap::new();
        let mut odps_products = HashMap::new();
        let mut cads_assets = HashMap::new();

        // Load workspace.yaml to get domain/system structure
        let workspace = self.load_workspace(workspace_path).await?;

        // If workspace.yaml exists, use its domain definitions
        if let Some(ws) = &workspace {
            for domain_ref in &ws.domains {
                domains.push(Domain::new(domain_ref.name.clone()));
            }
        }

        // Load all flat files from workspace root
        let files = self.storage.list_files(workspace_path).await?;

        for file_name in files {
            let Some(asset_type) = AssetType::from_filename(&file_name) else {
                continue;
            };

            // Skip workspace-level files
            if asset_type.is_workspace_level() {
                continue;
            }

            let file_path = format!("{}/{}", workspace_path, file_name);

            match asset_type {
                AssetType::Odcs => {
                    // Load ODCS table
                    match self.load_odcs_table_from_file(&file_path).await {
                        Ok(table) => {
                            tables.insert(table.id, table);
                        }
                        Err(e) => {
                            warn!("Failed to load ODCS table from {}: {}", file_path, e);
                        }
                    }
                }
                AssetType::Odps => {
                    // Load ODPS product
                    match self.load_odps_product_from_file(&file_path).await {
                        Ok(product) => {
                            odps_products.insert(
                                Uuid::parse_str(&product.id).unwrap_or_else(|_| Uuid::new_v4()),
                                product,
                            );
                        }
                        Err(e) => {
                            warn!("Failed to load ODPS product from {}: {}", file_path, e);
                        }
                    }
                }
                AssetType::Cads => {
                    // Load CADS asset
                    match self.load_cads_asset_from_file(&file_path).await {
                        Ok(asset) => {
                            cads_assets.insert(
                                Uuid::parse_str(&asset.id).unwrap_or_else(|_| Uuid::new_v4()),
                                asset,
                            );
                        }
                        Err(e) => {
                            warn!("Failed to load CADS asset from {}: {}", file_path, e);
                        }
                    }
                }
                _ => {
                    // Skip other asset types for now (BPMN, DMN, OpenAPI handled separately)
                }
            }
        }

        info!(
            "Loaded {} domains, {} tables, {} ODPS products, {} CADS assets from workspace {}",
            domains.len(),
            tables.len(),
            odps_products.len(),
            cads_assets.len(),
            workspace_path
        );

        Ok(DomainLoadResult {
            domains,
            tables,
            odps_products,
            cads_assets,
        })
    }

    /// Load an ODCS table from a file
    async fn load_odcs_table_from_file(&self, file_path: &str) -> Result<Table, StorageError> {
        let content = self.storage.read_file(file_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        let mut importer = ODCSImporter::new();
        let (table, _parse_errors) = importer.parse_table(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse ODCS table: {}", e))
        })?;

        Ok(table)
    }

    /// Load an ODPS product from a file
    async fn load_odps_product_from_file(
        &self,
        file_path: &str,
    ) -> Result<ODPSDataProduct, StorageError> {
        let content = self.storage.read_file(file_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        let importer = ODPSImporter::new();
        importer
            .import(&yaml_content)
            .map_err(|e| StorageError::SerializationError(format!("Failed to parse ODPS: {}", e)))
    }

    /// Load a CADS asset from a file
    async fn load_cads_asset_from_file(&self, file_path: &str) -> Result<CADSAsset, StorageError> {
        let content = self.storage.read_file(file_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        let importer = CADSImporter::new();
        importer.import(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse CADS asset: {}", e))
        })
    }

    /// Load all domains from explicit domain directory names (DEPRECATED)
    ///
    /// This method is deprecated. Use load_domains() with flat file structure instead.
    #[deprecated(
        since = "2.0.0",
        note = "Use load_domains() with flat file structure instead"
    )]
    #[allow(dead_code)]
    async fn load_domains_legacy(
        &self,
        workspace_path: &str,
    ) -> Result<DomainLoadResult, StorageError> {
        let domains = Vec::new();
        let tables = HashMap::new();
        let odps_products = HashMap::new();
        let cads_assets = HashMap::new();

        info!(
            "Legacy domain loading is deprecated. Use flat file structure instead. Workspace: {}",
            workspace_path
        );

        Ok(DomainLoadResult {
            domains,
            tables,
            odps_products,
            cads_assets,
        })
    }

    /// Load domains from explicit domain directory names (DEPRECATED)
    ///
    /// This method is deprecated. Use load_domains() with flat file structure instead.
    #[deprecated(
        since = "2.0.0",
        note = "Use load_domains() with flat file structure instead. Domain directories are no longer supported."
    )]
    #[allow(dead_code)]
    pub async fn load_domains_from_list(
        &self,
        workspace_path: &str,
        _domain_directory_names: &[String],
    ) -> Result<DomainLoadResult, StorageError> {
        warn!(
            "load_domains_from_list is deprecated. Using flat file structure for workspace: {}",
            workspace_path
        );

        // Delegate to the new flat file loading
        self.load_domains(workspace_path).await
    }

    /// Load a single domain from a domain directory (DEPRECATED)
    #[deprecated(
        since = "2.0.0",
        note = "Domain directories are no longer supported. Use flat file structure."
    )]
    #[allow(dead_code)]
    async fn load_domain_legacy(&self, domain_dir: &str) -> Result<Domain, StorageError> {
        let domain_yaml_path = format!("{}/domain.yaml", domain_dir);
        let content = self.storage.read_file(&domain_yaml_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        Domain::from_yaml(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse domain YAML: {}", e))
        })
    }

    /// Load ODCS tables from a domain directory (DEPRECATED)
    #[deprecated(
        since = "2.0.0",
        note = "Domain directories are no longer supported. Use flat file structure."
    )]
    #[allow(dead_code)]
    async fn load_domain_odcs_tables_legacy(
        &self,
        domain_dir: &str,
    ) -> Result<Vec<Table>, StorageError> {
        let mut tables = Vec::new();
        let files = self.storage.list_files(domain_dir).await?;

        for file_name in files {
            if file_name.ends_with(".odcs.yaml") || file_name.ends_with(".odcs.yml") {
                let file_path = format!("{}/{}", domain_dir, file_name);
                match self.load_table_from_yaml(&file_path, domain_dir).await {
                    Ok(table_data) => {
                        // Parse the table from ODCS YAML
                        let mut importer = ODCSImporter::new();
                        match importer.parse_table(&table_data.yaml_content) {
                            Ok((table, _parse_errors)) => {
                                tables.push(table);
                            }
                            Err(e) => {
                                warn!("Failed to parse ODCS table from {}: {}", file_path, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load ODCS table from {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(tables)
    }

    /// Load ODPS products from a domain directory (DEPRECATED)
    #[deprecated(
        since = "2.0.0",
        note = "Domain directories are no longer supported. Use flat file structure."
    )]
    #[allow(dead_code)]
    async fn load_domain_odps_products_legacy(
        &self,
        domain_dir: &str,
    ) -> Result<Vec<ODPSDataProduct>, StorageError> {
        let mut products = Vec::new();
        let files = self.storage.list_files(domain_dir).await?;

        for file_name in files {
            if file_name.ends_with(".odps.yaml") || file_name.ends_with(".odps.yml") {
                let file_path = format!("{}/{}", domain_dir, file_name);
                let content = self.storage.read_file(&file_path).await?;
                let yaml_content = String::from_utf8(content).map_err(|e| {
                    StorageError::SerializationError(format!("Invalid UTF-8: {}", e))
                })?;

                let importer = ODPSImporter::new();
                match importer.import(&yaml_content) {
                    Ok(product) => {
                        products.push(product);
                    }
                    Err(e) => {
                        warn!("Failed to parse ODPS product from {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(products)
    }

    /// Load CADS assets from a domain directory (DEPRECATED)
    #[deprecated(
        since = "2.0.0",
        note = "Domain directories are no longer supported. Use flat file structure."
    )]
    #[allow(dead_code)]
    async fn load_domain_cads_assets_legacy(
        &self,
        domain_dir: &str,
    ) -> Result<Vec<CADSAsset>, StorageError> {
        let mut assets = Vec::new();
        let files = self.storage.list_files(domain_dir).await?;

        for file_name in files {
            if file_name.ends_with(".cads.yaml") || file_name.ends_with(".cads.yml") {
                let file_path = format!("{}/{}", domain_dir, file_name);
                let content = self.storage.read_file(&file_path).await?;
                let yaml_content = String::from_utf8(content).map_err(|e| {
                    StorageError::SerializationError(format!("Invalid UTF-8: {}", e))
                })?;

                let importer = CADSImporter::new();
                match importer.import(&yaml_content) {
                    Ok(asset) => {
                        assets.push(asset);
                    }
                    Err(e) => {
                        warn!("Failed to parse CADS asset from {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(assets)
    }

    /// Load all BPMN models from workspace using flat file structure
    #[cfg(feature = "bpmn")]
    pub async fn load_bpmn_models(
        &self,
        workspace_path: &str,
        _domain_name: &str,
    ) -> Result<Vec<BPMNModel>, StorageError> {
        let mut models = Vec::new();
        let files = self.storage.list_files(workspace_path).await?;

        for file_name in files {
            if file_name.ends_with(".bpmn.xml") {
                let file_path = format!("{}/{}", workspace_path, file_name);
                match self.load_bpmn_model_from_file(&file_path, &file_name).await {
                    Ok(model) => models.push(model),
                    Err(e) => {
                        warn!("Failed to load BPMN model from {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(models)
    }

    /// Load a specific BPMN model from a file
    #[cfg(feature = "bpmn")]
    async fn load_bpmn_model_from_file(
        &self,
        file_path: &str,
        file_name: &str,
    ) -> Result<BPMNModel, StorageError> {
        let content = self.storage.read_file(file_path).await?;
        let xml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        // Extract model name from filename (remove .bpmn.xml extension)
        let model_name = file_name
            .strip_suffix(".bpmn.xml")
            .unwrap_or(file_name)
            .to_string();

        // Generate a domain ID (can be extracted from filename if using naming convention)
        let domain_id = Uuid::new_v4();

        // Import using BPMNImporter
        let mut importer = BPMNImporter::new();
        let model = importer
            .import(&xml_content, domain_id, Some(&model_name))
            .map_err(|e| {
                StorageError::SerializationError(format!("Failed to import BPMN model: {}", e))
            })?;

        Ok(model)
    }

    /// Load a specific BPMN model by name from a domain directory (DEPRECATED)
    #[cfg(feature = "bpmn")]
    #[deprecated(
        since = "2.0.0",
        note = "Use load_bpmn_model_from_file with flat file structure instead"
    )]
    #[allow(dead_code)]
    pub async fn load_bpmn_model(
        &self,
        domain_dir: &str,
        file_name: &str,
    ) -> Result<BPMNModel, StorageError> {
        let file_path = format!("{}/{}", domain_dir, file_name);
        self.load_bpmn_model_from_file(&file_path, file_name).await
    }

    /// Load BPMN XML content from workspace
    #[cfg(feature = "bpmn")]
    pub async fn load_bpmn_xml(
        &self,
        workspace_path: &str,
        _domain_name: &str,
        model_name: &str,
    ) -> Result<String, StorageError> {
        let sanitized_model_name = sanitize_filename(model_name);
        // Try to find the file with any naming pattern
        let files = self.storage.list_files(workspace_path).await?;

        for file_name in files {
            if file_name.ends_with(".bpmn.xml") && file_name.contains(&sanitized_model_name) {
                let file_path = format!("{}/{}", workspace_path, file_name);
                let content = self.storage.read_file(&file_path).await?;
                return String::from_utf8(content).map_err(|e| {
                    StorageError::SerializationError(format!("Invalid UTF-8: {}", e))
                });
            }
        }

        Err(StorageError::IoError(format!(
            "BPMN model '{}' not found in workspace",
            model_name
        )))
    }

    /// Load all DMN models from workspace using flat file structure
    #[cfg(feature = "dmn")]
    pub async fn load_dmn_models(
        &self,
        workspace_path: &str,
        _domain_name: &str,
    ) -> Result<Vec<DMNModel>, StorageError> {
        let mut models = Vec::new();
        let files = self.storage.list_files(workspace_path).await?;

        for file_name in files {
            if file_name.ends_with(".dmn.xml") {
                let file_path = format!("{}/{}", workspace_path, file_name);
                match self.load_dmn_model_from_file(&file_path, &file_name).await {
                    Ok(model) => models.push(model),
                    Err(e) => {
                        warn!("Failed to load DMN model from {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(models)
    }

    /// Load a specific DMN model from a file
    #[cfg(feature = "dmn")]
    async fn load_dmn_model_from_file(
        &self,
        file_path: &str,
        file_name: &str,
    ) -> Result<DMNModel, StorageError> {
        let content = self.storage.read_file(file_path).await?;
        let xml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        // Extract model name from filename (remove .dmn.xml extension)
        let model_name = file_name
            .strip_suffix(".dmn.xml")
            .unwrap_or(file_name)
            .to_string();

        // Generate a domain ID (can be extracted from filename if using naming convention)
        let domain_id = Uuid::new_v4();

        // Import using DMNImporter
        let mut importer = DMNImporter::new();
        let model = importer
            .import(&xml_content, domain_id, Some(&model_name))
            .map_err(|e| {
                StorageError::SerializationError(format!("Failed to import DMN model: {}", e))
            })?;

        Ok(model)
    }

    /// Load a specific DMN model by name from a domain directory (DEPRECATED)
    #[cfg(feature = "dmn")]
    #[deprecated(
        since = "2.0.0",
        note = "Use load_dmn_model_from_file with flat file structure instead"
    )]
    #[allow(dead_code)]
    pub async fn load_dmn_model(
        &self,
        domain_dir: &str,
        file_name: &str,
    ) -> Result<DMNModel, StorageError> {
        let file_path = format!("{}/{}", domain_dir, file_name);
        self.load_dmn_model_from_file(&file_path, file_name).await
    }

    /// Load DMN XML content from workspace
    #[cfg(feature = "dmn")]
    pub async fn load_dmn_xml(
        &self,
        workspace_path: &str,
        _domain_name: &str,
        model_name: &str,
    ) -> Result<String, StorageError> {
        let sanitized_model_name = sanitize_filename(model_name);
        let files = self.storage.list_files(workspace_path).await?;

        for file_name in files {
            if file_name.ends_with(".dmn.xml") && file_name.contains(&sanitized_model_name) {
                let file_path = format!("{}/{}", workspace_path, file_name);
                let content = self.storage.read_file(&file_path).await?;
                return String::from_utf8(content).map_err(|e| {
                    StorageError::SerializationError(format!("Invalid UTF-8: {}", e))
                });
            }
        }

        Err(StorageError::IoError(format!(
            "DMN model '{}' not found in workspace",
            model_name
        )))
    }

    /// Load all OpenAPI specifications from workspace using flat file structure
    #[cfg(feature = "openapi")]
    pub async fn load_openapi_models(
        &self,
        workspace_path: &str,
        _domain_name: &str,
    ) -> Result<Vec<OpenAPIModel>, StorageError> {
        let mut models = Vec::new();
        let files = self.storage.list_files(workspace_path).await?;

        for file_name in files {
            if file_name.ends_with(".openapi.yaml")
                || file_name.ends_with(".openapi.yml")
                || file_name.ends_with(".openapi.json")
            {
                let file_path = format!("{}/{}", workspace_path, file_name);
                match self
                    .load_openapi_model_from_file(&file_path, &file_name)
                    .await
                {
                    Ok(model) => models.push(model),
                    Err(e) => {
                        warn!("Failed to load OpenAPI spec from {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(models)
    }

    /// Load a specific OpenAPI model from a file
    #[cfg(feature = "openapi")]
    async fn load_openapi_model_from_file(
        &self,
        file_path: &str,
        file_name: &str,
    ) -> Result<OpenAPIModel, StorageError> {
        let content = self.storage.read_file(file_path).await?;
        let spec_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        // Extract API name from filename (remove .openapi.yaml/.openapi.json extension)
        let api_name = file_name
            .strip_suffix(".openapi.yaml")
            .or_else(|| file_name.strip_suffix(".openapi.yml"))
            .or_else(|| file_name.strip_suffix(".openapi.json"))
            .unwrap_or(file_name)
            .to_string();

        // Generate a domain ID (can be extracted from filename if using naming convention)
        let domain_id = Uuid::new_v4();

        // Import using OpenAPIImporter
        let mut importer = OpenAPIImporter::new();
        let model = importer
            .import(&spec_content, domain_id, Some(&api_name))
            .map_err(|e| {
                StorageError::SerializationError(format!("Failed to import OpenAPI spec: {}", e))
            })?;

        Ok(model)
    }

    /// Load a specific OpenAPI model by name from a domain directory (DEPRECATED)
    #[cfg(feature = "openapi")]
    #[deprecated(
        since = "2.0.0",
        note = "Use load_openapi_model_from_file with flat file structure instead"
    )]
    #[allow(dead_code)]
    pub async fn load_openapi_model(
        &self,
        domain_dir: &str,
        file_name: &str,
    ) -> Result<OpenAPIModel, StorageError> {
        let file_path = format!("{}/{}", domain_dir, file_name);
        self.load_openapi_model_from_file(&file_path, file_name)
            .await
    }

    /// Load OpenAPI content from workspace
    #[cfg(feature = "openapi")]
    pub async fn load_openapi_content(
        &self,
        workspace_path: &str,
        _domain_name: &str,
        api_name: &str,
        format: Option<OpenAPIFormat>,
    ) -> Result<String, StorageError> {
        let sanitized_api_name = sanitize_filename(api_name);

        // Try to find the file with the requested format, or any format
        let extensions: Vec<&str> = if let Some(fmt) = format {
            match fmt {
                OpenAPIFormat::Yaml => vec!["yaml", "yml"],
                OpenAPIFormat::Json => vec!["json"],
            }
        } else {
            vec!["yaml", "yml", "json"]
        };

        let files = self.storage.list_files(workspace_path).await?;

        for file_name in files {
            for ext in &extensions {
                let suffix = format!(".openapi.{}", ext);
                if file_name.ends_with(&suffix) && file_name.contains(&sanitized_api_name) {
                    let file_path = format!("{}/{}", workspace_path, file_name);
                    let content = self.storage.read_file(&file_path).await?;
                    return String::from_utf8(content).map_err(|e| {
                        StorageError::SerializationError(format!("Invalid UTF-8: {}", e))
                    });
                }
            }
        }

        Err(StorageError::IoError(format!(
            "OpenAPI spec '{}' not found in workspace",
            api_name
        )))
    }

    // ==================== Workspace and Domain Config Loading ====================

    /// Load workspace configuration from workspace.yaml
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to the workspace directory
    ///
    /// # Returns
    ///
    /// The Workspace configuration if found, or None if workspace.yaml doesn't exist
    pub async fn load_workspace(
        &self,
        workspace_path: &str,
    ) -> Result<Option<Workspace>, StorageError> {
        let workspace_file = format!("{}/workspace.yaml", workspace_path);

        if !self.storage.file_exists(&workspace_file).await? {
            return Ok(None);
        }

        let content = self.storage.read_file(&workspace_file).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        let workspace: Workspace = serde_yaml::from_str(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse workspace.yaml: {}", e))
        })?;

        Ok(Some(workspace))
    }

    /// Save workspace configuration to workspace.yaml
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to the workspace directory
    /// * `workspace` - The Workspace configuration to save
    pub async fn save_workspace(
        &self,
        workspace_path: &str,
        workspace: &Workspace,
    ) -> Result<(), StorageError> {
        let workspace_file = format!("{}/workspace.yaml", workspace_path);

        let yaml_content = serde_yaml::to_string(workspace).map_err(|e| {
            StorageError::SerializationError(format!("Failed to serialize workspace: {}", e))
        })?;

        self.storage
            .write_file(&workspace_file, yaml_content.as_bytes())
            .await?;

        Ok(())
    }

    /// Load domain configuration from domain.yaml
    ///
    /// # Arguments
    ///
    /// * `domain_dir` - Path to the domain directory
    ///
    /// # Returns
    ///
    /// The DomainConfig if found, or None if domain.yaml doesn't exist
    pub async fn load_domain_config(
        &self,
        domain_dir: &str,
    ) -> Result<Option<DomainConfig>, StorageError> {
        let domain_file = format!("{}/domain.yaml", domain_dir);

        if !self.storage.file_exists(&domain_file).await? {
            return Ok(None);
        }

        let content = self.storage.read_file(&domain_file).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        let config: DomainConfig = serde_yaml::from_str(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse domain.yaml: {}", e))
        })?;

        Ok(Some(config))
    }

    /// Save domain configuration to domain.yaml
    ///
    /// # Arguments
    ///
    /// * `domain_dir` - Path to the domain directory
    /// * `config` - The DomainConfig to save
    pub async fn save_domain_config(
        &self,
        domain_dir: &str,
        config: &DomainConfig,
    ) -> Result<(), StorageError> {
        let domain_file = format!("{}/domain.yaml", domain_dir);

        let yaml_content = serde_yaml::to_string(config).map_err(|e| {
            StorageError::SerializationError(format!("Failed to serialize domain config: {}", e))
        })?;

        self.storage
            .write_file(&domain_file, yaml_content.as_bytes())
            .await?;

        Ok(())
    }

    /// Load domain configuration by name from a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to the workspace directory
    /// * `domain_name` - Name of the domain (folder name)
    ///
    /// # Returns
    ///
    /// The DomainConfig if found
    pub async fn load_domain_config_by_name(
        &self,
        workspace_path: &str,
        domain_name: &str,
    ) -> Result<Option<DomainConfig>, StorageError> {
        let sanitized_domain_name = sanitize_filename(domain_name);
        let domain_dir = format!("{}/{}", workspace_path, sanitized_domain_name);
        self.load_domain_config(&domain_dir).await
    }

    /// Get domain ID from domain.yaml, or None if not found
    ///
    /// Get domain ID from domain.yaml (DEPRECATED)
    ///
    /// This method is deprecated. Domain information is now stored in workspace.yaml.
    #[deprecated(
        since = "2.0.0",
        note = "Domain directories are no longer supported. Domain info is in workspace.yaml"
    )]
    #[allow(dead_code)]
    pub async fn get_domain_id(&self, domain_dir: &str) -> Result<Option<Uuid>, StorageError> {
        match self.load_domain_config(domain_dir).await? {
            Some(config) => Ok(Some(config.id)),
            None => Ok(None),
        }
    }

    /// Load all domain configurations from a workspace (DEPRECATED)
    ///
    /// This method is deprecated. Use load_workspace() and access domains from the workspace.
    #[deprecated(
        since = "2.0.0",
        note = "Domain directories are no longer supported. Use load_workspace() instead"
    )]
    #[allow(dead_code)]
    pub async fn load_all_domain_configs(
        &self,
        workspace_path: &str,
    ) -> Result<Vec<DomainConfig>, StorageError> {
        warn!(
            "load_all_domain_configs is deprecated. Use load_workspace() for workspace: {}",
            workspace_path
        );

        // Return empty as domain directories are no longer supported
        Ok(Vec::new())
    }
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

/// Result of loading a model
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelLoadResult {
    pub tables: Vec<TableData>,
    pub relationships: Vec<RelationshipData>,
    pub orphaned_relationships: Vec<RelationshipData>,
}

/// Table data loaded from storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub id: Uuid,
    pub name: String,
    pub yaml_file_path: Option<String>,
    pub yaml_content: String,
}

/// Relationship data loaded from storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipData {
    pub id: Uuid,
    pub source_table_id: Uuid,
    pub target_table_id: Uuid,
}

/// Result of loading domains
#[derive(Debug)]
pub struct DomainLoadResult {
    pub domains: Vec<Domain>,
    pub tables: HashMap<Uuid, Table>,
    pub odps_products: HashMap<Uuid, ODPSDataProduct>,
    pub cads_assets: HashMap<Uuid, CADSAsset>,
}
