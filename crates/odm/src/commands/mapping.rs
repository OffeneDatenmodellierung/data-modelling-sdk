//! CLI commands for schema mapping operations

use std::path::PathBuf;

use crate::error::CliError;
use data_modelling_core::mapping::{
    MappingConfig, SchemaMatcher, TransformFormat, generate_transform,
};

/// Arguments for the `map` command
pub struct MapArgs {
    /// Source schema file
    pub source: PathBuf,
    /// Target schema file
    pub target: PathBuf,
    /// Output mapping file
    pub output: Option<PathBuf>,
    /// Minimum similarity threshold
    pub min_similarity: f64,
    /// Enable fuzzy matching
    pub fuzzy: bool,
    /// Enable case-insensitive matching
    pub case_insensitive: bool,
    /// Transform output format
    pub transform_format: String,
    /// Transform output file
    pub transform_output: Option<PathBuf>,
    /// Verbose output
    pub verbose: bool,
}

/// Handle the `map` command
pub fn handle_map(args: &MapArgs) -> Result<(), CliError> {
    // Load source schema
    let source_content = std::fs::read_to_string(&args.source)
        .map_err(|e| CliError::MappingError(format!("Failed to read source schema: {}", e)))?;

    let source_schema: serde_json::Value = serde_json::from_str(&source_content)
        .map_err(|e| CliError::MappingError(format!("Failed to parse source schema: {}", e)))?;

    // Load target schema
    let target_content = std::fs::read_to_string(&args.target)
        .map_err(|e| CliError::MappingError(format!("Failed to read target schema: {}", e)))?;

    let target_schema: serde_json::Value = serde_json::from_str(&target_content)
        .map_err(|e| CliError::MappingError(format!("Failed to parse target schema: {}", e)))?;

    // Parse transform format
    let transform_format: TransformFormat = args
        .transform_format
        .parse()
        .map_err(|e| CliError::InvalidArgument(format!("Invalid transform format: {}", e)))?;

    // Build config
    let config = MappingConfig::new()
        .with_min_confidence(args.min_similarity)
        .with_fuzzy_matching(args.fuzzy)
        .with_case_insensitive(args.case_insensitive)
        .with_transform_format(transform_format);

    if args.verbose {
        eprintln!("Mapping schemas...");
        eprintln!("  Source: {}", args.source.display());
        eprintln!("  Target: {}", args.target.display());
        eprintln!("  Min similarity: {:.0}%", args.min_similarity * 100.0);
        eprintln!("  Fuzzy matching: {}", args.fuzzy);
    }

    // Run mapping
    let matcher = SchemaMatcher::with_config(config.clone());
    let mapping = matcher
        .match_schemas(&source_schema, &target_schema)
        .map_err(|e| CliError::MappingError(format!("Mapping failed: {}", e)))?;

    // Print summary
    eprintln!();
    eprintln!("Mapping Results");
    eprintln!("===============");
    eprintln!(
        "Compatibility score: {:.1}%",
        mapping.compatibility_score * 100.0
    );
    eprintln!("Direct mappings: {}", mapping.direct_mappings.len());
    eprintln!("Transformations: {}", mapping.transformations.len());
    eprintln!(
        "Gaps: {} ({} required)",
        mapping.gaps.len(),
        mapping.stats.required_gaps
    );
    eprintln!("Extras: {}", mapping.extras.len());

    // Show direct mappings
    if args.verbose && !mapping.direct_mappings.is_empty() {
        eprintln!();
        eprintln!("Direct Mappings:");
        for m in &mapping.direct_mappings {
            let compat = if m.type_compatible {
                ""
            } else {
                " (type mismatch)"
            };
            eprintln!(
                "  {} -> {} ({:.0}%, {}){}",
                m.source_path,
                m.target_path,
                m.confidence * 100.0,
                m.match_method,
                compat
            );
        }
    }

    // Show transformations
    if args.verbose && !mapping.transformations.is_empty() {
        eprintln!();
        eprintln!("Transformations:");
        for t in &mapping.transformations {
            eprintln!(
                "  {:?} -> {}: {}",
                t.source_paths, t.target_path, t.description
            );
        }
    }

    // Show gaps
    if !mapping.gaps.is_empty() {
        eprintln!();
        eprintln!("Gaps (unmapped target fields):");
        for gap in &mapping.gaps {
            let required = if gap.required { " [REQUIRED]" } else { "" };
            eprintln!("  {}: {}{}", gap.target_path, gap.target_type, required);
            if !gap.suggestions.is_empty() && args.verbose {
                eprintln!("    Suggestions: {}", gap.suggestions.join(", "));
            }
        }
    }

    // Show extras
    if args.verbose && !mapping.extras.is_empty() {
        eprintln!();
        eprintln!("Extras (unmapped source fields):");
        for extra in &mapping.extras {
            eprintln!("  {}", extra);
        }
    }

    // Output mapping JSON
    if let Some(ref output_path) = args.output {
        let mapping_json = serde_json::to_string_pretty(&mapping)
            .map_err(|e| CliError::MappingError(format!("Failed to serialize mapping: {}", e)))?;

        std::fs::write(output_path, &mapping_json)
            .map_err(|e| CliError::MappingError(format!("Failed to write mapping file: {}", e)))?;

        eprintln!();
        eprintln!("Mapping written to: {}", output_path.display());
    } else {
        // Print to stdout
        let mapping_json = serde_json::to_string_pretty(&mapping)
            .map_err(|e| CliError::MappingError(format!("Failed to serialize mapping: {}", e)))?;
        println!("{}", mapping_json);
    }

    // Generate transform script
    if let Some(ref transform_path) = args.transform_output {
        let script = generate_transform(&mapping, transform_format, "source_table", "target_table")
            .map_err(|e| CliError::MappingError(format!("Failed to generate transform: {}", e)))?;

        std::fs::write(transform_path, &script).map_err(|e| {
            CliError::MappingError(format!("Failed to write transform file: {}", e))
        })?;

        eprintln!("Transform script written to: {}", transform_path.display());
    }

    // Return error if there are required gaps
    if mapping.stats.required_gaps > 0 {
        eprintln!();
        eprintln!(
            "WARNING: {} required target fields have no mapping!",
            mapping.stats.required_gaps
        );
    }

    Ok(())
}
