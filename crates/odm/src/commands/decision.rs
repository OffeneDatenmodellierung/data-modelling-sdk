//! Decision (ADR) CLI commands
//!
//! Provides CLI commands for managing architecture decision records (ADRs).
//!
//! Note: These commands are defined but not yet wired into the main CLI.
//! They will be enabled in a future release.

#![allow(clippy::collapsible_if)]
#![allow(dead_code)]

use crate::error::CliError;
use chrono::Utc;
use data_modelling_core::export::decision::DecisionExporter;
use data_modelling_core::export::markdown::MarkdownExporter;
use data_modelling_core::import::decision::DecisionImporter;
use data_modelling_core::models::decision::{
    Decision, DecisionCategory, DecisionIndex, DecisionStatus,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Arguments for the `decision new` command
#[derive(Debug)]
pub struct DecisionNewArgs {
    /// Decision title
    pub title: String,
    /// Decision category
    pub category: String,
    /// Domain this decision belongs to (optional)
    pub domain: Option<String>,
    /// Workspace path
    pub workspace: PathBuf,
    /// Also export as Markdown
    pub export_markdown: bool,
}

/// Arguments for the `decision list` command
#[derive(Debug)]
pub struct DecisionListArgs {
    /// Workspace path
    pub workspace: PathBuf,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by category
    pub category: Option<String>,
    /// Filter by domain
    pub domain: Option<String>,
    /// Output format (table, json, csv)
    pub format: String,
}

/// Arguments for the `decision show` command
#[derive(Debug)]
pub struct DecisionShowArgs {
    /// Decision number (e.g., "ADR-0001" or just "1")
    pub number: String,
    /// Workspace path
    pub workspace: PathBuf,
    /// Output format (yaml, markdown, json)
    pub format: String,
}

/// Arguments for the `decision status` command
#[derive(Debug)]
pub struct DecisionStatusArgs {
    /// Decision number
    pub number: String,
    /// New status
    pub status: String,
    /// Workspace path
    pub workspace: PathBuf,
}

/// Arguments for the `decision export` command
#[derive(Debug)]
pub struct DecisionExportArgs {
    /// Workspace path
    pub workspace: PathBuf,
    /// Output directory for Markdown files
    pub output: Option<PathBuf>,
    /// Export only specific decision number
    pub number: Option<String>,
    /// Generate index file
    pub generate_index: bool,
}

/// Handle `decision new` command
pub fn handle_decision_new(args: &DecisionNewArgs) -> Result<(), CliError> {
    // Parse category
    let category = parse_category(&args.category)?;

    // Load or create decisions index
    let index_path = args.workspace.join("decisions.yaml");
    let mut index = if index_path.exists() {
        let content = fs::read_to_string(&index_path)
            .map_err(|e| CliError::IoError(format!("Failed to read decisions.yaml: {}", e)))?;
        let importer = DecisionImporter;
        importer
            .import_index(&content)
            .map_err(|e| CliError::ParseError(format!("Failed to parse decisions.yaml: {}", e)))?
    } else {
        DecisionIndex::new()
    };

    // Get next number
    let number = index.next_number;
    index.next_number += 1;

    // Create the decision with placeholder content
    let mut decision = Decision::new(
        number,
        &args.title,
        "[Describe the context and problem statement here]",
        "[Describe the decision that was made]",
        "[Add author email or name]",
    );

    // Apply category and domain
    decision.category = category;
    if let Some(ref domain) = args.domain {
        decision.domain = Some(domain.clone());
    }

    // Export to YAML
    let exporter = DecisionExporter;
    let yaml_content = exporter
        .export(&decision)
        .map_err(|e| CliError::IoError(format!("Failed to export decision: {}", e)))?;

    // Generate filename - use formatted number (handles both sequential and timestamp)
    let number_str = if number >= 1000000000 {
        format!("{}", number) // Timestamp format
    } else {
        format!("{:04}", number) // Sequential format
    };
    let filename = format!("adr-{}.madr.yaml", number_str);
    let file_path = if let Some(ref domain) = args.domain {
        args.workspace.join(format!(
            "{}_{}",
            domain.to_lowercase().replace(' ', "_"),
            filename
        ))
    } else {
        args.workspace.join(&filename)
    };

    // Write YAML file
    fs::write(&file_path, &yaml_content)
        .map_err(|e| CliError::IoError(format!("Failed to write decision file: {}", e)))?;

    // Add to index
    index.add_decision(
        &decision,
        file_path.file_name().unwrap().to_str().unwrap().to_string(),
    );

    // Save index
    let index_content = exporter
        .export_index(&index)
        .map_err(|e| CliError::IoError(format!("Failed to export index: {}", e)))?;
    fs::write(&index_path, &index_content)
        .map_err(|e| CliError::IoError(format!("Failed to write decisions.yaml: {}", e)))?;

    // Optionally export as Markdown
    if args.export_markdown {
        let md_exporter = MarkdownExporter;
        let markdown = md_exporter
            .export_decision(&decision)
            .map_err(|e| CliError::IoError(format!("Failed to export Markdown: {}", e)))?;

        let decisions_dir = args.workspace.join("decisions");
        if !decisions_dir.exists() {
            fs::create_dir_all(&decisions_dir).map_err(|e| {
                CliError::IoError(format!("Failed to create decisions directory: {}", e))
            })?;
        }

        let md_filename = decision.markdown_filename();
        let md_path = decisions_dir.join(&md_filename);
        fs::write(&md_path, &markdown)
            .map_err(|e| CliError::IoError(format!("Failed to write Markdown file: {}", e)))?;

        println!(
            "Created decision {}: {}",
            decision.formatted_number(),
            args.title
        );
        println!("  YAML: {}", file_path.display());
        println!("  Markdown: {}", md_path.display());
    } else {
        println!(
            "Created decision {}: {}",
            decision.formatted_number(),
            args.title
        );
        println!("  File: {}", file_path.display());
    }

    Ok(())
}

/// Handle `decision list` command
pub fn handle_decision_list(args: &DecisionListArgs) -> Result<(), CliError> {
    // Load all decisions from workspace
    let decisions = load_all_decisions(&args.workspace)?;

    // Apply filters
    let filtered: Vec<_> = decisions
        .into_iter()
        .filter(|d| {
            if let Some(ref status) = args.status {
                if d.status.to_string().to_lowercase() != status.to_lowercase() {
                    return false;
                }
            }
            if let Some(ref category) = args.category {
                if d.category.to_string().to_lowercase() != category.to_lowercase() {
                    return false;
                }
            }
            if let Some(ref domain) = args.domain {
                if d.domain.as_ref().map(|d| d.to_lowercase()) != Some(domain.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .collect();

    // Format output
    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&filtered).map_err(|e| {
                CliError::SerializationError(format!("Failed to serialize to JSON: {}", e))
            })?;
            println!("{}", json);
        }
        "csv" => {
            println!("number,title,status,category,domain,date");
            for d in &filtered {
                println!(
                    "{},{},{},{},{},{}",
                    d.formatted_number(),
                    escape_csv(&d.title),
                    d.status,
                    d.category,
                    d.domain.as_deref().unwrap_or(""),
                    d.date.format("%Y-%m-%d")
                );
            }
        }
        _ => {
            // Table format
            if filtered.is_empty() {
                println!("No decisions found.");
            } else {
                println!(
                    "{:<10} {:<40} {:<12} {:<15} {:<15}",
                    "Number", "Title", "Status", "Category", "Domain"
                );
                println!("{}", "-".repeat(92));
                for d in &filtered {
                    let title = if d.title.len() > 38 {
                        format!("{}...", &d.title[..35])
                    } else {
                        d.title.clone()
                    };
                    println!(
                        "{:<14} {:<40} {:<12} {:<15} {:<15}",
                        d.formatted_number(),
                        title,
                        d.status,
                        d.category,
                        d.domain.as_deref().unwrap_or("-")
                    );
                }
                println!("\n{} decision(s) found.", filtered.len());
            }
        }
    }

    Ok(())
}

/// Handle `decision show` command
pub fn handle_decision_show(args: &DecisionShowArgs) -> Result<(), CliError> {
    let number = parse_decision_number(&args.number)?;
    let decision = find_decision_by_number(&args.workspace, number)?;

    match args.format.as_str() {
        "yaml" => {
            let exporter = DecisionExporter;
            let yaml = exporter
                .export(&decision)
                .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;
            println!("{}", yaml);
        }
        "markdown" | "md" => {
            let exporter = MarkdownExporter;
            let markdown = exporter
                .export_decision(&decision)
                .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;
            println!("{}", markdown);
        }
        "json" => {
            let json = serde_json::to_string_pretty(&decision)
                .map_err(|e| CliError::SerializationError(format!("Failed to serialize: {}", e)))?;
            println!("{}", json);
        }
        _ => {
            return Err(CliError::InvalidArgument(format!(
                "Unknown format: {}. Use yaml, markdown, or json",
                args.format
            )));
        }
    }

    Ok(())
}

/// Handle `decision status` command
pub fn handle_decision_status(args: &DecisionStatusArgs) -> Result<(), CliError> {
    let number = parse_decision_number(&args.number)?;
    let new_status = parse_status(&args.status)?;

    // Find and load the decision
    let (file_path, mut decision) = find_decision_file_and_load(&args.workspace, number)?;

    let old_status = decision.status.clone();
    decision.status = new_status.clone();
    decision.updated_at = Utc::now();

    // Save the updated decision
    let exporter = DecisionExporter;
    let yaml = exporter
        .export(&decision)
        .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;
    fs::write(&file_path, &yaml)
        .map_err(|e| CliError::IoError(format!("Failed to write file: {}", e)))?;

    // Update index if it exists
    let index_path = args.workspace.join("decisions.yaml");
    if index_path.exists() {
        update_decision_in_index(&index_path, &decision)?;
    }

    println!(
        "Updated {} status: {} -> {}",
        decision.formatted_number(),
        old_status,
        new_status
    );

    Ok(())
}

/// Handle `decision export` command
pub fn handle_decision_export(args: &DecisionExportArgs) -> Result<(), CliError> {
    let output_dir = args
        .output
        .clone()
        .unwrap_or_else(|| args.workspace.join("decisions"));

    // Create output directory
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| CliError::IoError(format!("Failed to create output directory: {}", e)))?;
    }

    let md_exporter = MarkdownExporter;

    if let Some(ref number_str) = args.number {
        // Export single decision
        let number = parse_decision_number(number_str)?;
        let decision = find_decision_by_number(&args.workspace, number)?;

        let markdown = md_exporter
            .export_decision(&decision)
            .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;

        let filename = decision.markdown_filename();
        let file_path = output_dir.join(&filename);
        fs::write(&file_path, &markdown)
            .map_err(|e| CliError::IoError(format!("Failed to write file: {}", e)))?;

        println!("Exported: {}", file_path.display());
    } else {
        // Export all decisions
        let decisions = load_all_decisions(&args.workspace)?;
        let mut count = 0;

        for decision in &decisions {
            let markdown = md_exporter.export_decision(decision).map_err(|e| {
                CliError::IoError(format!(
                    "Failed to export {}: {}",
                    decision.formatted_number(),
                    e
                ))
            })?;

            let filename = decision.markdown_filename();
            let file_path = output_dir.join(&filename);
            fs::write(&file_path, &markdown)
                .map_err(|e| CliError::IoError(format!("Failed to write file: {}", e)))?;
            count += 1;
        }

        // Generate index if requested
        if args.generate_index {
            let index_content = md_exporter.generate_decisions_index(&decisions);
            let index_path = output_dir.join("README.md");
            fs::write(&index_path, &index_content)
                .map_err(|e| CliError::IoError(format!("Failed to write index: {}", e)))?;
            println!("Generated index: {}", index_path.display());
        }

        println!("Exported {} decision(s) to {}", count, output_dir.display());
    }

    Ok(())
}

// ==================== Helper Functions ====================

fn parse_category(s: &str) -> Result<DecisionCategory, CliError> {
    match s.to_lowercase().as_str() {
        "architecture" => Ok(DecisionCategory::Architecture),
        "datadesign" | "data-design" | "data_design" => Ok(DecisionCategory::DataDesign),
        "workflow" => Ok(DecisionCategory::Workflow),
        "model" => Ok(DecisionCategory::Model),
        "governance" => Ok(DecisionCategory::Governance),
        "security" => Ok(DecisionCategory::Security),
        "performance" => Ok(DecisionCategory::Performance),
        "compliance" => Ok(DecisionCategory::Compliance),
        "infrastructure" => Ok(DecisionCategory::Infrastructure),
        "tooling" => Ok(DecisionCategory::Tooling),
        "data" => Ok(DecisionCategory::Data),
        "integration" => Ok(DecisionCategory::Integration),
        _ => Err(CliError::InvalidArgument(format!(
            "Unknown category: {}. Valid categories: architecture, datadesign, workflow, model, governance, security, performance, compliance, infrastructure, tooling, data, integration",
            s
        ))),
    }
}

fn parse_status(s: &str) -> Result<DecisionStatus, CliError> {
    match s.to_lowercase().as_str() {
        "proposed" => Ok(DecisionStatus::Proposed),
        "accepted" => Ok(DecisionStatus::Accepted),
        "rejected" => Ok(DecisionStatus::Rejected),
        "deprecated" => Ok(DecisionStatus::Deprecated),
        "superseded" => Ok(DecisionStatus::Superseded),
        _ => Err(CliError::InvalidArgument(format!(
            "Unknown status: {}. Valid statuses: proposed, accepted, rejected, deprecated, superseded",
            s
        ))),
    }
}

fn parse_decision_number(s: &str) -> Result<u64, CliError> {
    // Handle "ADR-0001", "ADR-1", "0001", "1", or timestamp format "2601101234"
    let num_str = s
        .to_uppercase()
        .strip_prefix("ADR-")
        .map(|s| s.to_string())
        .unwrap_or_else(|| s.to_string());

    num_str
        .parse::<u64>()
        .map_err(|_| CliError::InvalidArgument(format!("Invalid decision number: {}", s)))
}

fn load_all_decisions(workspace: &Path) -> Result<Vec<Decision>, CliError> {
    let importer = DecisionImporter;
    let mut decisions = Vec::new();

    for entry in fs::read_dir(workspace)
        .map_err(|e| CliError::IoError(format!("Failed to read workspace: {}", e)))?
    {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".madr.yaml") {
                    let content = fs::read_to_string(&path).map_err(|e| {
                        CliError::IoError(format!("Failed to read {}: {}", path.display(), e))
                    })?;

                    match importer.import(&content) {
                        Ok(decision) => decisions.push(decision),
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    // Sort by number
    decisions.sort_by_key(|d| d.number);
    Ok(decisions)
}

fn find_decision_by_number(workspace: &Path, number: u64) -> Result<Decision, CliError> {
    let decisions = load_all_decisions(workspace)?;
    decisions
        .into_iter()
        .find(|d| d.number == number)
        .ok_or_else(|| CliError::NotFound(format!("Decision ADR-{} not found", number)))
}

fn find_decision_file_and_load(
    workspace: &Path,
    number: u64,
) -> Result<(PathBuf, Decision), CliError> {
    let importer = DecisionImporter;

    for entry in fs::read_dir(workspace)
        .map_err(|e| CliError::IoError(format!("Failed to read workspace: {}", e)))?
    {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".madr.yaml") {
                    let content = fs::read_to_string(&path).map_err(|e| {
                        CliError::IoError(format!("Failed to read {}: {}", path.display(), e))
                    })?;

                    if let Ok(decision) = importer.import(&content) {
                        if decision.number == number {
                            return Ok((path, decision));
                        }
                    }
                }
            }
        }
    }

    let formatted = if number >= 1000000000 {
        format!("ADR-{}", number)
    } else {
        format!("ADR-{:04}", number)
    };
    Err(CliError::NotFound(format!(
        "Decision {} not found",
        formatted
    )))
}

fn update_decision_in_index(index_path: &Path, decision: &Decision) -> Result<(), CliError> {
    let content = fs::read_to_string(index_path)
        .map_err(|e| CliError::IoError(format!("Failed to read index: {}", e)))?;

    let importer = DecisionImporter;
    let mut index = importer
        .import_index(&content)
        .map_err(|e| CliError::ParseError(format!("Failed to parse index: {}", e)))?;

    // Update the entry
    for entry in &mut index.decisions {
        if entry.number == decision.number {
            entry.status = decision.status.clone();
            entry.title = decision.title.clone();
            break;
        }
    }

    let exporter = DecisionExporter;
    let new_content = exporter
        .export_index(&index)
        .map_err(|e| CliError::IoError(format!("Failed to export index: {}", e)))?;

    fs::write(index_path, &new_content)
        .map_err(|e| CliError::IoError(format!("Failed to write index: {}", e)))?;

    Ok(())
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_category() {
        assert!(parse_category("architecture").is_ok());
        assert!(parse_category("DATADESIGN").is_ok());
        assert!(parse_category("data-design").is_ok());
        assert!(parse_category("invalid").is_err());
    }

    #[test]
    fn test_parse_status() {
        assert!(parse_status("proposed").is_ok());
        assert!(parse_status("ACCEPTED").is_ok());
        assert!(parse_status("invalid").is_err());
    }

    #[test]
    fn test_parse_decision_number() {
        assert_eq!(parse_decision_number("1").unwrap(), 1);
        assert_eq!(parse_decision_number("0001").unwrap(), 1);
        assert_eq!(parse_decision_number("ADR-0001").unwrap(), 1);
        assert_eq!(parse_decision_number("adr-42").unwrap(), 42);
        assert!(parse_decision_number("invalid").is_err());
    }

    #[test]
    fn test_escape_csv() {
        assert_eq!(escape_csv("simple"), "simple");
        assert_eq!(escape_csv("has,comma"), "\"has,comma\"");
        assert_eq!(escape_csv("has\"quote"), "\"has\"\"quote\"");
    }
}
