//! Decision model for MADR-compliant decision records
//!
//! Implements the Data Decision Log (DDL) feature for tracking architectural
//! and data-related decisions using the MADR (Markdown Any Decision Records)
//! template format.
//!
//! ## File Format
//!
//! Decision records are stored as `.madr.yaml` files following the naming convention:
//! `{workspace}_{domain}_adr-{number}.madr.yaml`
//!
//! ## Example
//!
//! ```yaml
//! id: 550e8400-e29b-41d4-a716-446655440000
//! number: 1
//! title: "Use ODCS v3.1.0 for all data contracts"
//! status: accepted
//! category: datadesign
//! domain: sales
//! date: 2026-01-07T10:00:00Z
//! deciders:
//!   - data-architecture@company.com
//! context: |
//!   We need a standard format for defining data contracts.
//! decision: |
//!   We will adopt ODCS v3.1.0 as the standard format.
//! consequences: |
//!   Positive: Consistent contracts across domains
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Tag;

/// Decision status in lifecycle
///
/// Decisions follow a lifecycle: Draft → Proposed → Accepted → [Deprecated | Superseded | Rejected]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DecisionStatus {
    /// Decision is in draft state, not yet proposed
    Draft,
    /// Decision has been proposed but not yet accepted
    #[default]
    Proposed,
    /// Decision has been accepted and is in effect
    Accepted,
    /// Decision was rejected
    Rejected,
    /// Decision has been replaced by another decision
    Superseded,
    /// Decision is no longer valid but not replaced
    Deprecated,
}

impl std::fmt::Display for DecisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionStatus::Draft => write!(f, "Draft"),
            DecisionStatus::Proposed => write!(f, "Proposed"),
            DecisionStatus::Accepted => write!(f, "Accepted"),
            DecisionStatus::Rejected => write!(f, "Rejected"),
            DecisionStatus::Superseded => write!(f, "Superseded"),
            DecisionStatus::Deprecated => write!(f, "Deprecated"),
        }
    }
}

/// Decision category
///
/// Categories help organize decisions by their domain of impact.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DecisionCategory {
    /// System architecture decisions
    #[default]
    Architecture,
    /// Technology choices
    Technology,
    /// Process-related decisions
    Process,
    /// Security-related decisions
    Security,
    /// Data-related decisions
    Data,
    /// Integration decisions
    Integration,
    /// Data design and modeling decisions
    DataDesign,
    /// Workflow and process decisions
    Workflow,
    /// Data model structure decisions
    Model,
    /// Data governance decisions
    Governance,
    /// Performance optimization decisions
    Performance,
    /// Compliance and regulatory decisions
    Compliance,
    /// Infrastructure decisions
    Infrastructure,
    /// Tooling choices
    Tooling,
}

impl std::fmt::Display for DecisionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionCategory::Architecture => write!(f, "Architecture"),
            DecisionCategory::Technology => write!(f, "Technology"),
            DecisionCategory::Process => write!(f, "Process"),
            DecisionCategory::Security => write!(f, "Security"),
            DecisionCategory::Data => write!(f, "Data"),
            DecisionCategory::Integration => write!(f, "Integration"),
            DecisionCategory::DataDesign => write!(f, "Data Design"),
            DecisionCategory::Workflow => write!(f, "Workflow"),
            DecisionCategory::Model => write!(f, "Model"),
            DecisionCategory::Governance => write!(f, "Governance"),
            DecisionCategory::Performance => write!(f, "Performance"),
            DecisionCategory::Compliance => write!(f, "Compliance"),
            DecisionCategory::Infrastructure => write!(f, "Infrastructure"),
            DecisionCategory::Tooling => write!(f, "Tooling"),
        }
    }
}

/// Priority level for decision drivers
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DriverPriority {
    High,
    #[default]
    Medium,
    Low,
}

/// Driver/reason for the decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionDriver {
    /// Description of why this is a driver
    pub description: String,
    /// Priority of this driver
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<DriverPriority>,
}

impl DecisionDriver {
    /// Create a new decision driver
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            priority: None,
        }
    }

    /// Create a decision driver with priority
    pub fn with_priority(description: impl Into<String>, priority: DriverPriority) -> Self {
        Self {
            description: description.into(),
            priority: Some(priority),
        }
    }
}

/// Option considered during decision making
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionOption {
    /// Name of the option
    pub name: String,
    /// Description of the option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Advantages of this option
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pros: Vec<String>,
    /// Disadvantages of this option
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cons: Vec<String>,
    /// Whether this option was selected
    pub selected: bool,
}

impl DecisionOption {
    /// Create a new decision option
    pub fn new(name: impl Into<String>, selected: bool) -> Self {
        Self {
            name: name.into(),
            description: None,
            pros: Vec::new(),
            cons: Vec::new(),
            selected,
        }
    }

    /// Create a decision option with full details
    pub fn with_details(
        name: impl Into<String>,
        description: impl Into<String>,
        pros: Vec<String>,
        cons: Vec<String>,
        selected: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            pros,
            cons,
            selected,
        }
    }
}

/// Relationship type between a decision and an asset
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AssetRelationship {
    /// Decision affects this asset
    Affects,
    /// Decision is implemented by this asset
    Implements,
    /// Decision deprecates this asset
    Deprecates,
}

/// Link to an asset (table, relationship, product, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetLink {
    /// Type of asset (odcs, odps, cads, relationship)
    #[serde(alias = "asset_type")]
    pub asset_type: String,
    /// UUID of the linked asset
    #[serde(alias = "asset_id")]
    pub asset_id: Uuid,
    /// Name of the linked asset
    #[serde(alias = "asset_name")]
    pub asset_name: String,
    /// Relationship between decision and asset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<AssetRelationship>,
}

impl AssetLink {
    /// Create a new asset link
    pub fn new(
        asset_type: impl Into<String>,
        asset_id: Uuid,
        asset_name: impl Into<String>,
    ) -> Self {
        Self {
            asset_type: asset_type.into(),
            asset_id,
            asset_name: asset_name.into(),
            relationship: None,
        }
    }

    /// Create an asset link with relationship
    pub fn with_relationship(
        asset_type: impl Into<String>,
        asset_id: Uuid,
        asset_name: impl Into<String>,
        relationship: AssetRelationship,
    ) -> Self {
        Self {
            asset_type: asset_type.into(),
            asset_id,
            asset_name: asset_name.into(),
            relationship: Some(relationship),
        }
    }
}

/// Compliance assessment for the decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceAssessment {
    /// Impact on regulatory requirements
    #[serde(skip_serializing_if = "Option::is_none", alias = "regulatory_impact")]
    pub regulatory_impact: Option<String>,
    /// Privacy impact assessment
    #[serde(skip_serializing_if = "Option::is_none", alias = "privacy_assessment")]
    pub privacy_assessment: Option<String>,
    /// Security impact assessment
    #[serde(skip_serializing_if = "Option::is_none", alias = "security_assessment")]
    pub security_assessment: Option<String>,
    /// Applicable compliance frameworks (GDPR, SOC2, HIPAA, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<String>,
}

impl ComplianceAssessment {
    /// Check if the assessment is empty
    pub fn is_empty(&self) -> bool {
        self.regulatory_impact.is_none()
            && self.privacy_assessment.is_none()
            && self.security_assessment.is_none()
            && self.frameworks.is_empty()
    }
}

/// Contact details for decision ownership
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DecisionContact {
    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Contact name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Role or team
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// RACI matrix for decision responsibility assignment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RaciMatrix {
    /// Responsible - Those who do the work to complete the task
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub responsible: Vec<String>,
    /// Accountable - The one ultimately answerable for the decision
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accountable: Vec<String>,
    /// Consulted - Those whose opinions are sought
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consulted: Vec<String>,
    /// Informed - Those who are kept up-to-date on progress
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub informed: Vec<String>,
}

impl RaciMatrix {
    /// Check if the RACI matrix is empty
    pub fn is_empty(&self) -> bool {
        self.responsible.is_empty()
            && self.accountable.is_empty()
            && self.consulted.is_empty()
            && self.informed.is_empty()
    }
}

/// MADR-compliant Decision Record
///
/// Represents an architectural or data decision following the MADR template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    /// Unique identifier for the decision
    pub id: Uuid,
    /// Decision number - can be sequential (1, 2, 3) or timestamp-based (YYMMDDHHmm format)
    /// Timestamp format prevents merge conflicts in distributed Git workflows
    pub number: u64,
    /// Short title describing the decision
    pub title: String,
    /// Current status of the decision
    pub status: DecisionStatus,
    /// Category of the decision
    pub category: DecisionCategory,
    /// Domain this decision belongs to (optional, string name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Domain UUID reference (optional)
    #[serde(skip_serializing_if = "Option::is_none", alias = "domain_id")]
    pub domain_id: Option<Uuid>,
    /// Workspace UUID reference (optional)
    #[serde(skip_serializing_if = "Option::is_none", alias = "workspace_id")]
    pub workspace_id: Option<Uuid>,

    // MADR template fields
    /// Date the decision was made
    pub date: DateTime<Utc>,
    /// When the decision was accepted/finalized
    #[serde(skip_serializing_if = "Option::is_none", alias = "decided_at")]
    pub decided_at: Option<DateTime<Utc>>,
    /// Authors of the decision record
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// People or teams who made the decision (deciders)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deciders: Vec<String>,
    /// People or teams consulted during decision making (RACI - Consulted)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consulted: Vec<String>,
    /// People or teams to be informed about the decision (RACI - Informed)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub informed: Vec<String>,
    /// Problem statement and context for the decision
    pub context: String,
    /// Reasons driving this decision
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drivers: Vec<DecisionDriver>,
    /// Options that were considered
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<DecisionOption>,
    /// The decision that was made
    pub decision: String,
    /// Positive and negative consequences of the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consequences: Option<String>,

    // Linking
    /// Assets affected by this decision
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "linked_assets"
    )]
    pub linked_assets: Vec<AssetLink>,
    /// ID of the decision this supersedes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<Uuid>,
    /// ID of the decision that superseded this
    #[serde(skip_serializing_if = "Option::is_none", alias = "superseded_by")]
    pub superseded_by: Option<Uuid>,
    /// IDs of related decisions
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "related_decisions"
    )]
    pub related_decisions: Vec<Uuid>,
    /// IDs of related knowledge articles
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "related_knowledge"
    )]
    pub related_knowledge: Vec<Uuid>,

    // Compliance (from feature request)
    /// Compliance assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance: Option<ComplianceAssessment>,

    // Confirmation tracking (from feature request)
    /// Date the decision was confirmed/reviewed
    #[serde(skip_serializing_if = "Option::is_none", alias = "confirmation_date")]
    pub confirmation_date: Option<DateTime<Utc>>,
    /// Notes from confirmation review
    #[serde(skip_serializing_if = "Option::is_none", alias = "confirmation_notes")]
    pub confirmation_notes: Option<String>,

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

impl Decision {
    /// Create a new decision with required fields
    pub fn new(
        number: u64,
        title: impl Into<String>,
        context: impl Into<String>,
        decision: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Self::generate_id(number),
            number,
            title: title.into(),
            status: DecisionStatus::Proposed,
            category: DecisionCategory::Architecture,
            domain: None,
            domain_id: None,
            workspace_id: None,
            date: now,
            decided_at: None,
            authors: Vec::new(),
            deciders: Vec::new(),
            consulted: Vec::new(),
            informed: Vec::new(),
            context: context.into(),
            drivers: Vec::new(),
            options: Vec::new(),
            decision: decision.into(),
            consequences: None,
            linked_assets: Vec::new(),
            supersedes: None,
            superseded_by: None,
            related_decisions: Vec::new(),
            related_knowledge: Vec::new(),
            compliance: None,
            confirmation_date: None,
            confirmation_notes: None,
            tags: Vec::new(),
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new decision with a timestamp-based number (YYMMDDHHmm format)
    /// This format prevents merge conflicts in distributed Git workflows
    pub fn new_with_timestamp(
        title: impl Into<String>,
        context: impl Into<String>,
        decision: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        let number = Self::generate_timestamp_number(&now);
        Self::new(number, title, context, decision)
    }

    /// Generate a timestamp-based decision number in YYMMDDHHmm format
    pub fn generate_timestamp_number(dt: &DateTime<Utc>) -> u64 {
        let formatted = dt.format("%y%m%d%H%M").to_string();
        formatted.parse().unwrap_or(0)
    }

    /// Generate a deterministic UUID for a decision based on its number
    pub fn generate_id(number: u64) -> Uuid {
        // Use UUID v5 with a namespace for decisions
        let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(); // URL namespace
        let name = format!("decision:{}", number);
        Uuid::new_v5(&namespace, name.as_bytes())
    }

    /// Add an author
    pub fn add_author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self.updated_at = Utc::now();
        self
    }

    /// Set consulted parties (RACI - Consulted)
    pub fn add_consulted(mut self, consulted: impl Into<String>) -> Self {
        self.consulted.push(consulted.into());
        self.updated_at = Utc::now();
        self
    }

    /// Set informed parties (RACI - Informed)
    pub fn add_informed(mut self, informed: impl Into<String>) -> Self {
        self.informed.push(informed.into());
        self.updated_at = Utc::now();
        self
    }

    /// Add a related decision
    pub fn add_related_decision(mut self, decision_id: Uuid) -> Self {
        self.related_decisions.push(decision_id);
        self.updated_at = Utc::now();
        self
    }

    /// Add a related knowledge article
    pub fn add_related_knowledge(mut self, article_id: Uuid) -> Self {
        self.related_knowledge.push(article_id);
        self.updated_at = Utc::now();
        self
    }

    /// Set decided_at timestamp
    pub fn with_decided_at(mut self, decided_at: DateTime<Utc>) -> Self {
        self.decided_at = Some(decided_at);
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

    /// Set the decision status
    pub fn with_status(mut self, status: DecisionStatus) -> Self {
        self.status = status;
        self.updated_at = Utc::now();
        self
    }

    /// Set the decision category
    pub fn with_category(mut self, category: DecisionCategory) -> Self {
        self.category = category;
        self.updated_at = Utc::now();
        self
    }

    /// Set the domain
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self.updated_at = Utc::now();
        self
    }

    /// Add a decider
    pub fn add_decider(mut self, decider: impl Into<String>) -> Self {
        self.deciders.push(decider.into());
        self.updated_at = Utc::now();
        self
    }

    /// Add a driver
    pub fn add_driver(mut self, driver: DecisionDriver) -> Self {
        self.drivers.push(driver);
        self.updated_at = Utc::now();
        self
    }

    /// Add an option
    pub fn add_option(mut self, option: DecisionOption) -> Self {
        self.options.push(option);
        self.updated_at = Utc::now();
        self
    }

    /// Set consequences
    pub fn with_consequences(mut self, consequences: impl Into<String>) -> Self {
        self.consequences = Some(consequences.into());
        self.updated_at = Utc::now();
        self
    }

    /// Add an asset link
    pub fn add_asset_link(mut self, link: AssetLink) -> Self {
        self.linked_assets.push(link);
        self.updated_at = Utc::now();
        self
    }

    /// Set compliance assessment
    pub fn with_compliance(mut self, compliance: ComplianceAssessment) -> Self {
        self.compliance = Some(compliance);
        self.updated_at = Utc::now();
        self
    }

    /// Mark this decision as superseding another
    pub fn supersedes_decision(mut self, other_id: Uuid) -> Self {
        self.supersedes = Some(other_id);
        self.updated_at = Utc::now();
        self
    }

    /// Mark this decision as superseded by another
    pub fn superseded_by_decision(&mut self, other_id: Uuid) {
        self.superseded_by = Some(other_id);
        self.status = DecisionStatus::Superseded;
        self.updated_at = Utc::now();
    }

    /// Add a tag
    pub fn add_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self.updated_at = Utc::now();
        self
    }

    /// Check if the decision number is timestamp-based (YYMMDDHHmm format - 10 digits)
    pub fn is_timestamp_number(&self) -> bool {
        self.number >= 1000000000 && self.number <= 9999999999
    }

    /// Format the decision number for display
    /// Returns "ADR-0001" for sequential or "ADR-2601101234" for timestamp-based
    pub fn formatted_number(&self) -> String {
        if self.is_timestamp_number() {
            format!("ADR-{}", self.number)
        } else {
            format!("ADR-{:04}", self.number)
        }
    }

    /// Generate the YAML filename for this decision
    pub fn filename(&self, workspace_name: &str) -> String {
        let number_str = if self.is_timestamp_number() {
            format!("{}", self.number)
        } else {
            format!("{:04}", self.number)
        };

        match &self.domain {
            Some(domain) => format!(
                "{}_{}_adr-{}.madr.yaml",
                sanitize_name(workspace_name),
                sanitize_name(domain),
                number_str
            ),
            None => format!(
                "{}_adr-{}.madr.yaml",
                sanitize_name(workspace_name),
                number_str
            ),
        }
    }

    /// Generate the Markdown filename for this decision
    pub fn markdown_filename(&self) -> String {
        let slug = slugify(&self.title);
        if self.is_timestamp_number() {
            format!("ADR-{}-{}.md", self.number, slug)
        } else {
            format!("ADR-{:04}-{}.md", self.number, slug)
        }
    }

    /// Import from YAML
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }

    /// Export to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Export to pretty YAML
    pub fn to_yaml_pretty(&self) -> Result<String, serde_yaml::Error> {
        // serde_yaml already produces pretty output
        serde_yaml::to_string(self)
    }
}

/// Decision index entry for the decisions.yaml file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionIndexEntry {
    /// Decision number (can be sequential or timestamp-based)
    pub number: u64,
    /// Decision UUID
    pub id: Uuid,
    /// Decision title
    pub title: String,
    /// Decision status
    pub status: DecisionStatus,
    /// Decision category
    pub category: DecisionCategory,
    /// Domain (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Filename of the decision YAML file
    pub file: String,
}

impl From<&Decision> for DecisionIndexEntry {
    fn from(decision: &Decision) -> Self {
        Self {
            number: decision.number,
            id: decision.id,
            title: decision.title.clone(),
            status: decision.status.clone(),
            category: decision.category.clone(),
            domain: decision.domain.clone(),
            file: String::new(), // Set by caller
        }
    }
}

/// Decision log index (decisions.yaml)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionIndex {
    /// Schema version
    #[serde(alias = "schema_version")]
    pub schema_version: String,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none", alias = "last_updated")]
    pub last_updated: Option<DateTime<Utc>>,
    /// List of decisions
    #[serde(default)]
    pub decisions: Vec<DecisionIndexEntry>,
    /// Next available decision number (for sequential numbering)
    #[serde(alias = "next_number")]
    pub next_number: u64,
    /// Whether to use timestamp-based numbering (YYMMDDHHmm format)
    #[serde(default, alias = "use_timestamp_numbering")]
    pub use_timestamp_numbering: bool,
}

impl Default for DecisionIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionIndex {
    /// Create a new empty decision index
    pub fn new() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            last_updated: Some(Utc::now()),
            decisions: Vec::new(),
            next_number: 1,
            use_timestamp_numbering: false,
        }
    }

    /// Create a new decision index with timestamp-based numbering
    pub fn new_with_timestamp_numbering() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            last_updated: Some(Utc::now()),
            decisions: Vec::new(),
            next_number: 1,
            use_timestamp_numbering: true,
        }
    }

    /// Add a decision to the index
    pub fn add_decision(&mut self, decision: &Decision, filename: String) {
        let mut entry = DecisionIndexEntry::from(decision);
        entry.file = filename;

        // Remove existing entry with same number if present
        self.decisions.retain(|d| d.number != decision.number);
        self.decisions.push(entry);

        // Sort by number
        self.decisions.sort_by_key(|d| d.number);

        // Update next number only for sequential numbering
        if !self.use_timestamp_numbering && decision.number >= self.next_number {
            self.next_number = decision.number + 1;
        }

        self.last_updated = Some(Utc::now());
    }

    /// Get the next available decision number
    /// For timestamp-based numbering, generates a new timestamp
    /// For sequential numbering, returns the next sequential number
    pub fn get_next_number(&self) -> u64 {
        if self.use_timestamp_numbering {
            Decision::generate_timestamp_number(&Utc::now())
        } else {
            self.next_number
        }
    }

    /// Find a decision by number
    pub fn find_by_number(&self, number: u64) -> Option<&DecisionIndexEntry> {
        self.decisions.iter().find(|d| d.number == number)
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
    fn test_decision_new() {
        let decision = Decision::new(
            1,
            "Use ODCS v3.1.0",
            "We need a standard format",
            "We will use ODCS v3.1.0",
        );

        assert_eq!(decision.number, 1);
        assert_eq!(decision.title, "Use ODCS v3.1.0");
        assert_eq!(decision.status, DecisionStatus::Proposed);
        assert_eq!(decision.category, DecisionCategory::Architecture);
    }

    #[test]
    fn test_decision_builder_pattern() {
        let decision = Decision::new(1, "Test", "Context", "Decision")
            .with_status(DecisionStatus::Accepted)
            .with_category(DecisionCategory::DataDesign)
            .with_domain("sales")
            .add_decider("team@example.com")
            .add_driver(DecisionDriver::with_priority(
                "Need consistency",
                DriverPriority::High,
            ))
            .with_consequences("Better consistency");

        assert_eq!(decision.status, DecisionStatus::Accepted);
        assert_eq!(decision.category, DecisionCategory::DataDesign);
        assert_eq!(decision.domain, Some("sales".to_string()));
        assert_eq!(decision.deciders.len(), 1);
        assert_eq!(decision.drivers.len(), 1);
        assert!(decision.consequences.is_some());
    }

    #[test]
    fn test_decision_id_generation() {
        let id1 = Decision::generate_id(1);
        let id2 = Decision::generate_id(1);
        let id3 = Decision::generate_id(2);

        // Same number should generate same ID
        assert_eq!(id1, id2);
        // Different numbers should generate different IDs
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_decision_filename() {
        let decision = Decision::new(1, "Test", "Context", "Decision");
        assert_eq!(
            decision.filename("enterprise"),
            "enterprise_adr-0001.madr.yaml"
        );

        let decision_with_domain = decision.with_domain("sales");
        assert_eq!(
            decision_with_domain.filename("enterprise"),
            "enterprise_sales_adr-0001.madr.yaml"
        );
    }

    #[test]
    fn test_decision_markdown_filename() {
        let decision = Decision::new(
            1,
            "Use ODCS v3.1.0 for all data contracts",
            "Context",
            "Decision",
        );
        let filename = decision.markdown_filename();
        assert!(filename.starts_with("ADR-0001-"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_decision_yaml_roundtrip() {
        let decision = Decision::new(1, "Test Decision", "Some context", "The decision")
            .with_status(DecisionStatus::Accepted)
            .with_domain("test");

        let yaml = decision.to_yaml().unwrap();
        let parsed = Decision::from_yaml(&yaml).unwrap();

        assert_eq!(decision.id, parsed.id);
        assert_eq!(decision.title, parsed.title);
        assert_eq!(decision.status, parsed.status);
        assert_eq!(decision.domain, parsed.domain);
    }

    #[test]
    fn test_decision_index() {
        let mut index = DecisionIndex::new();
        assert_eq!(index.get_next_number(), 1);

        let decision1 = Decision::new(1, "First", "Context", "Decision");
        index.add_decision(&decision1, "test_adr-0001.madr.yaml".to_string());

        assert_eq!(index.decisions.len(), 1);
        assert_eq!(index.get_next_number(), 2);

        let decision2 = Decision::new(2, "Second", "Context", "Decision");
        index.add_decision(&decision2, "test_adr-0002.madr.yaml".to_string());

        assert_eq!(index.decisions.len(), 2);
        assert_eq!(index.get_next_number(), 3);
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Use ODCS v3.1.0"), "use-odcs-v3-1-0");
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("test--double"), "test-double");
    }

    #[test]
    fn test_decision_status_display() {
        assert_eq!(format!("{}", DecisionStatus::Proposed), "Proposed");
        assert_eq!(format!("{}", DecisionStatus::Accepted), "Accepted");
        assert_eq!(format!("{}", DecisionStatus::Deprecated), "Deprecated");
        assert_eq!(format!("{}", DecisionStatus::Superseded), "Superseded");
        assert_eq!(format!("{}", DecisionStatus::Rejected), "Rejected");
    }

    #[test]
    fn test_asset_link() {
        let link = AssetLink::with_relationship(
            "odcs",
            Uuid::new_v4(),
            "orders",
            AssetRelationship::Implements,
        );

        assert_eq!(link.asset_type, "odcs");
        assert_eq!(link.asset_name, "orders");
        assert_eq!(link.relationship, Some(AssetRelationship::Implements));
    }

    #[test]
    fn test_timestamp_number_generation() {
        use chrono::TimeZone;
        let dt = Utc.with_ymd_and_hms(2026, 1, 10, 14, 30, 0).unwrap();
        let number = Decision::generate_timestamp_number(&dt);
        assert_eq!(number, 2601101430);
    }

    #[test]
    fn test_is_timestamp_number() {
        let sequential_decision = Decision::new(1, "Test", "Context", "Decision");
        assert!(!sequential_decision.is_timestamp_number());

        let timestamp_decision = Decision::new(2601101430, "Test", "Context", "Decision");
        assert!(timestamp_decision.is_timestamp_number());
    }

    #[test]
    fn test_timestamp_decision_filename() {
        let decision = Decision::new(2601101430, "Test", "Context", "Decision");
        assert_eq!(
            decision.filename("enterprise"),
            "enterprise_adr-2601101430.madr.yaml"
        );
    }

    #[test]
    fn test_timestamp_decision_markdown_filename() {
        let decision = Decision::new(2601101430, "Test Decision", "Context", "Decision");
        let filename = decision.markdown_filename();
        assert!(filename.starts_with("ADR-2601101430-"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_decision_with_consulted_informed() {
        let decision = Decision::new(1, "Test", "Context", "Decision")
            .add_consulted("security@example.com")
            .add_informed("stakeholders@example.com");

        assert_eq!(decision.consulted.len(), 1);
        assert_eq!(decision.informed.len(), 1);
        assert_eq!(decision.consulted[0], "security@example.com");
        assert_eq!(decision.informed[0], "stakeholders@example.com");
    }

    #[test]
    fn test_decision_with_authors() {
        let decision = Decision::new(1, "Test", "Context", "Decision")
            .add_author("author1@example.com")
            .add_author("author2@example.com");

        assert_eq!(decision.authors.len(), 2);
    }

    #[test]
    fn test_decision_index_with_timestamp_numbering() {
        let index = DecisionIndex::new_with_timestamp_numbering();
        assert!(index.use_timestamp_numbering);

        // The next number should be a timestamp
        let next = index.get_next_number();
        assert!(next >= 1000000000); // Timestamp format check
    }

    #[test]
    fn test_new_categories() {
        assert_eq!(format!("{}", DecisionCategory::Data), "Data");
        assert_eq!(format!("{}", DecisionCategory::Integration), "Integration");
    }

    #[test]
    fn test_decision_with_related() {
        let related_decision_id = Uuid::new_v4();
        let related_knowledge_id = Uuid::new_v4();

        let decision = Decision::new(1, "Test", "Context", "Decision")
            .add_related_decision(related_decision_id)
            .add_related_knowledge(related_knowledge_id);

        assert_eq!(decision.related_decisions.len(), 1);
        assert_eq!(decision.related_knowledge.len(), 1);
        assert_eq!(decision.related_decisions[0], related_decision_id);
        assert_eq!(decision.related_knowledge[0], related_knowledge_id);
    }

    #[test]
    fn test_decision_status_draft() {
        let decision =
            Decision::new(1, "Test", "Context", "Decision").with_status(DecisionStatus::Draft);
        assert_eq!(decision.status, DecisionStatus::Draft);
        assert_eq!(format!("{}", DecisionStatus::Draft), "Draft");
    }
}
