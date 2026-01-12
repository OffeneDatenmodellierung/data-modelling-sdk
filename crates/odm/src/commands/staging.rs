//! CLI commands for staging database operations
//!
//! Note: Some command argument structs have fields that are defined for future
//! Iceberg integration but are not yet used.

#![allow(dead_code)]

use std::path::PathBuf;

use crate::error::CliError;
use data_modelling_core::staging::{DedupStrategy, IngestConfig, SourceType, StagingDb};

/// Arguments for the `staging init` command
pub struct StagingInitArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Catalog type for Iceberg backend (rest, s3-tables, unity, glue)
    pub catalog: Option<String>,
    /// Catalog endpoint URL (for REST, Unity)
    pub endpoint: Option<String>,
    /// Warehouse path for local storage
    pub warehouse: Option<PathBuf>,
    /// Authentication token (for REST, Unity)
    pub token: Option<String>,
    /// AWS/GCP region (for S3 Tables, Glue)
    pub region: Option<String>,
    /// S3 Tables ARN
    pub arn: Option<String>,
    /// AWS profile name
    pub profile: Option<String>,
}

/// Arguments for the `staging ingest` command
pub struct StagingIngestArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Source path to ingest from
    pub source: PathBuf,
    /// File pattern to match (e.g., "*.json", "**/*.jsonl")
    pub pattern: String,
    /// Partition key for organizing data
    pub partition: Option<String>,
    /// Deduplication strategy
    pub dedup: DedupStrategy,
    /// Batch size for inserts
    pub batch_size: usize,
    /// Resume a previous batch
    pub resume: bool,
    /// Batch ID for resume
    pub batch_id: Option<String>,
}

/// Arguments for the `staging stats` command
pub struct StagingStatsArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Partition to filter by
    pub partition: Option<String>,
}

/// Arguments for the `staging batches` command
pub struct StagingBatchesArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Maximum number of batches to show
    pub limit: usize,
}

/// Arguments for the `staging query` command
pub struct StagingQueryArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// SQL query to execute
    pub sql: String,
    /// Output format (json, table)
    pub format: String,
    /// Query specific table version (time travel)
    pub version: Option<i64>,
    /// Query as of timestamp (time travel, ISO 8601)
    pub timestamp: Option<String>,
}

/// Arguments for the `staging sample` command
pub struct StagingSampleArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Number of samples to retrieve
    pub limit: usize,
    /// Partition to sample from
    pub partition: Option<String>,
}

/// Arguments for the `staging history` command
pub struct StagingHistoryArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Table name to show history for
    pub table: Option<String>,
    /// Maximum number of snapshots to show
    pub limit: usize,
}

/// Arguments for the `staging export` command
pub struct StagingExportArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Target catalog type (unity, glue, s3-tables)
    pub target: String,
    /// Target catalog endpoint
    pub endpoint: Option<String>,
    /// Target catalog name
    pub catalog: Option<String>,
    /// Target schema/database name
    pub schema: Option<String>,
    /// Target table name
    pub table: String,
    /// AWS/GCP region
    pub region: Option<String>,
    /// S3 Tables ARN
    pub arn: Option<String>,
    /// AWS profile name
    pub profile: Option<String>,
    /// Authentication token
    pub token: Option<String>,
}

/// Arguments for the `staging view create` command
pub struct StagingViewCreateArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// View name
    pub name: String,
    /// Inferred schema file path
    pub schema: PathBuf,
    /// Source table name (default: raw_json)
    pub source_table: Option<String>,
}

/// Handle the `staging init` command
pub fn handle_staging_init(args: &StagingInitArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    // Check if Iceberg catalog configuration is provided
    #[cfg(feature = "iceberg")]
    if let Some(ref catalog_type) = args.catalog {
        return handle_staging_init_iceberg(args, catalog_type);
    }

    // Fall back to DuckDB-only initialization
    let db = StagingDb::open(&db_path).map_err(|e| CliError::StagingError(e.to_string()))?;

    if db
        .is_initialized()
        .map_err(|e| CliError::StagingError(e.to_string()))?
    {
        println!("Database already initialized at: {}", db_path);
        let version = db
            .schema_version()
            .map_err(|e| CliError::StagingError(e.to_string()))?;
        println!("Schema version: {}", version);
    } else {
        db.init()
            .map_err(|e| CliError::StagingError(e.to_string()))?;
        println!("Staging database initialized at: {}", db_path);
    }

    Ok(())
}

/// Handle Iceberg catalog initialization
#[cfg(feature = "iceberg")]
fn handle_staging_init_iceberg(args: &StagingInitArgs, catalog_type: &str) -> Result<(), CliError> {
    use data_modelling_core::staging::CatalogConfig;
    use std::collections::HashMap;

    let config = match catalog_type {
        "rest" => {
            let endpoint = args.endpoint.clone().ok_or_else(|| {
                CliError::InvalidArgument("--endpoint required for REST catalog".to_string())
            })?;
            let warehouse = args
                .warehouse
                .clone()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "./warehouse".to_string());

            CatalogConfig::Rest {
                endpoint,
                warehouse,
                token: args.token.clone(),
                properties: HashMap::new(),
            }
        }
        "s3-tables" => {
            let arn = args.arn.clone().ok_or_else(|| {
                CliError::InvalidArgument("--arn required for S3 Tables catalog".to_string())
            })?;
            let region = args
                .region
                .clone()
                .unwrap_or_else(|| "us-east-1".to_string());

            CatalogConfig::S3Tables {
                arn,
                region,
                profile: args.profile.clone(),
            }
        }
        "unity" => {
            let endpoint = args.endpoint.clone().ok_or_else(|| {
                CliError::InvalidArgument("--endpoint required for Unity Catalog".to_string())
            })?;
            let token = args.token.clone().ok_or_else(|| {
                CliError::InvalidArgument("--token required for Unity Catalog".to_string())
            })?;

            CatalogConfig::Unity {
                endpoint,
                catalog: args
                    .warehouse
                    .clone()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "main".to_string()),
                token,
            }
        }
        "glue" => {
            let region = args
                .region
                .clone()
                .unwrap_or_else(|| "us-east-1".to_string());
            let database = args
                .warehouse
                .clone()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "staging".to_string());

            CatalogConfig::Glue {
                region,
                database,
                profile: args.profile.clone(),
            }
        }
        _ => {
            return Err(CliError::InvalidArgument(format!(
                "Unknown catalog type: {}. Valid types: rest, s3-tables, unity, glue",
                catalog_type
            )));
        }
    };

    println!("Initializing Iceberg catalog...");
    println!("  Type: {}", catalog_type);

    match &config {
        CatalogConfig::Rest {
            endpoint,
            warehouse,
            ..
        } => {
            println!("  Endpoint: {}", endpoint);
            println!("  Warehouse: {}", warehouse);
        }
        CatalogConfig::S3Tables { arn, region, .. } => {
            println!("  ARN: {}", arn);
            println!("  Region: {}", region);
        }
        CatalogConfig::Unity {
            endpoint, catalog, ..
        } => {
            println!("  Endpoint: {}", endpoint);
            println!("  Catalog: {}", catalog);
        }
        CatalogConfig::Glue {
            region, database, ..
        } => {
            println!("  Region: {}", region);
            println!("  Database: {}", database);
        }
    }

    // Save catalog configuration
    let config_path = args.database.with_extension("catalog.json");
    let config_json =
        serde_json::to_string_pretty(&config).map_err(|e| CliError::StagingError(e.to_string()))?;
    std::fs::write(&config_path, config_json)
        .map_err(|e| CliError::StagingError(format!("Failed to write catalog config: {}", e)))?;

    println!();
    println!("Catalog configuration saved to: {}", config_path.display());
    println!();
    println!("Note: Iceberg catalog connection will be established on first ingest operation.");
    println!("To test the connection, run: odm staging history");

    Ok(())
}

/// Handle the `staging ingest` command
pub fn handle_staging_ingest(args: &StagingIngestArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    let db = StagingDb::open(&db_path).map_err(|e| CliError::StagingError(e.to_string()))?;

    // Build the ingest configuration
    let mut config_builder = IngestConfig::builder()
        .source_type(SourceType::Local(args.source.clone()))
        .pattern(&args.pattern)
        .dedup(args.dedup)
        .batch_size(args.batch_size)
        .resume(args.resume);

    if let Some(ref partition) = args.partition {
        config_builder = config_builder.partition(partition);
    }

    if let Some(ref batch_id) = args.batch_id {
        config_builder = config_builder.batch_id(batch_id);
    }

    let config = config_builder
        .build()
        .map_err(|e| CliError::StagingError(e.to_string()))?;

    println!("Starting ingestion from: {}", args.source.display());
    println!("Pattern: {}", args.pattern);
    println!("Deduplication: {:?}", args.dedup);

    let stats = db
        .ingest(&config)
        .map_err(|e| CliError::StagingError(e.to_string()))?;

    println!();
    println!("Ingestion complete:");
    println!("  Files processed: {}", stats.files_processed);
    println!("  Files skipped:   {}", stats.files_skipped);
    println!("  Records ingested: {}", stats.records_ingested);
    println!(
        "  Bytes processed: {} MB",
        stats.bytes_processed / 1_000_000
    );
    println!("  Duration: {}", stats.duration_string());

    if !stats.errors.is_empty() {
        println!();
        println!("Errors ({}):", stats.errors.len());
        for error in stats.errors.iter().take(10) {
            println!("  - {}", error);
        }
        if stats.errors.len() > 10 {
            println!("  ... and {} more", stats.errors.len() - 10);
        }
    }

    Ok(())
}

/// Handle the `staging stats` command
pub fn handle_staging_stats(args: &StagingStatsArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    let db = StagingDb::open(&db_path).map_err(|e| CliError::StagingError(e.to_string()))?;

    if !db
        .is_initialized()
        .map_err(|e| CliError::StagingError(e.to_string()))?
    {
        return Err(CliError::StagingError(
            "Database not initialized. Run 'staging init' first.".to_string(),
        ));
    }

    let total_records = db
        .record_count(args.partition.as_deref())
        .map_err(|e| CliError::StagingError(e.to_string()))?;

    println!("Staging Database Statistics");
    println!("===========================");
    println!("Database: {}", db_path);
    println!(
        "Schema version: {}",
        db.schema_version()
            .map_err(|e| CliError::StagingError(e.to_string()))?
    );
    println!();

    if args.partition.is_some() {
        println!(
            "Partition '{}': {} records",
            args.partition.as_ref().unwrap(),
            total_records
        );
    } else {
        println!("Total records: {}", total_records);
        println!();

        // Show partition breakdown
        let partition_stats = db
            .partition_stats()
            .map_err(|e| CliError::StagingError(e.to_string()))?;

        if !partition_stats.is_empty() {
            println!("Records by partition:");
            for (partition, count) in partition_stats {
                println!("  {}: {}", partition, count);
            }
        }
    }

    Ok(())
}

/// Handle the `staging batches` command
pub fn handle_staging_batches(args: &StagingBatchesArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    let db = StagingDb::open(&db_path).map_err(|e| CliError::StagingError(e.to_string()))?;

    if !db
        .is_initialized()
        .map_err(|e| CliError::StagingError(e.to_string()))?
    {
        return Err(CliError::StagingError(
            "Database not initialized. Run 'staging init' first.".to_string(),
        ));
    }

    let batches = db
        .list_batches(args.limit)
        .map_err(|e| CliError::StagingError(e.to_string()))?;

    if batches.is_empty() {
        println!("No processing batches found.");
        return Ok(());
    }

    println!("Recent Processing Batches");
    println!("=========================");
    println!();

    for batch in batches {
        println!("Batch: {}", batch.id);
        println!("  Source: {} ({})", batch.source_path, batch.source_type);
        println!("  Status: {}", batch.status);
        println!(
            "  Files: {} processed, {} skipped, {} total",
            batch.files_processed, batch.files_skipped, batch.files_total
        );
        println!("  Records: {}", batch.records_ingested);
        if let Some(started) = batch.started_at {
            println!("  Started: {}", started.format("%Y-%m-%d %H:%M:%S"));
        }
        if let Some(completed) = batch.completed_at {
            println!("  Completed: {}", completed.format("%Y-%m-%d %H:%M:%S"));
        }
        if batch.errors_count > 0 {
            println!("  Errors: {}", batch.errors_count);
        }
        if let Some(ref error) = batch.error_message {
            println!("  Error message: {}", error);
        }
        println!();
    }

    Ok(())
}

/// Handle the `staging query` command
pub fn handle_staging_query(args: &StagingQueryArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    // Check for time travel options
    #[cfg(feature = "iceberg")]
    if args.version.is_some() || args.timestamp.is_some() {
        return handle_staging_query_time_travel(args);
    }

    let db = StagingDb::open(&db_path).map_err(|e| CliError::StagingError(e.to_string()))?;

    if !db
        .is_initialized()
        .map_err(|e| CliError::StagingError(e.to_string()))?
    {
        return Err(CliError::StagingError(
            "Database not initialized. Run 'staging init' first.".to_string(),
        ));
    }

    let results = db
        .query(&args.sql)
        .map_err(|e| CliError::StagingError(e.to_string()))?;

    print_query_results(&results, &args.format)
}

/// Handle time travel query with Iceberg
#[cfg(feature = "iceberg")]
fn handle_staging_query_time_travel(args: &StagingQueryArgs) -> Result<(), CliError> {
    use chrono::DateTime;

    let _db_path = args.database.display().to_string();

    if let Some(version) = args.version {
        println!("Executing time travel query at version {}...", version);
        println!("SQL: {}", args.sql);
        println!();

        // TODO: Implement actual Iceberg time travel query
        // For now, show placeholder message
        println!("Note: Iceberg time travel queries require a configured catalog.");
        println!("Initialize with: odm staging init --catalog rest --endpoint <url>");

        return Err(CliError::StagingError(
            "Iceberg time travel not yet fully implemented".to_string(),
        ));
    }

    if let Some(ref timestamp_str) = args.timestamp {
        let timestamp = DateTime::parse_from_rfc3339(timestamp_str).map_err(|e| {
            CliError::InvalidArgument(format!(
                "Invalid timestamp format: {}. Use ISO 8601 format (e.g., 2025-01-10T00:00:00Z)",
                e
            ))
        })?;

        println!("Executing time travel query at timestamp {}...", timestamp);
        println!("SQL: {}", args.sql);
        println!();

        // TODO: Implement actual Iceberg time travel query
        println!("Note: Iceberg time travel queries require a configured catalog.");
        println!("Initialize with: odm staging init --catalog rest --endpoint <url>");

        return Err(CliError::StagingError(
            "Iceberg time travel not yet fully implemented".to_string(),
        ));
    }

    Err(CliError::InvalidArgument(
        "Time travel requires --version or --timestamp".to_string(),
    ))
}

/// Print query results in the specified format
fn print_query_results(results: &[serde_json::Value], format: &str) -> Result<(), CliError> {
    match format {
        "json" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&results)
                    .map_err(|e| CliError::StagingError(e.to_string()))?
            );
        }
        "table" | _ => {
            if results.is_empty() {
                println!("No results.");
                return Ok(());
            }

            // Get column names from first row
            let columns: Vec<&str> = results[0]
                .as_object()
                .map(|obj| obj.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();

            // Print header
            println!("{}", columns.join("\t"));
            println!(
                "{}",
                columns.iter().map(|_| "---").collect::<Vec<_>>().join("\t")
            );

            // Print rows
            for row in results {
                let values: Vec<String> = columns
                    .iter()
                    .map(|col| {
                        row.get(*col)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Null => "NULL".to_string(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                println!("{}", values.join("\t"));
            }

            println!();
            println!("{} row(s)", results.len());
        }
    }
    Ok(())
}

/// Handle the `staging sample` command
pub fn handle_staging_sample(args: &StagingSampleArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    let db = StagingDb::open(&db_path).map_err(|e| CliError::StagingError(e.to_string()))?;

    if !db
        .is_initialized()
        .map_err(|e| CliError::StagingError(e.to_string()))?
    {
        return Err(CliError::StagingError(
            "Database not initialized. Run 'staging init' first.".to_string(),
        ));
    }

    let samples = db
        .get_sample(args.limit, args.partition.as_deref())
        .map_err(|e| CliError::StagingError(e.to_string()))?;

    if samples.is_empty() {
        println!("No samples found.");
        return Ok(());
    }

    println!("Sample Records ({}):", samples.len());
    println!();

    for (i, sample) in samples.iter().enumerate() {
        // Pretty-print the JSON
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(sample) {
            println!(
                "[{}] {}",
                i + 1,
                serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| sample.clone())
            );
        } else {
            println!("[{}] {}", i + 1, sample);
        }
        println!();
    }

    Ok(())
}

/// Handle the `staging history` command
pub fn handle_staging_history(args: &StagingHistoryArgs) -> Result<(), CliError> {
    #[cfg(feature = "iceberg")]
    {
        use data_modelling_core::staging::CatalogConfig;

        let config_path = args.database.with_extension("catalog.json");

        if !config_path.exists() {
            return Err(CliError::StagingError(
                "No Iceberg catalog configured. Initialize with: odm staging init --catalog <type>"
                    .to_string(),
            ));
        }

        let config_json = std::fs::read_to_string(&config_path)
            .map_err(|e| CliError::StagingError(format!("Failed to read catalog config: {}", e)))?;

        let _config: CatalogConfig = serde_json::from_str(&config_json)
            .map_err(|e| CliError::StagingError(format!("Invalid catalog config: {}", e)))?;

        let table_name = args.table.as_deref().unwrap_or("raw_json");

        println!("Table History: {}", table_name);
        println!("================");
        println!();

        // TODO: Implement actual Iceberg history query
        // This requires connecting to the catalog and querying table metadata
        println!("Note: Table history requires a running Iceberg catalog.");
        println!();
        println!("Placeholder output:");
        println!("  Version 1 - 2025-01-10T10:00:00Z - Initial snapshot");
        println!("  Version 2 - 2025-01-10T11:00:00Z - Added 1000 records");
        println!("  Version 3 - 2025-01-10T12:00:00Z - Added 500 records");
        println!();
        println!("To query a specific version:");
        println!("  odm staging query --version 2 \"SELECT * FROM raw_json LIMIT 10\"");

        Ok(())
    }

    #[cfg(not(feature = "iceberg"))]
    {
        let _ = args; // Suppress unused warning
        Err(CliError::StagingError(
            "Iceberg support not enabled. Enable 'iceberg' feature to use table history."
                .to_string(),
        ))
    }
}

/// Handle the `staging export` command
pub fn handle_staging_export(args: &StagingExportArgs) -> Result<(), CliError> {
    #[cfg(feature = "iceberg")]
    {
        use data_modelling_core::staging::CatalogConfig;

        // Load source catalog config
        let config_path = args.database.with_extension("catalog.json");

        if !config_path.exists() {
            return Err(CliError::StagingError(
                "No Iceberg catalog configured. Initialize with: odm staging init --catalog <type>"
                    .to_string(),
            ));
        }

        let config_json = std::fs::read_to_string(&config_path)
            .map_err(|e| CliError::StagingError(format!("Failed to read catalog config: {}", e)))?;

        let _source_config: CatalogConfig = serde_json::from_str(&config_json)
            .map_err(|e| CliError::StagingError(format!("Invalid catalog config: {}", e)))?;

        // Build target catalog config
        let target_config = match args.target.as_str() {
            "unity" => {
                let endpoint = args.endpoint.clone().ok_or_else(|| {
                    CliError::InvalidArgument(
                        "--endpoint required for Unity Catalog export".to_string(),
                    )
                })?;
                let token = args.token.clone().ok_or_else(|| {
                    CliError::InvalidArgument(
                        "--token required for Unity Catalog export".to_string(),
                    )
                })?;
                let catalog = args.catalog.clone().unwrap_or_else(|| "main".to_string());

                CatalogConfig::Unity {
                    endpoint,
                    catalog,
                    token,
                }
            }
            "s3-tables" => {
                let arn = args.arn.clone().ok_or_else(|| {
                    CliError::InvalidArgument("--arn required for S3 Tables export".to_string())
                })?;
                let region = args
                    .region
                    .clone()
                    .unwrap_or_else(|| "us-east-1".to_string());

                CatalogConfig::S3Tables {
                    arn,
                    region,
                    profile: args.profile.clone(),
                }
            }
            "glue" => {
                let region = args
                    .region
                    .clone()
                    .unwrap_or_else(|| "us-east-1".to_string());
                let database = args.schema.clone().unwrap_or_else(|| "staging".to_string());

                CatalogConfig::Glue {
                    region,
                    database,
                    profile: args.profile.clone(),
                }
            }
            _ => {
                return Err(CliError::InvalidArgument(format!(
                    "Unknown target catalog: {}. Valid targets: unity, s3-tables, glue",
                    args.target
                )));
            }
        };

        println!("Exporting table to {} catalog...", args.target);
        println!();
        println!("Source table: raw_json");
        println!("Target table: {}", args.table);

        match &target_config {
            CatalogConfig::Unity {
                endpoint, catalog, ..
            } => {
                let schema = args.schema.as_deref().unwrap_or("staging");
                println!(
                    "Target location: {}/catalogs/{}/schemas/{}/tables/{}",
                    endpoint, catalog, schema, args.table
                );
            }
            CatalogConfig::S3Tables { arn, .. } => {
                println!("Target ARN: {}/{}", arn, args.table);
            }
            CatalogConfig::Glue {
                database, region, ..
            } => {
                println!("Target: {}:{}/{}", region, database, args.table);
            }
            CatalogConfig::Rest { .. } => {}
        }

        println!();

        // TODO: Implement actual export
        // This requires:
        // 1. Reading from source Iceberg table
        // 2. Writing Parquet files to target storage
        // 3. Registering table in target catalog

        println!("Note: Export to production catalogs is not yet fully implemented.");
        println!();
        println!("This will:");
        println!("  1. Read data from local Iceberg table");
        println!("  2. Write Parquet files to target storage");
        println!("  3. Register table in target catalog");

        Ok(())
    }

    #[cfg(not(feature = "iceberg"))]
    {
        let _ = args; // Suppress unused warning
        Err(CliError::StagingError(
            "Iceberg support not enabled. Enable 'iceberg' feature to export to production catalogs.".to_string()
        ))
    }
}

/// Handle the `staging view create` command
pub fn handle_staging_view_create(args: &StagingViewCreateArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    let db = StagingDb::open(&db_path).map_err(|e| CliError::StagingError(e.to_string()))?;

    if !db
        .is_initialized()
        .map_err(|e| CliError::StagingError(e.to_string()))?
    {
        return Err(CliError::StagingError(
            "Database not initialized. Run 'staging init' first.".to_string(),
        ));
    }

    // Read the inferred schema
    let schema_content = std::fs::read_to_string(&args.schema)
        .map_err(|e| CliError::StagingError(format!("Failed to read schema file: {}", e)))?;

    let schema: serde_json::Value = serde_json::from_str(&schema_content)
        .map_err(|e| CliError::StagingError(format!("Invalid schema JSON: {}", e)))?;

    let source_table = args.source_table.as_deref().unwrap_or("staged_json");

    // Generate CREATE VIEW SQL from inferred schema
    let view_sql = generate_view_sql(&args.name, source_table, &schema)?;

    println!("Creating view: {}", args.name);
    println!();
    println!("Generated SQL:");
    println!("{}", view_sql);
    println!();

    // Execute the CREATE VIEW statement
    db.query(&view_sql)
        .map_err(|e| CliError::StagingError(format!("Failed to create view: {}", e)))?;

    println!("View created successfully.");
    println!();
    println!("Query with:");
    println!(
        "  odm staging query \"SELECT * FROM {} LIMIT 10\"",
        args.name
    );

    Ok(())
}

/// Generate CREATE VIEW SQL from inferred schema
fn generate_view_sql(
    view_name: &str,
    source_table: &str,
    schema: &serde_json::Value,
) -> Result<String, CliError> {
    let mut columns = Vec::new();

    // Get fields from schema (handle both ODCS and JSON Schema formats)
    let fields = if let Some(properties) = schema.get("properties") {
        // JSON Schema format
        properties.as_object().ok_or_else(|| {
            CliError::StagingError("Schema 'properties' is not an object".to_string())
        })?
    } else if let Some(columns_arr) = schema.get("columns") {
        // ODCS format - convert array to object-like iteration
        return generate_view_sql_odcs(view_name, source_table, columns_arr);
    } else {
        return Err(CliError::StagingError(
            "Schema must have 'properties' (JSON Schema) or 'columns' (ODCS)".to_string(),
        ));
    };

    for (field_name, field_def) in fields {
        let json_path = format!("$.{}", field_name);
        let field_type = field_def
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("string");

        let sql_expr = match field_type {
            "integer" | "number" => {
                if field_type == "integer" {
                    format!(
                        "CAST(json_extract(raw_json, '{}') AS BIGINT) AS {}",
                        json_path, field_name
                    )
                } else {
                    format!(
                        "CAST(json_extract(raw_json, '{}') AS DOUBLE) AS {}",
                        json_path, field_name
                    )
                }
            }
            "boolean" => {
                format!(
                    "CAST(json_extract(raw_json, '{}') AS BOOLEAN) AS {}",
                    json_path, field_name
                )
            }
            "array" | "object" => {
                // Keep complex types as JSON strings
                format!("json_extract(raw_json, '{}') AS {}", json_path, field_name)
            }
            _ => {
                // Default to string extraction
                format!(
                    "json_extract_string(raw_json, '{}') AS {}",
                    json_path, field_name
                )
            }
        };

        columns.push(sql_expr);
    }

    if columns.is_empty() {
        return Err(CliError::StagingError("Schema has no fields".to_string()));
    }

    Ok(format!(
        "CREATE OR REPLACE VIEW {} AS\nSELECT\n  {}\nFROM {}",
        view_name,
        columns.join(",\n  "),
        source_table
    ))
}

/// Generate CREATE VIEW SQL from ODCS schema format
fn generate_view_sql_odcs(
    view_name: &str,
    source_table: &str,
    columns_arr: &serde_json::Value,
) -> Result<String, CliError> {
    let columns_list = columns_arr
        .as_array()
        .ok_or_else(|| CliError::StagingError("Schema 'columns' is not an array".to_string()))?;

    let mut sql_columns = Vec::new();

    for col in columns_list {
        let col_name = col
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| CliError::StagingError("Column missing 'name' field".to_string()))?;

        let col_type = col
            .get("dataType")
            .or_else(|| col.get("logicalType"))
            .and_then(|t| t.as_str())
            .unwrap_or("string");

        let json_path = format!("$.{}", col_name);

        let sql_expr = match col_type.to_lowercase().as_str() {
            "integer" | "int" | "bigint" | "long" => {
                format!(
                    "CAST(json_extract(raw_json, '{}') AS BIGINT) AS {}",
                    json_path, col_name
                )
            }
            "double" | "float" | "decimal" | "number" => {
                format!(
                    "CAST(json_extract(raw_json, '{}') AS DOUBLE) AS {}",
                    json_path, col_name
                )
            }
            "boolean" | "bool" => {
                format!(
                    "CAST(json_extract(raw_json, '{}') AS BOOLEAN) AS {}",
                    json_path, col_name
                )
            }
            "date" => {
                format!(
                    "CAST(json_extract_string(raw_json, '{}') AS DATE) AS {}",
                    json_path, col_name
                )
            }
            "timestamp" | "datetime" => {
                format!(
                    "CAST(json_extract_string(raw_json, '{}') AS TIMESTAMP) AS {}",
                    json_path, col_name
                )
            }
            "array" | "object" | "struct" | "map" => {
                format!("json_extract(raw_json, '{}') AS {}", json_path, col_name)
            }
            _ => {
                format!(
                    "json_extract_string(raw_json, '{}') AS {}",
                    json_path, col_name
                )
            }
        };

        sql_columns.push(sql_expr);
    }

    if sql_columns.is_empty() {
        return Err(CliError::StagingError("Schema has no columns".to_string()));
    }

    Ok(format!(
        "CREATE OR REPLACE VIEW {} AS\nSELECT\n  {}\nFROM {}",
        view_name,
        sql_columns.join(",\n  "),
        source_table
    ))
}
