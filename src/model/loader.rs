//! Model loading functionality
//!
//! Loads models from storage backends, handling YAML parsing and validation.
//!
//! Supports both file-based loading (FileSystemStorageBackend, BrowserStorageBackend)
//! and API-based loading (ApiStorageBackend).
//!
//! File structure:
//! - Base directory (workspace_path)
//!   - Domain directories (e.g., `domain1/`, `domain2/`)
//!     - `domain.yaml` - Domain definition
//!     - `{name}.odcs.yaml` - ODCS table files
//!     - `{name}.odps.yaml` - ODPS product files
//!     - `{name}.cads.yaml` - CADS asset files
//!   - `tables/` - Legacy: tables not in any domain (backward compatibility)

use crate::import::{cads::CADSImporter, odcs::ODCSImporter, odps::ODPSImporter};
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
    /// - Loads from `tables/` subdirectory with YAML files
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

    /// Load model from file-based storage
    async fn load_model_from_files(
        &self,
        workspace_path: &str,
    ) -> Result<ModelLoadResult, StorageError> {
        let tables_dir = format!("{}/tables", workspace_path);

        // Ensure tables directory exists
        if !self.storage.dir_exists(&tables_dir).await? {
            self.storage.create_dir(&tables_dir).await?;
        }

        // Load tables from individual YAML files
        let mut tables = Vec::new();
        let mut table_ids: HashMap<Uuid, String> = HashMap::new();

        let files = self.storage.list_files(&tables_dir).await?;
        for file_name in files {
            if file_name.ends_with(".yaml") || file_name.ends_with(".yml") {
                let file_path = format!("{}/{}", tables_dir, file_name);
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
    /// Scans the workspace for domain directories and loads each domain along with
    /// its associated ODCS tables, ODPS products, and CADS assets.
    ///
    /// Domain directories are identified by the presence of a `domain.yaml` file.
    pub async fn load_domains(
        &self,
        workspace_path: &str,
    ) -> Result<DomainLoadResult, StorageError> {
        let mut domains = Vec::new();
        let mut tables = HashMap::new();
        let mut odps_products = HashMap::new();
        let mut cads_assets = HashMap::new();

        // Try to discover domain directories by checking for domain.yaml files
        // Since list_files only returns files, we need to check potential directories
        // by looking for domain.yaml files. We'll check common patterns or use
        // a recursive approach if the storage backend supports it.

        // For now, we'll check if entries that look like directories contain domain.yaml
        // This is a limitation - ideally we'd have a list_directories method
        let entries = self.storage.list_files(workspace_path).await?;

        // Collect potential domain directories (entries that don't look like files)
        let mut potential_domains = Vec::new();
        for entry in entries {
            // If entry doesn't end with .yaml/.yml and doesn't contain a dot, it might be a directory
            if !entry.ends_with(".yaml") && !entry.ends_with(".yml") && !entry.contains('.') {
                potential_domains.push(entry);
            }
        }

        // Also check for domain.yaml files directly in subdirectories
        // We'll try common domain directory patterns or scan recursively
        // For a more robust solution, we'd need storage backend support for listing directories

        // Check each potential domain directory
        for entry in potential_domains {
            let domain_dir = format!("{}/{}", workspace_path, entry);
            let domain_yaml_path = format!("{}/domain.yaml", domain_dir);

            if self.storage.file_exists(&domain_yaml_path).await? {
                // Load domain
                match self.load_domain(&domain_dir).await {
                    Ok(domain) => {
                        let domain_name = domain.name.clone();
                        domains.push(domain);

                        // Load ODCS tables from this domain directory
                        let domain_tables = self.load_domain_odcs_tables(&domain_dir).await?;
                        let table_count = domain_tables.len();
                        for table in domain_tables {
                            tables.insert(table.id, table);
                        }

                        // Load ODPS products from this domain directory
                        let domain_odps = self.load_domain_odps_products(&domain_dir).await?;
                        let odps_count = domain_odps.len();
                        for product in domain_odps {
                            odps_products.insert(
                                Uuid::parse_str(&product.id).unwrap_or_else(|_| Uuid::new_v4()),
                                product,
                            );
                        }

                        // Load CADS assets from this domain directory
                        let domain_cads = self.load_domain_cads_assets(&domain_dir).await?;
                        let cads_count = domain_cads.len();
                        for asset in domain_cads {
                            cads_assets.insert(
                                Uuid::parse_str(&asset.id).unwrap_or_else(|_| Uuid::new_v4()),
                                asset,
                            );
                        }

                        info!(
                            "Loaded domain '{}' with {} tables, {} ODPS products, {} CADS assets",
                            domain_name, table_count, odps_count, cads_count
                        );
                    }
                    Err(e) => {
                        warn!("Failed to load domain from {}: {}", domain_dir, e);
                    }
                }
            }
        }

        info!(
            "Loaded {} domains from workspace {}",
            domains.len(),
            workspace_path
        );

        Ok(DomainLoadResult {
            domains,
            tables,
            odps_products,
            cads_assets,
        })
    }

    /// Load domains from explicit domain directory names
    ///
    /// This is more reliable than `load_domains()` when you know the domain directory names,
    /// as it doesn't rely on directory discovery which may be limited by the storage backend.
    pub async fn load_domains_from_list(
        &self,
        workspace_path: &str,
        domain_directory_names: &[String],
    ) -> Result<DomainLoadResult, StorageError> {
        let mut domains = Vec::new();
        let mut tables = HashMap::new();
        let mut odps_products = HashMap::new();
        let mut cads_assets = HashMap::new();

        for domain_dir_name in domain_directory_names {
            let domain_dir = format!("{}/{}", workspace_path, domain_dir_name);
            let domain_yaml_path = format!("{}/domain.yaml", domain_dir);

            if self.storage.file_exists(&domain_yaml_path).await? {
                match self.load_domain(&domain_dir).await {
                    Ok(domain) => {
                        let domain_name = domain.name.clone();
                        domains.push(domain);

                        // Load ODCS tables from this domain directory
                        let domain_tables = self.load_domain_odcs_tables(&domain_dir).await?;
                        let table_count = domain_tables.len();
                        for table in domain_tables {
                            tables.insert(table.id, table);
                        }

                        // Load ODPS products from this domain directory
                        let domain_odps = self.load_domain_odps_products(&domain_dir).await?;
                        let odps_count = domain_odps.len();
                        for product in domain_odps {
                            odps_products.insert(
                                Uuid::parse_str(&product.id).unwrap_or_else(|_| Uuid::new_v4()),
                                product,
                            );
                        }

                        // Load CADS assets from this domain directory
                        let domain_cads = self.load_domain_cads_assets(&domain_dir).await?;
                        let cads_count = domain_cads.len();
                        for asset in domain_cads {
                            cads_assets.insert(
                                Uuid::parse_str(&asset.id).unwrap_or_else(|_| Uuid::new_v4()),
                                asset,
                            );
                        }

                        info!(
                            "Loaded domain '{}' with {} tables, {} ODPS products, {} CADS assets",
                            domain_name, table_count, odps_count, cads_count
                        );
                    }
                    Err(e) => {
                        warn!("Failed to load domain from {}: {}", domain_dir, e);
                    }
                }
            } else {
                warn!(
                    "Domain directory '{}' does not contain domain.yaml",
                    domain_dir
                );
            }
        }

        info!(
            "Loaded {} domains from workspace {}",
            domains.len(),
            workspace_path
        );

        Ok(DomainLoadResult {
            domains,
            tables,
            odps_products,
            cads_assets,
        })
    }

    /// Load a single domain from a domain directory
    async fn load_domain(&self, domain_dir: &str) -> Result<Domain, StorageError> {
        let domain_yaml_path = format!("{}/domain.yaml", domain_dir);
        let content = self.storage.read_file(&domain_yaml_path).await?;
        let yaml_content = String::from_utf8(content)
            .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

        Domain::from_yaml(&yaml_content).map_err(|e| {
            StorageError::SerializationError(format!("Failed to parse domain YAML: {}", e))
        })
    }

    /// Load ODCS tables from a domain directory
    async fn load_domain_odcs_tables(&self, domain_dir: &str) -> Result<Vec<Table>, StorageError> {
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

    /// Load ODPS products from a domain directory
    async fn load_domain_odps_products(
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

    /// Load CADS assets from a domain directory
    async fn load_domain_cads_assets(
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
