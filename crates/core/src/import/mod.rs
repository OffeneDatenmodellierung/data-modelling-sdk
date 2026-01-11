//! Import functionality
//!
//! Provides parsers for importing data models from various formats:
//! - SQL (CREATE TABLE statements)
//! - ODCS (Open Data Contract Standard) v3.1.0 YAML format (legacy ODCL formats supported for import)
//! - JSON Schema
//! - AVRO
//! - Protobuf
//! - Decision (MADR-compliant decision records)
//! - Knowledge (Knowledge Base articles)

pub mod avro;
#[cfg(feature = "bpmn")]
pub mod bpmn;
pub mod cads;
pub mod decision;
#[cfg(feature = "dmn")]
pub mod dmn;
pub mod json_schema;
pub mod knowledge;
pub mod odcl;
pub mod odcs;
pub mod odcs_shared;
pub mod odps;
#[cfg(feature = "openapi")]
pub mod openapi;
pub mod protobuf;
pub mod sql;

// anyhow::Result not currently used in this module

/// Result of an import operation.
///
/// Contains extracted tables and any errors/warnings from the import process.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[must_use = "import results should be processed or errors checked"]
pub struct ImportResult {
    /// Tables extracted from the import
    pub tables: Vec<TableData>,
    /// Tables that require name input (for SQL imports with unnamed tables)
    pub tables_requiring_name: Vec<TableRequiringName>,
    /// Parse errors/warnings
    pub errors: Vec<ImportError>,
    /// Whether AI suggestions are available
    pub ai_suggestions: Option<Vec<serde_json::Value>>,
}

/// Error during import
#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum ImportError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("BPMN validation error: {0}")]
    BPMNValidationError(String),
    #[error("DMN validation error: {0}")]
    DMNValidationError(String),
    #[error("OpenAPI validation error: {0}")]
    OpenAPIValidationError(String),
    #[error("BPMN parse error: {0}")]
    BPMNParseError(String),
    #[error("DMN parse error: {0}")]
    DMNParseError(String),
    #[error("OpenAPI parse error: {0}")]
    OpenAPIParseError(String),
}

/// Table data from import - preserves all ODCS v3.1.0 contract-level fields
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableData {
    /// Index of this table in the import result
    pub table_index: usize,

    // === ODCS Contract Identity Fields ===
    /// Table/Contract UUID from ODCS `id` field (preserved from source file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Contract/table name (ODCS: name)
    pub name: Option<String>,
    /// ODCS API version (e.g., "v3.1.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Contract version (ODCS: version)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Contract status (ODCS: status) - e.g., "draft", "active", "deprecated"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Contract kind (ODCS: kind) - typically "DataContract"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    // === Domain & Organization ===
    /// Domain name (ODCS: domain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Data product name (ODCS: dataProduct)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_product: Option<String>,
    /// Tenant identifier (ODCS: tenant)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    // === Description (ODCS description object) ===
    /// High-level description object containing usage, purpose, limitations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,

    // === Schema/Columns ===
    /// Column definitions (from ODCS schema.properties)
    pub columns: Vec<ColumnData>,

    // === Server Configuration ===
    /// Server definitions (ODCS: servers)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<serde_json::Value>,

    // === Team & Support ===
    /// Team information (ODCS: team)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<serde_json::Value>,
    /// Support information (ODCS: support)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support: Option<serde_json::Value>,

    // === Roles & Access ===
    /// Role definitions (ODCS: roles)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<serde_json::Value>,

    // === SLA & Quality ===
    /// SLA properties (ODCS: slaProperties)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sla_properties: Vec<serde_json::Value>,
    /// Contract-level quality rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quality: Vec<std::collections::HashMap<String, serde_json::Value>>,

    // === Pricing ===
    /// Pricing information (ODCS: price)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<serde_json::Value>,

    // === Tags & Custom Properties ===
    /// Contract-level tags (ODCS: tags)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Custom properties (ODCS: customProperties)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_properties: Vec<serde_json::Value>,
    /// Authoritative definitions (ODCS: authoritativeDefinitions)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authoritative_definitions: Vec<serde_json::Value>,

    // === Timestamps ===
    /// Contract creation timestamp (ODCS: contractCreatedTs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_created_ts: Option<String>,

    // === Legacy/Metadata Storage ===
    /// Additional ODCS metadata not captured in specific fields (for backward compatibility)
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub odcs_metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Column data from import - mirrors Column struct exactly to preserve all ODCS v3.1.0 fields
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnData {
    // === Core Identity Fields ===
    /// Stable technical identifier (ODCS: id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Column name (ODCS: name)
    pub name: String,
    /// Business name for the column (ODCS: businessName)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_name: Option<String>,
    /// Column description/documentation (ODCS: description)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // === Type Information ===
    /// Logical data type (ODCS: logicalType)
    #[serde(rename = "dataType")]
    pub data_type: String,
    /// Physical database type (ODCS: physicalType)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_type: Option<String>,
    /// Physical name in the data source (ODCS: physicalName)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_name: Option<String>,
    /// Additional type options (ODCS: logicalTypeOptions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_type_options: Option<crate::models::LogicalTypeOptions>,

    // === Key Constraints ===
    /// Whether this column is part of the primary key (ODCS: primaryKey)
    #[serde(default)]
    pub primary_key: bool,
    /// Position in composite primary key, 1-based (ODCS: primaryKeyPosition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key_position: Option<i32>,
    /// Whether the column contains unique values (ODCS: unique)
    #[serde(default)]
    pub unique: bool,
    /// Whether the column allows NULL values (inverse of ODCS: required)
    #[serde(default = "default_true")]
    pub nullable: bool,

    // === Partitioning & Clustering ===
    /// Whether the column is used for partitioning (ODCS: partitioned)
    #[serde(default)]
    pub partitioned: bool,
    /// Position in partition key, 1-based (ODCS: partitionKeyPosition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_key_position: Option<i32>,
    /// Whether the column is used for clustering
    #[serde(default)]
    pub clustered: bool,

    // === Data Classification & Security ===
    /// Data classification level (ODCS: classification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    /// Whether this is a critical data element (ODCS: criticalDataElement)
    #[serde(default)]
    pub critical_data_element: bool,
    /// Name of the encrypted version of this column (ODCS: encryptedName)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_name: Option<String>,

    // === Transformation Metadata ===
    /// Source objects used in transformation (ODCS: transformSourceObjects)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transform_source_objects: Vec<String>,
    /// Transformation logic/expression (ODCS: transformLogic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_logic: Option<String>,
    /// Human-readable transformation description (ODCS: transformDescription)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_description: Option<String>,

    // === Examples & Documentation ===
    /// Example values for this column (ODCS: examples)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<serde_json::Value>,
    /// Default value for the column
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,

    // === Relationships & References ===
    /// ODCS v3.1.0 relationships (property-level references)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<crate::models::PropertyRelationship>,
    /// Authoritative definitions (ODCS: authoritativeDefinitions)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authoritative_definitions: Vec<crate::models::AuthoritativeDefinition>,

    // === Quality & Validation ===
    /// Quality rules and checks (ODCS: quality)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Vec<std::collections::HashMap<String, serde_json::Value>>>,
    /// Enum values if this column is an enumeration type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,

    // === Tags & Custom Properties ===
    /// Property-level tags (ODCS: tags)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Custom properties for format-specific metadata
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub custom_properties: std::collections::HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl Default for ColumnData {
    fn default() -> Self {
        Self {
            // Core Identity
            id: None,
            name: String::new(),
            business_name: None,
            description: None,
            // Type Information
            data_type: String::new(),
            physical_type: None,
            physical_name: None,
            logical_type_options: None,
            // Key Constraints
            primary_key: false,
            primary_key_position: None,
            unique: false,
            nullable: true,
            // Partitioning & Clustering
            partitioned: false,
            partition_key_position: None,
            clustered: false,
            // Data Classification & Security
            classification: None,
            critical_data_element: false,
            encrypted_name: None,
            // Transformation Metadata
            transform_source_objects: Vec::new(),
            transform_logic: None,
            transform_description: None,
            // Examples & Documentation
            examples: Vec::new(),
            default_value: None,
            // Relationships & References
            relationships: Vec::new(),
            authoritative_definitions: Vec::new(),
            // Quality & Validation
            quality: None,
            enum_values: None,
            // Tags & Custom Properties
            tags: Vec::new(),
            custom_properties: std::collections::HashMap::new(),
        }
    }
}

// Re-export for convenience
pub use avro::AvroImporter;
pub use cads::CADSImporter;
pub use decision::DecisionImporter;
pub use json_schema::JSONSchemaImporter;
pub use knowledge::KnowledgeImporter;
pub use odcl::ODCLImporter;
pub use odcs::ODCSImporter;
pub use odcs_shared::ParserError;
pub use odps::ODPSImporter;
pub use protobuf::ProtobufImporter;
pub use sql::SQLImporter;

/// Table requiring name input (for SQL imports)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TableRequiringName {
    pub table_index: usize,
    pub suggested_name: Option<String>,
}
