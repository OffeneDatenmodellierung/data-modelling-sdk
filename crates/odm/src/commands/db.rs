//! Database management CLI commands
//!
//! Provides commands for initializing, syncing, and managing the database backend.

use std::path::{Path, PathBuf};

use crate::error::CliError;

#[cfg(feature = "duckdb-backend")]
use data_modelling_core::database::duckdb::DuckDBBackend;
#[cfg(feature = "duckdb-backend")]
use data_modelling_core::database::{
    DatabaseBackend,
    config::{CONFIG_FILENAME, DatabaseBackendType, DatabaseConfig},
    sync::{SyncEngine, generate_flat_filename},
};
#[cfg(feature = "duckdb-backend")]
use data_modelling_core::models::workspace::AssetType;

/// Database command arguments
#[derive(Debug, Clone)]
pub struct DbInitArgs {
    /// Workspace path
    pub workspace: PathBuf,
    /// Database backend type
    pub backend: String,
    /// PostgreSQL connection string (for postgres backend)
    pub connection_string: Option<String>,
}

/// Database sync arguments
#[derive(Debug, Clone)]
pub struct DbSyncArgs {
    /// Workspace path
    pub workspace: PathBuf,
    /// Force full resync
    pub force: bool,
}

/// Database status arguments
#[derive(Debug, Clone)]
pub struct DbStatusArgs {
    /// Workspace path
    pub workspace: PathBuf,
}

/// Database export arguments
#[derive(Debug, Clone)]
pub struct DbExportArgs {
    /// Workspace path
    pub workspace: PathBuf,
    /// Output directory
    pub output: Option<PathBuf>,
}

/// Initialize database for a workspace
#[cfg(feature = "duckdb-backend")]
pub fn handle_db_init(args: &DbInitArgs) -> Result<(), CliError> {
    let workspace_path = &args.workspace;

    // Check if workspace exists
    if !workspace_path.exists() {
        return Err(CliError::FileNotFound(workspace_path.clone()));
    }

    // Parse backend type
    let backend_type: DatabaseBackendType = args
        .backend
        .parse()
        .map_err(|e: String| CliError::InvalidArgument(e))?;

    // Create config
    let config = match backend_type {
        DatabaseBackendType::DuckDB => DatabaseConfig::duckdb(".data-model.duckdb"),
        DatabaseBackendType::Postgres => {
            let conn_str = args.connection_string.as_ref().ok_or_else(|| {
                CliError::InvalidArgument(
                    "PostgreSQL requires --connection-string argument".to_string(),
                )
            })?;
            DatabaseConfig::postgres(conn_str)
        }
    };

    // Save config
    config
        .save(workspace_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    println!("Created {}", workspace_path.join(CONFIG_FILENAME).display());

    // Initialize database
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::IoError(format!("Failed to create runtime: {}", e)))?;

    match backend_type {
        DatabaseBackendType::DuckDB => {
            let db_path = config.get_duckdb_path(workspace_path);

            let backend = DuckDBBackend::new(&db_path)
                .map_err(|e| CliError::IoError(format!("Failed to create database: {}", e)))?;

            rt.block_on(async {
                backend
                    .initialize()
                    .await
                    .map_err(|e| CliError::IoError(format!("Failed to initialize database: {}", e)))
            })?;

            println!("Initialized DuckDB database at {}", db_path.display());
        }
        DatabaseBackendType::Postgres => {
            #[cfg(feature = "postgres-backend")]
            {
                use data_modelling_core::database::postgres::PostgresBackend;

                let conn_str = args.connection_string.as_ref().unwrap();

                rt.block_on(async {
                    let backend = PostgresBackend::new(conn_str)
                        .await
                        .map_err(|e| CliError::IoError(format!("Failed to connect: {}", e)))?;

                    backend
                        .initialize()
                        .await
                        .map_err(|e| CliError::IoError(format!("Failed to initialize: {}", e)))?;

                    println!("Initialized PostgreSQL database");
                    Ok::<(), CliError>(())
                })?;
            }
            #[cfg(not(feature = "postgres-backend"))]
            {
                return Err(CliError::InvalidArgument(
                    "PostgreSQL backend not enabled. Build with --features postgres-backend"
                        .to_string(),
                ));
            }
        }
    }

    // Install git hooks if in a git repo
    let git_dir = workspace_path.join(".git");
    if git_dir.exists() && config.git.hooks_enabled {
        install_git_hooks(workspace_path)?;
        println!("Installed Git hooks");
    }

    println!("Database initialization complete.");
    Ok(())
}

#[cfg(not(feature = "duckdb-backend"))]
pub fn handle_db_init(_args: &DbInitArgs) -> Result<(), CliError> {
    Err(CliError::InvalidArgument(
        "Database support not enabled. Build with --features duckdb-backend".to_string(),
    ))
}

/// Sync YAML files to database
#[cfg(feature = "duckdb-backend")]
pub fn handle_db_sync(args: &DbSyncArgs) -> Result<(), CliError> {
    use data_modelling_core::model::ModelLoader;
    use data_modelling_core::models::Workspace;
    use data_modelling_core::storage::filesystem::FileSystemStorageBackend;

    let workspace_path = &args.workspace;

    // Load config
    let config = DatabaseConfig::load(workspace_path)
        .map_err(|e| CliError::IoError(format!("Failed to load config: {}", e)))?;

    // Check if initialized
    if !DatabaseConfig::is_initialized(workspace_path) {
        return Err(CliError::InvalidArgument(
            "Database not initialized. Run 'db init' first.".to_string(),
        ));
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::IoError(format!("Failed to create runtime: {}", e)))?;

    rt.block_on(async {
        // Load model from YAML files
        let storage = FileSystemStorageBackend::new(workspace_path);
        let loader = ModelLoader::new(storage);

        let workspace_path_str = workspace_path.to_string_lossy().to_string();
        let model_result = loader
            .load_model(&workspace_path_str)
            .await
            .map_err(|e| CliError::IoError(format!("Failed to load model: {}", e)))?;

        // Convert TableData to Table by parsing YAML content
        let mut tables = Vec::new();
        for table_data in &model_result.tables {
            let mut importer = data_modelling_core::import::odcs::ODCSImporter::new();
            match importer.parse_table(&table_data.yaml_content) {
                Ok((table, _)) => tables.push(table),
                Err(e) => {
                    eprintln!("Warning: Failed to parse table {}: {}", table_data.name, e);
                }
            }
        }

        // Convert RelationshipData to Relationship
        let relationships: Vec<data_modelling_core::models::Relationship> = model_result
            .relationships
            .iter()
            .map(|r| {
                let mut rel = data_modelling_core::models::Relationship::new(
                    r.source_table_id,
                    r.target_table_id,
                );
                rel.id = r.id;
                rel
            })
            .collect();

        // Create or get workspace
        let workspace_name = workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
            .to_string();
        let workspace = Workspace::new(workspace_name, uuid::Uuid::new_v4());

        // Connect to database and sync
        match config.database.backend {
            DatabaseBackendType::DuckDB => {
                let db_path = config.get_duckdb_path(workspace_path);
                let backend = DuckDBBackend::new(&db_path)
                    .map_err(|e| CliError::IoError(format!("Failed to open database: {}", e)))?;

                let sync_engine = SyncEngine::new(backend);

                // Load domains if available
                let domain_result = loader.load_domains(&workspace_path_str).await.ok();
                let domains = domain_result.map(|r| r.domains).unwrap_or_default();

                let result = sync_engine
                    .sync_workspace(&workspace, &tables, &relationships, &domains, args.force)
                    .await
                    .map_err(|e| CliError::IoError(format!("Sync failed: {}", e)))?;

                println!("Sync complete:");
                println!("  Tables:        {}", result.tables_synced);
                println!("  Columns:       {}", result.columns_synced);
                println!("  Relationships: {}", result.relationships_synced);
                println!("  Domains:       {}", result.domains_synced);
                println!("  Duration:      {}ms", result.duration_ms);

                if !result.errors.is_empty() {
                    println!("\nErrors:");
                    for err in &result.errors {
                        println!("  - {}", err);
                    }
                }
            }
            DatabaseBackendType::Postgres => {
                #[cfg(feature = "postgres-backend")]
                {
                    use data_modelling_core::database::postgres::PostgresBackend;

                    let conn_str = config.get_postgres_connection_string().ok_or_else(|| {
                        CliError::InvalidArgument(
                            "PostgreSQL connection string not configured".to_string(),
                        )
                    })?;

                    let backend = PostgresBackend::new(conn_str)
                        .await
                        .map_err(|e| CliError::IoError(format!("Failed to connect: {}", e)))?;

                    let sync_engine = SyncEngine::new(backend);

                    let domain_result = loader.load_domains(&workspace_path_str).await.ok();
                    let domains = domain_result.map(|r| r.domains).unwrap_or_default();

                    let result = sync_engine
                        .sync_workspace(&workspace, &tables, &relationships, &domains, args.force)
                        .await
                        .map_err(|e| CliError::IoError(format!("Sync failed: {}", e)))?;

                    println!("Sync complete:");
                    println!("  Tables:        {}", result.tables_synced);
                    println!("  Columns:       {}", result.columns_synced);
                    println!("  Relationships: {}", result.relationships_synced);
                    println!("  Domains:       {}", result.domains_synced);
                    println!("  Duration:      {}ms", result.duration_ms);
                }
                #[cfg(not(feature = "postgres-backend"))]
                {
                    return Err(CliError::InvalidArgument(
                        "PostgreSQL backend not enabled".to_string(),
                    ));
                }
            }
        }

        Ok(())
    })
}

#[cfg(not(feature = "duckdb-backend"))]
pub fn handle_db_sync(_args: &DbSyncArgs) -> Result<(), CliError> {
    Err(CliError::InvalidArgument(
        "Database support not enabled. Build with --features duckdb-backend".to_string(),
    ))
}

/// Show database sync status
#[cfg(feature = "duckdb-backend")]
pub fn handle_db_status(args: &DbStatusArgs) -> Result<(), CliError> {
    let workspace_path = &args.workspace;

    // Load config
    let config = DatabaseConfig::load(workspace_path)
        .map_err(|e| CliError::IoError(format!("Failed to load config: {}", e)))?;

    if !DatabaseConfig::is_initialized(workspace_path) {
        println!("Database not initialized.");
        println!(
            "Run 'data-modelling-cli db init {}' to initialize.",
            workspace_path.display()
        );
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::IoError(format!("Failed to create runtime: {}", e)))?;

    rt.block_on(async {
        match config.database.backend {
            DatabaseBackendType::DuckDB => {
                let db_path = config.get_duckdb_path(workspace_path);
                let backend = DuckDBBackend::new(&db_path)
                    .map_err(|e| CliError::IoError(format!("Failed to open database: {}", e)))?;

                // Get workspace - for now just show general status
                let result = backend
                    .execute_query("SELECT COUNT(*) as count FROM workspaces")
                    .await;

                println!("Database Status:");
                println!("  Backend:  DuckDB");
                println!("  Path:     {}", db_path.display());

                if let Ok(result) = result
                    && let Some(row) = result.rows.first()
                {
                    let count = row.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  Workspaces: {}", count);
                }

                // Get table count
                let table_result = backend
                    .execute_query("SELECT COUNT(*) as count FROM tables")
                    .await;
                if let Ok(result) = table_result
                    && let Some(row) = result.rows.first()
                {
                    let count = row.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  Tables:     {}", count);
                }

                // Get column count
                let column_result = backend
                    .execute_query("SELECT COUNT(*) as count FROM columns")
                    .await;
                if let Ok(result) = column_result
                    && let Some(row) = result.rows.first()
                {
                    let count = row.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  Columns:    {}", count);
                }

                // Get relationship count
                let rel_result = backend
                    .execute_query("SELECT COUNT(*) as count FROM relationships")
                    .await;
                if let Ok(result) = rel_result
                    && let Some(row) = result.rows.first()
                {
                    let count = row.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  Relationships: {}", count);
                }

                // Check health
                let healthy = backend.health_check().await.unwrap_or(false);
                println!("  Healthy:    {}", if healthy { "yes" } else { "no" });
            }
            DatabaseBackendType::Postgres => {
                println!("Database Status:");
                println!("  Backend:  PostgreSQL");
                if let Some(conn) = config.get_postgres_connection_string() {
                    // Mask password
                    let masked = if let Some(at_pos) = conn.find('@') {
                        if let Some(colon_pos) = conn[..at_pos].rfind(':') {
                            format!("{}****{}", &conn[..colon_pos + 1], &conn[at_pos..])
                        } else {
                            conn.to_string()
                        }
                    } else {
                        conn.to_string()
                    };
                    println!("  Connection: {}", masked);
                }
            }
        }

        Ok(())
    })
}

#[cfg(not(feature = "duckdb-backend"))]
pub fn handle_db_status(_args: &DbStatusArgs) -> Result<(), CliError> {
    Err(CliError::InvalidArgument(
        "Database support not enabled. Build with --features duckdb-backend".to_string(),
    ))
}

/// Export database to YAML files
#[cfg(feature = "duckdb-backend")]
pub fn handle_db_export(args: &DbExportArgs) -> Result<(), CliError> {
    let workspace_path = &args.workspace;
    let output_path = args.output.as_ref().unwrap_or(workspace_path);

    // Load config
    let config = DatabaseConfig::load(workspace_path)
        .map_err(|e| CliError::IoError(format!("Failed to load config: {}", e)))?;

    if !DatabaseConfig::is_initialized(workspace_path) {
        return Err(CliError::InvalidArgument(
            "Database not initialized. Run 'db init' first.".to_string(),
        ));
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::IoError(format!("Failed to create runtime: {}", e)))?;

    rt.block_on(async {
        match config.database.backend {
            DatabaseBackendType::DuckDB => {
                let db_path = config.get_duckdb_path(workspace_path);
                let backend = DuckDBBackend::new(&db_path)
                    .map_err(|e| CliError::IoError(format!("Failed to open database: {}", e)))?;

                let sync_engine = SyncEngine::new(backend);

                // Get workspace ID - for now use first workspace
                let ws_result = sync_engine
                    .query("SELECT id FROM workspaces LIMIT 1")
                    .await
                    .map_err(|e| CliError::IoError(format!("Query failed: {}", e)))?;

                if ws_result.rows.is_empty() {
                    println!("No workspace found in database.");
                    return Ok(());
                }

                let workspace_id: uuid::Uuid = ws_result.rows[0]
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
                    .ok_or_else(|| CliError::IoError("Invalid workspace ID".to_string()))?;

                let (workspace, tables, _relationships, domains) = sync_engine
                    .export_workspace(workspace_id)
                    .await
                    .map_err(|e| CliError::IoError(format!("Export failed: {}", e)))?;

                // Get workspace name for flat filename
                let workspace_name = workspace
                    .as_ref()
                    .map(|w| w.name.as_str())
                    .unwrap_or("workspace");

                // Build domain lookup from domains list
                let domain_name = domains
                    .first()
                    .map(|d| d.name.as_str())
                    .unwrap_or("default");

                // Export tables to YAML using new flat filename convention
                for table in &tables {
                    let yaml = data_modelling_core::export::ODCSExporter::export_table(
                        table,
                        "odcs_v3_1_0",
                    );
                    let filename = generate_flat_filename(
                        workspace_name,
                        domain_name,
                        None, // system_name - tables don't track system membership yet
                        &table.name,
                        &AssetType::Odcs,
                    );
                    let file_path = output_path.join(&filename);

                    std::fs::write(&file_path, yaml).map_err(|e| {
                        CliError::IoError(format!("Failed to write {}: {}", filename, e))
                    })?;

                    println!("Exported {}", filename);
                }

                println!(
                    "\nExported {} tables to {}",
                    tables.len(),
                    output_path.display()
                );
            }
            DatabaseBackendType::Postgres => {
                return Err(CliError::InvalidArgument(
                    "PostgreSQL export not yet implemented".to_string(),
                ));
            }
        }

        Ok(())
    })
}

#[cfg(not(feature = "duckdb-backend"))]
pub fn handle_db_export(_args: &DbExportArgs) -> Result<(), CliError> {
    Err(CliError::InvalidArgument(
        "Database support not enabled. Build with --features duckdb-backend".to_string(),
    ))
}

/// Install Git hooks for automatic database rebuild
fn install_git_hooks(workspace_path: &Path) -> Result<(), CliError> {
    let hooks_dir = workspace_path.join(".git/hooks");

    // Post-checkout hook
    let post_checkout = hooks_dir.join("post-checkout");
    let post_checkout_content = r#"#!/bin/sh
# Data Model SDK - Auto-rebuild database after checkout
if command -v data-modelling-cli &> /dev/null; then
    data-modelling-cli db sync . --force 2>/dev/null || true
fi
"#;

    std::fs::write(&post_checkout, post_checkout_content)
        .map_err(|e| CliError::IoError(format!("Failed to write post-checkout hook: {}", e)))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&post_checkout)
            .map_err(|e| CliError::IoError(e.to_string()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&post_checkout, perms)
            .map_err(|e| CliError::IoError(e.to_string()))?;
    }

    // Post-merge hook
    let post_merge = hooks_dir.join("post-merge");
    let post_merge_content = r#"#!/bin/sh
# Data Model SDK - Auto-rebuild database after merge
if command -v data-modelling-cli &> /dev/null; then
    data-modelling-cli db sync . --force 2>/dev/null || true
fi
"#;

    std::fs::write(&post_merge, post_merge_content)
        .map_err(|e| CliError::IoError(format!("Failed to write post-merge hook: {}", e)))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&post_merge)
            .map_err(|e| CliError::IoError(e.to_string()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&post_merge, perms)
            .map_err(|e| CliError::IoError(e.to_string()))?;
    }

    Ok(())
}
