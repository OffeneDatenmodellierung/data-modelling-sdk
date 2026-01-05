//! Data Modelling SDK - Shared library for model operations across platforms
//!
//! Provides unified interfaces for:
//! - File/folder operations (via storage backends)
//! - Model loading/saving
//! - Import/export functionality
//! - Validation logic
//! - Authentication types (shared across web, desktop, mobile)
//! - Workspace management types

pub mod auth;
#[cfg(feature = "cli")]
pub mod cli;
pub mod convert;
pub mod export;
#[cfg(feature = "git")]
pub mod git;
pub mod import;
pub mod model;
pub mod models;
pub mod storage;
pub mod validation;
pub mod workspace;

// Re-export commonly used types
#[cfg(feature = "api-backend")]
pub use storage::api::ApiStorageBackend;
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub use storage::browser::BrowserStorageBackend;
#[cfg(feature = "native-fs")]
pub use storage::filesystem::FileSystemStorageBackend;
pub use storage::{StorageBackend, StorageError};

pub use convert::{ConversionError, convert_to_odcs};
#[cfg(feature = "png-export")]
pub use export::PNGExporter;
pub use export::{
    AvroExporter, ExportError, ExportResult, JSONSchemaExporter, ODCSExporter, ProtobufExporter,
    SQLExporter,
};
pub use import::{
    AvroImporter, ImportError, ImportResult, JSONSchemaImporter, ODCSImporter, ProtobufImporter,
    SQLImporter,
};
#[cfg(feature = "api-backend")]
pub use model::ApiModelLoader;
pub use model::{ModelLoader, ModelSaver};
pub use validation::{
    RelationshipValidationError, RelationshipValidationResult, TableValidationError,
    TableValidationResult,
};

// Re-export models
pub use models::enums::*;
pub use models::{Column, ContactDetails, DataModel, ForeignKey, Relationship, SlaProperty, Table};

// Re-export auth types
pub use auth::{
    AuthMode, AuthState, GitHubEmail, InitiateOAuthRequest, InitiateOAuthResponse,
    SelectEmailRequest,
};

// Re-export workspace types
pub use workspace::{
    CreateWorkspaceRequest, CreateWorkspaceResponse, ListProfilesResponse, LoadProfileRequest,
    ProfileInfo, WorkspaceInfo,
};

// Re-export Git types
#[cfg(feature = "git")]
pub use git::{GitError, GitService, GitStatus};

// WASM bindings for import/export functions
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
mod wasm {
    use crate::export::ExportError;
    use crate::import::{ImportError, ImportResult};
    use crate::models::DataModel;
    use js_sys;
    use serde_json;
    use serde_yaml;
    use uuid;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures;

    /// Convert ImportError to JsValue for JavaScript error handling
    fn import_error_to_js(err: ImportError) -> JsValue {
        JsValue::from_str(&err.to_string())
    }

    /// Convert ExportError to JsValue for JavaScript error handling
    fn export_error_to_js(err: ExportError) -> JsValue {
        JsValue::from_str(&err.to_string())
    }

    /// Serialize ImportResult to JSON string
    fn serialize_import_result(result: &ImportResult) -> Result<String, JsValue> {
        serde_json::to_string(result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Deserialize workspace structure from JSON string
    fn deserialize_workspace(json: &str) -> Result<DataModel, JsValue> {
        serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))
    }

    /// Parse ODCS YAML content and return a structured workspace representation.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - ODCS YAML content as a string
    ///
    /// # Returns
    ///
    /// JSON string containing ImportResult object, or JsValue error
    #[wasm_bindgen]
    pub fn parse_odcs_yaml(yaml_content: &str) -> Result<String, JsValue> {
        let mut importer = crate::import::ODCSImporter::new();
        match importer.import(yaml_content) {
            Ok(result) => serialize_import_result(&result),
            Err(err) => Err(import_error_to_js(err)),
        }
    }

    /// Export a workspace structure to ODCS YAML format.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    ///
    /// # Returns
    ///
    /// ODCS YAML format string, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_odcs_yaml(workspace_json: &str) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;

        // Export all tables as separate YAML documents, joined with ---\n
        let exports = crate::export::ODCSExporter::export_model(&model, None, "odcs_v3_1_0");

        // Combine all YAML documents into a single multi-document string
        let yaml_docs: Vec<String> = exports.values().cloned().collect();
        Ok(yaml_docs.join("\n---\n"))
    }

    /// Import data model from SQL CREATE TABLE statements.
    ///
    /// # Arguments
    ///
    /// * `sql_content` - SQL CREATE TABLE statements
    /// * `dialect` - SQL dialect ("postgresql", "mysql", "sqlserver", "databricks")
    ///
    /// # Returns
    ///
    /// JSON string containing ImportResult object, or JsValue error
    #[wasm_bindgen]
    pub fn import_from_sql(sql_content: &str, dialect: &str) -> Result<String, JsValue> {
        let importer = crate::import::SQLImporter::new(dialect);
        match importer.parse(sql_content) {
            Ok(result) => serialize_import_result(&result),
            Err(err) => Err(JsValue::from_str(&format!("Parse error: {}", err))),
        }
    }

    /// Import data model from AVRO schema.
    ///
    /// # Arguments
    ///
    /// * `avro_content` - AVRO schema JSON as a string
    ///
    /// # Returns
    ///
    /// JSON string containing ImportResult object, or JsValue error
    #[wasm_bindgen]
    pub fn import_from_avro(avro_content: &str) -> Result<String, JsValue> {
        let importer = crate::import::AvroImporter::new();
        match importer.import(avro_content) {
            Ok(result) => serialize_import_result(&result),
            Err(err) => Err(import_error_to_js(err)),
        }
    }

    /// Import data model from JSON Schema definition.
    ///
    /// # Arguments
    ///
    /// * `json_schema_content` - JSON Schema definition as a string
    ///
    /// # Returns
    ///
    /// JSON string containing ImportResult object, or JsValue error
    #[wasm_bindgen]
    pub fn import_from_json_schema(json_schema_content: &str) -> Result<String, JsValue> {
        let importer = crate::import::JSONSchemaImporter::new();
        match importer.import(json_schema_content) {
            Ok(result) => serialize_import_result(&result),
            Err(err) => Err(import_error_to_js(err)),
        }
    }

    /// Import data model from Protobuf schema.
    ///
    /// # Arguments
    ///
    /// * `protobuf_content` - Protobuf schema text
    ///
    /// # Returns
    ///
    /// JSON string containing ImportResult object, or JsValue error
    #[wasm_bindgen]
    pub fn import_from_protobuf(protobuf_content: &str) -> Result<String, JsValue> {
        let importer = crate::import::ProtobufImporter::new();
        match importer.import(protobuf_content) {
            Ok(result) => serialize_import_result(&result),
            Err(err) => Err(import_error_to_js(err)),
        }
    }

    /// Export a data model to SQL CREATE TABLE statements.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `dialect` - SQL dialect ("postgresql", "mysql", "sqlserver", "databricks")
    ///
    /// # Returns
    ///
    /// SQL CREATE TABLE statements, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_sql(workspace_json: &str, dialect: &str) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let exporter = crate::export::SQLExporter;
        match exporter.export(&model.tables, Some(dialect)) {
            Ok(result) => Ok(result.content),
            Err(err) => Err(export_error_to_js(err)),
        }
    }

    /// Export a data model to AVRO schema.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    ///
    /// # Returns
    ///
    /// AVRO schema JSON string, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_avro(workspace_json: &str) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let exporter = crate::export::AvroExporter;
        match exporter.export(&model.tables) {
            Ok(result) => Ok(result.content),
            Err(err) => Err(export_error_to_js(err)),
        }
    }

    /// Export a data model to JSON Schema definition.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    ///
    /// # Returns
    ///
    /// JSON Schema definition string, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_json_schema(workspace_json: &str) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let exporter = crate::export::JSONSchemaExporter;
        match exporter.export(&model.tables) {
            Ok(result) => Ok(result.content),
            Err(err) => Err(export_error_to_js(err)),
        }
    }

    /// Export a data model to Protobuf schema.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    ///
    /// # Returns
    ///
    /// Protobuf schema text, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_protobuf(workspace_json: &str) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let exporter = crate::export::ProtobufExporter;
        match exporter.export(&model.tables) {
            Ok(result) => Ok(result.content),
            Err(err) => Err(export_error_to_js(err)),
        }
    }

    /// Import CADS YAML content and return a structured representation.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - CADS YAML content as a string
    ///
    /// # Returns
    ///
    /// JSON string containing CADS asset, or JsValue error
    #[wasm_bindgen]
    pub fn import_from_cads(yaml_content: &str) -> Result<String, JsValue> {
        let importer = crate::import::CADSImporter::new();
        match importer.import(yaml_content) {
            Ok(asset) => serde_json::to_string(&asset)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(err) => Err(import_error_to_js(err)),
        }
    }

    /// Export a CADS asset to YAML format.
    ///
    /// # Arguments
    ///
    /// * `asset_json` - JSON string containing CADS asset
    ///
    /// # Returns
    ///
    /// CADS YAML format string, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_cads(asset_json: &str) -> Result<String, JsValue> {
        let asset: crate::models::cads::CADSAsset = serde_json::from_str(asset_json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        let exporter = crate::export::CADSExporter;
        match exporter.export(&asset) {
            Ok(yaml) => Ok(yaml),
            Err(err) => Err(export_error_to_js(err)),
        }
    }

    /// Import ODPS YAML content and return a structured representation.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - ODPS YAML content as a string
    ///
    /// # Returns
    ///
    /// JSON string containing ODPS data product, or JsValue error
    #[wasm_bindgen]
    pub fn import_from_odps(yaml_content: &str) -> Result<String, JsValue> {
        let importer = crate::import::ODPSImporter::new();
        match importer.import(yaml_content) {
            Ok(product) => serde_json::to_string(&product)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(err) => Err(import_error_to_js(err)),
        }
    }

    /// Export an ODPS data product to YAML format.
    ///
    /// # Arguments
    ///
    /// * `product_json` - JSON string containing ODPS data product
    ///
    /// # Returns
    ///
    /// ODPS YAML format string, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_odps(product_json: &str) -> Result<String, JsValue> {
        let product: crate::models::odps::ODPSDataProduct = serde_json::from_str(product_json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        let exporter = crate::export::ODPSExporter;
        match exporter.export(&product) {
            Ok(yaml) => Ok(yaml),
            Err(err) => Err(export_error_to_js(err)),
        }
    }

    /// Validate ODPS YAML content against the ODPS JSON Schema.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - ODPS YAML content as a string
    ///
    /// # Returns
    ///
    /// Empty string on success, or error message string
    #[cfg(feature = "odps-validation")]
    #[wasm_bindgen]
    pub fn validate_odps(yaml_content: &str) -> Result<(), JsValue> {
        #[cfg(feature = "cli")]
        {
            use crate::cli::validation::validate_odps_internal;
            validate_odps_internal(yaml_content).map_err(|e| JsValue::from_str(&e.to_string()))
        }
        #[cfg(not(feature = "cli"))]
        {
            // Inline validation when CLI feature is not enabled
            use jsonschema::Validator;
            use serde_json::Value;

            let schema_content = include_str!("../schemas/odps-json-schema-latest.json");
            let schema: Value = serde_json::from_str(schema_content)
                .map_err(|e| JsValue::from_str(&format!("Failed to load ODPS schema: {}", e)))?;

            let validator = Validator::new(&schema)
                .map_err(|e| JsValue::from_str(&format!("Failed to compile ODPS schema: {}", e)))?;

            let data: Value = serde_yaml::from_str(yaml_content)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse YAML: {}", e)))?;

            if let Err(error) = validator.validate(&data) {
                return Err(JsValue::from_str(&format!(
                    "ODPS validation failed: {}",
                    error
                )));
            }

            Ok(())
        }
    }

    #[cfg(not(feature = "odps-validation"))]
    #[wasm_bindgen]
    pub fn validate_odps(_yaml_content: &str) -> Result<(), JsValue> {
        // Validation disabled - feature not enabled
        // Return success to maintain backward compatibility
        Ok(())
    }

    /// Create a new business domain.
    ///
    /// # Arguments
    ///
    /// * `name` - Domain name
    ///
    /// # Returns
    ///
    /// JSON string containing Domain, or JsValue error
    #[wasm_bindgen]
    pub fn create_domain(name: &str) -> Result<String, JsValue> {
        let domain = crate::models::domain::Domain::new(name.to_string());
        serde_json::to_string(&domain)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Import Domain YAML content and return a structured representation.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Domain YAML content as a string
    ///
    /// # Returns
    ///
    /// JSON string containing Domain, or JsValue error
    #[wasm_bindgen]
    pub fn import_from_domain(yaml_content: &str) -> Result<String, JsValue> {
        match crate::models::domain::Domain::from_yaml(yaml_content) {
            Ok(domain) => serde_json::to_string(&domain)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("Parse error: {}", e))),
        }
    }

    /// Export a Domain to YAML format.
    ///
    /// # Arguments
    ///
    /// * `domain_json` - JSON string containing Domain
    ///
    /// # Returns
    ///
    /// Domain YAML format string, or JsValue error
    #[wasm_bindgen]
    pub fn export_to_domain(domain_json: &str) -> Result<String, JsValue> {
        let domain: crate::models::domain::Domain = serde_json::from_str(domain_json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        domain
            .to_yaml()
            .map_err(|e| JsValue::from_str(&format!("YAML serialization error: {}", e)))
    }

    /// Migrate DataFlow YAML to Domain schema format.
    ///
    /// # Arguments
    ///
    /// * `dataflow_yaml` - DataFlow YAML content as a string
    /// * `domain_name` - Optional domain name (defaults to "MigratedDomain")
    ///
    /// # Returns
    ///
    /// JSON string containing Domain, or JsValue error
    #[wasm_bindgen]
    pub fn migrate_dataflow_to_domain(
        dataflow_yaml: &str,
        domain_name: Option<String>,
    ) -> Result<String, JsValue> {
        match crate::convert::migrate_dataflow::migrate_dataflow_to_domain(
            dataflow_yaml,
            domain_name.as_deref(),
        ) {
            Ok(domain) => serde_json::to_string(&domain)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("Migration error: {}", e))),
        }
    }

    /// Parse a tag string into a Tag enum.
    ///
    /// # Arguments
    ///
    /// * `tag_str` - Tag string (Simple, Pair, or List format)
    ///
    /// # Returns
    ///
    /// JSON string containing Tag, or JsValue error
    #[wasm_bindgen]
    pub fn parse_tag(tag_str: &str) -> Result<String, JsValue> {
        use crate::models::Tag;
        use std::str::FromStr;
        match Tag::from_str(tag_str) {
            Ok(tag) => serde_json::to_string(&tag)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(_) => Err(JsValue::from_str("Invalid tag format")),
        }
    }

    /// Serialize a Tag enum to string format.
    ///
    /// # Arguments
    ///
    /// * `tag_json` - JSON string containing Tag
    ///
    /// # Returns
    ///
    /// Tag string (Simple, Pair, or List format), or JsValue error
    #[wasm_bindgen]
    pub fn serialize_tag(tag_json: &str) -> Result<String, JsValue> {
        use crate::models::Tag;
        let tag: Tag = serde_json::from_str(tag_json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        Ok(tag.to_string())
    }

    /// Convert any format to ODCS v3.1.0 YAML format.
    ///
    /// # Arguments
    ///
    /// * `input` - Format-specific content as a string
    /// * `format` - Optional format identifier. If None, attempts auto-detection.
    ///   Supported formats: "sql", "json_schema", "avro", "protobuf", "odcl", "odcs", "cads", "odps", "domain"
    ///
    /// # Returns
    ///
    /// ODCS v3.1.0 YAML string, or JsValue error
    #[wasm_bindgen]
    pub fn convert_to_odcs(input: &str, format: Option<String>) -> Result<String, JsValue> {
        match crate::convert::convert_to_odcs(input, format.as_deref()) {
            Ok(yaml) => Ok(yaml),
            Err(e) => Err(JsValue::from_str(&format!("Conversion error: {}", e))),
        }
    }

    /// Filter Data Flow nodes (tables) by owner.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `owner` - Owner name to filter by (case-sensitive exact match)
    ///
    /// # Returns
    ///
    /// JSON string containing array of matching tables, or JsValue error
    #[wasm_bindgen]
    pub fn filter_nodes_by_owner(workspace_json: &str, owner: &str) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let filtered = model.filter_nodes_by_owner(owner);
        serde_json::to_string(&filtered)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Filter Data Flow relationships by owner.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `owner` - Owner name to filter by (case-sensitive exact match)
    ///
    /// # Returns
    ///
    /// JSON string containing array of matching relationships, or JsValue error
    #[wasm_bindgen]
    pub fn filter_relationships_by_owner(
        workspace_json: &str,
        owner: &str,
    ) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let filtered = model.filter_relationships_by_owner(owner);
        serde_json::to_string(&filtered)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Filter Data Flow nodes (tables) by infrastructure type.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `infrastructure_type` - Infrastructure type string (e.g., "Kafka", "PostgreSQL")
    ///
    /// # Returns
    ///
    /// JSON string containing array of matching tables, or JsValue error
    #[wasm_bindgen]
    pub fn filter_nodes_by_infrastructure_type(
        workspace_json: &str,
        infrastructure_type: &str,
    ) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let infra_type: crate::models::enums::InfrastructureType =
            serde_json::from_str(&format!("\"{}\"", infrastructure_type)).map_err(|e| {
                JsValue::from_str(&format!(
                    "Invalid infrastructure type '{}': {}",
                    infrastructure_type, e
                ))
            })?;
        let filtered = model.filter_nodes_by_infrastructure_type(infra_type);
        serde_json::to_string(&filtered)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Filter Data Flow relationships by infrastructure type.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `infrastructure_type` - Infrastructure type string (e.g., "Kafka", "PostgreSQL")
    ///
    /// # Returns
    ///
    /// JSON string containing array of matching relationships, or JsValue error
    #[wasm_bindgen]
    pub fn filter_relationships_by_infrastructure_type(
        workspace_json: &str,
        infrastructure_type: &str,
    ) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let infra_type: crate::models::enums::InfrastructureType =
            serde_json::from_str(&format!("\"{}\"", infrastructure_type)).map_err(|e| {
                JsValue::from_str(&format!(
                    "Invalid infrastructure type '{}': {}",
                    infrastructure_type, e
                ))
            })?;
        let filtered = model.filter_relationships_by_infrastructure_type(infra_type);
        serde_json::to_string(&filtered)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Filter Data Flow nodes and relationships by tag.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `tag` - Tag to filter by
    ///
    /// # Returns
    ///
    /// JSON string containing object with `nodes` and `relationships` arrays, or JsValue error
    #[wasm_bindgen]
    pub fn filter_by_tags(workspace_json: &str, tag: &str) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let (nodes, relationships) = model.filter_by_tags(tag);
        let result = serde_json::json!({
            "nodes": nodes,
            "relationships": relationships
        });
        serde_json::to_string(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    // ============================================================================
    // Domain Operations
    // ============================================================================

    /// Add a system to a domain in a DataModel.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `domain_id` - Domain UUID as string
    /// * `system_json` - JSON string containing System
    ///
    /// # Returns
    ///
    /// JSON string containing updated DataModel, or JsValue error
    #[wasm_bindgen]
    pub fn add_system_to_domain(
        workspace_json: &str,
        domain_id: &str,
        system_json: &str,
    ) -> Result<String, JsValue> {
        let mut model = deserialize_workspace(workspace_json)?;
        let domain_uuid = uuid::Uuid::parse_str(domain_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid domain ID: {}", e)))?;
        let system: crate::models::domain::System = serde_json::from_str(system_json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        model
            .add_system_to_domain(domain_uuid, system)
            .map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&model)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Add a CADS node to a domain in a DataModel.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `domain_id` - Domain UUID as string
    /// * `node_json` - JSON string containing CADSNode
    ///
    /// # Returns
    ///
    /// JSON string containing updated DataModel, or JsValue error
    #[wasm_bindgen]
    pub fn add_cads_node_to_domain(
        workspace_json: &str,
        domain_id: &str,
        node_json: &str,
    ) -> Result<String, JsValue> {
        let mut model = deserialize_workspace(workspace_json)?;
        let domain_uuid = uuid::Uuid::parse_str(domain_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid domain ID: {}", e)))?;
        let node: crate::models::domain::CADSNode = serde_json::from_str(node_json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        model
            .add_cads_node_to_domain(domain_uuid, node)
            .map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&model)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Add an ODCS node to a domain in a DataModel.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `domain_id` - Domain UUID as string
    /// * `node_json` - JSON string containing ODCSNode
    ///
    /// # Returns
    ///
    /// JSON string containing updated DataModel, or JsValue error
    #[wasm_bindgen]
    pub fn add_odcs_node_to_domain(
        workspace_json: &str,
        domain_id: &str,
        node_json: &str,
    ) -> Result<String, JsValue> {
        let mut model = deserialize_workspace(workspace_json)?;
        let domain_uuid = uuid::Uuid::parse_str(domain_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid domain ID: {}", e)))?;
        let node: crate::models::domain::ODCSNode = serde_json::from_str(node_json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        model
            .add_odcs_node_to_domain(domain_uuid, node)
            .map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&model)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    // ============================================================================
    // Validation Functions
    // ============================================================================

    /// Validate a table name.
    ///
    /// # Arguments
    ///
    /// * `name` - Table name to validate
    ///
    /// # Returns
    ///
    /// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
    #[wasm_bindgen]
    pub fn validate_table_name(name: &str) -> Result<String, JsValue> {
        match crate::validation::input::validate_table_name(name) {
            Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
            Err(err) => {
                Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string())
            }
        }
    }

    /// Validate a column name.
    ///
    /// # Arguments
    ///
    /// * `name` - Column name to validate
    ///
    /// # Returns
    ///
    /// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
    #[wasm_bindgen]
    pub fn validate_column_name(name: &str) -> Result<String, JsValue> {
        match crate::validation::input::validate_column_name(name) {
            Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
            Err(err) => {
                Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string())
            }
        }
    }

    /// Validate a UUID string.
    ///
    /// # Arguments
    ///
    /// * `id` - UUID string to validate
    ///
    /// # Returns
    ///
    /// JSON string with validation result: `{"valid": true, "uuid": "..."}` or `{"valid": false, "error": "error message"}`
    #[wasm_bindgen]
    pub fn validate_uuid(id: &str) -> Result<String, JsValue> {
        match crate::validation::input::validate_uuid(id) {
            Ok(uuid) => {
                Ok(serde_json::json!({"valid": true, "uuid": uuid.to_string()}).to_string())
            }
            Err(err) => {
                Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string())
            }
        }
    }

    /// Validate a data type string.
    ///
    /// # Arguments
    ///
    /// * `data_type` - Data type string to validate
    ///
    /// # Returns
    ///
    /// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
    #[wasm_bindgen]
    pub fn validate_data_type(data_type: &str) -> Result<String, JsValue> {
        match crate::validation::input::validate_data_type(data_type) {
            Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
            Err(err) => {
                Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string())
            }
        }
    }

    /// Validate a description string.
    ///
    /// # Arguments
    ///
    /// * `desc` - Description string to validate
    ///
    /// # Returns
    ///
    /// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
    #[wasm_bindgen]
    pub fn validate_description(desc: &str) -> Result<String, JsValue> {
        match crate::validation::input::validate_description(desc) {
            Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
            Err(err) => {
                Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string())
            }
        }
    }

    /// Sanitize a SQL identifier by quoting it.
    ///
    /// # Arguments
    ///
    /// * `name` - SQL identifier to sanitize
    /// * `dialect` - SQL dialect ("postgresql", "mysql", "sqlserver", etc.)
    ///
    /// # Returns
    ///
    /// Sanitized SQL identifier string
    #[wasm_bindgen]
    pub fn sanitize_sql_identifier(name: &str, dialect: &str) -> String {
        crate::validation::input::sanitize_sql_identifier(name, dialect)
    }

    /// Sanitize a description string.
    ///
    /// # Arguments
    ///
    /// * `desc` - Description string to sanitize
    ///
    /// # Returns
    ///
    /// Sanitized description string
    #[wasm_bindgen]
    pub fn sanitize_description(desc: &str) -> String {
        crate::validation::input::sanitize_description(desc)
    }

    /// Detect naming conflicts between existing and new tables.
    ///
    /// # Arguments
    ///
    /// * `existing_tables_json` - JSON string containing array of existing tables
    /// * `new_tables_json` - JSON string containing array of new tables
    ///
    /// # Returns
    ///
    /// JSON string containing array of naming conflicts
    #[wasm_bindgen]
    pub fn detect_naming_conflicts(
        existing_tables_json: &str,
        new_tables_json: &str,
    ) -> Result<String, JsValue> {
        let existing_tables: Vec<crate::models::Table> = serde_json::from_str(existing_tables_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse existing tables: {}", e)))?;
        let new_tables: Vec<crate::models::Table> = serde_json::from_str(new_tables_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse new tables: {}", e)))?;

        let validator = crate::validation::tables::TableValidator::new();
        let conflicts = validator.detect_naming_conflicts(&existing_tables, &new_tables);

        serde_json::to_string(&conflicts)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Validate pattern exclusivity for a table (SCD pattern and Data Vault classification are mutually exclusive).
    ///
    /// # Arguments
    ///
    /// * `table_json` - JSON string containing table to validate
    ///
    /// # Returns
    ///
    /// JSON string with validation result: `{"valid": true}` or `{"valid": false, "violation": {...}}`
    #[wasm_bindgen]
    pub fn validate_pattern_exclusivity(table_json: &str) -> Result<String, JsValue> {
        let table: crate::models::Table = serde_json::from_str(table_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse table: {}", e)))?;

        let validator = crate::validation::tables::TableValidator::new();
        match validator.validate_pattern_exclusivity(&table) {
            Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
            Err(violation) => {
                Ok(serde_json::json!({"valid": false, "violation": violation}).to_string())
            }
        }
    }

    /// Check for circular dependencies in relationships.
    ///
    /// # Arguments
    ///
    /// * `relationships_json` - JSON string containing array of existing relationships
    /// * `source_table_id` - Source table ID (UUID string) of the new relationship
    /// * `target_table_id` - Target table ID (UUID string) of the new relationship
    ///
    /// # Returns
    ///
    /// JSON string with result: `{"has_cycle": true/false, "cycle_path": [...]}` or error
    #[wasm_bindgen]
    pub fn check_circular_dependency(
        relationships_json: &str,
        source_table_id: &str,
        target_table_id: &str,
    ) -> Result<String, JsValue> {
        let relationships: Vec<crate::models::Relationship> =
            serde_json::from_str(relationships_json)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse relationships: {}", e)))?;

        let source_id = uuid::Uuid::parse_str(source_table_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid source_table_id: {}", e)))?;
        let target_id = uuid::Uuid::parse_str(target_table_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid target_table_id: {}", e)))?;

        let validator = crate::validation::relationships::RelationshipValidator::new();
        match validator.check_circular_dependency(&relationships, source_id, target_id) {
            Ok((has_cycle, cycle_path)) => {
                let cycle_path_strs: Vec<String> = cycle_path
                    .map(|path| path.iter().map(|id| id.to_string()).collect())
                    .unwrap_or_default();
                Ok(serde_json::json!({
                    "has_cycle": has_cycle,
                    "cycle_path": cycle_path_strs
                })
                .to_string())
            }
            Err(err) => Err(JsValue::from_str(&format!("Validation error: {}", err))),
        }
    }

    /// Validate that source and target tables are different (no self-reference).
    ///
    /// # Arguments
    ///
    /// * `source_table_id` - Source table ID (UUID string)
    /// * `target_table_id` - Target table ID (UUID string)
    ///
    /// # Returns
    ///
    /// JSON string with validation result: `{"valid": true}` or `{"valid": false, "self_reference": {...}}`
    #[wasm_bindgen]
    pub fn validate_no_self_reference(
        source_table_id: &str,
        target_table_id: &str,
    ) -> Result<String, JsValue> {
        let source_id = uuid::Uuid::parse_str(source_table_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid source_table_id: {}", e)))?;
        let target_id = uuid::Uuid::parse_str(target_table_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid target_table_id: {}", e)))?;

        let validator = crate::validation::relationships::RelationshipValidator::new();
        match validator.validate_no_self_reference(source_id, target_id) {
            Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
            Err(self_ref) => {
                Ok(serde_json::json!({"valid": false, "self_reference": self_ref}).to_string())
            }
        }
    }

    // ============================================================================
    // PNG Export
    // ============================================================================

    /// Export a data model to PNG image format.
    ///
    /// # Arguments
    ///
    /// * `workspace_json` - JSON string containing workspace/data model structure
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Returns
    ///
    /// Base64-encoded PNG image string, or JsValue error
    #[cfg(feature = "png-export")]
    #[wasm_bindgen]
    pub fn export_to_png(workspace_json: &str, width: u32, height: u32) -> Result<String, JsValue> {
        let model = deserialize_workspace(workspace_json)?;
        let exporter = crate::export::PNGExporter::new();
        match exporter.export(&model.tables, width, height) {
            Ok(result) => Ok(result.content), // Already base64-encoded
            Err(err) => Err(export_error_to_js(err)),
        }
    }

    // ============================================================================
    // Model Loading/Saving (Async)
    // ============================================================================

    /// Load a model from browser storage (IndexedDB/localStorage).
    ///
    /// # Arguments
    ///
    /// * `db_name` - IndexedDB database name
    /// * `store_name` - Object store name
    /// * `workspace_path` - Workspace path to load from
    ///
    /// # Returns
    ///
    /// Promise that resolves to JSON string containing ModelLoadResult, or rejects with error
    #[wasm_bindgen]
    pub fn load_model(db_name: &str, store_name: &str, workspace_path: &str) -> js_sys::Promise {
        let db_name = db_name.to_string();
        let store_name = store_name.to_string();
        let workspace_path = workspace_path.to_string();

        wasm_bindgen_futures::future_to_promise(async move {
            let storage = crate::storage::browser::BrowserStorageBackend::new(db_name, store_name);
            let loader = crate::model::ModelLoader::new(storage);
            match loader.load_model(&workspace_path).await {
                Ok(result) => serde_json::to_string(&result)
                    .map(|s| JsValue::from_str(&s))
                    .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
                Err(err) => Err(JsValue::from_str(&format!("Storage error: {}", err))),
            }
        })
    }

    /// Save a model to browser storage (IndexedDB/localStorage).
    ///
    /// # Arguments
    ///
    /// * `db_name` - IndexedDB database name
    /// * `store_name` - Object store name
    /// * `workspace_path` - Workspace path to save to
    /// * `model_json` - JSON string containing DataModel to save
    ///
    /// # Returns
    ///
    /// Promise that resolves to success message, or rejects with error
    #[wasm_bindgen]
    pub fn save_model(
        db_name: &str,
        store_name: &str,
        workspace_path: &str,
        model_json: &str,
    ) -> js_sys::Promise {
        let db_name = db_name.to_string();
        let store_name = store_name.to_string();
        let workspace_path = workspace_path.to_string();
        let model_json = model_json.to_string();

        wasm_bindgen_futures::future_to_promise(async move {
            let model: crate::models::DataModel = serde_json::from_str(&model_json)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse model: {}", e)))?;

            let storage = crate::storage::browser::BrowserStorageBackend::new(db_name, store_name);
            let saver = crate::model::ModelSaver::new(storage);

            // Convert DataModel to table/relationship data for saving
            // For each table, save as YAML
            for table in &model.tables {
                // Export table to ODCS YAML
                let yaml = crate::export::ODCSExporter::export_table(table, "odcs_v3_1_0");
                let table_data = crate::model::saver::TableData {
                    id: table.id,
                    name: table.name.clone(),
                    yaml_file_path: Some(format!("tables/{}.yaml", table.name)),
                    yaml_value: serde_yaml::from_str(&yaml)
                        .map_err(|e| JsValue::from_str(&format!("Failed to parse YAML: {}", e)))?,
                };
                saver
                    .save_table(&workspace_path, &table_data)
                    .await
                    .map_err(|e| JsValue::from_str(&format!("Failed to save table: {}", e)))?;
            }

            // Save relationships
            if !model.relationships.is_empty() {
                let rel_data: Vec<crate::model::saver::RelationshipData> = model
                    .relationships
                    .iter()
                    .map(|rel| {
                        let yaml_value = serde_json::json!({
                            "id": rel.id.to_string(),
                            "source_table_id": rel.source_table_id.to_string(),
                            "target_table_id": rel.target_table_id.to_string(),
                        });
                        // Convert JSON value to YAML value
                        let yaml_str = serde_json::to_string(&yaml_value)
                            .map_err(|e| format!("Failed to serialize relationship: {}", e))?;
                        let yaml_value = serde_yaml::from_str(&yaml_str)
                            .map_err(|e| format!("Failed to convert to YAML: {}", e))?;
                        Ok(crate::model::saver::RelationshipData {
                            id: rel.id,
                            source_table_id: rel.source_table_id,
                            target_table_id: rel.target_table_id,
                            yaml_value,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()
                    .map_err(|e| JsValue::from_str(&e))?;

                saver
                    .save_relationships(&workspace_path, &rel_data)
                    .await
                    .map_err(|e| {
                        JsValue::from_str(&format!("Failed to save relationships: {}", e))
                    })?;
            }

            Ok(JsValue::from_str("Model saved successfully"))
        })
    }

    // BPMN WASM Bindings
    /// Import a BPMN model from XML content.
    ///
    /// # Arguments
    ///
    /// * `domain_id` - Domain UUID as string
    /// * `xml_content` - BPMN XML content as a string
    /// * `model_name` - Optional model name (extracted from XML if not provided)
    ///
    /// # Returns
    ///
    /// JSON string containing BPMNModel, or JsValue error
    #[cfg(feature = "bpmn")]
    #[wasm_bindgen]
    pub fn import_bpmn_model(
        domain_id: &str,
        xml_content: &str,
        model_name: Option<String>,
    ) -> Result<String, JsValue> {
        use crate::import::bpmn::BPMNImporter;
        use uuid::Uuid;

        let domain_uuid = Uuid::parse_str(domain_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid domain ID: {}", e)))?;

        let mut importer = BPMNImporter::new();
        match importer.import(xml_content, domain_uuid, model_name.as_deref()) {
            Ok(model) => serde_json::to_string(&model)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("Import error: {}", e))),
        }
    }

    /// Export a BPMN model to XML content.
    ///
    /// # Arguments
    ///
    /// * `xml_content` - BPMN XML content as a string
    ///
    /// # Returns
    ///
    /// BPMN XML content as string, or JsValue error
    #[cfg(feature = "bpmn")]
    #[wasm_bindgen]
    pub fn export_bpmn_model(xml_content: &str) -> Result<String, JsValue> {
        use crate::export::bpmn::BPMNExporter;
        let exporter = BPMNExporter::new();
        exporter
            .export(xml_content)
            .map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))
    }

    // DMN WASM Bindings
    /// Import a DMN model from XML content.
    ///
    /// # Arguments
    ///
    /// * `domain_id` - Domain UUID as string
    /// * `xml_content` - DMN XML content as a string
    /// * `model_name` - Optional model name (extracted from XML if not provided)
    ///
    /// # Returns
    ///
    /// JSON string containing DMNModel, or JsValue error
    #[cfg(feature = "dmn")]
    #[wasm_bindgen]
    pub fn import_dmn_model(
        domain_id: &str,
        xml_content: &str,
        model_name: Option<String>,
    ) -> Result<String, JsValue> {
        use crate::import::dmn::DMNImporter;
        use uuid::Uuid;

        let domain_uuid = Uuid::parse_str(domain_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid domain ID: {}", e)))?;

        let mut importer = DMNImporter::new();
        match importer.import(xml_content, domain_uuid, model_name.as_deref()) {
            Ok(model) => serde_json::to_string(&model)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("Import error: {}", e))),
        }
    }

    /// Export a DMN model to XML content.
    ///
    /// # Arguments
    ///
    /// * `xml_content` - DMN XML content as a string
    ///
    /// # Returns
    ///
    /// DMN XML content as string, or JsValue error
    #[cfg(feature = "dmn")]
    #[wasm_bindgen]
    pub fn export_dmn_model(xml_content: &str) -> Result<String, JsValue> {
        use crate::export::dmn::DMNExporter;
        let exporter = DMNExporter::new();
        exporter
            .export(xml_content)
            .map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))
    }

    // OpenAPI WASM Bindings
    /// Import an OpenAPI specification from YAML or JSON content.
    ///
    /// # Arguments
    ///
    /// * `domain_id` - Domain UUID as string
    /// * `content` - OpenAPI YAML or JSON content as a string
    /// * `api_name` - Optional API name (extracted from info.title if not provided)
    ///
    /// # Returns
    ///
    /// JSON string containing OpenAPIModel, or JsValue error
    #[cfg(feature = "openapi")]
    #[wasm_bindgen]
    pub fn import_openapi_spec(
        domain_id: &str,
        content: &str,
        api_name: Option<String>,
    ) -> Result<String, JsValue> {
        use crate::import::openapi::OpenAPIImporter;
        use uuid::Uuid;

        let domain_uuid = Uuid::parse_str(domain_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid domain ID: {}", e)))?;

        let mut importer = OpenAPIImporter::new();
        match importer.import(content, domain_uuid, api_name.as_deref()) {
            Ok(model) => serde_json::to_string(&model)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("Import error: {}", e))),
        }
    }

    /// Export an OpenAPI specification to YAML or JSON content.
    ///
    /// # Arguments
    ///
    /// * `content` - OpenAPI content as a string
    /// * `source_format` - Source format ("yaml" or "json")
    /// * `target_format` - Optional target format for conversion (None to keep original)
    ///
    /// # Returns
    ///
    /// OpenAPI content in requested format, or JsValue error
    #[cfg(feature = "openapi")]
    #[wasm_bindgen]
    pub fn export_openapi_spec(
        content: &str,
        source_format: &str,
        target_format: Option<String>,
    ) -> Result<String, JsValue> {
        use crate::export::openapi::OpenAPIExporter;
        use crate::models::openapi::OpenAPIFormat;

        let source_fmt = match source_format {
            "yaml" | "yml" => OpenAPIFormat::Yaml,
            "json" => OpenAPIFormat::Json,
            _ => {
                return Err(JsValue::from_str(
                    "Invalid source format. Use 'yaml' or 'json'",
                ));
            }
        };

        let target_fmt = if let Some(tf) = target_format {
            match tf.as_str() {
                "yaml" | "yml" => Some(OpenAPIFormat::Yaml),
                "json" => Some(OpenAPIFormat::Json),
                _ => {
                    return Err(JsValue::from_str(
                        "Invalid target format. Use 'yaml' or 'json'",
                    ));
                }
            }
        } else {
            None
        };

        let exporter = OpenAPIExporter::new();
        exporter
            .export(content, source_fmt, target_fmt)
            .map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))
    }

    /// Convert an OpenAPI schema component to an ODCS table.
    ///
    /// # Arguments
    ///
    /// * `openapi_content` - OpenAPI YAML or JSON content as a string
    /// * `component_name` - Name of the schema component to convert
    /// * `table_name` - Optional desired ODCS table name (uses component_name if None)
    ///
    /// # Returns
    ///
    /// JSON string containing ODCS Table, or JsValue error
    #[cfg(feature = "openapi")]
    #[wasm_bindgen]
    pub fn convert_openapi_to_odcs(
        openapi_content: &str,
        component_name: &str,
        table_name: Option<String>,
    ) -> Result<String, JsValue> {
        use crate::convert::openapi_to_odcs::OpenAPIToODCSConverter;

        let converter = OpenAPIToODCSConverter::new();
        match converter.convert_component(openapi_content, component_name, table_name.as_deref()) {
            Ok(table) => serde_json::to_string(&table)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("Conversion error: {}", e))),
        }
    }

    /// Analyze an OpenAPI component for conversion feasibility.
    ///
    /// # Arguments
    ///
    /// * `openapi_content` - OpenAPI YAML or JSON content as a string
    /// * `component_name` - Name of the schema component to analyze
    ///
    /// # Returns
    ///
    /// JSON string containing ConversionReport, or JsValue error
    #[cfg(feature = "openapi")]
    #[wasm_bindgen]
    pub fn analyze_openapi_conversion(
        openapi_content: &str,
        component_name: &str,
    ) -> Result<String, JsValue> {
        use crate::convert::openapi_to_odcs::OpenAPIToODCSConverter;

        let converter = OpenAPIToODCSConverter::new();
        match converter.analyze_conversion(openapi_content, component_name) {
            Ok(report) => serde_json::to_string(&report)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => Err(JsValue::from_str(&format!("Analysis error: {}", e))),
        }
    }
}
