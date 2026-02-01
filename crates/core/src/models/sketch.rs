//! Sketch model for Excalidraw diagrams
//!
//! Implements sketch support for storing and organizing Excalidraw diagrams
//! within workspaces. Sketches can be linked to knowledge articles, decisions,
//! and other assets.
//!
//! ## File Format
//!
//! Sketches are stored as `.sketch.yaml` files following the naming convention:
//! `{workspace}_{domain}_sketch-{number}.sketch.yaml`
//!
//! ## Example
//!
//! ```yaml
//! id: 770e8400-e29b-41d4-a716-446655440001
//! number: 1
//! title: "Sales Domain Architecture"
//! sketchType: architecture
//! status: published
//! domain: sales
//! description: "High-level architecture diagram for sales domain"
//! excalidrawData: '{"type":"excalidraw","version":2,"elements":[...]}'
//! thumbnailPath: thumbnails/sketch-0001.png
//! authors:
//!   - architect@company.com
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Tag;
use super::decision::AssetLink;

/// Sketch status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SketchStatus {
    /// Sketch is being drafted
    #[default]
    Draft,
    /// Sketch is under review
    Review,
    /// Sketch is published and active
    Published,
    /// Sketch is archived (historical reference)
    Archived,
}

impl std::fmt::Display for SketchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SketchStatus::Draft => write!(f, "Draft"),
            SketchStatus::Review => write!(f, "Review"),
            SketchStatus::Published => write!(f, "Published"),
            SketchStatus::Archived => write!(f, "Archived"),
        }
    }
}

/// Sketch type/category
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SketchType {
    /// Architecture diagram
    #[default]
    Architecture,
    /// Data flow diagram
    DataFlow,
    /// Entity relationship diagram
    EntityRelationship,
    /// Sequence diagram
    Sequence,
    /// Flowchart
    Flowchart,
    /// Wireframe/mockup
    Wireframe,
    /// Concept/mind map
    Concept,
    /// Infrastructure diagram
    Infrastructure,
    /// Other/general sketch
    Other,
}

impl std::fmt::Display for SketchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SketchType::Architecture => write!(f, "Architecture"),
            SketchType::DataFlow => write!(f, "Data Flow"),
            SketchType::EntityRelationship => write!(f, "Entity Relationship"),
            SketchType::Sequence => write!(f, "Sequence"),
            SketchType::Flowchart => write!(f, "Flowchart"),
            SketchType::Wireframe => write!(f, "Wireframe"),
            SketchType::Concept => write!(f, "Concept"),
            SketchType::Infrastructure => write!(f, "Infrastructure"),
            SketchType::Other => write!(f, "Other"),
        }
    }
}

/// Custom deserializer for sketch number that supports both:
/// - Legacy string format: "SKETCH-0001"
/// - New numeric format: 1 or 2601101234 (timestamp)
fn deserialize_sketch_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct NumberVisitor;

    impl<'de> Visitor<'de> for NumberVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number or a string like 'SKETCH-0001'")
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
            // Handle "SKETCH-0001" format
            let num_str = value
                .to_uppercase()
                .strip_prefix("SKETCH-")
                .map(|s| s.to_string())
                .unwrap_or_else(|| value.to_string());

            num_str
                .parse::<u64>()
                .map_err(|_| E::custom(format!("invalid sketch number format: {}", value)))
        }
    }

    deserializer.deserialize_any(NumberVisitor)
}

/// Excalidraw Sketch
///
/// Represents an Excalidraw sketch that can be categorized by domain,
/// type, and linked to other assets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sketch {
    /// Unique identifier for the sketch
    pub id: Uuid,
    /// Sketch number - can be sequential (1, 2, 3) or timestamp-based (YYMMDDHHmm format)
    /// Timestamp format prevents merge conflicts in distributed Git workflows
    #[serde(deserialize_with = "deserialize_sketch_number")]
    pub number: u64,
    /// Sketch title
    pub title: String,
    /// Type of sketch
    #[serde(alias = "sketch_type")]
    pub sketch_type: SketchType,
    /// Publication status
    pub status: SketchStatus,
    /// Domain this sketch belongs to (optional, string name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Domain UUID reference (optional)
    #[serde(skip_serializing_if = "Option::is_none", alias = "domain_id")]
    pub domain_id: Option<Uuid>,
    /// Workspace UUID reference (optional)
    #[serde(skip_serializing_if = "Option::is_none", alias = "workspace_id")]
    pub workspace_id: Option<Uuid>,

    // Content
    /// Brief description of the sketch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Excalidraw scene data as JSON string
    #[serde(alias = "excalidraw_data")]
    pub excalidraw_data: String,
    /// Optional path to PNG thumbnail (relative path, e.g., "thumbnails/sketch-0001.png")
    #[serde(skip_serializing_if = "Option::is_none", alias = "thumbnail_path")]
    pub thumbnail_path: Option<String>,

    // Authorship
    /// Sketch authors (emails or names)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,

    // Linking
    /// Assets referenced by this sketch
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "linked_assets"
    )]
    pub linked_assets: Vec<AssetLink>,
    /// UUIDs of related decisions
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "linked_decisions"
    )]
    pub linked_decisions: Vec<Uuid>,
    /// UUIDs of related knowledge articles
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "linked_knowledge"
    )]
    pub linked_knowledge: Vec<Uuid>,
    /// UUIDs of related sketches
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "related_sketches"
    )]
    pub related_sketches: Vec<Uuid>,

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

impl Sketch {
    /// Create a new sketch with required fields
    pub fn new(number: u64, title: impl Into<String>, excalidraw_data: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Self::generate_id(number),
            number,
            title: title.into(),
            sketch_type: SketchType::Architecture,
            status: SketchStatus::Draft,
            domain: None,
            domain_id: None,
            workspace_id: None,
            description: None,
            excalidraw_data: excalidraw_data.into(),
            thumbnail_path: None,
            authors: Vec::new(),
            linked_assets: Vec::new(),
            linked_decisions: Vec::new(),
            linked_knowledge: Vec::new(),
            related_sketches: Vec::new(),
            tags: Vec::new(),
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new sketch with a timestamp-based number (YYMMDDHHmm format)
    /// This format prevents merge conflicts in distributed Git workflows
    pub fn new_with_timestamp(
        title: impl Into<String>,
        excalidraw_data: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        let number = Self::generate_timestamp_number(&now);
        Self::new(number, title, excalidraw_data)
    }

    /// Generate a timestamp-based sketch number in YYMMDDHHmm format
    pub fn generate_timestamp_number(dt: &DateTime<Utc>) -> u64 {
        let formatted = dt.format("%y%m%d%H%M").to_string();
        formatted.parse().unwrap_or(0)
    }

    /// Generate a deterministic UUID for a sketch based on its number
    pub fn generate_id(number: u64) -> Uuid {
        // Use UUID v5 with a namespace for sketches
        let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(); // URL namespace
        let name = format!("sketch:{}", number);
        Uuid::new_v5(&namespace, name.as_bytes())
    }

    /// Check if the sketch number is timestamp-based (YYMMDDHHmm format - 10 digits)
    pub fn is_timestamp_number(&self) -> bool {
        self.number >= 1000000000 && self.number <= 9999999999
    }

    /// Format the sketch number for display
    /// Returns "SKETCH-0001" for sequential or "SKETCH-2601101234" for timestamp-based
    pub fn formatted_number(&self) -> String {
        if self.is_timestamp_number() {
            format!("SKETCH-{}", self.number)
        } else {
            format!("SKETCH-{:04}", self.number)
        }
    }

    /// Generate the YAML filename for this sketch
    pub fn filename(&self, workspace_name: &str) -> String {
        let number_str = if self.is_timestamp_number() {
            format!("{}", self.number)
        } else {
            format!("{:04}", self.number)
        };

        match &self.domain {
            Some(domain) => format!(
                "{}_{}_sketch-{}.sketch.yaml",
                sanitize_name(workspace_name),
                sanitize_name(domain),
                number_str
            ),
            None => format!(
                "{}_sketch-{}.sketch.yaml",
                sanitize_name(workspace_name),
                number_str
            ),
        }
    }

    /// Generate the thumbnail filename for this sketch
    pub fn thumbnail_filename(&self) -> String {
        let number_str = if self.is_timestamp_number() {
            format!("{}", self.number)
        } else {
            format!("{:04}", self.number)
        };
        format!("thumbnails/sketch-{}.png", number_str)
    }

    /// Set the sketch type
    pub fn with_type(mut self, sketch_type: SketchType) -> Self {
        self.sketch_type = sketch_type;
        self.updated_at = Utc::now();
        self
    }

    /// Set the sketch status
    pub fn with_status(mut self, status: SketchStatus) -> Self {
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

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self.updated_at = Utc::now();
        self
    }

    /// Set the thumbnail path
    pub fn with_thumbnail(mut self, thumbnail_path: impl Into<String>) -> Self {
        self.thumbnail_path = Some(thumbnail_path.into());
        self.updated_at = Utc::now();
        self
    }

    /// Add an author
    pub fn add_author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
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

    /// Link to a knowledge article
    pub fn link_knowledge(mut self, knowledge_id: Uuid) -> Self {
        if !self.linked_knowledge.contains(&knowledge_id) {
            self.linked_knowledge.push(knowledge_id);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Add a related sketch
    pub fn add_related_sketch(mut self, sketch_id: Uuid) -> Self {
        if !self.related_sketches.contains(&sketch_id) {
            self.related_sketches.push(sketch_id);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Add a tag
    pub fn add_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self.updated_at = Utc::now();
        self
    }

    /// Set notes
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self.updated_at = Utc::now();
        self
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

/// Sketch index entry for the sketches.yaml file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchIndexEntry {
    /// Sketch number (can be sequential or timestamp-based)
    pub number: u64,
    /// Sketch UUID
    pub id: Uuid,
    /// Sketch title
    pub title: String,
    /// Sketch type
    #[serde(alias = "sketch_type")]
    pub sketch_type: SketchType,
    /// Sketch status
    pub status: SketchStatus,
    /// Domain (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Filename of the sketch YAML file
    pub file: String,
    /// Optional thumbnail path
    #[serde(skip_serializing_if = "Option::is_none", alias = "thumbnail_path")]
    pub thumbnail_path: Option<String>,
}

impl From<&Sketch> for SketchIndexEntry {
    fn from(sketch: &Sketch) -> Self {
        Self {
            number: sketch.number,
            id: sketch.id,
            title: sketch.title.clone(),
            sketch_type: sketch.sketch_type.clone(),
            status: sketch.status.clone(),
            domain: sketch.domain.clone(),
            file: String::new(), // Set by caller
            thumbnail_path: sketch.thumbnail_path.clone(),
        }
    }
}

/// Sketch index (sketches.yaml)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchIndex {
    /// Schema version
    #[serde(alias = "schema_version")]
    pub schema_version: String,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none", alias = "last_updated")]
    pub last_updated: Option<DateTime<Utc>>,
    /// List of sketches
    #[serde(default)]
    pub sketches: Vec<SketchIndexEntry>,
    /// Next available sketch number (for sequential numbering)
    #[serde(alias = "next_number")]
    pub next_number: u64,
    /// Whether to use timestamp-based numbering (YYMMDDHHmm format)
    #[serde(default, alias = "use_timestamp_numbering")]
    pub use_timestamp_numbering: bool,
}

impl Default for SketchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SketchIndex {
    /// Create a new empty sketch index
    pub fn new() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            last_updated: Some(Utc::now()),
            sketches: Vec::new(),
            next_number: 1,
            use_timestamp_numbering: false,
        }
    }

    /// Create a new sketch index with timestamp-based numbering
    pub fn new_with_timestamp_numbering() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            last_updated: Some(Utc::now()),
            sketches: Vec::new(),
            next_number: 1,
            use_timestamp_numbering: true,
        }
    }

    /// Add a sketch to the index
    pub fn add_sketch(&mut self, sketch: &Sketch, filename: String) {
        let mut entry = SketchIndexEntry::from(sketch);
        entry.file = filename;

        // Remove existing entry with same number if present
        self.sketches.retain(|s| s.number != sketch.number);
        self.sketches.push(entry);

        // Sort by number
        self.sketches.sort_by(|a, b| a.number.cmp(&b.number));

        // Update next number only for sequential numbering
        if !self.use_timestamp_numbering && sketch.number >= self.next_number {
            self.next_number = sketch.number + 1;
        }

        self.last_updated = Some(Utc::now());
    }

    /// Get the next available sketch number
    /// For timestamp-based numbering, generates a new timestamp
    /// For sequential numbering, returns the next sequential number
    pub fn get_next_number(&self) -> u64 {
        if self.use_timestamp_numbering {
            Sketch::generate_timestamp_number(&Utc::now())
        } else {
            self.next_number
        }
    }

    /// Find a sketch by number
    pub fn find_by_number(&self, number: u64) -> Option<&SketchIndexEntry> {
        self.sketches.iter().find(|s| s.number == number)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sketch_new() {
        let sketch = Sketch::new(1, "Architecture Diagram", "{}");

        assert_eq!(sketch.number, 1);
        assert_eq!(sketch.formatted_number(), "SKETCH-0001");
        assert_eq!(sketch.title, "Architecture Diagram");
        assert_eq!(sketch.status, SketchStatus::Draft);
        assert_eq!(sketch.sketch_type, SketchType::Architecture);
    }

    #[test]
    fn test_sketch_builder_pattern() {
        let sketch = Sketch::new(1, "Test", "{}")
            .with_type(SketchType::DataFlow)
            .with_status(SketchStatus::Published)
            .with_domain("sales")
            .with_description("Test description")
            .add_author("architect@example.com");

        assert_eq!(sketch.sketch_type, SketchType::DataFlow);
        assert_eq!(sketch.status, SketchStatus::Published);
        assert_eq!(sketch.domain, Some("sales".to_string()));
        assert_eq!(sketch.description, Some("Test description".to_string()));
        assert_eq!(sketch.authors.len(), 1);
    }

    #[test]
    fn test_sketch_id_generation() {
        let id1 = Sketch::generate_id(1);
        let id2 = Sketch::generate_id(1);
        let id3 = Sketch::generate_id(2);

        // Same number should generate same ID
        assert_eq!(id1, id2);
        // Different numbers should generate different IDs
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_sketch_filename() {
        let sketch = Sketch::new(1, "Test", "{}");
        assert_eq!(
            sketch.filename("enterprise"),
            "enterprise_sketch-0001.sketch.yaml"
        );

        let sketch_with_domain = sketch.with_domain("sales");
        assert_eq!(
            sketch_with_domain.filename("enterprise"),
            "enterprise_sales_sketch-0001.sketch.yaml"
        );
    }

    #[test]
    fn test_sketch_thumbnail_filename() {
        let sketch = Sketch::new(1, "Test", "{}");
        assert_eq!(sketch.thumbnail_filename(), "thumbnails/sketch-0001.png");

        let timestamp_sketch = Sketch::new(2601101430, "Test", "{}");
        assert_eq!(
            timestamp_sketch.thumbnail_filename(),
            "thumbnails/sketch-2601101430.png"
        );
    }

    #[test]
    fn test_sketch_yaml_roundtrip() {
        let sketch = Sketch::new(1, "Test Sketch", r#"{"elements":[]}"#)
            .with_status(SketchStatus::Published)
            .with_domain("test");

        let yaml = sketch.to_yaml().unwrap();
        let parsed = Sketch::from_yaml(&yaml).unwrap();

        assert_eq!(sketch.id, parsed.id);
        assert_eq!(sketch.title, parsed.title);
        assert_eq!(sketch.status, parsed.status);
        assert_eq!(sketch.domain, parsed.domain);
    }

    #[test]
    fn test_sketch_index() {
        let mut index = SketchIndex::new();
        assert_eq!(index.get_next_number(), 1);

        let sketch1 = Sketch::new(1, "First", "{}");
        index.add_sketch(&sketch1, "test_sketch-0001.sketch.yaml".to_string());

        assert_eq!(index.sketches.len(), 1);
        assert_eq!(index.get_next_number(), 2);

        let sketch2 = Sketch::new(2, "Second", "{}");
        index.add_sketch(&sketch2, "test_sketch-0002.sketch.yaml".to_string());

        assert_eq!(index.sketches.len(), 2);
        assert_eq!(index.get_next_number(), 3);
    }

    #[test]
    fn test_sketch_type_display() {
        assert_eq!(format!("{}", SketchType::Architecture), "Architecture");
        assert_eq!(format!("{}", SketchType::DataFlow), "Data Flow");
        assert_eq!(
            format!("{}", SketchType::EntityRelationship),
            "Entity Relationship"
        );
        assert_eq!(format!("{}", SketchType::Concept), "Concept");
    }

    #[test]
    fn test_sketch_status_display() {
        assert_eq!(format!("{}", SketchStatus::Draft), "Draft");
        assert_eq!(format!("{}", SketchStatus::Review), "Review");
        assert_eq!(format!("{}", SketchStatus::Published), "Published");
        assert_eq!(format!("{}", SketchStatus::Archived), "Archived");
    }

    #[test]
    fn test_timestamp_number_generation() {
        use chrono::TimeZone;
        let dt = Utc.with_ymd_and_hms(2026, 1, 10, 14, 30, 0).unwrap();
        let number = Sketch::generate_timestamp_number(&dt);
        assert_eq!(number, 2601101430);
    }

    #[test]
    fn test_is_timestamp_number() {
        let sequential_sketch = Sketch::new(1, "Test", "{}");
        assert!(!sequential_sketch.is_timestamp_number());

        let timestamp_sketch = Sketch::new(2601101430, "Test", "{}");
        assert!(timestamp_sketch.is_timestamp_number());
    }

    #[test]
    fn test_timestamp_sketch_filename() {
        let sketch = Sketch::new(2601101430, "Test", "{}");
        assert_eq!(
            sketch.filename("enterprise"),
            "enterprise_sketch-2601101430.sketch.yaml"
        );
    }

    #[test]
    fn test_sketch_index_with_timestamp_numbering() {
        let index = SketchIndex::new_with_timestamp_numbering();
        assert!(index.use_timestamp_numbering);

        // The next number should be a timestamp
        let next = index.get_next_number();
        assert!(next >= 1000000000); // Timestamp format check
    }

    #[test]
    fn test_sketch_linking() {
        let decision_id = Uuid::new_v4();
        let knowledge_id = Uuid::new_v4();
        let sketch_id = Uuid::new_v4();

        let sketch = Sketch::new(1, "Test", "{}")
            .link_decision(decision_id)
            .link_knowledge(knowledge_id)
            .add_related_sketch(sketch_id);

        assert_eq!(sketch.linked_decisions.len(), 1);
        assert_eq!(sketch.linked_knowledge.len(), 1);
        assert_eq!(sketch.related_sketches.len(), 1);
    }
}
