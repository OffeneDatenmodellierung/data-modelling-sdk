//! YAML to Database sync logic
//!
//! Provides bidirectional synchronization between YAML files and the database.
//! Uses SHA256 hashing for change detection to enable incremental syncs.
//!
//! ## File Naming Convention
//!
//! All files use a flat naming pattern:
//! - `workspace.yaml` - workspace metadata with references to all assets
//! - `{workspace}_{domain}_{system}_{resource}.odcs.yaml` - ODCS table files
//! - `{workspace}_{domain}_{system}_{resource}.odps.yaml` - ODPS product files
//! - `{workspace}_{domain}_{system}_{resource}.cads.yaml` - CADS asset files
//! - `relationships.yaml` - relationship definitions
//!
//! Where `{system}` is optional if the resource is at the domain level.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::{DatabaseBackend, DatabaseError, DatabaseResult, SyncStatus};
use crate::models::workspace::AssetType;
use crate::models::{Domain, Relationship, Table, Workspace};

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Workspace ID that was synced
    pub workspace_id: Uuid,
    /// Number of tables synced
    pub tables_synced: usize,
    /// Number of columns synced
    pub columns_synced: usize,
    /// Number of relationships synced
    pub relationships_synced: usize,
    /// Number of domains synced
    pub domains_synced: usize,
    /// Files that were skipped (unchanged)
    pub files_skipped: usize,
    /// Errors encountered during sync
    pub errors: Vec<String>,
    /// Duration of sync in milliseconds
    pub duration_ms: u64,
}

impl SyncResult {
    /// Create a new empty sync result
    pub fn new(workspace_id: Uuid) -> Self {
        Self {
            workspace_id,
            tables_synced: 0,
            columns_synced: 0,
            relationships_synced: 0,
            domains_synced: 0,
            files_skipped: 0,
            errors: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Check if sync was successful (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get total items synced
    pub fn total_synced(&self) -> usize {
        self.tables_synced + self.relationships_synced + self.domains_synced
    }
}

/// File information for sync
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Relative path from workspace root
    pub path: String,
    /// SHA256 hash of file content
    pub hash: String,
    /// File content
    pub content: Vec<u8>,
}

impl FileInfo {
    /// Create new file info with computed hash
    pub fn new(path: impl Into<String>, content: Vec<u8>) -> Self {
        let hash = compute_hash(&content);
        Self {
            path: path.into(),
            hash,
            content,
        }
    }
}

/// Sync engine for YAML to database synchronization
pub struct SyncEngine<B: DatabaseBackend> {
    backend: B,
}

impl<B: DatabaseBackend> SyncEngine<B> {
    /// Create a new sync engine with the given database backend
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Get reference to the database backend
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Initialize the database (run migrations)
    pub async fn initialize(&self) -> DatabaseResult<()> {
        self.backend.initialize().await
    }

    /// Sync a workspace from YAML files to database
    ///
    /// # Arguments
    /// * `workspace` - Workspace metadata
    /// * `tables` - Tables to sync
    /// * `relationships` - Relationships to sync
    /// * `domains` - Domains to sync
    /// * `force` - If true, ignore change detection and sync everything
    ///
    /// # Returns
    /// Sync result with counts and any errors
    pub async fn sync_workspace(
        &self,
        workspace: &Workspace,
        tables: &[Table],
        relationships: &[Relationship],
        domains: &[Domain],
        force: bool,
    ) -> DatabaseResult<SyncResult> {
        let start = std::time::Instant::now();
        let mut result = SyncResult::new(workspace.id);

        // Upsert workspace first
        self.backend.upsert_workspace(workspace).await?;

        // Sync domains
        if !domains.is_empty() || force {
            match self.backend.sync_domains(workspace.id, domains).await {
                Ok(count) => result.domains_synced = count,
                Err(e) => result.errors.push(format!("Domain sync error: {}", e)),
            }
        }

        // Sync tables
        if !tables.is_empty() || force {
            match self.backend.sync_tables(workspace.id, tables).await {
                Ok(count) => {
                    result.tables_synced = count;
                    // Count columns
                    result.columns_synced = tables.iter().map(|t| t.columns.len()).sum();
                }
                Err(e) => result.errors.push(format!("Table sync error: {}", e)),
            }
        }

        // Sync relationships
        if !relationships.is_empty() || force {
            match self
                .backend
                .sync_relationships(workspace.id, relationships)
                .await
            {
                Ok(count) => result.relationships_synced = count,
                Err(e) => result
                    .errors
                    .push(format!("Relationship sync error: {}", e)),
            }
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Sync tables with change detection
    ///
    /// Only syncs tables whose YAML file has changed since last sync.
    pub async fn sync_tables_incremental(
        &self,
        workspace_id: Uuid,
        tables: &[Table],
        file_hashes: &HashMap<Uuid, String>,
    ) -> DatabaseResult<SyncResult> {
        let start = std::time::Instant::now();
        let mut result = SyncResult::new(workspace_id);

        let mut tables_to_sync = Vec::new();

        for table in tables {
            let new_hash = file_hashes.get(&table.id);

            // Check if file has changed
            let should_sync = if let Some(new_hash) = new_hash {
                // Get stored hash
                let stored_hash = self
                    .backend
                    .get_file_hash(workspace_id, &table.id.to_string())
                    .await?;

                match stored_hash {
                    Some(stored) => &stored != new_hash,
                    None => true, // No stored hash, need to sync
                }
            } else {
                true // No hash provided, sync anyway
            };

            if should_sync {
                tables_to_sync.push(table.clone());
            } else {
                result.files_skipped += 1;
            }
        }

        // Sync changed tables
        if !tables_to_sync.is_empty() {
            result.tables_synced = self
                .backend
                .sync_tables(workspace_id, &tables_to_sync)
                .await?;
            result.columns_synced = tables_to_sync.iter().map(|t| t.columns.len()).sum();

            // Update file hashes
            for table in &tables_to_sync {
                if let Some(hash) = file_hashes.get(&table.id) {
                    self.backend
                        .record_file_hash(workspace_id, &table.id.to_string(), hash)
                        .await?;
                }
            }
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Export workspace from database to models
    pub async fn export_workspace(
        &self,
        workspace_id: Uuid,
    ) -> DatabaseResult<(
        Option<Workspace>,
        Vec<Table>,
        Vec<Relationship>,
        Vec<Domain>,
    )> {
        let workspace = self.backend.get_workspace(workspace_id).await?;
        let tables = self.backend.export_tables(workspace_id).await?;
        let relationships = self.backend.export_relationships(workspace_id).await?;
        let domains = self.backend.export_domains(workspace_id).await?;

        Ok((workspace, tables, relationships, domains))
    }

    /// Get sync status for a workspace
    pub async fn get_status(&self, workspace_id: Uuid) -> DatabaseResult<SyncStatus> {
        self.backend.get_sync_status(workspace_id).await
    }

    /// Check database health
    pub async fn health_check(&self) -> DatabaseResult<bool> {
        self.backend.health_check().await
    }

    /// Execute a raw SQL query
    pub async fn query(&self, sql: &str) -> DatabaseResult<super::QueryResult> {
        self.backend.execute_query(sql).await
    }

    /// Close the database connection
    pub async fn close(&self) -> DatabaseResult<()> {
        self.backend.close().await
    }
}

/// Compute SHA256 hash of content
pub fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Compute SHA256 hash of a file
pub fn compute_file_hash(path: &Path) -> DatabaseResult<String> {
    let content = std::fs::read(path)
        .map_err(|e| DatabaseError::IoError(format!("Failed to read file: {}", e)))?;
    Ok(compute_hash(&content))
}

/// Parse flat filename to extract workspace, domain, system, and resource name
///
/// Format: `{workspace}_{domain}_{system}_{resource}.{type}.yaml`
/// Where system is optional.
///
/// Returns: (workspace, domain, system, resource_name, asset_type)
pub fn parse_flat_filename(
    filename: &str,
) -> Option<(String, String, Option<String>, String, AssetType)> {
    // Get asset type from filename
    let asset_type = AssetType::from_filename(filename)?;

    // Skip workspace-level files
    if asset_type.is_workspace_level() {
        return None;
    }

    // Remove file extension based on asset type
    let base = match asset_type {
        AssetType::Odcs => filename.strip_suffix(".odcs.yaml")?,
        AssetType::Odps => filename.strip_suffix(".odps.yaml")?,
        AssetType::Cads => filename.strip_suffix(".cads.yaml")?,
        AssetType::Bpmn => filename.strip_suffix(".bpmn.xml")?,
        AssetType::Dmn => filename.strip_suffix(".dmn.xml")?,
        AssetType::Openapi => filename
            .strip_suffix(".openapi.yaml")
            .or_else(|| filename.strip_suffix(".openapi.json"))?,
        _ => return None,
    };

    let parts: Vec<&str> = base.split('_').collect();

    match parts.len() {
        // workspace_domain_resource (no system)
        3 => Some((
            parts[0].to_string(),
            parts[1].to_string(),
            None,
            parts[2].to_string(),
            asset_type,
        )),
        // workspace_domain_system_resource
        4 => Some((
            parts[0].to_string(),
            parts[1].to_string(),
            Some(parts[2].to_string()),
            parts[3].to_string(),
            asset_type,
        )),
        // More than 4 parts - treat remaining as resource name with underscores
        n if n > 4 => Some((
            parts[0].to_string(),
            parts[1].to_string(),
            Some(parts[2].to_string()),
            parts[3..].join("_"),
            asset_type,
        )),
        _ => None,
    }
}

/// Generate flat filename from workspace, domain, system, and resource name
///
/// Format: `{workspace}_{domain}_{system}_{resource}.{type}.yaml`
pub fn generate_flat_filename(
    workspace_name: &str,
    domain_name: &str,
    system_name: Option<&str>,
    resource_name: &str,
    asset_type: &AssetType,
) -> String {
    let mut parts = vec![sanitize_name(workspace_name), sanitize_name(domain_name)];

    if let Some(system) = system_name {
        parts.push(sanitize_name(system));
    }

    parts.push(sanitize_name(resource_name));

    format!("{}.{}", parts.join("_"), asset_type.extension())
}

/// Sanitize a name for use in filenames (replace spaces/special chars with hyphens, lowercase)
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            ' ' | '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// Scan workspace directory for supported YAML/XML files
///
/// Only scans the root directory for flat files using the naming convention.
/// Does not scan subdirectories (legacy domain directory structure is not supported).
///
/// Returns a list of file paths relative to the workspace root.
pub fn scan_workspace_files(workspace_path: &Path) -> DatabaseResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Scan root directory for flat files only
    if let Ok(entries) = std::fs::read_dir(workspace_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && AssetType::is_supported_file(name)
            {
                files.push(path.strip_prefix(workspace_path).unwrap().to_path_buf());
            }
        }
    }

    Ok(files)
}

/// Detect changes between stored and current file hashes
pub fn detect_changes(
    stored_hashes: &HashMap<String, String>,
    current_hashes: &HashMap<String, String>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    // Find added and modified files
    for (path, hash) in current_hashes {
        match stored_hashes.get(path) {
            Some(stored_hash) => {
                if stored_hash != hash {
                    modified.push(path.clone());
                }
            }
            None => {
                added.push(path.clone());
            }
        }
    }

    // Find deleted files
    for path in stored_hashes.keys() {
        if !current_hashes.contains_key(path) {
            deleted.push(path.clone());
        }
    }

    (added, modified, deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let content = b"hello world";
        let hash = compute_hash(content);
        // SHA256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_parse_flat_filename() {
        // workspace_domain_system_resource
        let result = parse_flat_filename("enterprise_sales_kafka_orders.odcs.yaml");
        assert!(result.is_some());
        let (workspace, domain, system, resource, asset_type) = result.unwrap();
        assert_eq!(workspace, "enterprise");
        assert_eq!(domain, "sales");
        assert_eq!(system, Some("kafka".to_string()));
        assert_eq!(resource, "orders");
        assert_eq!(asset_type, AssetType::Odcs);

        // workspace_domain_resource (no system)
        let result = parse_flat_filename("enterprise_sales_orders.odcs.yaml");
        assert!(result.is_some());
        let (workspace, domain, system, resource, _) = result.unwrap();
        assert_eq!(workspace, "enterprise");
        assert_eq!(domain, "sales");
        assert_eq!(system, None);
        assert_eq!(resource, "orders");

        // with underscores in resource name
        let result = parse_flat_filename("enterprise_sales_kafka_order_items.odcs.yaml");
        assert!(result.is_some());
        let (_, _, system, resource, _) = result.unwrap();
        assert_eq!(system, Some("kafka".to_string()));
        assert_eq!(resource, "order_items");

        // ODPS type
        let result = parse_flat_filename("enterprise_sales_analytics.odps.yaml");
        assert!(result.is_some());
        let (_, _, _, _, asset_type) = result.unwrap();
        assert_eq!(asset_type, AssetType::Odps);

        // workspace.yaml should return None (workspace-level file)
        let result = parse_flat_filename("workspace.yaml");
        assert!(result.is_none());

        // relationships.yaml should return None (workspace-level file)
        let result = parse_flat_filename("relationships.yaml");
        assert!(result.is_none());

        // Invalid format (less than 3 parts)
        let result = parse_flat_filename("orders.odcs.yaml");
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_flat_filename() {
        assert_eq!(
            generate_flat_filename(
                "enterprise",
                "sales",
                Some("kafka"),
                "orders",
                &AssetType::Odcs
            ),
            "enterprise_sales_kafka_orders.odcs.yaml"
        );

        assert_eq!(
            generate_flat_filename("enterprise", "sales", None, "orders", &AssetType::Odcs),
            "enterprise_sales_orders.odcs.yaml"
        );

        assert_eq!(
            generate_flat_filename("enterprise", "finance", None, "accounts", &AssetType::Odps),
            "enterprise_finance_accounts.odps.yaml"
        );

        // Test with spaces in names (should be sanitized)
        assert_eq!(
            generate_flat_filename(
                "My Workspace",
                "Sales Domain",
                None,
                "Order Items",
                &AssetType::Odcs
            ),
            "my-workspace_sales-domain_order-items.odcs.yaml"
        );
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("Hello World"), "hello-world");
        assert_eq!(sanitize_name("Test/Path"), "test-path");
        assert_eq!(sanitize_name("Normal"), "normal");
        assert_eq!(sanitize_name("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_detect_changes() {
        let mut stored = HashMap::new();
        stored.insert("a.yaml".to_string(), "hash1".to_string());
        stored.insert("b.yaml".to_string(), "hash2".to_string());
        stored.insert("c.yaml".to_string(), "hash3".to_string());

        let mut current = HashMap::new();
        current.insert("a.yaml".to_string(), "hash1".to_string()); // unchanged
        current.insert("b.yaml".to_string(), "hash2_modified".to_string()); // modified
        current.insert("d.yaml".to_string(), "hash4".to_string()); // added

        let (added, modified, deleted) = detect_changes(&stored, &current);

        assert_eq!(added, vec!["d.yaml"]);
        assert_eq!(modified, vec!["b.yaml"]);
        assert_eq!(deleted, vec!["c.yaml"]);
    }

    #[test]
    fn test_sync_result() {
        let result = SyncResult::new(Uuid::new_v4());
        assert!(result.is_success());
        assert_eq!(result.total_synced(), 0);
    }
}
