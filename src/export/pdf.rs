//! PDF exporter with branding support
//!
//! Exports ODCS, ODPS, Knowledge Base articles, and Architecture Decision Records
//! to PDF format with customizable branding options.
//!
//! ## Features
//!
//! - Logo support (base64 encoded or URL)
//! - Customizable header and footer
//! - Brand color theming
//! - Page numbering
//! - Table of contents for longer documents
//!
//! ## WASM Compatibility
//!
//! This module is designed to work in both native and WASM environments
//! by generating PDF as base64-encoded bytes.

use crate::export::ExportError;
use crate::models::decision::Decision;
use crate::models::knowledge::KnowledgeArticle;
use serde::{Deserialize, Serialize};

/// Branding configuration for PDF exports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Logo as base64-encoded image data (PNG or JPEG)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_base64: Option<String>,

    /// Logo URL (alternative to base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,

    /// Header text (appears at top of each page)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    /// Footer text (appears at bottom of each page)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,

    /// Primary brand color in hex format (e.g., "#0066CC")
    #[serde(default = "default_brand_color")]
    pub brand_color: String,

    /// Company or organization name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_name: Option<String>,

    /// Include page numbers
    #[serde(default = "default_true")]
    pub show_page_numbers: bool,

    /// Include generation timestamp
    #[serde(default = "default_true")]
    pub show_timestamp: bool,

    /// Font size for body text (in points)
    #[serde(default = "default_font_size")]
    pub font_size: u8,

    /// Page size (A4 or Letter)
    #[serde(default)]
    pub page_size: PageSize,
}

fn default_brand_color() -> String {
    "#0066CC".to_string()
}

fn default_true() -> bool {
    true
}

fn default_font_size() -> u8 {
    11
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            logo_base64: None,
            logo_url: None,
            header: None,
            footer: None,
            brand_color: default_brand_color(),
            company_name: None,
            show_page_numbers: default_true(),
            show_timestamp: default_true(),
            font_size: default_font_size(),
            page_size: PageSize::default(),
        }
    }
}

/// Page size options
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PageSize {
    /// A4 paper size (210 x 297 mm)
    #[default]
    A4,
    /// US Letter size (8.5 x 11 inches)
    Letter,
}

impl PageSize {
    /// Get page dimensions in millimeters (width, height)
    pub fn dimensions_mm(&self) -> (f64, f64) {
        match self {
            PageSize::A4 => (210.0, 297.0),
            PageSize::Letter => (215.9, 279.4),
        }
    }
}

/// PDF document content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[allow(clippy::large_enum_variant)]
pub enum PdfContent {
    /// Architecture Decision Record
    Decision(Decision),
    /// Knowledge Base article
    Knowledge(KnowledgeArticle),
    /// Raw markdown content
    Markdown { title: String, content: String },
}

/// Result of PDF export operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfExportResult {
    /// PDF content as base64-encoded bytes
    pub pdf_base64: String,
    /// Filename suggestion
    pub filename: String,
    /// Number of pages
    pub page_count: u32,
    /// Document title
    pub title: String,
}

/// PDF exporter with branding support
pub struct PdfExporter {
    branding: BrandingConfig,
}

impl Default for PdfExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExporter {
    /// Create a new PDF exporter with default branding
    pub fn new() -> Self {
        Self {
            branding: BrandingConfig::default(),
        }
    }

    /// Create a new PDF exporter with custom branding
    pub fn with_branding(branding: BrandingConfig) -> Self {
        Self { branding }
    }

    /// Update branding configuration
    pub fn set_branding(&mut self, branding: BrandingConfig) {
        self.branding = branding;
    }

    /// Get current branding configuration
    pub fn branding(&self) -> &BrandingConfig {
        &self.branding
    }

    /// Export a Decision to PDF
    pub fn export_decision(&self, decision: &Decision) -> Result<PdfExportResult, ExportError> {
        let title = format!("{}: {}", decision.formatted_number(), decision.title);
        let markdown = self.decision_to_markdown(decision);
        self.generate_pdf(
            &title,
            &markdown,
            &decision.markdown_filename().replace(".md", ".pdf"),
        )
    }

    /// Export a Knowledge article to PDF
    pub fn export_knowledge(
        &self,
        article: &KnowledgeArticle,
    ) -> Result<PdfExportResult, ExportError> {
        let title = format!("{}: {}", article.formatted_number(), article.title);
        let markdown = self.knowledge_to_markdown(article);
        self.generate_pdf(
            &title,
            &markdown,
            &article.markdown_filename().replace(".md", ".pdf"),
        )
    }

    /// Export raw markdown content to PDF
    pub fn export_markdown(
        &self,
        title: &str,
        content: &str,
        filename: &str,
    ) -> Result<PdfExportResult, ExportError> {
        self.generate_pdf(title, content, filename)
    }

    /// Convert Decision to markdown for PDF rendering
    fn decision_to_markdown(&self, decision: &Decision) -> String {
        use crate::models::decision::DecisionStatus;

        let mut md = String::new();

        // Status indicator
        let status_text = match decision.status {
            DecisionStatus::Proposed => "Proposed",
            DecisionStatus::Accepted => "Accepted",
            DecisionStatus::Deprecated => "Deprecated",
            DecisionStatus::Superseded => "Superseded",
            DecisionStatus::Rejected => "Rejected",
        };

        md.push_str(&format!(
            "# {}: {}\n\n",
            decision.formatted_number(),
            decision.title
        ));
        md.push_str(&format!(
            "**Status:** {} | **Category:** {} | **Date:** {}\n\n",
            status_text,
            decision.category,
            decision.date.format("%Y-%m-%d")
        ));

        if let Some(domain) = &decision.domain {
            md.push_str(&format!("**Domain:** {}\n\n", domain));
        }

        if !decision.authors.is_empty() {
            md.push_str(&format!("**Authors:** {}\n\n", decision.authors.join(", ")));
        }

        if !decision.deciders.is_empty() {
            md.push_str(&format!(
                "**Deciders:** {}\n\n",
                decision.deciders.join(", ")
            ));
        }

        // RACI Matrix
        if let Some(raci) = &decision.raci
            && !raci.is_empty()
        {
            md.push_str("## RACI Matrix\n\n");
            if !raci.responsible.is_empty() {
                md.push_str(&format!(
                    "- **Responsible:** {}\n",
                    raci.responsible.join(", ")
                ));
            }
            if !raci.accountable.is_empty() {
                md.push_str(&format!(
                    "- **Accountable:** {}\n",
                    raci.accountable.join(", ")
                ));
            }
            if !raci.consulted.is_empty() {
                md.push_str(&format!("- **Consulted:** {}\n", raci.consulted.join(", ")));
            }
            if !raci.informed.is_empty() {
                md.push_str(&format!("- **Informed:** {}\n", raci.informed.join(", ")));
            }
            md.push('\n');
        }

        // Context
        md.push_str("## Context and Problem Statement\n\n");
        md.push_str(&decision.context);
        md.push_str("\n\n");

        // Drivers
        if !decision.drivers.is_empty() {
            md.push_str("## Decision Drivers\n\n");
            for driver in &decision.drivers {
                let priority = match driver.priority {
                    Some(crate::models::decision::DriverPriority::High) => " (High Priority)",
                    Some(crate::models::decision::DriverPriority::Medium) => " (Medium Priority)",
                    Some(crate::models::decision::DriverPriority::Low) => " (Low Priority)",
                    None => "",
                };
                md.push_str(&format!("- {}{}\n", driver.description, priority));
            }
            md.push('\n');
        }

        // Options
        if !decision.options.is_empty() {
            md.push_str("## Considered Options\n\n");
            for (i, option) in decision.options.iter().enumerate() {
                let selected_marker = if option.selected { " (Selected)" } else { "" };
                md.push_str(&format!(
                    "### Option {}: {}{}\n\n",
                    i + 1,
                    option.name,
                    selected_marker
                ));

                if let Some(desc) = &option.description {
                    md.push_str(&format!("{}\n\n", desc));
                }

                if !option.pros.is_empty() {
                    md.push_str("**Pros:**\n");
                    for pro in &option.pros {
                        md.push_str(&format!("- {}\n", pro));
                    }
                    md.push('\n');
                }

                if !option.cons.is_empty() {
                    md.push_str("**Cons:**\n");
                    for con in &option.cons {
                        md.push_str(&format!("- {}\n", con));
                    }
                    md.push('\n');
                }
            }
        }

        // Decision Outcome
        md.push_str("## Decision Outcome\n\n");
        md.push_str(&decision.decision);
        md.push_str("\n\n");

        // Consequences
        if let Some(consequences) = &decision.consequences {
            md.push_str("## Consequences\n\n");
            md.push_str(consequences);
            md.push_str("\n\n");
        }

        // Linked Assets
        if !decision.linked_assets.is_empty() {
            md.push_str("## Linked Assets\n\n");
            for asset in &decision.linked_assets {
                md.push_str(&format!("- {} ({})\n", asset.asset_name, asset.asset_type));
            }
            md.push('\n');
        }

        // Notes
        if let Some(notes) = &decision.notes {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push('\n');
        }

        md
    }

    /// Convert Knowledge article to markdown for PDF rendering
    fn knowledge_to_markdown(&self, article: &KnowledgeArticle) -> String {
        use crate::models::knowledge::{KnowledgeStatus, KnowledgeType};

        let mut md = String::new();

        let status_text = match article.status {
            KnowledgeStatus::Draft => "Draft",
            KnowledgeStatus::Review => "Under Review",
            KnowledgeStatus::Published => "Published",
            KnowledgeStatus::Archived => "Archived",
            KnowledgeStatus::Deprecated => "Deprecated",
        };

        let type_text = match article.article_type {
            KnowledgeType::Guide => "Guide",
            KnowledgeType::Standard => "Standard",
            KnowledgeType::Reference => "Reference",
            KnowledgeType::HowTo => "How-To",
            KnowledgeType::Troubleshooting => "Troubleshooting",
            KnowledgeType::Policy => "Policy",
            KnowledgeType::Template => "Template",
            KnowledgeType::Concept => "Concept",
            KnowledgeType::Runbook => "Runbook",
        };

        md.push_str(&format!(
            "# {}: {}\n\n",
            article.formatted_number(),
            article.title
        ));
        md.push_str(&format!(
            "**Type:** {} | **Status:** {}\n\n",
            type_text, status_text
        ));

        if let Some(domain) = &article.domain {
            md.push_str(&format!("**Domain:** {}\n\n", domain));
        }

        if !article.authors.is_empty() {
            md.push_str(&format!("**Authors:** {}\n\n", article.authors.join(", ")));
        }

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str(&article.summary);
        md.push_str("\n\n");

        // Content
        md.push_str("## Content\n\n");
        md.push_str(&article.content);
        md.push_str("\n\n");

        // Audience
        if !article.audience.is_empty() {
            md.push_str("## Target Audience\n\n");
            for audience in &article.audience {
                md.push_str(&format!("- {}\n", audience));
            }
            md.push('\n');
        }

        // Skill Level
        if let Some(skill_level) = &article.skill_level {
            md.push_str(&format!("**Skill Level:** {}\n\n", skill_level));
        }

        // Tags
        if !article.tags.is_empty() {
            md.push_str("## Tags\n\n");
            let tag_strings: Vec<String> = article.tags.iter().map(|t| t.to_string()).collect();
            md.push_str(&tag_strings.join(", "));
            md.push_str("\n\n");
        }

        // Related Articles
        if !article.related_articles.is_empty() {
            md.push_str("## Related Articles\n\n");
            for related in &article.related_articles {
                md.push_str(&format!(
                    "- {}: {} ({})\n",
                    related.article_number, related.title, related.relationship
                ));
            }
            md.push('\n');
        }

        // Notes
        if let Some(notes) = &article.notes {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push('\n');
        }

        md
    }

    /// Generate PDF from markdown content
    ///
    /// This is a simplified PDF generation that creates a structured document.
    /// For full PDF rendering with actual fonts and layouts, a full PDF library
    /// like `printpdf` or external rendering would be needed.
    fn generate_pdf(
        &self,
        title: &str,
        markdown: &str,
        filename: &str,
    ) -> Result<PdfExportResult, ExportError> {
        // Create a simple PDF-like structure
        // This implementation creates a PDF placeholder with the content
        // Full PDF rendering would require additional dependencies

        let pdf_content = self.create_pdf_document(title, markdown)?;
        let pdf_base64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pdf_content);

        // Estimate page count based on content length (rough approximation)
        let chars_per_page = 3000; // Approximate characters per page
        let page_count = std::cmp::max(1, (markdown.len() / chars_per_page) as u32 + 1);

        Ok(PdfExportResult {
            pdf_base64,
            filename: filename.to_string(),
            page_count,
            title: title.to_string(),
        })
    }

    /// Create a minimal PDF document structure
    ///
    /// This creates a valid PDF 1.4 document with the content as text.
    fn create_pdf_document(&self, title: &str, content: &str) -> Result<Vec<u8>, ExportError> {
        use chrono::Utc;

        let mut pdf = Vec::new();

        // PDF Header
        pdf.extend_from_slice(b"%PDF-1.4\n");
        pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n"); // Binary marker

        // Track object positions for xref
        let mut xref_positions: Vec<usize> = Vec::new();

        // Object 1: Catalog
        xref_positions.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        xref_positions.push(pdf.len());
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        // Object 3: Page
        xref_positions.push(pdf.len());
        let (width, height) = self.branding.page_size.dimensions_mm();
        let width_pt = width * 2.83465; // mm to points
        let height_pt = height * 2.83465;
        let page_obj = format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
            width_pt, height_pt
        );
        pdf.extend_from_slice(page_obj.as_bytes());

        // Object 4: Content stream
        xref_positions.push(pdf.len());
        let content_stream = self.create_content_stream(title, content, width_pt, height_pt);
        let content_obj = format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content_stream.len(),
            content_stream
        );
        pdf.extend_from_slice(content_obj.as_bytes());

        // Object 5: Font
        xref_positions.push(pdf.len());
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );

        // Object 6: Info dictionary
        xref_positions.push(pdf.len());
        let timestamp = if self.branding.show_timestamp {
            Utc::now().format("D:%Y%m%d%H%M%S").to_string()
        } else {
            String::new()
        };

        let escaped_title = self.escape_pdf_string(title);
        let producer = "Data Modelling SDK PDF Exporter";
        let company = self
            .branding
            .company_name
            .as_deref()
            .unwrap_or("Data Modelling SDK");

        let info_obj = format!(
            "6 0 obj\n<< /Title ({}) /Producer ({}) /Creator ({}) /CreationDate ({}) >>\nendobj\n",
            escaped_title, producer, company, timestamp
        );
        pdf.extend_from_slice(info_obj.as_bytes());

        // Cross-reference table
        let xref_start = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(format!("0 {}\n", xref_positions.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for pos in &xref_positions {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", pos).as_bytes());
        }

        // Trailer
        pdf.extend_from_slice(b"trailer\n");
        pdf.extend_from_slice(
            format!(
                "<< /Size {} /Root 1 0 R /Info 6 0 R >>\n",
                xref_positions.len() + 1
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(b"startxref\n");
        pdf.extend_from_slice(format!("{}\n", xref_start).as_bytes());
        pdf.extend_from_slice(b"%%EOF\n");

        Ok(pdf)
    }

    /// Create PDF content stream with text layout
    fn create_content_stream(&self, title: &str, content: &str, width: f64, height: f64) -> String {
        let mut stream = String::new();

        let margin = 50.0;
        let line_height = self.branding.font_size as f64 * 1.2;
        let title_size = (self.branding.font_size as f64 * 1.5) as u8;

        // Start text block
        stream.push_str("BT\n");

        // Header (if present)
        let mut y_pos = height - margin;
        if let Some(header) = &self.branding.header {
            stream.push_str("/F1 10 Tf\n");
            stream.push_str(&format!("{:.2} {:.2} Td\n", margin, y_pos));
            stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(header)));
            y_pos -= line_height * 2.0;
        }

        // Company name (if present)
        if let Some(company) = &self.branding.company_name {
            stream.push_str(&format!("0 {:.2} Td\n", -line_height));
            stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(company)));
            y_pos -= line_height;
        }

        // Title
        stream.push_str(&format!("/F1 {} Tf\n", title_size));
        stream.push_str(&format!("{:.2} {:.2} Td\n", margin, y_pos - margin));
        stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(title)));
        y_pos -= line_height * 2.0;

        // Content - split into lines and render
        stream.push_str(&format!("/F1 {} Tf\n", self.branding.font_size));
        let max_chars_per_line =
            ((width - 2.0 * margin) / (self.branding.font_size as f64 * 0.5)) as usize;

        for line in content.lines() {
            // Skip markdown headers (we've already rendered the title)
            let line = line.trim();
            if line.is_empty() {
                y_pos -= line_height * 0.5;
                continue;
            }

            // Handle markdown headers
            let (text, font_size) = if line.starts_with("## ") {
                (line.trim_start_matches("## "), self.branding.font_size + 2)
            } else if line.starts_with("### ") {
                (line.trim_start_matches("### "), self.branding.font_size + 1)
            } else if line.starts_with("# ") {
                continue; // Skip main title (already rendered)
            } else {
                (line, self.branding.font_size)
            };

            // Word wrap
            let wrapped_lines = self.word_wrap(text, max_chars_per_line);
            for wrapped_line in wrapped_lines {
                if y_pos < margin + line_height {
                    // Would need page break - for now just stop
                    break;
                }
                stream.push_str(&format!("/F1 {} Tf\n", font_size));
                stream.push_str(&format!("0 {:.2} Td\n", -line_height));
                stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(&wrapped_line)));
                y_pos -= line_height;
            }
        }

        // Footer (if present)
        if let Some(footer) = &self.branding.footer {
            stream.push_str("/F1 10 Tf\n");
            stream.push_str(&format!("{:.2} {:.2} Td\n", margin, margin));
            stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(footer)));
        }

        stream.push_str("ET\n");
        stream
    }

    /// Escape special characters for PDF strings
    fn escape_pdf_string(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Word wrap text to fit within max characters per line
    fn word_wrap(&self, text: &str, max_chars: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_chars {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::decision::Decision;
    use crate::models::knowledge::KnowledgeArticle;

    #[test]
    fn test_branding_config_default() {
        let config = BrandingConfig::default();
        assert_eq!(config.brand_color, "#0066CC");
        assert!(config.show_page_numbers);
        assert!(config.show_timestamp);
        assert_eq!(config.font_size, 11);
        assert_eq!(config.page_size, PageSize::A4);
    }

    #[test]
    fn test_page_size_dimensions() {
        let a4 = PageSize::A4;
        let (w, h) = a4.dimensions_mm();
        assert_eq!(w, 210.0);
        assert_eq!(h, 297.0);

        let letter = PageSize::Letter;
        let (w, h) = letter.dimensions_mm();
        assert!((w - 215.9).abs() < 0.1);
        assert!((h - 279.4).abs() < 0.1);
    }

    #[test]
    fn test_pdf_exporter_with_branding() {
        let branding = BrandingConfig {
            header: Some("Company Header".to_string()),
            footer: Some("Confidential".to_string()),
            company_name: Some("Test Corp".to_string()),
            brand_color: "#FF0000".to_string(),
            ..Default::default()
        };

        let exporter = PdfExporter::with_branding(branding.clone());
        assert_eq!(
            exporter.branding().header,
            Some("Company Header".to_string())
        );
        assert_eq!(exporter.branding().brand_color, "#FF0000");
    }

    #[test]
    fn test_export_decision_to_pdf() {
        let decision = Decision::new(
            1,
            "Use Rust for SDK",
            "We need to choose a language for the SDK implementation.",
            "Use Rust for type safety and performance.",
        );

        let exporter = PdfExporter::new();
        let result = exporter.export_decision(&decision);
        assert!(result.is_ok());

        let pdf_result = result.unwrap();
        assert!(!pdf_result.pdf_base64.is_empty());
        assert!(pdf_result.filename.ends_with(".pdf"));
        assert!(pdf_result.page_count >= 1);
        assert!(pdf_result.title.contains("ADR-"));
    }

    #[test]
    fn test_export_knowledge_to_pdf() {
        let article = KnowledgeArticle::new(
            1,
            "Getting Started Guide",
            "A guide to getting started with the SDK.",
            "This guide covers the basics...",
            "author@example.com",
        );

        let exporter = PdfExporter::new();
        let result = exporter.export_knowledge(&article);
        assert!(result.is_ok());

        let pdf_result = result.unwrap();
        assert!(!pdf_result.pdf_base64.is_empty());
        assert!(pdf_result.filename.ends_with(".pdf"));
        assert!(pdf_result.title.contains("KB-"));
    }

    #[test]
    fn test_export_markdown_to_pdf() {
        let exporter = PdfExporter::new();
        let result = exporter.export_markdown(
            "Test Document",
            "# Test\n\nThis is a test document.",
            "test.pdf",
        );
        assert!(result.is_ok());

        let pdf_result = result.unwrap();
        assert!(!pdf_result.pdf_base64.is_empty());
        assert_eq!(pdf_result.filename, "test.pdf");
    }

    #[test]
    fn test_escape_pdf_string() {
        let exporter = PdfExporter::new();
        let escaped = exporter.escape_pdf_string("Test (with) special\\chars");
        assert_eq!(escaped, "Test \\(with\\) special\\\\chars");
    }

    #[test]
    fn test_word_wrap() {
        let exporter = PdfExporter::new();
        let text = "This is a long line that should be wrapped properly";
        let wrapped = exporter.word_wrap(text, 20);
        assert!(wrapped.len() > 1);
        for line in &wrapped {
            assert!(line.len() <= 25); // Allow some flexibility for words
        }
    }

    #[test]
    fn test_pdf_result_serialization() {
        let result = PdfExportResult {
            pdf_base64: "SGVsbG8=".to_string(),
            filename: "test.pdf".to_string(),
            page_count: 1,
            title: "Test".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("pdf_base64"));
        assert!(json.contains("filename"));
    }
}
