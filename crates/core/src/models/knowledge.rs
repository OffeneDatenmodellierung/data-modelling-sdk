//! Knowledge Base model for domain-partitioned knowledge articles
//!
//! Implements the Knowledge Base (KB) feature for storing and organizing
//! documentation, guides, standards, and other knowledge resources.
//!
//! ## File Format
//!
//! Knowledge articles are stored as `.kb.yaml` files following the naming convention:
//! `{workspace}_{domain}_kb-{number}.kb.yaml`
//!
//! ## Example
//!
//! ```yaml
//! id: 660e8400-e29b-41d4-a716-446655440000
//! number: "KB-0001"
//! title: "Data Classification Guide"
//! article_type: guide
//! status: published
//! domain: sales
//! summary: |
//!   This guide explains data classification.
//! content: |
//!   ## Overview
//!   Data classification is essential...
//! author: data-governance@company.com
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Tag;
use super::decision::AssetLink;

/// Knowledge article type
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeType {
    /// How-to guide or tutorial
    #[default]
    Guide,
    /// Standard or specification
    Standard,
    /// Reference documentation
    Reference,
    /// Step-by-step how-to
    HowTo,
    /// Troubleshooting guide
    Troubleshooting,
    /// Policy document
    Policy,
    /// Template or boilerplate
    Template,
    /// Conceptual documentation
    Concept,
    /// Runbook for operations
    Runbook,
    /// Tutorial (step-by-step learning)
    Tutorial,
    /// Glossary of terms
    Glossary,
}

impl std::fmt::Display for KnowledgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeType::Guide => write!(f, "Guide"),
            KnowledgeType::Standard => write!(f, "Standard"),
            KnowledgeType::Reference => write!(f, "Reference"),
            KnowledgeType::HowTo => write!(f, "How-To"),
            KnowledgeType::Troubleshooting => write!(f, "Troubleshooting"),
            KnowledgeType::Policy => write!(f, "Policy"),
            KnowledgeType::Template => write!(f, "Template"),
            KnowledgeType::Concept => write!(f, "Concept"),
            KnowledgeType::Runbook => write!(f, "Runbook"),
            KnowledgeType::Tutorial => write!(f, "Tutorial"),
            KnowledgeType::Glossary => write!(f, "Glossary"),
        }
    }
}

/// Knowledge article status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeStatus {
    /// Article is being drafted
    #[default]
    Draft,
    /// Article is under review
    Review,
    /// Article is published and active
    Published,
    /// Article is archived (historical reference)
    Archived,
    /// Article is deprecated (should not be used)
    Deprecated,
}

impl std::fmt::Display for KnowledgeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeStatus::Draft => write!(f, "Draft"),
            KnowledgeStatus::Review => write!(f, "Review"),
            KnowledgeStatus::Published => write!(f, "Published"),
            KnowledgeStatus::Archived => write!(f, "Archived"),
            KnowledgeStatus::Deprecated => write!(f, "Deprecated"),
        }
    }
}

/// Review frequency for knowledge articles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewFrequency {
    /// Review monthly
    Monthly,
    /// Review quarterly
    Quarterly,
    /// Review yearly
    Yearly,
}

impl std::fmt::Display for ReviewFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewFrequency::Monthly => write!(f, "Monthly"),
            ReviewFrequency::Quarterly => write!(f, "Quarterly"),
            ReviewFrequency::Yearly => write!(f, "Yearly"),
        }
    }
}

/// Skill level required for the article
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillLevel {
    /// For beginners
    Beginner,
    /// For intermediate users
    Intermediate,
    /// For advanced users
    Advanced,
}

impl std::fmt::Display for SkillLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillLevel::Beginner => write!(f, "Beginner"),
            SkillLevel::Intermediate => write!(f, "Intermediate"),
            SkillLevel::Advanced => write!(f, "Advanced"),
        }
    }
}

/// Relationship type between articles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ArticleRelationship {
    /// General related article
    Related,
    /// Prerequisite article (should be read first)
    Prerequisite,
    /// This article supersedes the referenced one
    Supersedes,
}

impl std::fmt::Display for ArticleRelationship {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArticleRelationship::Related => write!(f, "Related"),
            ArticleRelationship::Prerequisite => write!(f, "Prerequisite"),
            ArticleRelationship::Supersedes => write!(f, "Supersedes"),
        }
    }
}

/// Reference to a related article
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelatedArticle {
    /// UUID of the related article
    #[serde(alias = "article_id")]
    pub article_id: Uuid,
    /// Article number (e.g., "KB-0001")
    #[serde(alias = "article_number")]
    pub article_number: String,
    /// Article title
    pub title: String,
    /// Type of relationship
    pub relationship: ArticleRelationship,
}

impl RelatedArticle {
    /// Create a new related article reference
    pub fn new(
        article_id: Uuid,
        article_number: impl Into<String>,
        title: impl Into<String>,
        relationship: ArticleRelationship,
    ) -> Self {
        Self {
            article_id,
            article_number: article_number.into(),
            title: title.into(),
            relationship,
        }
    }
}

/// Custom deserializer for knowledge article number that supports both:
/// - Legacy string format: "KB-0001"
/// - New numeric format: 1 or 2601101234 (timestamp)
fn deserialize_knowledge_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct NumberVisitor;

    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number or a string like 'KB-0001'")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value >= 0 {
                Ok(value as u64)
            } else {
                Err(E::custom("negative numbers are not allowed"))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            // Handle "KB-0001" format
            let num_str = value
                .to_uppercase()
                .strip_prefix("KB-")
                .map(|s| s.to_string())
                .unwrap_or_else(|| value.to_string());

            num_str
                .parse::<u64>()
                .map_err(|_| E::custom(format!("invalid knowledge number format: {}", value)))
        }
    }

    deserializer.deserialize_any(NumberVisitor)
}

/// Knowledge Base Article
///
/// Represents a knowledge article that can be categorized by domain,
/// type, and audience.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeArticle {
    /// Unique identifier for the article
    pub id: Uuid,
    /// Article number - can be sequential (1, 2, 3) or timestamp-based (YYMMDDHHmm format)
    /// Timestamp format prevents merge conflicts in distributed Git workflows
    #[serde(deserialize_with = "deserialize_knowledge_number")]
    pub number: u64,
    /// Article title
    pub title: String,
    /// Type of article
    #[serde(alias = "article_type")]
    pub article_type: KnowledgeType,
    /// Publication status
    pub status: KnowledgeStatus,
    /// Domain this article belongs to (optional, string name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Domain UUID reference (optional)
    #[serde(skip_serializing_if = "Option::is_none", alias = "domain_id")]
    pub domain_id: Option<Uuid>,
    /// Workspace UUID reference (optional)
    #[serde(skip_serializing_if = "Option::is_none", alias = "workspace_id")]
    pub workspace_id: Option<Uuid>,

    // Content
    /// Brief summary of the article
    pub summary: String,
    /// Full article content in Markdown
    pub content: String,

    // Authorship
    /// Article authors (emails or names) - changed from single author to array
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// List of reviewers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewers: Vec<String>,
    /// Date of last review (legacy field name)
    #[serde(skip_serializing_if = "Option::is_none", alias = "last_reviewed")]
    pub last_reviewed: Option<DateTime<Utc>>,
    /// Last review timestamp (camelCase alias)
    #[serde(skip_serializing_if = "Option::is_none", alias = "reviewed_at")]
    pub reviewed_at: Option<DateTime<Utc>>,
    /// When the article was published
    #[serde(skip_serializing_if = "Option::is_none", alias = "published_at")]
    pub published_at: Option<DateTime<Utc>>,
    /// When the article was archived
    #[serde(skip_serializing_if = "Option::is_none", alias = "archived_at")]
    pub archived_at: Option<DateTime<Utc>>,
    /// How often the article should be reviewed
    #[serde(skip_serializing_if = "Option::is_none", alias = "review_frequency")]
    pub review_frequency: Option<ReviewFrequency>,

    // Classification
    /// Target audience for the article
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<String>,
    /// Required skill level
    #[serde(skip_serializing_if = "Option::is_none", alias = "skill_level")]
    pub skill_level: Option<SkillLevel>,

    // Linking
    /// Assets referenced by this article
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "linked_assets"
    )]
    pub linked_assets: Vec<AssetLink>,
    /// UUIDs of related decisions (legacy field)
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "linked_decisions"
    )]
    pub linked_decisions: Vec<Uuid>,
    /// IDs of related decision records
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "related_decisions"
    )]
    pub related_decisions: Vec<Uuid>,
    /// Related articles (detailed info)
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "related_articles"
    )]
    pub related_articles: Vec<RelatedArticle>,
    /// IDs of prerequisite articles (must read first)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<Uuid>,
    /// IDs of 'See Also' articles for further reading
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "see_also")]
    pub see_also: Vec<Uuid>,
    /// UUIDs of related sketches
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "linked_sketches"
    )]
    pub linked_sketches: Vec<Uuid>,

    // Standard metadata
    /// Tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    /// Additional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Creation timestamp
    #[serde(alias = "created_at")]
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    #[serde(alias = "updated_at")]
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeArticle {
    /// Create a new knowledge article with required fields
    pub fn new(
        number: u64,
        title: impl Into<String>,
        summary: impl Into<String>,
        content: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Self::generate_id(number),
            number,
            title: title.into(),
            article_type: KnowledgeType::Guide,
            status: KnowledgeStatus::Draft,
            domain: None,
            domain_id: None,
            workspace_id: None,
            summary: summary.into(),
            content: content.into(),
            authors: vec![author.into()],
            reviewers: Vec::new(),
            last_reviewed: None,
            reviewed_at: None,
            published_at: None,
            archived_at: None,
            review_frequency: None,
            audience: Vec::new(),
            skill_level: None,
            linked_assets: Vec::new(),
            linked_decisions: Vec::new(),
            related_decisions: Vec::new(),
            related_articles: Vec::new(),
            prerequisites: Vec::new(),
            see_also: Vec::new(),
            linked_sketches: Vec::new(),
            tags: Vec::new(),
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new knowledge article with a timestamp-based number (YYMMDDHHmm format)
    /// This format prevents merge conflicts in distributed Git workflows
    pub fn new_with_timestamp(
        title: impl Into<String>,
        summary: impl Into<String>,
        content: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        let number = Self::generate_timestamp_number(&now);
        Self::new(number, title, summary, content, author)
    }

    /// Generate a timestamp-based article number in YYMMDDHHmm format
    pub fn generate_timestamp_number(dt: &DateTime<Utc>) -> u64 {
        let formatted = dt.format("%y%m%d%H%M").to_string();
        formatted.parse().unwrap_or(0)
    }

    /// Generate a deterministic UUID for an article based on its number
    pub fn generate_id(number: u64) -> Uuid {
        // Use UUID v5 with a namespace for knowledge articles
        let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(); // URL namespace
        let name = format!("knowledge:{}", number);
        Uuid::new_v5(&namespace, name.as_bytes())
    }

    /// Check if the article number is timestamp-based (YYMMDDHHmm format - 10 digits)
    pub fn is_timestamp_number(&self) -> bool {
        self.number >= 1000000000 && self.number <= 9999999999
    }

    /// Format the article number for display
    /// Returns "KB-0001" for sequential or "KB-2601101234" for timestamp-based
    pub fn formatted_number(&self) -> String {
        if self.is_timestamp_number() {
            format!("KB-{}", self.number)
        } else {
            format!("KB-{:04}", self.number)
        }
    }

    /// Add an author
    pub fn add_author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self.updated_at = Utc::now();
        self
    }

    /// Set the domain ID
    pub fn with_domain_id(mut self, domain_id: Uuid) -> Self {
        self.domain_id = Some(domain_id);
        self.updated_at = Utc::now();
        self
    }

    /// Set the workspace ID
    pub fn with_workspace_id(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self.updated_at = Utc::now();
        self
    }

    /// Set the article type
    pub fn with_type(mut self, article_type: KnowledgeType) -> Self {
        self.article_type = article_type;
        self.updated_at = Utc::now();
        self
    }

    /// Set the article status
    pub fn with_status(mut self, status: KnowledgeStatus) -> Self {
        self.status = status;
        self.updated_at = Utc::now();
        self
    }

    /// Set the domain
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self.updated_at = Utc::now();
        self
    }

    /// Add a reviewer
    pub fn add_reviewer(mut self, reviewer: impl Into<String>) -> Self {
        self.reviewers.push(reviewer.into());
        self.updated_at = Utc::now();
        self
    }

    /// Set review frequency
    pub fn with_review_frequency(mut self, frequency: ReviewFrequency) -> Self {
        self.review_frequency = Some(frequency);
        self.updated_at = Utc::now();
        self
    }

    /// Add an audience
    pub fn add_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience.push(audience.into());
        self.updated_at = Utc::now();
        self
    }

    /// Set skill level
    pub fn with_skill_level(mut self, level: SkillLevel) -> Self {
        self.skill_level = Some(level);
        self.updated_at = Utc::now();
        self
    }

    /// Add an asset link
    pub fn add_asset_link(mut self, link: AssetLink) -> Self {
        self.linked_assets.push(link);
        self.updated_at = Utc::now();
        self
    }

    /// Link to a decision
    pub fn link_decision(mut self, decision_id: Uuid) -> Self {
        if !self.linked_decisions.contains(&decision_id) {
            self.linked_decisions.push(decision_id);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Add a related article
    pub fn add_related_article(mut self, article: RelatedArticle) -> Self {
        self.related_articles.push(article);
        self.updated_at = Utc::now();
        self
    }

    /// Add a related decision
    pub fn add_related_decision(mut self, decision_id: Uuid) -> Self {
        if !self.related_decisions.contains(&decision_id) {
            self.related_decisions.push(decision_id);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Add a prerequisite article
    pub fn add_prerequisite(mut self, article_id: Uuid) -> Self {
        if !self.prerequisites.contains(&article_id) {
            self.prerequisites.push(article_id);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Add a "see also" article reference
    pub fn add_see_also(mut self, article_id: Uuid) -> Self {
        if !self.see_also.contains(&article_id) {
            self.see_also.push(article_id);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Link to a sketch
    pub fn link_sketch(mut self, sketch_id: Uuid) -> Self {
        if !self.linked_sketches.contains(&sketch_id) {
            self.linked_sketches.push(sketch_id);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Set the published timestamp
    pub fn with_published_at(mut self, published_at: DateTime<Utc>) -> Self {
        self.published_at = Some(published_at);
        self.updated_at = Utc::now();
        self
    }

    /// Set the archived timestamp
    pub fn with_archived_at(mut self, archived_at: DateTime<Utc>) -> Self {
        self.archived_at = Some(archived_at);
        self.updated_at = Utc::now();
        self
    }

    /// Add a tag
    pub fn add_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self.updated_at = Utc::now();
        self
    }

    /// Mark the article as reviewed
    pub fn mark_reviewed(&mut self) {
        let now = Utc::now();
        self.last_reviewed = Some(now);
        self.reviewed_at = Some(now);
        self.updated_at = now;
    }

    /// Generate the YAML filename for this article
    pub fn filename(&self, workspace_name: &str) -> String {
        let number_str = if self.is_timestamp_number() {
            format!("{}", self.number)
        } else {
            format!("{:04}", self.number)
        };

        match &self.domain {
            Some(domain) => format!(
                "{}_{}_kb-{}.kb.yaml",
                sanitize_name(workspace_name),
                sanitize_name(domain),
                number_str
            ),
            None => format!(
                "{}_kb-{}.kb.yaml",
                sanitize_name(workspace_name),
                number_str
            ),
        }
    }

    /// Generate the Markdown filename for this article
    pub fn markdown_filename(&self) -> String {
        let slug = slugify(&self.title);
        format!("{}-{}.md", self.formatted_number(), slug)
    }

    /// Import from YAML
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }

    /// Export to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

/// Knowledge article index entry for the knowledge.yaml file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeIndexEntry {
    /// Article number (can be sequential or timestamp-based)
    pub number: u64,
    /// Article UUID
    pub id: Uuid,
    /// Article title
    pub title: String,
    /// Article type
    #[serde(alias = "article_type")]
    pub article_type: KnowledgeType,
    /// Article status
    pub status: KnowledgeStatus,
    /// Domain (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Filename of the article YAML file
    pub file: String,
}

impl From<&KnowledgeArticle> for KnowledgeIndexEntry {
    fn from(article: &KnowledgeArticle) -> Self {
        Self {
            number: article.number,
            id: article.id,
            title: article.title.clone(),
            article_type: article.article_type.clone(),
            status: article.status.clone(),
            domain: article.domain.clone(),
            file: String::new(), // Set by caller
        }
    }
}

/// Knowledge base index (knowledge.yaml)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeIndex {
    /// Schema version
    #[serde(alias = "schema_version")]
    pub schema_version: String,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none", alias = "last_updated")]
    pub last_updated: Option<DateTime<Utc>>,
    /// List of articles
    #[serde(default)]
    pub articles: Vec<KnowledgeIndexEntry>,
    /// Next available article number (for sequential numbering)
    #[serde(alias = "next_number")]
    pub next_number: u64,
    /// Whether to use timestamp-based numbering (YYMMDDHHmm format)
    #[serde(default, alias = "use_timestamp_numbering")]
    pub use_timestamp_numbering: bool,
}

impl Default for KnowledgeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeIndex {
    /// Create a new empty knowledge index
    pub fn new() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            last_updated: Some(Utc::now()),
            articles: Vec::new(),
            next_number: 1,
            use_timestamp_numbering: false,
        }
    }

    /// Create a new knowledge index with timestamp-based numbering
    pub fn new_with_timestamp_numbering() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            last_updated: Some(Utc::now()),
            articles: Vec::new(),
            next_number: 1,
            use_timestamp_numbering: true,
        }
    }

    /// Add an article to the index
    pub fn add_article(&mut self, article: &KnowledgeArticle, filename: String) {
        let mut entry = KnowledgeIndexEntry::from(article);
        entry.file = filename;

        // Remove existing entry with same number if present
        self.articles.retain(|a| a.number != article.number);
        self.articles.push(entry);

        // Sort by number
        self.articles.sort_by(|a, b| a.number.cmp(&b.number));

        // Update next number only for sequential numbering
        if !self.use_timestamp_numbering && article.number >= self.next_number {
            self.next_number = article.number + 1;
        }

        self.last_updated = Some(Utc::now());
    }

    /// Get the next available article number
    /// For timestamp-based numbering, generates a new timestamp
    /// For sequential numbering, returns the next sequential number
    pub fn get_next_number(&self) -> u64 {
        if self.use_timestamp_numbering {
            KnowledgeArticle::generate_timestamp_number(&Utc::now())
        } else {
            self.next_number
        }
    }

    /// Find an article by number
    pub fn find_by_number(&self, number: u64) -> Option<&KnowledgeIndexEntry> {
        self.articles.iter().find(|a| a.number == number)
    }

    /// Import from YAML
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }

    /// Export to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

/// Sanitize a name for use in filenames
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            ' ' | '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// Create a URL-friendly slug from a title
fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(50) // Limit slug length
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_article_new() {
        let article = KnowledgeArticle::new(
            1,
            "Data Classification Guide",
            "This guide explains classification",
            "## Overview\n\nData classification is important...",
            "data-governance@example.com",
        );

        assert_eq!(article.number, 1);
        assert_eq!(article.formatted_number(), "KB-0001");
        assert_eq!(article.title, "Data Classification Guide");
        assert_eq!(article.status, KnowledgeStatus::Draft);
        assert_eq!(article.article_type, KnowledgeType::Guide);
        assert_eq!(article.authors.len(), 1);
    }

    #[test]
    fn test_knowledge_article_builder_pattern() {
        let article = KnowledgeArticle::new(1, "Test", "Summary", "Content", "author@example.com")
            .with_type(KnowledgeType::Standard)
            .with_status(KnowledgeStatus::Published)
            .with_domain("sales")
            .add_reviewer("reviewer@example.com")
            .with_review_frequency(ReviewFrequency::Quarterly)
            .add_audience("data-engineers")
            .with_skill_level(SkillLevel::Intermediate);

        assert_eq!(article.article_type, KnowledgeType::Standard);
        assert_eq!(article.status, KnowledgeStatus::Published);
        assert_eq!(article.domain, Some("sales".to_string()));
        assert_eq!(article.reviewers.len(), 1);
        assert_eq!(article.review_frequency, Some(ReviewFrequency::Quarterly));
        assert_eq!(article.audience.len(), 1);
        assert_eq!(article.skill_level, Some(SkillLevel::Intermediate));
    }

    #[test]
    fn test_knowledge_article_id_generation() {
        let id1 = KnowledgeArticle::generate_id(1);
        let id2 = KnowledgeArticle::generate_id(1);
        let id3 = KnowledgeArticle::generate_id(2);

        // Same number should generate same ID
        assert_eq!(id1, id2);
        // Different numbers should generate different IDs
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_knowledge_article_filename() {
        let article = KnowledgeArticle::new(1, "Test", "Summary", "Content", "author@example.com");
        assert_eq!(article.filename("enterprise"), "enterprise_kb-0001.kb.yaml");

        let article_with_domain = article.with_domain("sales");
        assert_eq!(
            article_with_domain.filename("enterprise"),
            "enterprise_sales_kb-0001.kb.yaml"
        );
    }

    #[test]
    fn test_knowledge_article_markdown_filename() {
        let article = KnowledgeArticle::new(
            1,
            "Data Classification Guide",
            "Summary",
            "Content",
            "author@example.com",
        );
        let filename = article.markdown_filename();
        assert!(filename.starts_with("KB-0001-"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_knowledge_article_yaml_roundtrip() {
        let article = KnowledgeArticle::new(
            1,
            "Test Article",
            "Test summary",
            "Test content",
            "author@example.com",
        )
        .with_status(KnowledgeStatus::Published)
        .with_domain("test");

        let yaml = article.to_yaml().unwrap();
        let parsed = KnowledgeArticle::from_yaml(&yaml).unwrap();

        assert_eq!(article.id, parsed.id);
        assert_eq!(article.title, parsed.title);
        assert_eq!(article.status, parsed.status);
        assert_eq!(article.domain, parsed.domain);
    }

    #[test]
    fn test_knowledge_index() {
        let mut index = KnowledgeIndex::new();
        assert_eq!(index.get_next_number(), 1);

        let article1 =
            KnowledgeArticle::new(1, "First", "Summary", "Content", "author@example.com");
        index.add_article(&article1, "test_kb-0001.kb.yaml".to_string());

        assert_eq!(index.articles.len(), 1);
        assert_eq!(index.get_next_number(), 2);

        let article2 =
            KnowledgeArticle::new(2, "Second", "Summary", "Content", "author@example.com");
        index.add_article(&article2, "test_kb-0002.kb.yaml".to_string());

        assert_eq!(index.articles.len(), 2);
        assert_eq!(index.get_next_number(), 3);
    }

    #[test]
    fn test_related_article() {
        let related = RelatedArticle::new(
            Uuid::new_v4(),
            "KB-0002",
            "PII Handling",
            ArticleRelationship::Related,
        );

        assert_eq!(related.article_number, "KB-0002");
        assert_eq!(related.relationship, ArticleRelationship::Related);
    }

    #[test]
    fn test_knowledge_type_display() {
        assert_eq!(format!("{}", KnowledgeType::Guide), "Guide");
        assert_eq!(format!("{}", KnowledgeType::Standard), "Standard");
        assert_eq!(format!("{}", KnowledgeType::HowTo), "How-To");
        assert_eq!(format!("{}", KnowledgeType::Concept), "Concept");
        assert_eq!(format!("{}", KnowledgeType::Runbook), "Runbook");
    }

    #[test]
    fn test_knowledge_status_display() {
        assert_eq!(format!("{}", KnowledgeStatus::Draft), "Draft");
        assert_eq!(format!("{}", KnowledgeStatus::Review), "Review");
        assert_eq!(format!("{}", KnowledgeStatus::Published), "Published");
        assert_eq!(format!("{}", KnowledgeStatus::Archived), "Archived");
    }

    #[test]
    fn test_timestamp_number_generation() {
        use chrono::TimeZone;
        let dt = Utc.with_ymd_and_hms(2026, 1, 10, 14, 30, 0).unwrap();
        let number = KnowledgeArticle::generate_timestamp_number(&dt);
        assert_eq!(number, 2601101430);
    }

    #[test]
    fn test_is_timestamp_number() {
        let sequential_article =
            KnowledgeArticle::new(1, "Test", "Summary", "Content", "author@example.com");
        assert!(!sequential_article.is_timestamp_number());

        let timestamp_article = KnowledgeArticle::new(
            2601101430,
            "Test",
            "Summary",
            "Content",
            "author@example.com",
        );
        assert!(timestamp_article.is_timestamp_number());
    }

    #[test]
    fn test_timestamp_article_filename() {
        let article = KnowledgeArticle::new(
            2601101430,
            "Test",
            "Summary",
            "Content",
            "author@example.com",
        );
        assert_eq!(
            article.filename("enterprise"),
            "enterprise_kb-2601101430.kb.yaml"
        );
    }

    #[test]
    fn test_timestamp_article_markdown_filename() {
        let article = KnowledgeArticle::new(
            2601101430,
            "Test Article",
            "Summary",
            "Content",
            "author@example.com",
        );
        let filename = article.markdown_filename();
        assert!(filename.starts_with("KB-2601101430-"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_article_with_multiple_authors() {
        let article = KnowledgeArticle::new(1, "Test", "Summary", "Content", "author1@example.com")
            .add_author("author2@example.com")
            .add_author("author3@example.com");

        assert_eq!(article.authors.len(), 3);
    }

    #[test]
    fn test_knowledge_index_with_timestamp_numbering() {
        let index = KnowledgeIndex::new_with_timestamp_numbering();
        assert!(index.use_timestamp_numbering);

        // The next number should be a timestamp
        let next = index.get_next_number();
        assert!(next >= 1000000000); // Timestamp format check
    }
}
