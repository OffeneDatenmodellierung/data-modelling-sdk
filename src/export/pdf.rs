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
//! - Proper GitHub Flavored Markdown rendering
//!
//! ## WASM Compatibility
//!
//! This module is designed to work in both native and WASM environments
//! by generating PDF as base64-encoded bytes.

use crate::export::ExportError;
use crate::models::decision::Decision;
use crate::models::knowledge::KnowledgeArticle;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Default logo URL for Open Data Modelling
const DEFAULT_LOGO_URL: &str = "https://opendatamodelling.com/logo.png";

/// Default copyright footer
const DEFAULT_COPYRIGHT: &str = "© opendatamodelling.com";

/// Branding configuration for PDF exports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Logo as base64-encoded image data (PNG or JPEG)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_base64: Option<String>,

    /// Logo URL (alternative to base64)
    #[serde(default = "default_logo_url")]
    pub logo_url: Option<String>,

    /// Header text (appears at top of each page)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    /// Footer text (appears at bottom of each page)
    #[serde(default = "default_footer")]
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

fn default_logo_url() -> Option<String> {
    Some(DEFAULT_LOGO_URL.to_string())
}

fn default_footer() -> Option<String> {
    Some(DEFAULT_COPYRIGHT.to_string())
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
            logo_url: default_logo_url(),
            header: None,
            footer: default_footer(),
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
            "Decision Record",
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
            "Knowledge Base",
        )
    }

    /// Export raw markdown content to PDF
    pub fn export_markdown(
        &self,
        title: &str,
        content: &str,
        filename: &str,
    ) -> Result<PdfExportResult, ExportError> {
        self.generate_pdf(title, content, filename, "Document")
    }

    /// Export an ODCS Data Contract (Table) to PDF
    pub fn export_table(
        &self,
        table: &crate::models::Table,
    ) -> Result<PdfExportResult, ExportError> {
        let title = table.name.clone();
        let markdown = self.table_to_markdown(table);
        let filename = format!("{}.pdf", table.name.to_lowercase().replace(' ', "_"));
        self.generate_pdf(&title, &markdown, &filename, "Data Contract")
    }

    /// Export an ODPS Data Product to PDF
    pub fn export_data_product(
        &self,
        product: &crate::models::odps::ODPSDataProduct,
    ) -> Result<PdfExportResult, ExportError> {
        let title = product.name.clone().unwrap_or_else(|| product.id.clone());
        let markdown = self.data_product_to_markdown(product);
        let filename = format!(
            "{}.pdf",
            title.to_lowercase().replace(' ', "_").replace('/', "-")
        );
        self.generate_pdf(&title, &markdown, &filename, "Data Product")
    }

    /// Export a CADS Asset to PDF
    pub fn export_cads_asset(
        &self,
        asset: &crate::models::cads::CADSAsset,
    ) -> Result<PdfExportResult, ExportError> {
        let title = asset.name.clone();
        let markdown = self.cads_asset_to_markdown(asset);
        let filename = format!(
            "{}.pdf",
            title.to_lowercase().replace(' ', "_").replace('/', "-")
        );
        self.generate_pdf(&title, &markdown, &filename, "Compute Asset")
    }

    // ============================================================================
    // Public Markdown Generation Methods (for WASM bindings)
    // ============================================================================

    /// Convert an ODCS Table (Data Contract) to Markdown format.
    ///
    /// This is a public wrapper for use by WASM bindings.
    pub fn table_to_markdown_public(&self, table: &crate::models::Table) -> String {
        self.table_to_markdown(table)
    }

    /// Convert an ODPS Data Product to Markdown format.
    ///
    /// This is a public wrapper for use by WASM bindings.
    pub fn data_product_to_markdown_public(
        &self,
        product: &crate::models::odps::ODPSDataProduct,
    ) -> String {
        self.data_product_to_markdown(product)
    }

    /// Convert a CADS Asset to Markdown format.
    ///
    /// This is a public wrapper for use by WASM bindings.
    pub fn cads_asset_to_markdown_public(&self, asset: &crate::models::cads::CADSAsset) -> String {
        self.cads_asset_to_markdown(asset)
    }

    /// Convert Decision to properly formatted GFM markdown for PDF rendering
    /// Note: Logo and copyright footer are rendered as part of the PDF template,
    /// not in the markdown content.
    fn decision_to_markdown(&self, decision: &Decision) -> String {
        use crate::models::decision::DecisionStatus;

        let mut md = String::new();

        // Main title
        md.push_str(&format!(
            "# {}: {}\n\n",
            decision.formatted_number(),
            decision.title
        ));

        // Metadata table
        let status_text = match decision.status {
            DecisionStatus::Draft => "Draft",
            DecisionStatus::Proposed => "Proposed",
            DecisionStatus::Accepted => "Accepted",
            DecisionStatus::Deprecated => "Deprecated",
            DecisionStatus::Superseded => "Superseded",
            DecisionStatus::Rejected => "Rejected",
        };

        md.push_str("| Property | Value |\n");
        md.push_str("|----------|-------|\n");
        md.push_str(&format!("| **Status** | {} |\n", status_text));
        md.push_str(&format!("| **Category** | {} |\n", decision.category));
        md.push_str(&format!(
            "| **Date** | {} |\n",
            decision.date.format("%Y-%m-%d")
        ));

        if !decision.authors.is_empty() {
            md.push_str(&format!(
                "| **Authors** | {} |\n",
                decision.authors.join(", ")
            ));
        }

        if let Some(domain) = &decision.domain {
            md.push_str(&format!("| **Domain** | {} |\n", domain));
        }

        md.push_str("\n---\n\n");

        // Context section
        md.push_str("## Context\n\n");
        md.push_str(&decision.context);
        md.push_str("\n\n");

        // Decision section
        md.push_str("## Decision\n\n");
        md.push_str(&decision.decision);
        md.push_str("\n\n");

        // Consequences section
        if let Some(consequences) = &decision.consequences {
            md.push_str("## Consequences\n\n");
            md.push_str(consequences);
            md.push_str("\n\n");
        }

        // Stakeholders section (if present)
        if !decision.consulted.is_empty() || !decision.informed.is_empty() {
            md.push_str("## Stakeholders\n\n");
            md.push_str("| Role | Participants |\n");
            md.push_str("|------|-------------|\n");

            if !decision.deciders.is_empty() {
                md.push_str(&format!(
                    "| **Deciders** | {} |\n",
                    decision.deciders.join(", ")
                ));
            }
            if !decision.consulted.is_empty() {
                md.push_str(&format!(
                    "| **Consulted** | {} |\n",
                    decision.consulted.join(", ")
                ));
            }
            if !decision.informed.is_empty() {
                md.push_str(&format!(
                    "| **Informed** | {} |\n",
                    decision.informed.join(", ")
                ));
            }
            md.push('\n');
        }

        // Decision Drivers (if any)
        if !decision.drivers.is_empty() {
            md.push_str("## Decision Drivers\n\n");
            for driver in &decision.drivers {
                let priority = match driver.priority {
                    Some(crate::models::decision::DriverPriority::High) => " *(High Priority)*",
                    Some(crate::models::decision::DriverPriority::Medium) => " *(Medium Priority)*",
                    Some(crate::models::decision::DriverPriority::Low) => " *(Low Priority)*",
                    None => "",
                };
                md.push_str(&format!("- {}{}\n", driver.description, priority));
            }
            md.push('\n');
        }

        // Options Considered (if any) - with side-by-side Pros/Cons
        if !decision.options.is_empty() {
            md.push_str("## Options Considered\n\n");
            for (i, option) in decision.options.iter().enumerate() {
                let selected_marker = if option.selected {
                    " **(Selected)**"
                } else {
                    ""
                };
                md.push_str(&format!(
                    "### Option {}: {}{}\n\n",
                    i + 1,
                    option.name,
                    selected_marker
                ));

                if let Some(desc) = &option.description {
                    md.push_str(&format!("{}\n\n", desc));
                }

                // Render Pros and Cons side by side using a table
                if !option.pros.is_empty() || !option.cons.is_empty() {
                    md.push_str("| Pros | Cons |\n");
                    md.push_str("|------|------|\n");

                    let max_rows = std::cmp::max(option.pros.len(), option.cons.len());
                    for row in 0..max_rows {
                        let pro = option
                            .pros
                            .get(row)
                            .map(|s| format!("+ {}", s))
                            .unwrap_or_default();
                        let con = option
                            .cons
                            .get(row)
                            .map(|s| format!("- {}", s))
                            .unwrap_or_default();
                        md.push_str(&format!("| {} | {} |\n", pro, con));
                    }
                    md.push('\n');
                }
            }
        }

        // Linked Assets (if any)
        if !decision.linked_assets.is_empty() {
            md.push_str("## Linked Assets\n\n");
            md.push_str("| Asset | Type |\n");
            md.push_str("|-------|------|\n");
            for asset in &decision.linked_assets {
                md.push_str(&format!(
                    "| {} | {} |\n",
                    asset.asset_name, asset.asset_type
                ));
            }
            md.push('\n');
        }

        // Notes (if present)
        if let Some(notes) = &decision.notes {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push_str("\n\n");
        }

        // Horizontal rule before footer
        md.push_str("---\n\n");

        // Tags
        if !decision.tags.is_empty() {
            let tag_strings: Vec<String> =
                decision.tags.iter().map(|t| format!("`{}`", t)).collect();
            md.push_str(&format!("**Tags:** {}\n\n", tag_strings.join(" ")));
        }

        // Horizontal rule
        md.push_str("---\n\n");

        // Timestamps
        md.push_str(&format!(
            "*Created: {} | Last Updated: {}*\n\n",
            decision.created_at.format("%Y-%m-%d %H:%M UTC"),
            decision.updated_at.format("%Y-%m-%d %H:%M UTC")
        ));

        md
    }

    /// Convert Knowledge article to properly formatted GFM markdown for PDF rendering
    /// Note: Logo and copyright footer are rendered as part of the PDF template,
    /// not in the markdown content.
    fn knowledge_to_markdown(&self, article: &KnowledgeArticle) -> String {
        use crate::models::knowledge::{KnowledgeStatus, KnowledgeType};

        let mut md = String::new();

        // Main title
        md.push_str(&format!(
            "# {}: {}\n\n",
            article.formatted_number(),
            article.title
        ));

        // Metadata table
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
            KnowledgeType::Tutorial => "Tutorial",
            KnowledgeType::Glossary => "Glossary",
        };

        md.push_str("| Property | Value |\n");
        md.push_str("|----------|-------|\n");
        md.push_str(&format!("| **Type** | {} |\n", type_text));
        md.push_str(&format!("| **Status** | {} |\n", status_text));

        if let Some(domain) = &article.domain {
            md.push_str(&format!("| **Domain** | {} |\n", domain));
        }

        if !article.authors.is_empty() {
            md.push_str(&format!(
                "| **Authors** | {} |\n",
                article.authors.join(", ")
            ));
        }

        if let Some(skill_level) = &article.skill_level {
            md.push_str(&format!("| **Skill Level** | {} |\n", skill_level));
        }

        if !article.audience.is_empty() {
            md.push_str(&format!(
                "| **Audience** | {} |\n",
                article.audience.join(", ")
            ));
        }

        md.push_str("\n---\n\n");

        // Summary section
        md.push_str("## Summary\n\n");
        md.push_str(&article.summary);
        md.push_str("\n\n---\n\n");

        // Content section (the main article content - already in markdown)
        md.push_str(&article.content);
        md.push_str("\n\n");

        // Related Articles (if any)
        if !article.related_articles.is_empty() {
            md.push_str("---\n\n");
            md.push_str("## Related Articles\n\n");
            md.push_str("| Article | Relationship |\n");
            md.push_str("|---------|-------------|\n");
            for related in &article.related_articles {
                md.push_str(&format!(
                    "| {}: {} | {} |\n",
                    related.article_number, related.title, related.relationship
                ));
            }
            md.push('\n');
        }

        // Notes (if present)
        if let Some(notes) = &article.notes {
            md.push_str("---\n\n");
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push_str("\n\n");
        }

        // Horizontal rule before footer
        md.push_str("---\n\n");

        // Tags
        if !article.tags.is_empty() {
            let tag_strings: Vec<String> =
                article.tags.iter().map(|t| format!("`{}`", t)).collect();
            md.push_str(&format!("**Tags:** {}\n\n", tag_strings.join(" ")));
        }

        // Horizontal rule
        md.push_str("---\n\n");

        // Timestamps
        md.push_str(&format!(
            "*Created: {} | Last Updated: {}*\n\n",
            article.created_at.format("%Y-%m-%d %H:%M UTC"),
            article.updated_at.format("%Y-%m-%d %H:%M UTC")
        ));

        md
    }

    /// Convert Table (ODCS Data Contract) to properly formatted GFM markdown for PDF rendering
    fn table_to_markdown(&self, table: &crate::models::Table) -> String {
        let mut md = String::new();

        // Main title
        md.push_str(&format!("# {}\n\n", table.name));

        // Metadata table
        md.push_str("| Property | Value |\n");
        md.push_str("|----------|-------|\n");

        if let Some(db_type) = &table.database_type {
            md.push_str(&format!("| **Database Type** | {:?} |\n", db_type));
        }

        if let Some(catalog) = &table.catalog_name {
            md.push_str(&format!("| **Catalog** | {} |\n", catalog));
        }

        if let Some(schema) = &table.schema_name {
            md.push_str(&format!("| **Schema** | {} |\n", schema));
        }

        if let Some(owner) = &table.owner {
            md.push_str(&format!("| **Owner** | {} |\n", owner));
        }

        if !table.medallion_layers.is_empty() {
            let layers: Vec<String> = table
                .medallion_layers
                .iter()
                .map(|l| format!("{:?}", l))
                .collect();
            md.push_str(&format!(
                "| **Medallion Layers** | {} |\n",
                layers.join(", ")
            ));
        }

        if let Some(scd) = &table.scd_pattern {
            md.push_str(&format!("| **SCD Pattern** | {:?} |\n", scd));
        }

        if let Some(dv) = &table.data_vault_classification {
            md.push_str(&format!("| **Data Vault** | {:?} |\n", dv));
        }

        if let Some(level) = &table.modeling_level {
            md.push_str(&format!("| **Modeling Level** | {:?} |\n", level));
        }

        if let Some(infra) = &table.infrastructure_type {
            md.push_str(&format!("| **Infrastructure** | {:?} |\n", infra));
        }

        md.push_str(&format!("| **Columns** | {} |\n", table.columns.len()));

        md.push_str("\n---\n\n");

        // Notes section
        if let Some(notes) = &table.notes {
            md.push_str("## Description\n\n");
            md.push_str(notes);
            md.push_str("\n\n---\n\n");
        }

        // Columns section
        md.push_str("## Columns\n\n");
        md.push_str("| Column | Type | Nullable | PK | Description |\n");
        md.push_str("|--------|------|----------|----|--------------|\n");

        for col in &table.columns {
            let nullable = if col.nullable { "Yes" } else { "No" };
            let pk = if col.primary_key { "Yes" } else { "" };
            let desc = col
                .description
                .chars()
                .take(50)
                .collect::<String>()
                .replace('|', "/");
            let desc_display = if col.description.len() > 50 {
                format!("{}...", desc)
            } else {
                desc
            };

            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                col.name, col.data_type, nullable, pk, desc_display
            ));
        }
        md.push('\n');

        // Column Details (for columns with descriptions, business names, constraints, etc.)
        let cols_with_details: Vec<_> = table
            .columns
            .iter()
            .filter(|c| {
                !c.description.is_empty()
                    || c.business_name.is_some()
                    || !c.enum_values.is_empty()
                    || c.physical_type.is_some()
                    || c.unique
                    || c.partitioned
                    || c.classification.is_some()
                    || c.critical_data_element
            })
            .collect();

        if !cols_with_details.is_empty() {
            md.push_str("## Column Details\n\n");
            for col in cols_with_details {
                md.push_str(&format!("### {}\n\n", col.name));

                if let Some(biz_name) = &col.business_name {
                    md.push_str(&format!("**Business Name:** {}\n\n", biz_name));
                }

                if !col.description.is_empty() {
                    md.push_str(&format!("{}\n\n", col.description));
                }

                // Physical type if different from logical
                if let Some(phys) = &col.physical_type
                    && phys != &col.data_type
                {
                    md.push_str(&format!("**Physical Type:** {}\n\n", phys));
                }

                // Constraints
                let mut constraints = Vec::new();
                if col.unique {
                    constraints.push("Unique");
                }
                if col.partitioned {
                    constraints.push("Partitioned");
                }
                if col.clustered {
                    constraints.push("Clustered");
                }
                if col.critical_data_element {
                    constraints.push("Critical Data Element");
                }
                if !constraints.is_empty() {
                    md.push_str(&format!("**Constraints:** {}\n\n", constraints.join(", ")));
                }

                // Classification
                if let Some(class) = &col.classification {
                    md.push_str(&format!("**Classification:** {}\n\n", class));
                }

                // Enum values
                if !col.enum_values.is_empty() {
                    md.push_str("**Allowed Values:**\n");
                    for val in &col.enum_values {
                        md.push_str(&format!("- `{}`\n", val));
                    }
                    md.push('\n');
                }

                // Examples
                if !col.examples.is_empty() {
                    let examples_str: Vec<String> =
                        col.examples.iter().map(|v| v.to_string()).collect();
                    md.push_str(&format!("**Examples:** {}\n\n", examples_str.join(", ")));
                }

                // Default value
                if let Some(default) = &col.default_value {
                    md.push_str(&format!("**Default:** {}\n\n", default));
                }
            }
        }

        // SLA section
        if let Some(sla) = &table.sla
            && !sla.is_empty()
        {
            md.push_str("---\n\n## Service Level Agreements\n\n");
            md.push_str("| Property | Value | Unit | Description |\n");
            md.push_str("|----------|-------|------|-------------|\n");
            for sla_prop in sla {
                let desc = sla_prop
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .replace('|', "/");
                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    sla_prop.property, sla_prop.value, sla_prop.unit, desc
                ));
            }
            md.push('\n');
        }

        // Contact Details
        if let Some(contact) = &table.contact_details {
            md.push_str("---\n\n## Contact Information\n\n");
            if let Some(name) = &contact.name {
                md.push_str(&format!("- **Name:** {}\n", name));
            }
            if let Some(email) = &contact.email {
                md.push_str(&format!("- **Email:** {}\n", email));
            }
            if let Some(role) = &contact.role {
                md.push_str(&format!("- **Role:** {}\n", role));
            }
            if let Some(phone) = &contact.phone {
                md.push_str(&format!("- **Phone:** {}\n", phone));
            }
            md.push('\n');
        }

        // Quality Rules
        if !table.quality.is_empty() {
            md.push_str("---\n\n## Quality Rules\n\n");
            for (i, rule) in table.quality.iter().enumerate() {
                md.push_str(&format!("**Rule {}:**\n", i + 1));
                for (key, value) in rule {
                    md.push_str(&format!("- {}: {}\n", key, value));
                }
                md.push('\n');
            }
        }

        // ODCS Metadata (legacy format fields preserved from import)
        if !table.odcl_metadata.is_empty() {
            md.push_str("---\n\n## ODCS Contract Metadata\n\n");
            // Sort keys for consistent output
            let mut keys: Vec<_> = table.odcl_metadata.keys().collect();
            keys.sort();
            for key in keys {
                if let Some(value) = table.odcl_metadata.get(key) {
                    // Format the value based on its type
                    let formatted = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Array(arr) => {
                            let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                            items.join(", ")
                        }
                        serde_json::Value::Object(_) => {
                            // For nested objects, show as formatted JSON-like structure
                            serde_json::to_string_pretty(value)
                                .unwrap_or_else(|_| value.to_string())
                        }
                        _ => value.to_string(),
                    };
                    md.push_str(&format!("- **{}:** {}\n", key, formatted));
                }
            }
            md.push('\n');
        }

        // Tags
        if !table.tags.is_empty() {
            md.push_str("---\n\n");
            let tag_strings: Vec<String> = table.tags.iter().map(|t| format!("`{}`", t)).collect();
            md.push_str(&format!("**Tags:** {}\n\n", tag_strings.join(" ")));
        }

        // Timestamps
        md.push_str("---\n\n");
        md.push_str(&format!(
            "*Created: {} | Last Updated: {}*\n\n",
            table.created_at.format("%Y-%m-%d %H:%M UTC"),
            table.updated_at.format("%Y-%m-%d %H:%M UTC")
        ));

        md
    }

    /// Convert ODPS Data Product to properly formatted GFM markdown for PDF rendering
    fn data_product_to_markdown(&self, product: &crate::models::odps::ODPSDataProduct) -> String {
        use crate::models::odps::ODPSStatus;

        let mut md = String::new();

        // Main title
        let title = product.name.as_deref().unwrap_or(&product.id);
        md.push_str(&format!("# {}\n\n", title));

        // Metadata table
        let status_text = match product.status {
            ODPSStatus::Proposed => "Proposed",
            ODPSStatus::Draft => "Draft",
            ODPSStatus::Active => "Active",
            ODPSStatus::Deprecated => "Deprecated",
            ODPSStatus::Retired => "Retired",
        };

        md.push_str("| Property | Value |\n");
        md.push_str("|----------|-------|\n");
        md.push_str(&format!("| **ID** | {} |\n", product.id));
        md.push_str(&format!("| **Status** | {} |\n", status_text));
        md.push_str(&format!("| **API Version** | {} |\n", product.api_version));

        if let Some(version) = &product.version {
            md.push_str(&format!("| **Version** | {} |\n", version));
        }

        if let Some(domain) = &product.domain {
            md.push_str(&format!("| **Domain** | {} |\n", domain));
        }

        if let Some(tenant) = &product.tenant {
            md.push_str(&format!("| **Tenant** | {} |\n", tenant));
        }

        md.push_str("\n---\n\n");

        // Description section
        if let Some(desc) = &product.description {
            md.push_str("## Description\n\n");
            if let Some(purpose) = &desc.purpose {
                md.push_str(&format!("**Purpose:** {}\n\n", purpose));
            }
            if let Some(usage) = &desc.usage {
                md.push_str(&format!("**Usage:** {}\n\n", usage));
            }
            if let Some(limitations) = &desc.limitations {
                md.push_str(&format!("**Limitations:** {}\n\n", limitations));
            }
            md.push_str("---\n\n");
        }

        // Input Ports
        if let Some(input_ports) = &product.input_ports
            && !input_ports.is_empty()
        {
            md.push_str("## Input Ports\n\n");
            md.push_str("| Name | Version | Contract ID |\n");
            md.push_str("|------|---------|-------------|\n");
            for port in input_ports {
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    port.name, port.version, port.contract_id
                ));
            }
            md.push('\n');
        }

        // Output Ports
        if let Some(output_ports) = &product.output_ports
            && !output_ports.is_empty()
        {
            md.push_str("## Output Ports\n\n");
            md.push_str("| Name | Version | Type | Contract ID |\n");
            md.push_str("|------|---------|------|-------------|\n");
            for port in output_ports {
                let port_type = port.r#type.as_deref().unwrap_or("-");
                let contract = port.contract_id.as_deref().unwrap_or("-");
                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    port.name, port.version, port_type, contract
                ));
            }
            md.push('\n');

            // Output port details
            for port in output_ports {
                if port.description.is_some()
                    || port.sbom.is_some()
                    || port.input_contracts.is_some()
                {
                    md.push_str(&format!("### {}\n\n", port.name));
                    if let Some(desc) = &port.description {
                        md.push_str(&format!("{}\n\n", desc));
                    }
                    if let Some(sbom) = &port.sbom
                        && !sbom.is_empty()
                    {
                        md.push_str("**SBOM:**\n");
                        for s in sbom {
                            let stype = s.r#type.as_deref().unwrap_or("unknown");
                            md.push_str(&format!("- {} ({})\n", s.url, stype));
                        }
                        md.push('\n');
                    }
                    if let Some(contracts) = &port.input_contracts
                        && !contracts.is_empty()
                    {
                        md.push_str("**Input Contracts:**\n");
                        for c in contracts {
                            md.push_str(&format!("- {} v{}\n", c.id, c.version));
                        }
                        md.push('\n');
                    }
                }
            }
        }

        // Management Ports
        if let Some(mgmt_ports) = &product.management_ports
            && !mgmt_ports.is_empty()
        {
            md.push_str("## Management Ports\n\n");
            md.push_str("| Name | Type | Content |\n");
            md.push_str("|------|------|--------|\n");
            for port in mgmt_ports {
                let port_type = port.r#type.as_deref().unwrap_or("-");
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    port.name, port_type, port.content
                ));
            }
            md.push('\n');
        }

        // Support Channels
        if let Some(support) = &product.support
            && !support.is_empty()
        {
            md.push_str("## Support Channels\n\n");
            md.push_str("| Channel | URL | Description |\n");
            md.push_str("|---------|-----|-------------|\n");
            for s in support {
                let desc = s.description.as_deref().unwrap_or("-").replace('|', "/");
                md.push_str(&format!("| {} | {} | {} |\n", s.channel, s.url, desc));
            }
            md.push('\n');
        }

        // Team
        if let Some(team) = &product.team {
            md.push_str("## Team\n\n");
            if let Some(name) = &team.name {
                md.push_str(&format!("**Team Name:** {}\n\n", name));
            }
            if let Some(desc) = &team.description {
                md.push_str(&format!("{}\n\n", desc));
            }
            if let Some(members) = &team.members
                && !members.is_empty()
            {
                md.push_str("### Team Members\n\n");
                md.push_str("| Username | Name | Role |\n");
                md.push_str("|----------|------|------|\n");
                for member in members {
                    let name = member.name.as_deref().unwrap_or("-");
                    let role = member.role.as_deref().unwrap_or("-");
                    md.push_str(&format!("| {} | {} | {} |\n", member.username, name, role));
                }
                md.push('\n');
            }
        }

        // Tags
        if !product.tags.is_empty() {
            md.push_str("---\n\n");
            let tag_strings: Vec<String> =
                product.tags.iter().map(|t| format!("`{}`", t)).collect();
            md.push_str(&format!("**Tags:** {}\n\n", tag_strings.join(" ")));
        }

        // Timestamps
        if product.created_at.is_some() || product.updated_at.is_some() {
            md.push_str("---\n\n");
            if let Some(created) = &product.created_at {
                md.push_str(&format!(
                    "*Created: {}",
                    created.format("%Y-%m-%d %H:%M UTC")
                ));
                if let Some(updated) = &product.updated_at {
                    md.push_str(&format!(
                        " | Last Updated: {}",
                        updated.format("%Y-%m-%d %H:%M UTC")
                    ));
                }
                md.push_str("*\n\n");
            } else if let Some(updated) = &product.updated_at {
                md.push_str(&format!(
                    "*Last Updated: {}*\n\n",
                    updated.format("%Y-%m-%d %H:%M UTC")
                ));
            }
        }

        md
    }

    /// Convert CADS Asset to properly formatted GFM markdown for PDF rendering
    fn cads_asset_to_markdown(&self, asset: &crate::models::cads::CADSAsset) -> String {
        use crate::models::cads::{CADSKind, CADSStatus};

        let mut md = String::new();

        // Main title
        md.push_str(&format!("# {}\n\n", asset.name));

        // Metadata table
        let kind_text = match asset.kind {
            CADSKind::AIModel => "AI Model",
            CADSKind::MLPipeline => "ML Pipeline",
            CADSKind::Application => "Application",
            CADSKind::DataPipeline => "Data Pipeline",
            CADSKind::ETLProcess => "ETL Process",
            CADSKind::ETLPipeline => "ETL Pipeline",
            CADSKind::SourceSystem => "Source System",
            CADSKind::DestinationSystem => "Destination System",
        };

        let status_text = match asset.status {
            CADSStatus::Draft => "Draft",
            CADSStatus::Validated => "Validated",
            CADSStatus::Production => "Production",
            CADSStatus::Deprecated => "Deprecated",
        };

        md.push_str("| Property | Value |\n");
        md.push_str("|----------|-------|\n");
        md.push_str(&format!("| **ID** | {} |\n", asset.id));
        md.push_str(&format!("| **Kind** | {} |\n", kind_text));
        md.push_str(&format!("| **Version** | {} |\n", asset.version));
        md.push_str(&format!("| **Status** | {} |\n", status_text));
        md.push_str(&format!("| **API Version** | {} |\n", asset.api_version));

        if let Some(domain) = &asset.domain {
            md.push_str(&format!("| **Domain** | {} |\n", domain));
        }

        md.push_str("\n---\n\n");

        // Description section
        if let Some(desc) = &asset.description {
            md.push_str("## Description\n\n");
            if let Some(purpose) = &desc.purpose {
                md.push_str(&format!("**Purpose:** {}\n\n", purpose));
            }
            if let Some(usage) = &desc.usage {
                md.push_str(&format!("**Usage:** {}\n\n", usage));
            }
            if let Some(limitations) = &desc.limitations {
                md.push_str(&format!("**Limitations:** {}\n\n", limitations));
            }
            if let Some(links) = &desc.external_links
                && !links.is_empty()
            {
                md.push_str("**External Links:**\n");
                for link in links {
                    let desc = link.description.as_deref().unwrap_or("");
                    md.push_str(&format!("- {} {}\n", link.url, desc));
                }
                md.push('\n');
            }
            md.push_str("---\n\n");
        }

        // Runtime section
        if let Some(runtime) = &asset.runtime {
            md.push_str("## Runtime\n\n");
            if let Some(env) = &runtime.environment {
                md.push_str(&format!("**Environment:** {}\n\n", env));
            }
            if let Some(endpoints) = &runtime.endpoints
                && !endpoints.is_empty()
            {
                md.push_str("**Endpoints:**\n");
                for ep in endpoints {
                    md.push_str(&format!("- {}\n", ep));
                }
                md.push('\n');
            }
            if let Some(container) = &runtime.container
                && let Some(image) = &container.image
            {
                md.push_str(&format!("**Container Image:** {}\n\n", image));
            }
            if let Some(resources) = &runtime.resources {
                md.push_str("**Resources:**\n");
                if let Some(cpu) = &resources.cpu {
                    md.push_str(&format!("- CPU: {}\n", cpu));
                }
                if let Some(memory) = &resources.memory {
                    md.push_str(&format!("- Memory: {}\n", memory));
                }
                if let Some(gpu) = &resources.gpu {
                    md.push_str(&format!("- GPU: {}\n", gpu));
                }
                md.push('\n');
            }
        }

        // SLA section
        if let Some(sla) = &asset.sla
            && let Some(props) = &sla.properties
            && !props.is_empty()
        {
            md.push_str("## Service Level Agreements\n\n");
            md.push_str("| Element | Value | Unit | Driver |\n");
            md.push_str("|---------|-------|------|--------|\n");
            for prop in props {
                let driver = prop.driver.as_deref().unwrap_or("-");
                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    prop.element, prop.value, prop.unit, driver
                ));
            }
            md.push('\n');
        }

        // Pricing section
        if let Some(pricing) = &asset.pricing {
            md.push_str("## Pricing\n\n");
            if let Some(model) = &pricing.model {
                md.push_str(&format!("**Model:** {:?}\n\n", model));
            }
            if let Some(currency) = &pricing.currency
                && let Some(cost) = pricing.unit_cost
            {
                let unit = pricing.billing_unit.as_deref().unwrap_or("unit");
                md.push_str(&format!("**Cost:** {} {} per {}\n\n", cost, currency, unit));
            }
            if let Some(notes) = &pricing.notes {
                md.push_str(&format!("**Notes:** {}\n\n", notes));
            }
        }

        // Team section
        if let Some(team) = &asset.team
            && !team.is_empty()
        {
            md.push_str("## Team\n\n");
            md.push_str("| Role | Name | Contact |\n");
            md.push_str("|------|------|--------|\n");
            for member in team {
                let contact = member.contact.as_deref().unwrap_or("-");
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    member.role, member.name, contact
                ));
            }
            md.push('\n');
        }

        // Risk section
        if let Some(risk) = &asset.risk {
            md.push_str("## Risk Management\n\n");
            if let Some(classification) = &risk.classification {
                md.push_str(&format!("**Classification:** {:?}\n\n", classification));
            }
            if let Some(areas) = &risk.impact_areas
                && !areas.is_empty()
            {
                let areas_str: Vec<String> = areas.iter().map(|a| format!("{:?}", a)).collect();
                md.push_str(&format!("**Impact Areas:** {}\n\n", areas_str.join(", ")));
            }
            if let Some(intended) = &risk.intended_use {
                md.push_str(&format!("**Intended Use:** {}\n\n", intended));
            }
            if let Some(out_of_scope) = &risk.out_of_scope_use {
                md.push_str(&format!("**Out of Scope:** {}\n\n", out_of_scope));
            }
            if let Some(mitigations) = &risk.mitigations
                && !mitigations.is_empty()
            {
                md.push_str("**Mitigations:**\n");
                for m in mitigations {
                    md.push_str(&format!("- {} ({:?})\n", m.description, m.status));
                }
                md.push('\n');
            }
        }

        // Compliance section
        if let Some(compliance) = &asset.compliance {
            md.push_str("## Compliance\n\n");
            if let Some(frameworks) = &compliance.frameworks
                && !frameworks.is_empty()
            {
                md.push_str("### Frameworks\n\n");
                md.push_str("| Name | Category | Status |\n");
                md.push_str("|------|----------|--------|\n");
                for fw in frameworks {
                    let cat = fw.category.as_deref().unwrap_or("-");
                    md.push_str(&format!("| {} | {} | {:?} |\n", fw.name, cat, fw.status));
                }
                md.push('\n');
            }
            if let Some(controls) = &compliance.controls
                && !controls.is_empty()
            {
                md.push_str("### Controls\n\n");
                md.push_str("| ID | Description |\n");
                md.push_str("|----|-------------|\n");
                for ctrl in controls {
                    md.push_str(&format!("| {} | {} |\n", ctrl.id, ctrl.description));
                }
                md.push('\n');
            }
        }

        // Tags
        if !asset.tags.is_empty() {
            md.push_str("---\n\n");
            let tag_strings: Vec<String> = asset.tags.iter().map(|t| format!("`{}`", t)).collect();
            md.push_str(&format!("**Tags:** {}\n\n", tag_strings.join(" ")));
        }

        // Timestamps
        if asset.created_at.is_some() || asset.updated_at.is_some() {
            md.push_str("---\n\n");
            if let Some(created) = &asset.created_at {
                md.push_str(&format!(
                    "*Created: {}",
                    created.format("%Y-%m-%d %H:%M UTC")
                ));
                if let Some(updated) = &asset.updated_at {
                    md.push_str(&format!(
                        " | Last Updated: {}",
                        updated.format("%Y-%m-%d %H:%M UTC")
                    ));
                }
                md.push_str("*\n\n");
            } else if let Some(updated) = &asset.updated_at {
                md.push_str(&format!(
                    "*Last Updated: {}*\n\n",
                    updated.format("%Y-%m-%d %H:%M UTC")
                ));
            }
        }

        md
    }

    /// Generate PDF from markdown content
    fn generate_pdf(
        &self,
        title: &str,
        markdown: &str,
        filename: &str,
        doc_type: &str,
    ) -> Result<PdfExportResult, ExportError> {
        let pdf_content = self.create_pdf_document(title, markdown, doc_type)?;
        let pdf_base64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pdf_content);

        // Estimate page count based on content length
        let chars_per_page = 3000;
        let page_count = std::cmp::max(1, (markdown.len() / chars_per_page) as u32 + 1);

        Ok(PdfExportResult {
            pdf_base64,
            filename: filename.to_string(),
            page_count,
            title: title.to_string(),
        })
    }

    /// Create a PDF document with proper GFM rendering and multi-page support
    fn create_pdf_document(
        &self,
        title: &str,
        markdown: &str,
        doc_type: &str,
    ) -> Result<Vec<u8>, ExportError> {
        let (width, height) = self.branding.page_size.dimensions_mm();
        let width_pt = width * 2.83465;
        let height_pt = height * 2.83465;

        // Generate all page content streams
        let page_streams =
            self.render_markdown_to_pdf_pages(title, markdown, width_pt, height_pt, doc_type);
        let page_count = page_streams.len();

        let mut pdf = Vec::new();

        // PDF Header
        pdf.extend_from_slice(b"%PDF-1.4\n");
        pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

        let mut xref_positions: Vec<usize> = Vec::new();

        // Object 1: Catalog
        xref_positions.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages - will be written later after we know all page refs
        let pages_obj_position = xref_positions.len();
        xref_positions.push(0); // Placeholder, will update

        // For each page, we need: Page object + Content stream object
        // Object numbering: 3,4 for page 1; 5,6 for page 2; etc.
        // Then fonts start after all pages
        let mut page_obj_ids: Vec<usize> = Vec::new();
        let font_obj_start = 3 + (page_count * 2); // First font object ID

        for (page_idx, content_stream) in page_streams.iter().enumerate() {
            let page_obj_id = 3 + (page_idx * 2);
            let content_obj_id = page_obj_id + 1;
            page_obj_ids.push(page_obj_id);

            // Page object
            xref_positions.push(pdf.len());
            let page_obj = format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Contents {} 0 R /Resources << /Font << /F1 {} 0 R /F2 {} 0 R >> >> >>\nendobj\n",
                page_obj_id,
                width_pt,
                height_pt,
                content_obj_id,
                font_obj_start,
                font_obj_start + 1
            );
            pdf.extend_from_slice(page_obj.as_bytes());

            // Content stream object
            xref_positions.push(pdf.len());
            let content_obj = format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                content_obj_id,
                content_stream.len(),
                content_stream
            );
            pdf.extend_from_slice(content_obj.as_bytes());
        }

        // Now write the Pages object with correct kids list
        let pages_position = pdf.len();
        let kids_list: Vec<String> = page_obj_ids
            .iter()
            .map(|id| format!("{} 0 R", id))
            .collect();
        let pages_obj = format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids_list.join(" "),
            page_count
        );
        pdf.extend_from_slice(pages_obj.as_bytes());
        xref_positions[pages_obj_position] = pages_position;

        // Font objects
        xref_positions.push(pdf.len());
        let font1_obj = format!(
            "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n",
            font_obj_start
        );
        pdf.extend_from_slice(font1_obj.as_bytes());

        xref_positions.push(pdf.len());
        let font2_obj = format!(
            "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>\nendobj\n",
            font_obj_start + 1
        );
        pdf.extend_from_slice(font2_obj.as_bytes());

        // Info dictionary
        let info_obj_id = font_obj_start + 2;
        xref_positions.push(pdf.len());
        let timestamp = if self.branding.show_timestamp {
            Utc::now().format("D:%Y%m%d%H%M%S").to_string()
        } else {
            String::new()
        };

        let escaped_title = self.escape_pdf_string(title);
        let producer = "Open Data Modelling SDK";
        let company = self
            .branding
            .company_name
            .as_deref()
            .unwrap_or("opendatamodelling.com");

        let info_obj = format!(
            "{} 0 obj\n<< /Title ({}) /Producer ({}) /Creator ({}) /CreationDate ({}) >>\nendobj\n",
            info_obj_id, escaped_title, producer, company, timestamp
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
                "<< /Size {} /Root 1 0 R /Info {} 0 R >>\n",
                xref_positions.len() + 1,
                info_obj_id
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(b"startxref\n");
        pdf.extend_from_slice(format!("{}\n", xref_start).as_bytes());
        pdf.extend_from_slice(b"%%EOF\n");

        Ok(pdf)
    }

    /// Render markdown to PDF content streams with proper formatting (multi-page)
    fn render_markdown_to_pdf_pages(
        &self,
        title: &str,
        markdown: &str,
        width: f64,
        height: f64,
        doc_type: &str,
    ) -> Vec<String> {
        let mut pages: Vec<String> = Vec::new();
        let mut stream = String::new();
        let margin = 50.0;
        let footer_height = 40.0; // Reserve space for footer
        let header_height = 100.0; // Reserve space for header/logo/title/doc type
        let body_font_size = self.branding.font_size as f64;
        let line_height = body_font_size * 1.4;
        let max_width = width - (2.0 * margin);
        let mut page_num = 1;

        // === HEADER SECTION WITH LOGO ===

        // Draw the logo circle with gradient-like effect (blue circle)
        let logo_cx = margin + 15.0;
        let logo_cy = height - margin - 10.0;
        let logo_r = 12.0;

        // Draw filled blue circle
        stream.push_str("q\n");
        stream.push_str("0 0.4 0.8 rg\n"); // RGB for #0066CC
        stream.push_str(&format!("{:.2} {:.2} m\n", logo_cx + logo_r, logo_cy));
        // Approximate circle with bezier curves
        let k = 0.5523; // bezier constant for circle
        stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
            logo_cx + logo_r,
            logo_cy + logo_r * k,
            logo_cx + logo_r * k,
            logo_cy + logo_r,
            logo_cx,
            logo_cy + logo_r
        ));
        stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
            logo_cx - logo_r * k,
            logo_cy + logo_r,
            logo_cx - logo_r,
            logo_cy + logo_r * k,
            logo_cx - logo_r,
            logo_cy
        ));
        stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
            logo_cx - logo_r,
            logo_cy - logo_r * k,
            logo_cx - logo_r * k,
            logo_cy - logo_r,
            logo_cx,
            logo_cy - logo_r
        ));
        stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c\n",
            logo_cx + logo_r * k,
            logo_cy - logo_r,
            logo_cx + logo_r,
            logo_cy - logo_r * k,
            logo_cx + logo_r,
            logo_cy
        ));
        stream.push_str("f\n"); // Fill the circle
        stream.push_str("Q\n");

        // Draw white cross inside the circle
        stream.push_str("q\n");
        stream.push_str("1 1 1 RG\n"); // White stroke
        stream.push_str("2 w\n"); // Line width
        stream.push_str("1 J\n"); // Round line cap
        // Vertical line
        stream.push_str(&format!(
            "{:.2} {:.2} m\n{:.2} {:.2} l\nS\n",
            logo_cx,
            logo_cy - logo_r * 0.6,
            logo_cx,
            logo_cy + logo_r * 0.6
        ));
        // Horizontal line
        stream.push_str(&format!(
            "{:.2} {:.2} m\n{:.2} {:.2} l\nS\n",
            logo_cx - logo_r * 0.6,
            logo_cy,
            logo_cx + logo_r * 0.6,
            logo_cy
        ));
        stream.push_str("Q\n");

        // Render "Open Data Modelling" text next to logo
        stream.push_str("BT\n");
        let logo_text_x = margin + 35.0;
        let logo_text_y = height - margin - 5.0;
        stream.push_str("/F2 11 Tf\n"); // Bold font
        stream.push_str(&format!("{:.2} {:.2} Td\n", logo_text_x, logo_text_y));
        stream.push_str("(Open Data) Tj\n");
        stream.push_str(&format!("0 {:.2} Td\n", -12.0));
        stream.push_str("(Modelling) Tj\n");
        stream.push_str("ET\n");

        // Draw header line below logo
        let header_line_y = height - margin - 30.0;
        stream.push_str(&format!(
            "q\n0.7 G\n{:.2} {:.2} m\n{:.2} {:.2} l\nS\nQ\n",
            margin,
            header_line_y,
            width - margin,
            header_line_y
        ));

        // === DOCUMENT TYPE TITLE (e.g., "DECISION RECORD" or "KNOWLEDGE BASE") ===
        stream.push_str("BT\n");
        let doc_type_y = height - margin - 48.0;
        stream.push_str("/F2 12 Tf\n"); // Bold font for document type
        stream.push_str("0.3 0.3 0.3 rg\n"); // Dark gray color
        stream.push_str(&format!("{:.2} {:.2} Td\n", margin, doc_type_y));
        stream.push_str(&format!(
            "({}) Tj\n",
            self.escape_pdf_string(&doc_type.to_uppercase())
        ));
        stream.push_str("ET\n");

        // === DOCUMENT TITLE ===
        stream.push_str("BT\n");
        stream.push_str("0 0 0 rg\n"); // Black color for title
        let title_y = height - margin - 68.0;
        stream.push_str("/F2 16 Tf\n"); // Bold, large font for title
        stream.push_str(&format!("{:.2} {:.2} Td\n", margin, title_y));
        stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(title)));
        stream.push_str("ET\n");

        // Note: Footer is rendered by render_page_header_footer closure for all pages
        // This ensures consistent footer across all pages including page numbers

        // === BODY CONTENT ===
        let content_top = height - margin - header_height;
        let content_bottom = margin + footer_height;
        let mut y_pos = content_top;
        let mut in_table = false;
        let mut in_code_block = false;

        // Helper closure to render footer on each page
        let render_page_header_footer =
            |stream: &mut String,
             page_num: u32,
             width: f64,
             height: f64,
             margin: f64,
             footer_height: f64| {
                // Draw logo on continuation pages (smaller, simpler)
                if page_num > 1 {
                    // Small logo indicator
                    stream.push_str("BT\n");
                    stream.push_str("/F2 9 Tf\n");
                    stream.push_str("0.3 0.3 0.3 rg\n");
                    stream.push_str(&format!(
                        "1 0 0 1 {:.2} {:.2} Tm\n",
                        margin,
                        height - margin - 10.0
                    ));
                    stream.push_str("(Open Data Modelling) Tj\n");
                    stream.push_str("ET\n");
                }

                // Footer line
                let footer_line_y = margin + footer_height - 10.0;
                stream.push_str(&format!(
                    "q\n0.3 G\n{:.2} {:.2} m\n{:.2} {:.2} l\nS\nQ\n",
                    margin,
                    footer_line_y,
                    width - margin,
                    footer_line_y
                ));

                let footer_y = margin + 15.0;

                // Copyright text on the left (use octal \251 for © symbol)
                stream.push_str("BT\n");
                stream.push_str("/F1 9 Tf\n");
                stream.push_str("0 0 0 rg\n");
                stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", margin, footer_y));
                stream.push_str("(\\251 opendatamodelling.com) Tj\n");
                stream.push_str("ET\n");

                // Page number on the right
                stream.push_str("BT\n");
                stream.push_str("/F1 9 Tf\n");
                stream.push_str("0 0 0 rg\n");
                stream.push_str(&format!(
                    "1 0 0 1 {:.2} {:.2} Tm\n",
                    width - margin - 40.0,
                    footer_y
                ));
                stream.push_str(&format!("(Page {}) Tj\n", page_num));
                stream.push_str("ET\n");
            };

        for line in markdown.lines() {
            // Check if we need a new page
            if y_pos < content_bottom + line_height {
                // Render footer on current page
                render_page_header_footer(
                    &mut stream,
                    page_num,
                    width,
                    height,
                    margin,
                    footer_height,
                );

                // Save current page and start new one
                pages.push(stream);
                stream = String::new();
                page_num += 1;
                y_pos = height - margin - 30.0; // Start content higher on continuation pages
            }

            let trimmed = line.trim();

            // Handle code blocks
            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                y_pos -= line_height * 0.5;
                continue;
            }

            if in_code_block {
                // Draw dark background for code block line
                let code_bg_padding = 3.0;
                let code_line_height = line_height * 0.9;
                stream.push_str("q\n");
                stream.push_str("0.15 0.15 0.15 rg\n"); // Dark gray background (#262626)
                stream.push_str(&format!(
                    "{:.2} {:.2} {:.2} {:.2} re f\n",
                    margin + 15.0,
                    y_pos - code_bg_padding,
                    max_width - 15.0,
                    code_line_height + code_bg_padding
                ));
                stream.push_str("Q\n");

                // Render code with light text using absolute positioning
                stream.push_str("BT\n");
                let code_font_size = body_font_size - 1.0;
                stream.push_str(&format!("/F1 {:.1} Tf\n", code_font_size));
                stream.push_str("0.9 0.9 0.9 rg\n"); // Light gray text for code
                stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", margin + 20.0, y_pos));
                stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(line)));
                stream.push_str("ET\n");
                y_pos -= code_line_height;
                continue;
            }

            // Skip image references and markdown footer (already rendered)
            if trimmed.starts_with("![") {
                continue;
            }

            // Skip the copyright line in content (already in footer)
            if trimmed.starts_with("©") || trimmed == DEFAULT_COPYRIGHT {
                continue;
            }

            // Handle horizontal rules
            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                y_pos -= line_height * 0.3;
                // Draw a line
                stream.push_str(&format!(
                    "q\n0.7 G\n{:.2} {:.2} m\n{:.2} {:.2} l\nS\nQ\n",
                    margin,
                    y_pos,
                    width - margin,
                    y_pos
                ));
                y_pos -= line_height * 0.5;
                continue;
            }

            // Handle table rows
            if trimmed.starts_with("|") && trimmed.ends_with("|") {
                // Skip separator rows
                if trimmed.contains("---") {
                    in_table = true;
                    continue;
                }

                let cells: Vec<&str> = trimmed
                    .trim_matches('|')
                    .split('|')
                    .map(|s| s.trim())
                    .collect();

                let cell_width = max_width / cells.len() as f64;

                // Calculate max chars per cell based on cell width and font size
                let font_size = if in_table {
                    body_font_size - 1.0
                } else {
                    body_font_size
                };
                // Approximate character width for Helvetica
                // Using a conservative factor to ensure text fits
                let char_width_factor = 0.45;
                let max_chars_per_line =
                    ((cell_width - 10.0) / (font_size * char_width_factor)) as usize;
                let max_chars_per_line = max_chars_per_line.max(10); // Minimum 10 chars per line

                // Word-wrap each cell and find maximum number of lines needed
                let mut wrapped_cells: Vec<(Vec<String>, bool)> = Vec::new();
                let mut max_lines = 1usize;

                for cell in &cells {
                    // Check if cell content is bold
                    let (text, is_bold) = if cell.starts_with("**") && cell.ends_with("**") {
                        (cell.trim_matches('*'), true)
                    } else {
                        (*cell, false)
                    };

                    // Word wrap the text
                    let lines = self.word_wrap(text, max_chars_per_line);
                    max_lines = max_lines.max(lines.len());
                    wrapped_cells.push((lines, is_bold));
                }

                // Check if we have enough space for this row
                let row_height = line_height * max_lines as f64;
                if y_pos - row_height < content_bottom {
                    // Need a new page
                    render_page_header_footer(
                        &mut stream,
                        page_num,
                        width,
                        height,
                        margin,
                        footer_height,
                    );
                    pages.push(stream);
                    stream = String::new();
                    page_num += 1;
                    y_pos = height - margin - 30.0;
                }

                // Render each line of each cell
                for line_idx in 0..max_lines {
                    let mut x_pos = margin;
                    let line_y = y_pos - (line_idx as f64 * line_height);

                    for (lines, is_bold) in &wrapped_cells {
                        let font = if *is_bold || !in_table { "/F2" } else { "/F1" };
                        let text = lines.get(line_idx).map(|s| s.as_str()).unwrap_or("");

                        if !text.is_empty() {
                            stream.push_str("BT\n");
                            stream.push_str(&format!("{} {:.1} Tf\n", font, font_size));
                            stream.push_str("0 0 0 rg\n");
                            stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", x_pos, line_y));
                            stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(text)));
                            stream.push_str("ET\n");
                        }
                        x_pos += cell_width;
                    }
                }

                y_pos -= row_height + (line_height * 0.2); // Add small padding between rows
                in_table = true;
                continue;
            } else if in_table && !trimmed.is_empty() {
                in_table = false;
                y_pos -= line_height * 0.3;
            }

            // Handle headings - Skip H1 since we render the title in the header
            if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
                // Skip the main H1 title as it's already in the header
                continue;
            }

            if trimmed.starts_with("## ") {
                let text = trimmed.trim_start_matches("## ");
                let h2_size = body_font_size + 3.0;

                // Check if we have enough space for heading + at least 4 lines of content
                // If not, start a new page to keep section together
                let min_section_space = line_height * 5.0;
                if y_pos - min_section_space < content_bottom {
                    render_page_header_footer(
                        &mut stream,
                        page_num,
                        width,
                        height,
                        margin,
                        footer_height,
                    );
                    pages.push(stream);
                    stream = String::new();
                    page_num += 1;
                    y_pos = height - margin - 30.0;
                }

                y_pos -= line_height * 0.3;
                stream.push_str("BT\n");
                stream.push_str(&format!("/F2 {:.1} Tf\n", h2_size));
                stream.push_str("0 0 0 rg\n");
                stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", margin, y_pos));
                stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(text)));
                stream.push_str("ET\n");
                y_pos -= line_height * 1.2;
                continue;
            }

            if trimmed.starts_with("### ") {
                let text = trimmed.trim_start_matches("### ");

                // Check if we have enough space for subheading + at least 3 lines of content
                let min_subsection_space = line_height * 4.0;
                if y_pos - min_subsection_space < content_bottom {
                    render_page_header_footer(
                        &mut stream,
                        page_num,
                        width,
                        height,
                        margin,
                        footer_height,
                    );
                    pages.push(stream);
                    stream = String::new();
                    page_num += 1;
                    y_pos = height - margin - 30.0;
                }
                let h3_size = body_font_size + 1.0;
                stream.push_str("BT\n");
                stream.push_str(&format!("/F2 {:.1} Tf\n", h3_size));
                stream.push_str("0 0 0 rg\n");
                stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", margin, y_pos));
                stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(text)));
                stream.push_str("ET\n");
                y_pos -= line_height * 1.1;
                continue;
            }

            // Handle list items
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let text = trimmed[2..].to_string();
                stream.push_str("BT\n");
                stream.push_str(&format!("/F1 {:.1} Tf\n", body_font_size));
                stream.push_str("0 0 0 rg\n");
                stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", margin + 10.0, y_pos));
                stream.push_str(&format!(
                    "(\\267 {}) Tj\n",
                    self.escape_pdf_string(&self.strip_markdown_formatting(&text))
                ));
                stream.push_str("ET\n");
                y_pos -= line_height;
                continue;
            }

            // Handle numbered list items
            if let Some(rest) = self.parse_numbered_list(trimmed) {
                stream.push_str("BT\n");
                stream.push_str(&format!("/F1 {:.1} Tf\n", body_font_size));
                stream.push_str("0 0 0 rg\n");
                stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", margin + 10.0, y_pos));
                stream.push_str(&format!(
                    "({}) Tj\n",
                    self.escape_pdf_string(&self.strip_markdown_formatting(rest))
                ));
                stream.push_str("ET\n");
                y_pos -= line_height;
                continue;
            }

            // Handle empty lines
            if trimmed.is_empty() {
                y_pos -= line_height * 0.5;
                continue;
            }

            // Handle italic text (*text*)
            let display_text = self.strip_markdown_formatting(trimmed);

            // Check if this is bold text
            let (text, font) = if trimmed.starts_with("**") && trimmed.ends_with("**") {
                (display_text.as_str(), "/F2")
            } else if trimmed.starts_with("*")
                && trimmed.ends_with("*")
                && !trimmed.starts_with("**")
            {
                // Italic - we don't have an italic font, so just use regular
                (display_text.as_str(), "/F1")
            } else {
                (display_text.as_str(), "/F1")
            };

            // Word wrap regular text
            let wrapped_lines = self.word_wrap(text, (max_width / (body_font_size * 0.5)) as usize);
            for wrapped_line in wrapped_lines {
                // Check for page break
                if y_pos < content_bottom + line_height {
                    render_page_header_footer(
                        &mut stream,
                        page_num,
                        width,
                        height,
                        margin,
                        footer_height,
                    );
                    pages.push(stream);
                    stream = String::new();
                    page_num += 1;
                    y_pos = height - margin - 30.0;
                }
                stream.push_str("BT\n");
                stream.push_str(&format!("{} {:.1} Tf\n", font, body_font_size));
                stream.push_str("0 0 0 rg\n");
                stream.push_str(&format!("1 0 0 1 {:.2} {:.2} Tm\n", margin, y_pos));
                stream.push_str(&format!("({}) Tj\n", self.escape_pdf_string(&wrapped_line)));
                stream.push_str("ET\n");
                y_pos -= line_height;
            }
        }

        // Render footer on last page and add it to pages
        render_page_header_footer(&mut stream, page_num, width, height, margin, footer_height);
        pages.push(stream);

        pages
    }

    /// Strip markdown formatting from text for display
    fn strip_markdown_formatting(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove bold markers
        while result.contains("**") {
            result = result.replacen("**", "", 2);
        }

        // Remove italic markers (single asterisk)
        // Be careful not to remove list markers
        let chars: Vec<char> = result.chars().collect();
        let mut cleaned = String::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] != '*' && chars[i + 1] != ' '
            {
                // This might be italic start
                if result[i + 1..].contains('*') {
                    // Skip the asterisk
                    i += 1;
                    continue;
                }
            }
            cleaned.push(chars[i]);
            i += 1;
        }

        // Remove backticks (inline code)
        result = cleaned.replace('`', "");

        // Remove link formatting [text](url) -> text
        while let Some(start) = result.find('[') {
            if let Some(mid) = result[start..].find("](")
                && let Some(end) = result[start + mid..].find(')')
            {
                let link_text = &result[start + 1..start + mid];
                let before = &result[..start];
                let after = &result[start + mid + end + 1..];
                result = format!("{}{}{}", before, link_text, after);
                continue;
            }
            break;
        }

        result
    }

    /// Parse numbered list item, returns the text after the number
    fn parse_numbered_list<'a>(&self, text: &'a str) -> Option<&'a str> {
        let bytes = text.as_bytes();
        let mut i = 0;

        // Skip digits
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }

        // Check for period and space
        if i > 0 && i < bytes.len() - 1 && bytes[i] == b'.' && bytes[i + 1] == b' ' {
            return Some(&text[i + 2..]);
        }

        None
    }

    /// Escape special characters for PDF strings
    fn escape_pdf_string(&self, s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                '\\' => result.push_str("\\\\"),
                '(' => result.push_str("\\("),
                ')' => result.push_str("\\)"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                // Handle special characters by using octal encoding
                '©' => result.push_str("\\251"), // Copyright symbol in WinAnsiEncoding
                '®' => result.push_str("\\256"), // Registered trademark
                '™' => result.push_str("\\231"), // Trademark
                '•' => result.push_str("\\267"), // Bullet
                '–' => result.push_str("\\226"), // En dash
                '—' => result.push_str("\\227"), // Em dash
                _ if c.is_ascii() => result.push(c),
                // For non-ASCII characters, try to use closest ASCII equivalent
                _ => result.push('?'),
            }
        }
        result
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
        assert_eq!(config.logo_url, Some(DEFAULT_LOGO_URL.to_string()));
        assert_eq!(config.footer, Some(DEFAULT_COPYRIGHT.to_string()));
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
    fn test_export_table_to_pdf() {
        use crate::models::{Column, Table};

        let mut table = Table::new(
            "users".to_string(),
            vec![
                Column::new("id".to_string(), "BIGINT".to_string()),
                Column::new("name".to_string(), "VARCHAR(255)".to_string()),
                Column::new("email".to_string(), "VARCHAR(255)".to_string()),
            ],
        );
        table.schema_name = Some("public".to_string());
        table.owner = Some("Data Engineering".to_string());
        table.notes = Some("Core user table for the application".to_string());

        let exporter = PdfExporter::new();
        let result = exporter.export_table(&table);
        assert!(result.is_ok());

        let pdf_result = result.unwrap();
        assert!(!pdf_result.pdf_base64.is_empty());
        assert!(pdf_result.filename.ends_with(".pdf"));
        assert_eq!(pdf_result.title, "users");
    }

    #[test]
    fn test_export_data_product_to_pdf() {
        use crate::models::odps::{ODPSDataProduct, ODPSDescription, ODPSOutputPort, ODPSStatus};

        let product = ODPSDataProduct {
            api_version: "v1.0.0".to_string(),
            kind: "DataProduct".to_string(),
            id: "dp-customer-360".to_string(),
            name: Some("Customer 360".to_string()),
            version: Some("1.0.0".to_string()),
            status: ODPSStatus::Active,
            domain: Some("Customer".to_string()),
            tenant: None,
            authoritative_definitions: None,
            description: Some(ODPSDescription {
                purpose: Some("Unified customer view across all touchpoints".to_string()),
                limitations: Some("Does not include real-time data".to_string()),
                usage: Some("Use for analytics and reporting".to_string()),
                authoritative_definitions: None,
                custom_properties: None,
            }),
            custom_properties: None,
            tags: vec![],
            input_ports: None,
            output_ports: Some(vec![ODPSOutputPort {
                name: "customer-data".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Customer master data".to_string()),
                r#type: Some("table".to_string()),
                contract_id: Some("contract-123".to_string()),
                sbom: None,
                input_contracts: None,
                tags: vec![],
                custom_properties: None,
                authoritative_definitions: None,
            }]),
            management_ports: None,
            support: None,
            team: None,
            product_created_ts: None,
            created_at: None,
            updated_at: None,
        };

        let exporter = PdfExporter::new();
        let result = exporter.export_data_product(&product);
        assert!(result.is_ok());

        let pdf_result = result.unwrap();
        assert!(!pdf_result.pdf_base64.is_empty());
        assert!(pdf_result.filename.ends_with(".pdf"));
        assert_eq!(pdf_result.title, "Customer 360");
    }

    #[test]
    fn test_export_cads_asset_to_pdf() {
        use crate::models::cads::{
            CADSAsset, CADSDescription, CADSKind, CADSStatus, CADSTeamMember,
        };

        let asset = CADSAsset {
            api_version: "v1.0".to_string(),
            kind: CADSKind::AIModel,
            id: "model-sentiment-v1".to_string(),
            name: "Sentiment Analysis Model".to_string(),
            version: "1.0.0".to_string(),
            status: CADSStatus::Production,
            domain: Some("NLP".to_string()),
            tags: vec![],
            description: Some(CADSDescription {
                purpose: Some("Analyze sentiment in customer feedback".to_string()),
                usage: Some("Call the /predict endpoint with text input".to_string()),
                limitations: Some("English language only".to_string()),
                external_links: None,
            }),
            runtime: None,
            sla: None,
            pricing: None,
            team: Some(vec![CADSTeamMember {
                role: "Owner".to_string(),
                name: "ML Team".to_string(),
                contact: Some("ml-team@example.com".to_string()),
            }]),
            risk: None,
            compliance: None,
            validation_profiles: None,
            bpmn_models: None,
            dmn_models: None,
            openapi_specs: None,
            custom_properties: None,
            created_at: None,
            updated_at: None,
        };

        let exporter = PdfExporter::new();
        let result = exporter.export_cads_asset(&asset);
        assert!(result.is_ok());

        let pdf_result = result.unwrap();
        assert!(!pdf_result.pdf_base64.is_empty());
        assert!(pdf_result.filename.ends_with(".pdf"));
        assert_eq!(pdf_result.title, "Sentiment Analysis Model");
    }

    #[test]
    fn test_export_markdown_to_pdf() {
        let exporter = PdfExporter::new();
        let result = exporter.export_markdown(
            "Test Document",
            "# Test\n\nThis is a test document.\n\n## Section\n\n- Item 1\n- Item 2",
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
        assert_eq!(exporter.escape_pdf_string("Hello"), "Hello");
        assert_eq!(exporter.escape_pdf_string("(test)"), "\\(test\\)");
        assert_eq!(exporter.escape_pdf_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_word_wrap() {
        let exporter = PdfExporter::new();

        let wrapped = exporter.word_wrap("Hello world this is a test", 10);
        assert!(wrapped.len() > 1);

        let wrapped = exporter.word_wrap("Short", 100);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "Short");
    }

    #[test]
    fn test_pdf_result_serialization() {
        let result = PdfExportResult {
            pdf_base64: "dGVzdA==".to_string(),
            filename: "test.pdf".to_string(),
            page_count: 1,
            title: "Test".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("pdf_base64"));
        assert!(json.contains("filename"));
    }

    #[test]
    fn test_strip_markdown_formatting() {
        let exporter = PdfExporter::new();
        assert_eq!(exporter.strip_markdown_formatting("**bold**"), "bold");
        assert_eq!(exporter.strip_markdown_formatting("`code`"), "code");
        assert_eq!(
            exporter.strip_markdown_formatting("[link](http://example.com)"),
            "link"
        );
    }

    /// Generate sample PDFs for visual inspection (writes to /tmp)
    /// Run with: cargo test generate_sample_pdfs_for_inspection -- --ignored --nocapture
    #[test]
    #[ignore]
    fn generate_sample_pdfs_for_inspection() {
        use crate::models::decision::DecisionOption;
        use base64::Engine;

        // Create a Decision with rich content including options with pros/cons
        let mut decision = Decision::new(
            2501100001,
            "Use Rust for SDK Implementation",
            "We need to choose a programming language for the SDK implementation.\n\nKey requirements:\n- Type safety\n- Performance\n- Cross-platform compilation\n- WASM support\n\nThe decision will impact the entire development team and future maintenance of the codebase. We need to carefully consider all options before making a final choice.",
            "We will use Rust as the primary programming language.\n\nRust provides:\n1. Strong type safety through its ownership system\n2. Excellent performance comparable to C/C++\n3. Cross-platform compilation via LLVM\n4. First-class WASM support\n\nThis decision was made after careful evaluation of all alternatives and considering the long-term maintainability of the project.",
        );

        // Add options with pros and cons
        decision.options = vec![
            DecisionOption::with_details(
                "Rust",
                "A systems programming language focused on safety and performance.",
                vec![
                    "Memory safety without garbage collection".to_string(),
                    "Excellent performance".to_string(),
                    "Strong type system".to_string(),
                    "First-class WASM support".to_string(),
                    "Growing ecosystem".to_string(),
                ],
                vec![
                    "Steeper learning curve".to_string(),
                    "Longer compilation times".to_string(),
                    "Smaller talent pool".to_string(),
                ],
                true, // selected
            ),
            DecisionOption::with_details(
                "TypeScript",
                "A typed superset of JavaScript.",
                vec![
                    "Large developer community".to_string(),
                    "Easy to learn".to_string(),
                    "Good tooling".to_string(),
                ],
                vec![
                    "Runtime type checking only".to_string(),
                    "Performance limitations".to_string(),
                    "Node.js dependency".to_string(),
                ],
                false,
            ),
            DecisionOption::with_details(
                "Go",
                "A statically typed language designed at Google.",
                vec![
                    "Simple syntax".to_string(),
                    "Fast compilation".to_string(),
                    "Good concurrency support".to_string(),
                ],
                vec![
                    "Limited generics".to_string(),
                    "No WASM support".to_string(),
                    "Verbose error handling".to_string(),
                ],
                false,
            ),
        ];

        decision.consequences =
            Some("This decision will have significant impact on the project.".to_string());

        // Debug: print the generated markdown to see what's being rendered
        let exporter = PdfExporter::new();
        let md = exporter.decision_to_markdown(&decision);
        println!("Generated markdown length: {} chars", md.len());
        println!(
            "Contains 'Options Considered': {}",
            md.contains("Options Considered")
        );
        println!("Contains 'Pros': {}", md.contains("Pros"));

        // Create a Knowledge Article with code blocks
        let article = KnowledgeArticle::new(
            2501100001,
            "Getting Started with the SDK",
            "A comprehensive guide to getting started with the Open Data Modelling SDK.",
            r#"## Installation

Install the SDK using cargo:

```bash
cargo add data-modelling-sdk
```

## Basic Usage

Here's a simple example:

```rust
use data_modelling_sdk::models::decision::Decision;

fn main() {
    let decision = Decision::new(
        1,
        "Use microservices",
        "Context here",
        "Decision here",
    );
    println!("Created: {}", decision.title);
}
```

## Configuration

Configure using YAML:

```yaml
sdk:
  log_level: info
  storage_path: ./data
```

For more information, see the documentation."#,
            "docs@opendatamodelling.com",
        );

        let exporter = PdfExporter::new();

        // Export Decision
        let result = exporter.export_decision(&decision).unwrap();
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(&result.pdf_base64)
            .unwrap();
        std::fs::write("/tmp/sample_decision.pdf", &pdf_bytes).unwrap();
        println!("Wrote /tmp/sample_decision.pdf ({} bytes)", pdf_bytes.len());

        // Export Knowledge Article
        let result = exporter.export_knowledge(&article).unwrap();
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(&result.pdf_base64)
            .unwrap();
        std::fs::write("/tmp/sample_knowledge.pdf", &pdf_bytes).unwrap();
        println!(
            "Wrote /tmp/sample_knowledge.pdf ({} bytes)",
            pdf_bytes.len()
        );

        // Export Data Contract (Table)
        use crate::models::{Column, Table};

        let mut table = Table::new(
            "customer_orders".to_string(),
            vec![
                {
                    let mut col = Column::new("order_id".to_string(), "BIGINT".to_string());
                    col.primary_key = true;
                    col.description = "Unique identifier for each order".to_string();
                    col
                },
                {
                    let mut col = Column::new("customer_id".to_string(), "BIGINT".to_string());
                    col.description = "Foreign key reference to customers table".to_string();
                    col
                },
                {
                    let mut col = Column::new("order_date".to_string(), "TIMESTAMP".to_string());
                    col.description = "Date and time when the order was placed".to_string();
                    col.nullable = false;
                    col
                },
                {
                    let mut col = Column::new("status".to_string(), "VARCHAR(50)".to_string());
                    col.description = "Current status of the order".to_string();
                    col.enum_values = vec![
                        "pending".to_string(),
                        "processing".to_string(),
                        "shipped".to_string(),
                        "delivered".to_string(),
                        "cancelled".to_string(),
                    ];
                    col.business_name = Some("Order Status".to_string());
                    col
                },
                {
                    let mut col =
                        Column::new("total_amount".to_string(), "DECIMAL(10,2)".to_string());
                    col.description = "Total order amount in USD".to_string();
                    col
                },
            ],
        );
        table.schema_name = Some("sales".to_string());
        table.catalog_name = Some("production".to_string());
        table.owner = Some("Data Engineering Team".to_string());
        table.notes = Some("Contains all customer orders including historical data. This table is partitioned by order_date for query performance. Updated daily via ETL pipeline.".to_string());

        // Add ODCS metadata to test full export
        table
            .odcl_metadata
            .insert("apiVersion".to_string(), serde_json::json!("v3.0.2"));
        table
            .odcl_metadata
            .insert("kind".to_string(), serde_json::json!("DataContract"));
        table
            .odcl_metadata
            .insert("status".to_string(), serde_json::json!("active"));
        table
            .odcl_metadata
            .insert("version".to_string(), serde_json::json!("1.2.0"));
        table
            .odcl_metadata
            .insert("domain".to_string(), serde_json::json!("Sales"));
        table.odcl_metadata.insert(
            "dataProduct".to_string(),
            serde_json::json!("Customer Orders Analytics"),
        );

        // Add SLA information
        use crate::models::table::SlaProperty;
        table.sla = Some(vec![
            SlaProperty {
                property: "availability".to_string(),
                value: serde_json::json!("99.9"),
                unit: "%".to_string(),
                element: None,
                driver: Some("operational".to_string()),
                description: Some("Guaranteed uptime for data access".to_string()),
                scheduler: None,
                schedule: None,
            },
            SlaProperty {
                property: "freshness".to_string(),
                value: serde_json::json!(24),
                unit: "hours".to_string(),
                element: None,
                driver: Some("analytics".to_string()),
                description: Some("Maximum data staleness".to_string()),
                scheduler: None,
                schedule: None,
            },
        ]);

        // Add contact details
        use crate::models::table::ContactDetails;
        table.contact_details = Some(ContactDetails {
            name: Some("John Smith".to_string()),
            email: Some("john.smith@example.com".to_string()),
            role: Some("Data Steward".to_string()),
            phone: Some("+1-555-0123".to_string()),
            other: None,
        });

        let result = exporter.export_table(&table).unwrap();
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(&result.pdf_base64)
            .unwrap();
        std::fs::write("/tmp/sample_table.pdf", &pdf_bytes).unwrap();
        println!("Wrote /tmp/sample_table.pdf ({} bytes)", pdf_bytes.len());

        // Export Data Product (ODPS)
        use crate::models::odps::{
            ODPSDataProduct, ODPSDescription, ODPSInputPort, ODPSOutputPort, ODPSStatus,
            ODPSSupport, ODPSTeam, ODPSTeamMember,
        };

        let product = ODPSDataProduct {
            api_version: "v1.0.0".to_string(),
            kind: "DataProduct".to_string(),
            id: "dp-customer-360-view".to_string(),
            name: Some("Customer 360 View".to_string()),
            version: Some("2.1.0".to_string()),
            status: ODPSStatus::Active,
            domain: Some("Customer Intelligence".to_string()),
            tenant: Some("ACME Corp".to_string()),
            authoritative_definitions: None,
            description: Some(ODPSDescription {
                purpose: Some("Provides a unified 360-degree view of customers by aggregating data from multiple sources including CRM, transactions, support tickets, and marketing interactions.".to_string()),
                limitations: Some("Data is refreshed daily at 2 AM UTC. Real-time updates are not supported. Historical data is retained for 7 years.".to_string()),
                usage: Some("Use this data product for customer analytics, segmentation, personalization, and churn prediction models.".to_string()),
                authoritative_definitions: None,
                custom_properties: None,
            }),
            custom_properties: None,
            tags: vec![],
            input_ports: Some(vec![
                ODPSInputPort {
                    name: "crm-contacts".to_string(),
                    version: "1.0.0".to_string(),
                    contract_id: "contract-crm-001".to_string(),
                    tags: vec![],
                    custom_properties: None,
                    authoritative_definitions: None,
                },
                ODPSInputPort {
                    name: "transaction-history".to_string(),
                    version: "2.0.0".to_string(),
                    contract_id: "contract-txn-002".to_string(),
                    tags: vec![],
                    custom_properties: None,
                    authoritative_definitions: None,
                },
            ]),
            output_ports: Some(vec![
                ODPSOutputPort {
                    name: "customer-profile".to_string(),
                    version: "2.1.0".to_string(),
                    description: Some("Unified customer profile with demographics, preferences, and behavioral scores".to_string()),
                    r#type: Some("table".to_string()),
                    contract_id: Some("contract-profile-001".to_string()),
                    sbom: None,
                    input_contracts: None,
                    tags: vec![],
                    custom_properties: None,
                    authoritative_definitions: None,
                },
                ODPSOutputPort {
                    name: "customer-segments".to_string(),
                    version: "1.5.0".to_string(),
                    description: Some("Customer segmentation based on RFM analysis and behavioral clustering".to_string()),
                    r#type: Some("table".to_string()),
                    contract_id: Some("contract-segments-001".to_string()),
                    sbom: None,
                    input_contracts: None,
                    tags: vec![],
                    custom_properties: None,
                    authoritative_definitions: None,
                },
            ]),
            management_ports: None,
            support: Some(vec![ODPSSupport {
                channel: "Slack".to_string(),
                url: "https://acme.slack.com/channels/customer-data".to_string(),
                description: Some("Primary support channel for data product questions".to_string()),
                tool: Some("Slack".to_string()),
                scope: None,
                invitation_url: None,
                tags: vec![],
                custom_properties: None,
                authoritative_definitions: None,
            }]),
            team: Some(ODPSTeam {
                name: Some("Customer Data Team".to_string()),
                description: Some("Responsible for customer data products and analytics".to_string()),
                members: Some(vec![
                    ODPSTeamMember {
                        username: "john.doe@acme.com".to_string(),
                        name: Some("John Doe".to_string()),
                        role: Some("Product Owner".to_string()),
                        description: None,
                        date_in: None,
                        date_out: None,
                        replaced_by_username: None,
                        tags: vec![],
                        custom_properties: None,
                        authoritative_definitions: None,
                    },
                    ODPSTeamMember {
                        username: "jane.smith@acme.com".to_string(),
                        name: Some("Jane Smith".to_string()),
                        role: Some("Data Engineer".to_string()),
                        description: None,
                        date_in: None,
                        date_out: None,
                        replaced_by_username: None,
                        tags: vec![],
                        custom_properties: None,
                        authoritative_definitions: None,
                    },
                ]),
                tags: vec![],
                custom_properties: None,
                authoritative_definitions: None,
            }),
            product_created_ts: None,
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        let result = exporter.export_data_product(&product).unwrap();
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(&result.pdf_base64)
            .unwrap();
        std::fs::write("/tmp/sample_data_product.pdf", &pdf_bytes).unwrap();
        println!(
            "Wrote /tmp/sample_data_product.pdf ({} bytes)",
            pdf_bytes.len()
        );

        // Export CADS Asset
        use crate::models::cads::{
            CADSAsset, CADSDescription, CADSImpactArea, CADSKind, CADSRisk, CADSRiskClassification,
            CADSRuntime, CADSRuntimeResources, CADSStatus, CADSTeamMember,
        };

        let asset = CADSAsset {
            api_version: "v1.0".to_string(),
            kind: CADSKind::AIModel,
            id: "urn:cads:ai-model:sentiment-analysis:v2".to_string(),
            name: "Customer Sentiment Analysis Model".to_string(),
            version: "2.3.1".to_string(),
            status: CADSStatus::Production,
            domain: Some("Natural Language Processing".to_string()),
            tags: vec![],
            description: Some(CADSDescription {
                purpose: Some("Analyzes customer feedback, reviews, and support tickets to determine sentiment polarity (positive, negative, neutral) and emotion categories.".to_string()),
                usage: Some("Send text via REST API to /v2/predict endpoint. Supports batch processing up to 100 items per request.".to_string()),
                limitations: Some("English language only. Maximum 5000 characters per text input. Not suitable for sarcasm detection.".to_string()),
                external_links: None,
            }),
            runtime: Some(CADSRuntime {
                environment: Some("Kubernetes".to_string()),
                endpoints: Some(vec![
                    "https://api.example.com/ml/sentiment/v2".to_string(),
                ]),
                container: None,
                resources: Some(CADSRuntimeResources {
                    cpu: Some("4 cores".to_string()),
                    memory: Some("16 GB".to_string()),
                    gpu: Some("1x NVIDIA T4".to_string()),
                }),
            }),
            sla: None,
            pricing: None,
            team: Some(vec![
                CADSTeamMember {
                    role: "Model Owner".to_string(),
                    name: "Dr. Sarah Chen".to_string(),
                    contact: Some("sarah.chen@example.com".to_string()),
                },
                CADSTeamMember {
                    role: "ML Engineer".to_string(),
                    name: "Alex Kumar".to_string(),
                    contact: Some("alex.kumar@example.com".to_string()),
                },
            ]),
            risk: Some(CADSRisk {
                classification: Some(CADSRiskClassification::Medium),
                impact_areas: Some(vec![CADSImpactArea::Fairness, CADSImpactArea::Privacy]),
                intended_use: Some("Analyzing customer sentiment for product improvement and support prioritization".to_string()),
                out_of_scope_use: Some("Medical diagnosis, legal decisions, credit scoring".to_string()),
                assessment: None,
                mitigations: None,
            }),
            compliance: None,
            validation_profiles: None,
            bpmn_models: None,
            dmn_models: None,
            openapi_specs: None,
            custom_properties: None,
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        let result = exporter.export_cads_asset(&asset).unwrap();
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(&result.pdf_base64)
            .unwrap();
        std::fs::write("/tmp/sample_cads_asset.pdf", &pdf_bytes).unwrap();
        println!(
            "Wrote /tmp/sample_cads_asset.pdf ({} bytes)",
            pdf_bytes.len()
        );
    }
}
