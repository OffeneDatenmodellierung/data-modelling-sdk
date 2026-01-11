//! CLI commands for staging database operations

use std::path::PathBuf;

use crate::error::CliError;
use data_modelling_core::staging::{DedupStrategy, IngestConfig, SourceType, StagingDb};

/// Arguments for the `staging init` command
pub struct StagingInitArgs {
    /// Path to the staging database file
    pub database: PathBuf,
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

/// Handle the `staging init` command
pub fn handle_staging_init(args: &StagingInitArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

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

    match args.format.as_str() {
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
            for row in &results {
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
