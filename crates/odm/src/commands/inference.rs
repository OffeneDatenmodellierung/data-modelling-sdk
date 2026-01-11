//! CLI commands for schema inference operations

use std::path::PathBuf;

use crate::error::CliError;
use data_modelling_core::inference::{
    InferenceConfig, InferredSchema, InferredType, SchemaInferrer, group_similar_schemas,
    merge_schemas,
};
use data_modelling_core::staging::StagingDb;

/// Arguments for the `inference infer` command
pub struct InferenceInferArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Partition to infer schema from
    pub partition: Option<String>,
    /// Sample size for inference
    pub sample_size: usize,
    /// Minimum field frequency (0.0-1.0)
    pub min_frequency: f64,
    /// Maximum depth for nested objects
    pub max_depth: usize,
    /// Enable format detection
    pub detect_formats: bool,
    /// Output format (json, yaml, json-schema)
    pub format: String,
    /// Output file path (stdout if not provided)
    pub output: Option<PathBuf>,
}

/// Arguments for the `inference schemas` command
pub struct InferenceSchemasArgs {
    /// Path to the staging database file
    pub database: PathBuf,
    /// Similarity threshold for grouping (0.0-1.0)
    pub threshold: f64,
    /// Output format (json, table)
    pub format: String,
}

/// Handle the `inference infer` command
pub fn handle_inference_infer(args: &InferenceInferArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    let db = StagingDb::open(&db_path).map_err(|e| CliError::InferenceError(e.to_string()))?;

    if !db
        .is_initialized()
        .map_err(|e| CliError::InferenceError(e.to_string()))?
    {
        return Err(CliError::InferenceError(
            "Database not initialized. Run 'staging init' first.".to_string(),
        ));
    }

    // Build inference configuration
    let config = InferenceConfig::builder()
        .sample_size(args.sample_size)
        .min_field_frequency(args.min_frequency)
        .detect_formats(args.detect_formats)
        .max_depth(args.max_depth)
        .build();

    eprintln!("Inferring schema from staging database...");
    eprintln!("  Sample size: {}", args.sample_size);
    eprintln!("  Min frequency: {:.0}%", args.min_frequency * 100.0);
    eprintln!("  Format detection: {}", args.detect_formats);

    // Get samples from the database
    let samples = db
        .get_sample(args.sample_size, args.partition.as_deref())
        .map_err(|e| CliError::InferenceError(e.to_string()))?;

    if samples.is_empty() {
        return Err(CliError::InferenceError(
            "No records found in staging database.".to_string(),
        ));
    }

    eprintln!("  Records sampled: {}", samples.len());

    // Create inferrer and process samples
    let mut inferrer = SchemaInferrer::with_config(config);

    for sample in &samples {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(sample) {
            let _ = inferrer.add_value(&value);
        }
    }

    let stats = inferrer.stats();
    let schema = inferrer
        .finalize()
        .map_err(|e| CliError::InferenceError(e.to_string()))?;

    eprintln!();
    eprintln!("Inference complete:");
    eprintln!("  Records processed: {}", stats.records_processed);
    eprintln!("  Fields discovered: {}", stats.fields_discovered);

    // Format output
    let output_str = match args.format.as_str() {
        "json-schema" => {
            let json_schema = schema.to_json_schema();
            serde_json::to_string_pretty(&json_schema)
                .map_err(|e| CliError::InferenceError(e.to_string()))?
        }
        "yaml" => {
            serde_yaml::to_string(&schema).map_err(|e| CliError::InferenceError(e.to_string()))?
        }
        "json" | _ => serde_json::to_string_pretty(&schema)
            .map_err(|e| CliError::InferenceError(e.to_string()))?,
    };

    // Write output
    if let Some(ref output_path) = args.output {
        std::fs::write(output_path, &output_str)
            .map_err(|e| CliError::InferenceError(e.to_string()))?;
        eprintln!();
        eprintln!("Schema written to: {}", output_path.display());
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

/// Handle the `inference schemas` command
pub fn handle_inference_schemas(args: &InferenceSchemasArgs) -> Result<(), CliError> {
    let db_path = args.database.display().to_string();

    let db = StagingDb::open(&db_path).map_err(|e| CliError::InferenceError(e.to_string()))?;

    if !db
        .is_initialized()
        .map_err(|e| CliError::InferenceError(e.to_string()))?
    {
        return Err(CliError::InferenceError(
            "Database not initialized. Run 'staging init' first.".to_string(),
        ));
    }

    // Get partition stats to infer schemas per partition
    let partition_stats = db
        .partition_stats()
        .map_err(|e| CliError::InferenceError(e.to_string()))?;

    if partition_stats.is_empty() {
        return Err(CliError::InferenceError(
            "No partitions found in staging database.".to_string(),
        ));
    }

    eprintln!(
        "Analyzing {} partitions with similarity threshold {:.0}%...",
        partition_stats.len(),
        args.threshold * 100.0
    );

    let config = InferenceConfig::builder()
        .sample_size(100)
        .detect_formats(true)
        .build();

    let mut partition_schemas: Vec<InferredSchema> = Vec::new();

    for (partition, count) in &partition_stats {
        let samples = db
            .get_sample(100, Some(partition))
            .map_err(|e| CliError::InferenceError(e.to_string()))?;

        if samples.is_empty() {
            continue;
        }

        let mut inferrer = SchemaInferrer::with_config(config.clone());
        for sample in &samples {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(sample) {
                let _ = inferrer.add_value(&value);
            }
        }

        match inferrer.finalize() {
            Ok(mut schema) => {
                schema.partition = Some(partition.clone());
                schema.record_count = *count as usize;
                partition_schemas.push(schema);
            }
            Err(_) => continue,
        }
    }

    if partition_schemas.is_empty() {
        return Err(CliError::InferenceError(
            "Could not infer schemas from any partition.".to_string(),
        ));
    }

    // Group similar schemas
    let groups = group_similar_schemas(&partition_schemas, args.threshold);

    match args.format.as_str() {
        "json" => {
            let output: Vec<serde_json::Value> = groups
                .iter()
                .enumerate()
                .map(|(i, group)| {
                    let partitions: Vec<&str> = group
                        .iter()
                        .filter_map(|&idx| partition_schemas[idx].partition.as_deref())
                        .collect();
                    let total_records: usize = group
                        .iter()
                        .map(|&idx| partition_schemas[idx].record_count)
                        .sum();

                    // Merge schemas in this group
                    let schemas_to_merge: Vec<_> = group
                        .iter()
                        .map(|&idx| partition_schemas[idx].clone())
                        .collect();
                    let merged = merge_schemas(schemas_to_merge);

                    serde_json::json!({
                        "group": i + 1,
                        "partitions": partitions,
                        "partition_count": group.len(),
                        "total_records": total_records,
                        "schema": merged
                    })
                })
                .collect();

            println!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .map_err(|e| CliError::InferenceError(e.to_string()))?
            );
        }
        "table" | _ => {
            println!("Schema Groups (threshold: {:.0}%)", args.threshold * 100.0);
            println!("{}", "=".repeat(50));
            println!();

            for (i, group) in groups.iter().enumerate() {
                let partitions: Vec<&str> = group
                    .iter()
                    .filter_map(|&idx| partition_schemas[idx].partition.as_deref())
                    .collect();
                let total_records: usize = group
                    .iter()
                    .map(|&idx| partition_schemas[idx].record_count)
                    .sum();

                println!(
                    "Group {} ({} partitions, {} records):",
                    i + 1,
                    group.len(),
                    total_records
                );
                for partition in partitions {
                    println!("  - {}", partition);
                }

                // Show field summary from first schema in group
                if let Some(&first_idx) = group.first() {
                    if let InferredType::Object { ref properties } =
                        partition_schemas[first_idx].root
                    {
                        println!("  Fields: {}", properties.len());
                        for (name, field) in properties.iter().take(5) {
                            let required = if field.required { "" } else { " (optional)" };
                            println!("    - {}: {:?}{}", name, field.field_type, required);
                        }
                        if properties.len() > 5 {
                            println!("    ... and {} more fields", properties.len() - 5);
                        }
                    }
                }
                println!();
            }

            println!(
                "Total: {} groups from {} partitions",
                groups.len(),
                partition_stats.len()
            );
        }
    }

    Ok(())
}
