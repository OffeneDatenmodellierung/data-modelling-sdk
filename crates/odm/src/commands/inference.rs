//! CLI commands for schema inference operations
//!
//! Note: Some fields in argument structs are defined for future LLM integration
//! but are not yet used.

#![allow(dead_code)]

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
    /// LLM mode (none, online, offline)
    pub llm_mode: String,
    /// Ollama URL for online mode
    pub ollama_url: String,
    /// Model name
    pub model: String,
    /// Path to GGUF model for offline mode
    pub model_path: Option<PathBuf>,
    /// Path to documentation file
    pub doc_path: Option<PathBuf>,
    /// Skip LLM refinement
    pub no_refine: bool,
    /// Temperature for generation
    pub temperature: f32,
    /// Verbose LLM output
    pub verbose_llm: bool,
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

    // Convert to JSON Schema for potential LLM refinement
    let json_schema = schema.to_json_schema();

    // Apply LLM refinement if enabled
    let final_schema = if !args.no_refine && args.llm_mode != "none" {
        #[cfg(feature = "llm")]
        {
            refine_with_llm(args, &json_schema, &samples)?
        }
        #[cfg(not(feature = "llm"))]
        {
            if args.llm_mode != "none" {
                eprintln!("Warning: LLM refinement requested but 'llm' feature not enabled");
            }
            json_schema
        }
    } else {
        json_schema
    };

    // Format output
    let output_str = match args.format.as_str() {
        "json-schema" | "json" => serde_json::to_string_pretty(&final_schema)
            .map_err(|e| CliError::InferenceError(e.to_string()))?,
        "yaml" => serde_yaml::to_string(&final_schema)
            .map_err(|e| CliError::InferenceError(e.to_string()))?,
        _ => serde_json::to_string_pretty(&final_schema)
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

/// Refine schema using LLM (feature-gated)
#[cfg(feature = "llm")]
fn refine_with_llm(
    args: &InferenceInferArgs,
    schema: &serde_json::Value,
    samples: &[String],
) -> Result<serde_json::Value, CliError> {
    use data_modelling_core::llm::{LlmClient, LlmMode, RefinementConfig, refine_schema};

    eprintln!();
    eprintln!("Refining schema with LLM...");
    eprintln!("  Mode: {}", args.llm_mode);

    // Build LLM configuration
    let llm_mode = match args.llm_mode.as_str() {
        "online" => {
            eprintln!("  URL: {}", args.ollama_url);
            eprintln!("  Model: {}", args.model);
            LlmMode::Online {
                url: args.ollama_url.clone(),
                model: args.model.clone(),
            }
        }
        "offline" => {
            let model_path = args.model_path.clone().ok_or_else(|| {
                CliError::InferenceError(
                    "Offline mode requires --model-path to be specified".to_string(),
                )
            })?;
            eprintln!("  Model path: {}", model_path.display());
            LlmMode::Offline {
                model_path,
                gpu_layers: 0,
            }
        }
        _ => {
            return Err(CliError::InferenceError(format!(
                "Invalid LLM mode: {}. Use 'online' or 'offline'",
                args.llm_mode
            )));
        }
    };

    let config = RefinementConfig {
        llm_mode,
        documentation_path: args.doc_path.clone(),
        documentation_text: None,
        max_context_tokens: 4096,
        timeout_seconds: 120,
        max_retries: 3,
        temperature: args.temperature,
        include_samples: true,
        max_samples: 5,
        verbose: args.verbose_llm,
    };

    if let Some(ref doc_path) = args.doc_path {
        eprintln!("  Documentation: {}", doc_path.display());
    }

    // Create async runtime for LLM call
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::InferenceError(format!("Failed to create runtime: {}", e)))?;

    let result = rt.block_on(async {
        // Create client based on mode
        #[cfg(feature = "llm-online")]
        if matches!(config.llm_mode, LlmMode::Online { .. }) {
            if let LlmMode::Online { ref url, ref model } = config.llm_mode {
                let client = data_modelling_core::llm::OllamaClient::new(url, model);

                // Check if model is available
                if !client.is_ready().await {
                    return Err(CliError::InferenceError(
                        "Ollama server not reachable or model not available".to_string(),
                    ));
                }

                let sample_strings: Option<Vec<String>> = if config.include_samples {
                    Some(samples.iter().take(config.max_samples).cloned().collect())
                } else {
                    None
                };

                return refine_schema(&client, schema, &config, sample_strings)
                    .await
                    .map_err(|e| {
                        CliError::InferenceError(format!("LLM refinement failed: {}", e))
                    });
            }
        }

        #[cfg(feature = "llm-offline")]
        if matches!(config.llm_mode, LlmMode::Offline { .. }) {
            if let LlmMode::Offline {
                ref model_path,
                gpu_layers,
            } = config.llm_mode
            {
                let client = data_modelling_core::llm::LlamaCppClient::new(model_path, gpu_layers)
                    .map_err(|e| {
                        CliError::InferenceError(format!("Failed to load model: {}", e))
                    })?;

                let sample_strings: Option<Vec<String>> = if config.include_samples {
                    Some(samples.iter().take(config.max_samples).cloned().collect())
                } else {
                    None
                };

                return refine_schema(&client, schema, &config, sample_strings)
                    .await
                    .map_err(|e| {
                        CliError::InferenceError(format!("LLM refinement failed: {}", e))
                    });
            }
        }

        // Fallback if neither feature is enabled for the mode
        Err(CliError::InferenceError(
            "LLM mode not supported. Enable llm-online or llm-offline feature.".to_string(),
        ))
    })?;

    if result.was_refined {
        eprintln!(
            "  Refinement successful ({}ms)",
            result.duration_ms.unwrap_or(0)
        );
        if !result.warnings.is_empty() {
            eprintln!("  Warnings:");
            for warning in &result.warnings {
                eprintln!("    - {}", warning);
            }
        }
    } else {
        eprintln!("  Schema unchanged");
    }

    Ok(result.schema)
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
