//! Manual ODPS import/export test script
//!
//! This script allows users to test ODPS import/export round-trips by:
//! 1. Importing an ODPS YAML file
//! 2. Displaying the imported data
//! 3. Exporting it back to ODPS YAML
//! 4. Validating both import and export

#[cfg(feature = "odps-validation")]
use clap::Parser;
#[cfg(all(feature = "odps-validation", feature = "cli"))]
use data_modelling_sdk::export::odps::ODPSExporter;
#[cfg(all(feature = "odps-validation", feature = "cli"))]
use data_modelling_sdk::import::odps::ODPSImporter;
#[cfg(all(feature = "odps-validation", feature = "cli"))]
use data_modelling_sdk::models::odps::ODPSDataProduct;
#[cfg(all(feature = "odps-validation", feature = "cli"))]
use std::fs;
#[cfg(all(feature = "odps-validation", feature = "cli"))]
use std::path::PathBuf;

#[cfg(all(feature = "odps-validation", feature = "cli"))]
#[derive(Parser)]
#[command(name = "test-odps")]
#[command(about = "Manual ODPS import/export test script")]
#[command(version)]
struct Args {
    /// ODPS YAML file to import
    file: PathBuf,

    /// Output file path (default: <input>.exported.yaml)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Verbose output (show detailed field information)
    #[arg(short, long)]
    verbose: bool,

    /// Skip validation (import/export without schema validation)
    #[arg(long)]
    no_validate: bool,
}

#[cfg(all(feature = "odps-validation", feature = "cli"))]
fn display_product(product: &ODPSDataProduct, verbose: bool) {
    println!("\n=== Imported ODPS Data Product ===\n");

    println!("ID:              {}", product.id);
    if let Some(name) = &product.name {
        println!("Name:            {}", name);
    }
    if let Some(version) = &product.version {
        println!("Version:         {}", version);
    }
    println!("Status:          {:?}", product.status);
    if let Some(domain) = &product.domain {
        println!("Domain:          {}", domain);
    }
    if let Some(tenant) = &product.tenant {
        println!("Tenant:          {}", tenant);
    }

    if !product.tags.is_empty() {
        println!("\nTags:");
        for tag in &product.tags {
            println!("  - {}", tag);
        }
    }

    if let Some(description) = &product.description {
        println!("\nDescription:");
        if let Some(purpose) = &description.purpose {
            println!("  Purpose:       {}", purpose);
        }
        if let Some(usage) = &description.usage {
            println!("  Usage:         {}", usage);
        }
        if let Some(limitations) = &description.limitations {
            println!("  Limitations:   {}", limitations);
        }
    }

    if let Some(input_ports) = &product.input_ports {
        println!("\nInput Ports ({}):", input_ports.len());
        for (i, port) in input_ports.iter().enumerate() {
            println!("  [{}] {}", i + 1, port.name);
            println!("      Version:    {}", port.version);
            println!("      Contract:   {}", port.contract_id);
            if verbose && !port.tags.is_empty() {
                println!("      Tags:       {:?}", port.tags);
            }
        }
    }

    if let Some(output_ports) = &product.output_ports {
        println!("\nOutput Ports ({}):", output_ports.len());
        for (i, port) in output_ports.iter().enumerate() {
            println!("  [{}] {}", i + 1, port.name);
            println!("      Version:    {}", port.version);
            if let Some(contract_id) = &port.contract_id {
                println!("      Contract:   {}", contract_id);
            }
            if let Some(desc) = &port.description {
                println!("      Description: {}", desc);
            }
            if verbose && !port.tags.is_empty() {
                println!("      Tags:       {:?}", port.tags);
            }
        }
    }

    if let Some(support) = &product.support {
        println!("\nSupport Channels ({}):", support.len());
        for s in support {
            println!("  - {}: {}", s.channel, s.url);
        }
    }

    if verbose {
        if let Some(custom_properties) = &product.custom_properties
            && !custom_properties.is_empty()
        {
            println!("\nCustom Properties:");
            for prop in custom_properties {
                println!("  {}: {}", prop.property, prop.value);
            }
        }

        if let Some(auth_defs) = &product.authoritative_definitions
            && !auth_defs.is_empty()
        {
            println!("\nAuthoritative Definitions:");
            for def in auth_defs {
                println!("  {}: {}", def.r#type, def.url);
            }
        }
    }

    println!();
}

#[cfg(all(feature = "odps-validation", feature = "cli"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input_path = &args.file;

    if !input_path.exists() {
        eprintln!("Error: File not found: {}", input_path.display());
        std::process::exit(1);
    }

    println!("📥 Importing ODPS file: {}", input_path.display());

    // Read input file
    let content =
        fs::read_to_string(input_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Validate if enabled
    if !args.no_validate {
        println!("🔍 Validating ODPS schema...");
        #[cfg(feature = "cli")]
        {
            use data_modelling_sdk::cli::validation::validate_odps;
            validate_odps(&content).map_err(|e| format!("Validation failed: {}", e))?;
        }
        #[cfg(not(feature = "cli"))]
        {
            // Inline validation
            use jsonschema::Validator;
            use serde_json::Value;

            let schema_content = include_str!("../../schemas/odps-json-schema-latest.json");
            let schema: Value = serde_json::from_str(schema_content)
                .map_err(|e| format!("Failed to load ODPS schema: {}", e))?;

            let validator = Validator::new(&schema)
                .map_err(|e| format!("Failed to compile ODPS schema: {}", e))?;

            let data: Value = serde_yaml::from_str(&content)
                .map_err(|e| format!("Failed to parse YAML: {}", e))?;

            if let Err(errors) = validator.validate(&data) {
                let error_messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                return Err(
                    format!("ODPS validation failed:\n{}", error_messages.join("\n")).into(),
                );
            }
        }
        println!("✅ Validation passed");
    } else {
        println!("⚠️  Validation skipped (--no-validate)");
    }

    // Import ODPS
    println!("📦 Parsing ODPS data...");
    let importer = ODPSImporter::new();
    let product = importer
        .import(&content)
        .map_err(|e| format!("Import failed: {}", e))?;
    println!("✅ Import successful");

    // Display imported data
    display_product(&product, args.verbose);

    // Export back to ODPS
    println!("📤 Exporting to ODPS YAML...");
    let exporter = ODPSExporter;
    let exported_yaml = exporter
        .export(&product)
        .map_err(|e| format!("Export failed: {}", e))?;

    // Validate exported YAML if enabled
    if !args.no_validate {
        println!("🔍 Validating exported YAML...");
        #[cfg(feature = "cli")]
        {
            use data_modelling_sdk::cli::validation::validate_odps;
            validate_odps(&exported_yaml)
                .map_err(|e| format!("Exported YAML validation failed: {}", e))?;
        }
        #[cfg(not(feature = "cli"))]
        {
            // Inline validation
            use jsonschema::Validator;
            use serde_json::Value;

            let schema_content = include_str!("../../schemas/odps-json-schema-latest.json");
            let schema: Value = serde_json::from_str(schema_content)
                .map_err(|e| format!("Failed to load ODPS schema: {}", e))?;

            let validator = Validator::new(&schema)
                .map_err(|e| format!("Failed to compile ODPS schema: {}", e))?;

            let data: Value = serde_yaml::from_str(&exported_yaml)
                .map_err(|e| format!("Failed to parse exported YAML: {}", e))?;

            if let Err(errors) = validator.validate(&data) {
                let error_messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                return Err(format!(
                    "Exported YAML validation failed:\n{}",
                    error_messages.join("\n")
                )
                .into());
            }
        }
        println!("✅ Exported YAML validation passed");
    }

    // Determine output path
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = input_path.clone();
        if let Some(stem) = path.file_stem() {
            path.set_file_name(format!("{}.exported.yaml", stem.to_string_lossy()));
        } else {
            path.set_file_name("output.exported.yaml");
        }
        path
    });

    // Write output (clone exported_yaml before using it in comparison)
    let exported_yaml_for_comparison = exported_yaml.clone();
    fs::write(&output_path, &exported_yaml)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    println!("✅ Exported to: {}", output_path.display());

    // Field preservation comparison (if verbose)
    if args.verbose {
        println!("\n🔍 Field Preservation Analysis:");
        match compare_yaml_files(&content, &exported_yaml_for_comparison) {
            Ok(differences) => {
                if differences.is_empty() {
                    println!("✅ All fields preserved perfectly!");
                } else {
                    println!("⚠️  Found {} field differences:", differences.len());
                    for diff in differences.iter().take(10) {
                        println!("  - {}", diff);
                    }
                    if differences.len() > 10 {
                        println!("  ... and {} more", differences.len() - 10);
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Could not compare files: {}", e);
            }
        }
    }

    println!("\n🎉 Round-trip test completed successfully!");

    Ok(())
}

/// Compare two YAML files field-by-field and return differences
#[cfg(all(feature = "odps-validation", feature = "cli"))]
fn compare_yaml_files(
    original: &str,
    exported: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    use serde_json::Value;

    // Parse both YAML files to JSON for comparison
    let original_json: Value = serde_yaml::from_str(original)?;
    let exported_json: Value = serde_yaml::from_str(exported)?;

    let mut differences = Vec::new();
    compare_json_values(&original_json, &exported_json, "", &mut differences);

    Ok(differences)
}

/// Recursively compare two JSON values
#[cfg(all(feature = "odps-validation", feature = "cli"))]
fn compare_json_values(
    original: &serde_json::Value,
    exported: &serde_json::Value,
    path: &str,
    differences: &mut Vec<String>,
) {
    use std::collections::HashSet;

    match (original, exported) {
        (serde_json::Value::Object(o1), serde_json::Value::Object(o2)) => {
            // Collect all keys from both objects
            let mut all_keys = HashSet::new();
            for key in o1.keys() {
                all_keys.insert(key.clone());
            }
            for key in o2.keys() {
                all_keys.insert(key.clone());
            }

            for key in all_keys {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                match (o1.get(&key), o2.get(&key)) {
                    (Some(v1), Some(v2)) => {
                        compare_json_values(v1, v2, &new_path, differences);
                    }
                    (Some(_), None) => {
                        differences.push(format!("{}: missing in exported", new_path));
                    }
                    (None, Some(_)) => {
                        differences.push(format!("{}: added in exported", new_path));
                    }
                    (None, None) => {}
                }
            }
        }
        (serde_json::Value::Array(a1), serde_json::Value::Array(a2)) => {
            if a1.len() != a2.len() {
                differences.push(format!(
                    "{}: array length differs ({} vs {})",
                    path,
                    a1.len(),
                    a2.len()
                ));
            }
            for (i, (v1, v2)) in a1.iter().zip(a2.iter()).enumerate() {
                compare_json_values(v1, v2, &format!("{}[{}]", path, i), differences);
            }
        }
        (v1, v2) => {
            if v1 != v2 {
                differences.push(format!("{}: value differs ({:?} vs {:?})", path, v1, v2));
            }
        }
    }
}

#[cfg(not(all(feature = "odps-validation", feature = "cli")))]
fn main() {
    eprintln!(
        "Error: ODPS validation and CLI features required. Build with --features odps-validation,cli"
    );
    std::process::exit(1);
}
