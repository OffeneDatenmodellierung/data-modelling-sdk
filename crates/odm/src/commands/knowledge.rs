//! Knowledge Base CLI commands
//!
//! Provides CLI commands for managing knowledge base articles.
//!
//! Note: These commands are defined but not yet wired into the main CLI.
//! They will be enabled in a future release.

#![allow(clippy::collapsible_if)]
#![allow(dead_code)]

use crate::error::CliError;
use chrono::Utc;
use data_modelling_core::export::knowledge::KnowledgeExporter;
use data_modelling_core::export::markdown::MarkdownExporter;
use data_modelling_core::import::knowledge::KnowledgeImporter;
use data_modelling_core::models::knowledge::{
    KnowledgeArticle, KnowledgeIndex, KnowledgeStatus, KnowledgeType,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Arguments for the `knowledge new` command
#[derive(Debug)]
pub struct KnowledgeNewArgs {
    /// Article title
    pub title: String,
    /// Article type
    pub article_type: String,
    /// Domain this article belongs to (optional)
    pub domain: Option<String>,
    /// Author name
    pub author: String,
    /// Author email (optional)
    pub author_email: Option<String>,
    /// Workspace path
    pub workspace: PathBuf,
    /// Also export as Markdown
    pub export_markdown: bool,
}

/// Arguments for the `knowledge list` command
#[derive(Debug)]
pub struct KnowledgeListArgs {
    /// Workspace path
    pub workspace: PathBuf,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by type
    pub article_type: Option<String>,
    /// Filter by domain
    pub domain: Option<String>,
    /// Filter by author
    pub author: Option<String>,
    /// Output format (table, json, csv)
    pub format: String,
}

/// Arguments for the `knowledge show` command
#[derive(Debug)]
pub struct KnowledgeShowArgs {
    /// Article number (e.g., "KB-0001" or just "1")
    pub number: String,
    /// Workspace path
    pub workspace: PathBuf,
    /// Output format (yaml, markdown, json)
    pub format: String,
}

/// Arguments for the `knowledge status` command
#[derive(Debug)]
pub struct KnowledgeStatusArgs {
    /// Article number
    pub number: String,
    /// New status
    pub status: String,
    /// Workspace path
    pub workspace: PathBuf,
}

/// Arguments for the `knowledge export` command
#[derive(Debug)]
pub struct KnowledgeExportArgs {
    /// Workspace path
    pub workspace: PathBuf,
    /// Output directory for Markdown files
    pub output: Option<PathBuf>,
    /// Export only specific article number
    pub number: Option<String>,
    /// Generate index file
    pub generate_index: bool,
    /// Organize by domain
    pub by_domain: bool,
}

/// Arguments for the `knowledge search` command
#[derive(Debug)]
pub struct KnowledgeSearchArgs {
    /// Search query
    pub query: String,
    /// Workspace path
    pub workspace: PathBuf,
    /// Output format (table, json)
    pub format: String,
}

/// Handle `knowledge new` command
pub fn handle_knowledge_new(args: &KnowledgeNewArgs) -> Result<(), CliError> {
    // Parse article type
    let article_type = parse_article_type(&args.article_type)?;

    // Load or create knowledge index
    let index_path = args.workspace.join("knowledge.yaml");
    let mut index = if index_path.exists() {
        let content = fs::read_to_string(&index_path)
            .map_err(|e| CliError::IoError(format!("Failed to read knowledge.yaml: {}", e)))?;
        let importer = KnowledgeImporter;
        importer
            .import_index(&content)
            .map_err(|e| CliError::ParseError(format!("Failed to parse knowledge.yaml: {}", e)))?
    } else {
        KnowledgeIndex::new()
    };

    // Get next number
    let number = index.next_number;
    index.next_number += 1;

    // Create the article with placeholder content
    let mut article = KnowledgeArticle::new(
        number,
        &args.title,
        "[Brief summary of this article]",
        "# Content\n\n[Write your article content here in Markdown format]",
        &args.author,
    );

    // Apply type and optional fields
    article.article_type = article_type;
    if let Some(ref domain) = args.domain {
        article.domain = Some(domain.clone());
    }

    // Export to YAML
    let exporter = KnowledgeExporter;
    let yaml_content = exporter
        .export(&article)
        .map_err(|e| CliError::IoError(format!("Failed to export article: {}", e)))?;

    // Generate filename - use formatted number (handles both sequential and timestamp)
    let number_str = if number >= 1000000000 {
        format!("{}", number) // Timestamp format
    } else {
        format!("{:04}", number) // Sequential format
    };
    let filename = format!("kb-{}.kb.yaml", number_str);
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
        .map_err(|e| CliError::IoError(format!("Failed to write article file: {}", e)))?;

    // Add to index
    index.add_article(
        &article,
        file_path.file_name().unwrap().to_str().unwrap().to_string(),
    );

    // Save index
    let index_content = exporter
        .export_index(&index)
        .map_err(|e| CliError::IoError(format!("Failed to export index: {}", e)))?;
    fs::write(&index_path, &index_content)
        .map_err(|e| CliError::IoError(format!("Failed to write knowledge.yaml: {}", e)))?;

    // Optionally export as Markdown
    if args.export_markdown {
        let md_exporter = MarkdownExporter;
        let markdown = md_exporter
            .export_knowledge(&article)
            .map_err(|e| CliError::IoError(format!("Failed to export Markdown: {}", e)))?;

        let knowledge_dir = args.workspace.join("knowledge");
        if !knowledge_dir.exists() {
            fs::create_dir_all(&knowledge_dir).map_err(|e| {
                CliError::IoError(format!("Failed to create knowledge directory: {}", e))
            })?;
        }

        let md_filename = article.markdown_filename();
        let md_path = knowledge_dir.join(&md_filename);
        fs::write(&md_path, &markdown)
            .map_err(|e| CliError::IoError(format!("Failed to write Markdown file: {}", e)))?;

        println!(
            "Created article {}: {}",
            article.formatted_number(),
            args.title
        );
        println!("  YAML: {}", file_path.display());
        println!("  Markdown: {}", md_path.display());
    } else {
        println!(
            "Created article {}: {}",
            article.formatted_number(),
            args.title
        );
        println!("  File: {}", file_path.display());
    }

    Ok(())
}

/// Handle `knowledge list` command
pub fn handle_knowledge_list(args: &KnowledgeListArgs) -> Result<(), CliError> {
    // Load all articles from workspace
    let articles = load_all_articles(&args.workspace)?;

    // Apply filters
    let filtered: Vec<_> = articles
        .into_iter()
        .filter(|a| {
            if let Some(ref status) = args.status {
                if a.status.to_string().to_lowercase() != status.to_lowercase() {
                    return false;
                }
            }
            if let Some(ref article_type) = args.article_type {
                if a.article_type.to_string().to_lowercase() != article_type.to_lowercase() {
                    return false;
                }
            }
            if let Some(ref domain) = args.domain {
                if a.domain.as_ref().map(|d| d.to_lowercase()) != Some(domain.to_lowercase()) {
                    return false;
                }
            }
            if let Some(ref author) = args.author {
                let authors_str = a.authors.join(", ").to_lowercase();
                if !authors_str.contains(&author.to_lowercase()) {
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
            println!("number,title,type,status,domain,authors");
            for a in &filtered {
                println!(
                    "{},{},{},{},{},{}",
                    a.formatted_number(),
                    escape_csv(&a.title),
                    a.article_type,
                    a.status,
                    a.domain.as_deref().unwrap_or(""),
                    escape_csv(&a.authors.join(", "))
                );
            }
        }
        _ => {
            // Table format
            if filtered.is_empty() {
                println!("No articles found.");
            } else {
                println!(
                    "{:<10} {:<35} {:<15} {:<12} {:<15}",
                    "Number", "Title", "Type", "Status", "Domain"
                );
                println!("{}", "-".repeat(87));
                for a in &filtered {
                    let title = if a.title.len() > 33 {
                        format!("{}...", &a.title[..30])
                    } else {
                        a.title.clone()
                    };
                    println!(
                        "{:<14} {:<35} {:<15} {:<12} {:<15}",
                        a.formatted_number(),
                        title,
                        a.article_type,
                        a.status,
                        a.domain.as_deref().unwrap_or("-")
                    );
                }
                println!("\n{} article(s) found.", filtered.len());
            }
        }
    }

    Ok(())
}

/// Handle `knowledge show` command
pub fn handle_knowledge_show(args: &KnowledgeShowArgs) -> Result<(), CliError> {
    let number = parse_article_number(&args.number)?;
    let article = find_article_by_number(&args.workspace, number)?;

    match args.format.as_str() {
        "yaml" => {
            let exporter = KnowledgeExporter;
            let yaml = exporter
                .export(&article)
                .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;
            println!("{}", yaml);
        }
        "markdown" | "md" => {
            let exporter = MarkdownExporter;
            let markdown = exporter
                .export_knowledge(&article)
                .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;
            println!("{}", markdown);
        }
        "json" => {
            let json = serde_json::to_string_pretty(&article)
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

/// Handle `knowledge status` command
pub fn handle_knowledge_status(args: &KnowledgeStatusArgs) -> Result<(), CliError> {
    let number = parse_article_number(&args.number)?;
    let new_status = parse_status(&args.status)?;

    // Find and load the article
    let (file_path, mut article) = find_article_file_and_load(&args.workspace, number)?;

    let old_status = article.status.clone();
    article.status = new_status.clone();
    article.updated_at = Utc::now();

    // Save the updated article
    let exporter = KnowledgeExporter;
    let yaml = exporter
        .export(&article)
        .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;
    fs::write(&file_path, &yaml)
        .map_err(|e| CliError::IoError(format!("Failed to write file: {}", e)))?;

    // Update index if it exists
    let index_path = args.workspace.join("knowledge.yaml");
    if index_path.exists() {
        update_article_in_index(&index_path, &article)?;
    }

    println!(
        "Updated {} status: {} -> {}",
        article.number, old_status, new_status
    );

    Ok(())
}

/// Handle `knowledge export` command
pub fn handle_knowledge_export(args: &KnowledgeExportArgs) -> Result<(), CliError> {
    let output_dir = args
        .output
        .clone()
        .unwrap_or_else(|| args.workspace.join("knowledge"));

    // Create output directory
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| CliError::IoError(format!("Failed to create output directory: {}", e)))?;
    }

    let md_exporter = MarkdownExporter;

    if let Some(ref number_str) = args.number {
        // Export single article
        let number = parse_article_number(number_str)?;
        let article = find_article_by_number(&args.workspace, number)?;

        let markdown = md_exporter
            .export_knowledge(&article)
            .map_err(|e| CliError::IoError(format!("Failed to export: {}", e)))?;

        let filename = article.markdown_filename();
        let file_path = output_dir.join(&filename);
        fs::write(&file_path, &markdown)
            .map_err(|e| CliError::IoError(format!("Failed to write file: {}", e)))?;

        println!("Exported: {}", file_path.display());
    } else {
        // Export all articles
        let articles = load_all_articles(&args.workspace)?;
        let mut count = 0;

        if args.by_domain {
            // Organize by domain
            for article in &articles {
                let domain_dir = if let Some(ref domain) = article.domain {
                    output_dir.join(domain.to_lowercase().replace(' ', "_"))
                } else {
                    output_dir.join("_general")
                };

                if !domain_dir.exists() {
                    fs::create_dir_all(&domain_dir).map_err(|e| {
                        CliError::IoError(format!("Failed to create domain directory: {}", e))
                    })?;
                }

                let markdown = md_exporter.export_knowledge(article).map_err(|e| {
                    CliError::IoError(format!("Failed to export {}: {}", article.number, e))
                })?;

                let filename = article.markdown_filename();
                let file_path = domain_dir.join(&filename);
                fs::write(&file_path, &markdown)
                    .map_err(|e| CliError::IoError(format!("Failed to write file: {}", e)))?;
                count += 1;
            }
        } else {
            // Flat structure
            for article in &articles {
                let markdown = md_exporter.export_knowledge(article).map_err(|e| {
                    CliError::IoError(format!("Failed to export {}: {}", article.number, e))
                })?;

                let filename = article.markdown_filename();
                let file_path = output_dir.join(&filename);
                fs::write(&file_path, &markdown)
                    .map_err(|e| CliError::IoError(format!("Failed to write file: {}", e)))?;
                count += 1;
            }
        }

        // Generate index if requested
        if args.generate_index {
            let index_content = md_exporter.generate_knowledge_index(&articles);
            let index_path = output_dir.join("README.md");
            fs::write(&index_path, &index_content)
                .map_err(|e| CliError::IoError(format!("Failed to write index: {}", e)))?;
            println!("Generated index: {}", index_path.display());
        }

        println!("Exported {} article(s) to {}", count, output_dir.display());
    }

    Ok(())
}

/// Handle `knowledge search` command
pub fn handle_knowledge_search(args: &KnowledgeSearchArgs) -> Result<(), CliError> {
    let articles = load_all_articles(&args.workspace)?;
    let query_lower = args.query.to_lowercase();

    // Search in title, summary, and content
    let matches: Vec<_> = articles
        .into_iter()
        .filter(|a| {
            a.title.to_lowercase().contains(&query_lower)
                || a.summary.to_lowercase().contains(&query_lower)
                || a.content.to_lowercase().contains(&query_lower)
                || a.tags
                    .iter()
                    .any(|t| t.to_string().to_lowercase().contains(&query_lower))
        })
        .collect();

    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&matches)
                .map_err(|e| CliError::SerializationError(format!("Failed to serialize: {}", e)))?;
            println!("{}", json);
        }
        _ => {
            if matches.is_empty() {
                println!("No articles matching '{}' found.", args.query);
            } else {
                println!(
                    "Found {} article(s) matching '{}':\n",
                    matches.len(),
                    args.query
                );
                for a in &matches {
                    println!("{}: {}", a.formatted_number(), a.title);
                    println!(
                        "  Type: {} | Status: {} | Authors: {}",
                        a.article_type,
                        a.status,
                        a.authors.join(", ")
                    );
                    // Show snippet of summary
                    let summary = if a.summary.len() > 80 {
                        format!("{}...", &a.summary[..77])
                    } else {
                        a.summary.clone()
                    };
                    println!("  {}", summary);
                    println!();
                }
            }
        }
    }

    Ok(())
}

// ==================== Helper Functions ====================

fn parse_article_type(s: &str) -> Result<KnowledgeType, CliError> {
    match s.to_lowercase().as_str() {
        "guide" => Ok(KnowledgeType::Guide),
        "standard" => Ok(KnowledgeType::Standard),
        "reference" => Ok(KnowledgeType::Reference),
        "howto" | "how-to" | "how_to" => Ok(KnowledgeType::HowTo),
        "troubleshooting" => Ok(KnowledgeType::Troubleshooting),
        "policy" => Ok(KnowledgeType::Policy),
        "template" => Ok(KnowledgeType::Template),
        "concept" => Ok(KnowledgeType::Concept),
        "runbook" => Ok(KnowledgeType::Runbook),
        _ => Err(CliError::InvalidArgument(format!(
            "Unknown article type: {}. Valid types: guide, standard, reference, howto, troubleshooting, policy, template, concept, runbook",
            s
        ))),
    }
}

fn parse_status(s: &str) -> Result<KnowledgeStatus, CliError> {
    match s.to_lowercase().as_str() {
        "draft" => Ok(KnowledgeStatus::Draft),
        "review" => Ok(KnowledgeStatus::Review),
        "published" => Ok(KnowledgeStatus::Published),
        "archived" => Ok(KnowledgeStatus::Archived),
        "deprecated" => Ok(KnowledgeStatus::Deprecated),
        _ => Err(CliError::InvalidArgument(format!(
            "Unknown status: {}. Valid statuses: draft, review, published, archived, deprecated",
            s
        ))),
    }
}

fn parse_article_number(s: &str) -> Result<u64, CliError> {
    // Handle "KB-0001", "KB-1", "0001", "1", or timestamp format "2601101234"
    let num_str = s
        .to_uppercase()
        .strip_prefix("KB-")
        .map(|s| s.to_string())
        .unwrap_or_else(|| s.to_string());

    num_str
        .parse::<u64>()
        .map_err(|_| CliError::InvalidArgument(format!("Invalid article number: {}", s)))
}

fn load_all_articles(workspace: &Path) -> Result<Vec<KnowledgeArticle>, CliError> {
    let importer = KnowledgeImporter;
    let mut articles = Vec::new();

    for entry in fs::read_dir(workspace)
        .map_err(|e| CliError::IoError(format!("Failed to read workspace: {}", e)))?
    {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".kb.yaml") {
                    let content = fs::read_to_string(&path).map_err(|e| {
                        CliError::IoError(format!("Failed to read {}: {}", path.display(), e))
                    })?;

                    match importer.import(&content) {
                        Ok(article) => articles.push(article),
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    // Sort by number
    articles.sort_by(|a, b| a.number.cmp(&b.number));
    Ok(articles)
}

fn find_article_by_number(workspace: &Path, number: u64) -> Result<KnowledgeArticle, CliError> {
    let articles = load_all_articles(workspace)?;
    articles
        .into_iter()
        .find(|a| a.number == number)
        .ok_or_else(|| CliError::NotFound(format!("Article KB-{} not found", number)))
}

fn find_article_file_and_load(
    workspace: &Path,
    number: u64,
) -> Result<(PathBuf, KnowledgeArticle), CliError> {
    let importer = KnowledgeImporter;

    for entry in fs::read_dir(workspace)
        .map_err(|e| CliError::IoError(format!("Failed to read workspace: {}", e)))?
    {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".kb.yaml") {
                    let content = fs::read_to_string(&path).map_err(|e| {
                        CliError::IoError(format!("Failed to read {}: {}", path.display(), e))
                    })?;

                    if let Ok(article) = importer.import(&content) {
                        if article.number == number {
                            return Ok((path, article));
                        }
                    }
                }
            }
        }
    }

    Err(CliError::NotFound(format!(
        "Article KB-{} not found",
        number
    )))
}

fn update_article_in_index(index_path: &Path, article: &KnowledgeArticle) -> Result<(), CliError> {
    let content = fs::read_to_string(index_path)
        .map_err(|e| CliError::IoError(format!("Failed to read index: {}", e)))?;

    let importer = KnowledgeImporter;
    let mut index = importer
        .import_index(&content)
        .map_err(|e| CliError::ParseError(format!("Failed to parse index: {}", e)))?;

    // Update the entry
    for entry in &mut index.articles {
        if entry.number == article.number {
            entry.status = article.status.clone();
            entry.title = article.title.clone();
            break;
        }
    }

    let exporter = KnowledgeExporter;
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
    fn test_parse_article_type() {
        assert!(parse_article_type("guide").is_ok());
        assert!(parse_article_type("HOWTO").is_ok());
        assert!(parse_article_type("how-to").is_ok());
        assert!(parse_article_type("invalid").is_err());
    }

    #[test]
    fn test_parse_status() {
        assert!(parse_status("draft").is_ok());
        assert!(parse_status("PUBLISHED").is_ok());
        assert!(parse_status("invalid").is_err());
    }

    #[test]
    fn test_parse_article_number() {
        assert_eq!(parse_article_number("1").unwrap(), 1);
        assert_eq!(parse_article_number("0001").unwrap(), 1);
        assert_eq!(parse_article_number("KB-0001").unwrap(), 1);
        assert_eq!(parse_article_number("kb-42").unwrap(), 42);
        assert!(parse_article_number("invalid").is_err());
    }
}
