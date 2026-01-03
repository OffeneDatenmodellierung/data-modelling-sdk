# OpenAPI API Contract

**Feature**: 004-bpmn-dmn-openapi
**Date**: 2026-01-03

## Overview

API contract for OpenAPI 3.1.1 specification import, export, and management operations.

## Importer API

### OpenAPIImporter

```rust
pub struct OpenAPIImporter {
    // No configuration needed - uses default JSON Schema from schemas/
}

impl OpenAPIImporter {
    /// Create a new OpenAPI importer
    pub fn new() -> Self {
        Self {}
    }

    /// Import an OpenAPI specification from YAML or JSON content
    ///
    /// # Arguments
    /// * `content` - OpenAPI 3.1.1 YAML or JSON content as string
    /// * `format` - Format hint (`Yaml` or `Json`, auto-detected if None)
    /// * `api_name` - Optional API name (if None, extracted from `info.title`)
    ///
    /// # Returns
    /// * `Ok(OpenAPIModel)` - Successfully imported model
    /// * `Err(ImportError)` - Import failed (validation, parsing, etc.)
    pub fn import(
        &self,
        content: &str,
        format: Option<OpenAPIFormat>,
        api_name: Option<&str>,
    ) -> Result<OpenAPIModel, ImportError>;

    /// Validate OpenAPI content against JSON Schema
    ///
    /// # Arguments
    /// * `content` - OpenAPI 3.1.1 YAML or JSON content
    /// * `format` - Format hint (auto-detected if None)
    ///
    /// # Returns
    /// * `Ok(())` - Valid OpenAPI specification
    /// * `Err(ImportError::ValidationFailed)` - Validation errors with details
    pub fn validate(&self, content: &str, format: Option<OpenAPIFormat>) -> Result<(), ImportError>;

    /// Extract metadata from OpenAPI specification
    ///
    /// # Arguments
    /// * `content` - OpenAPI 3.1.1 YAML or JSON content
    /// * `format` - Format hint (auto-detected if None)
    ///
    /// # Returns
    /// * `HashMap<String, serde_json::Value>` - Extracted metadata (version, description, etc.)
    pub fn extract_metadata(
        &self,
        content: &str,
        format: Option<OpenAPIFormat>,
    ) -> Result<HashMap<String, serde_json::Value>, ImportError>;

    /// Auto-detect format (YAML or JSON)
    ///
    /// # Arguments
    /// * `content` - OpenAPI content
    ///
    /// # Returns
    /// * `OpenAPIFormat` - Detected format
    pub fn detect_format(&self, content: &str) -> OpenAPIFormat;
}
```

## Exporter API

### OpenAPIExporter

```rust
pub struct OpenAPIExporter {
    // No configuration needed
}

impl OpenAPIExporter {
    /// Create a new OpenAPI exporter
    pub fn new() -> Self {
        Self {}
    }

    /// Export an OpenAPI model to YAML or JSON
    ///
    /// # Arguments
    /// * `model` - OpenAPIModel to export
    /// * `storage` - StorageBackend to read file from
    /// * `format` - Desired output format (preserves original if None)
    ///
    /// # Returns
    /// * `Ok(String)` - OpenAPI YAML or JSON content
    /// * `Err(ExportError)` - Export failed (file not found, I/O error, etc.)
    pub async fn export(
        &self,
        model: &OpenAPIModel,
        storage: &dyn StorageBackend,
        format: Option<OpenAPIFormat>,
    ) -> Result<String, ExportError>;
}
```

## Model Management API

### ModelSaver (Extended)

```rust
impl ModelSaver {
    /// Save an OpenAPI model to domain directory
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `model` - OpenAPIModel to save
    /// * `content` - OpenAPI YAML or JSON content
    ///
    /// # Returns
    /// * `Ok(())` - Successfully saved
    /// * `Err(StorageError)` - Save failed
    pub async fn save_openapi_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model: &OpenAPIModel,
        content: &str,
    ) -> Result<(), StorageError>;
}
```

### ModelLoader (Extended)

```rust
impl ModelLoader {
    /// Load all OpenAPI models from a domain
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    ///
    /// # Returns
    /// * `Ok(Vec<OpenAPIModel>)` - List of OpenAPI models
    /// * `Err(StorageError)` - Load failed
    pub async fn load_openapi_models(
        &self,
        workspace_path: &str,
        domain_name: &str,
    ) -> Result<Vec<OpenAPIModel>, StorageError>;

    /// Load a specific OpenAPI model by name
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `api_name` - API name
    ///
    /// # Returns
    /// * `Ok(OpenAPIModel)` - OpenAPI model
    /// * `Err(StorageError)` - Model not found or load failed
    pub async fn load_openapi_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        api_name: &str,
    ) -> Result<OpenAPIModel, StorageError>;

    /// Load OpenAPI content for a model
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `api_name` - API name
    ///
    /// # Returns
    /// * `Ok(String)` - OpenAPI YAML or JSON content
    /// * `Err(StorageError)` - File not found or read failed
    pub async fn load_openapi_content(
        &self,
        workspace_path: &str,
        domain_name: &str,
        api_name: &str,
    ) -> Result<String, StorageError>;
}
```

## WASM Bindings

```rust
#[wasm_bindgen]
pub fn import_openapi_spec(
    domain_id: &str,
    content: &str,
    format: Option<String>,  // "yaml" or "json"
    api_name: Option<String>,
) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn export_openapi_spec(
    domain_id: &str,
    api_name: &str,
    format: Option<String>,  // "yaml" or "json", None preserves original
) -> Result<String, JsValue>;

#[wasm_bindgen]
pub fn list_openapi_specs(domain_id: &str) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn delete_openapi_spec(domain_id: &str, api_name: &str) -> Result<(), JsValue>;
```

## Error Types

### ImportError (OpenAPI-specific)

- `InvalidFormat(String)` - Content is not valid YAML or JSON
- `ValidationFailed(String)` - JSON Schema validation failed (includes path to error)
- `FileTooLarge(u64)` - File exceeds 5MB limit
- `InvalidName(String)` - API name invalid
- `DuplicateName(String)` - API name already exists
- `UnsupportedVersion(String)` - OpenAPI version not 3.1.1

### ExportError (OpenAPI-specific)

- `ModelNotFound(Uuid)` - Model doesn't exist
- `IoError(String)` - File I/O error
- `SerializationError(String)` - YAML/JSON serialization failed
- `FormatConversionError(String)` - Failed to convert between YAML and JSON

## Usage Examples

### Import OpenAPI Specification

```rust
use data_modelling_sdk::import::openapi::{OpenAPIImporter, OpenAPIFormat};

let importer = OpenAPIImporter::new();
let yaml_content = std::fs::read_to_string("api.yaml")?;
let model = importer.import(&yaml_content, Some(OpenAPIFormat::Yaml), Some("orders-api"))?;

// Save to domain
let saver = ModelSaver::new(storage);
saver.save_openapi_model(workspace_path, "orders", &model, &yaml_content).await?;
```

### Export OpenAPI Specification

```rust
use data_modelling_sdk::export::openapi::OpenAPIExporter;

let exporter = OpenAPIExporter::new();
let loader = ModelLoader::new(storage);
let model = loader.load_openapi_model(workspace_path, "orders", "orders-api").await?;
let json_content = exporter.export(&model, storage, Some(OpenAPIFormat::Json)).await?;
```

### WASM Usage

```javascript
import { importOpenApiSpec, exportOpenApiSpec } from 'data-modelling-sdk';

// Import
const yamlContent = await fetch('api.yaml').then(r => r.text());
const result = importOpenApiSpec('domain-uuid', yamlContent, 'yaml', 'orders-api');
if (result.error) {
    console.error('Import failed:', result.error);
} else {
    console.log('API imported:', result.model);
}

// Export as JSON
const exported = exportOpenApiSpec('domain-uuid', 'orders-api', 'json');
if (exported.error) {
    console.error('Export failed:', exported.error);
} else {
    console.log('OpenAPI JSON:', exported.content);
}
```
