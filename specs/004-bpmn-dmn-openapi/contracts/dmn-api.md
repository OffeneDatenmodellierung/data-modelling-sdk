# DMN API Contract

**Feature**: 004-bpmn-dmn-openapi
**Date**: 2026-01-03

## Overview

API contract for DMN 1.3 model import, export, and management operations.

## Importer API

### DMNImporter

```rust
pub struct DMNImporter {
    // No configuration needed - uses default XSD schema from schemas/
}

impl DMNImporter {
    /// Create a new DMN importer
    pub fn new() -> Self {
        Self {}
    }

    /// Import a DMN model from XML content
    ///
    /// # Arguments
    /// * `xml_content` - DMN 1.3 XML content as string
    /// * `model_name` - Optional model name (if None, extracted from XML)
    ///
    /// # Returns
    /// * `Ok(DMNModel)` - Successfully imported model
    /// * `Err(ImportError)` - Import failed (validation, parsing, etc.)
    pub fn import(&self, xml_content: &str, model_name: Option<&str>) -> Result<DMNModel, ImportError>;

    /// Validate DMN XML against XSD schema
    ///
    /// # Arguments
    /// * `xml_content` - DMN 1.3 XML content
    ///
    /// # Returns
    /// * `Ok(())` - Valid DMN XML
    /// * `Err(ImportError::ValidationFailed)` - Validation errors with details
    pub fn validate(&self, xml_content: &str) -> Result<(), ImportError>;

    /// Extract metadata from DMN XML
    ///
    /// # Arguments
    /// * `xml_content` - DMN 1.3 XML content
    ///
    /// # Returns
    /// * `HashMap<String, serde_json::Value>` - Extracted metadata (namespace, version, etc.)
    pub fn extract_metadata(&self, xml_content: &str) -> Result<HashMap<String, serde_json::Value>, ImportError>;
}
```

## Exporter API

### DMNExporter

```rust
pub struct DMNExporter {
    // No configuration needed
}

impl DMNExporter {
    /// Create a new DMN exporter
    pub fn new() -> Self {
        Self {}
    }

    /// Export a DMN model to XML
    ///
    /// # Arguments
    /// * `model` - DMNModel to export
    /// * `storage` - StorageBackend to read file from
    ///
    /// # Returns
    /// * `Ok(String)` - DMN XML content
    /// * `Err(ExportError)` - Export failed (file not found, I/O error, etc.)
    pub async fn export(&self, model: &DMNModel, storage: &dyn StorageBackend) -> Result<String, ExportError>;
}
```

## Model Management API

### ModelSaver (Extended)

```rust
impl ModelSaver {
    /// Save a DMN model to domain directory
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `model` - DMNModel to save
    /// * `xml_content` - DMN XML content
    ///
    /// # Returns
    /// * `Ok(())` - Successfully saved
    /// * `Err(StorageError)` - Save failed
    pub async fn save_dmn_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model: &DMNModel,
        xml_content: &str,
    ) -> Result<(), StorageError>;
}
```

### ModelLoader (Extended)

```rust
impl ModelLoader {
    /// Load all DMN models from a domain
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    ///
    /// # Returns
    /// * `Ok(Vec<DMNModel>)` - List of DMN models
    /// * `Err(StorageError)` - Load failed
    pub async fn load_dmn_models(
        &self,
        workspace_path: &str,
        domain_name: &str,
    ) -> Result<Vec<DMNModel>, StorageError>;

    /// Load a specific DMN model by name
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `model_name` - Model name
    ///
    /// # Returns
    /// * `Ok(DMNModel)` - DMN model
    /// * `Err(StorageError)` - Model not found or load failed
    pub async fn load_dmn_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model_name: &str,
    ) -> Result<DMNModel, StorageError>;

    /// Load DMN XML content for a model
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `model_name` - Model name
    ///
    /// # Returns
    /// * `Ok(String)` - DMN XML content
    /// * `Err(StorageError)` - File not found or read failed
    pub async fn load_dmn_xml(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model_name: &str,
    ) -> Result<String, StorageError>;
}
```

## WASM Bindings

```rust
#[wasm_bindgen]
pub fn import_dmn_model(domain_id: &str, xml_content: &str, model_name: Option<String>) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn export_dmn_model(domain_id: &str, model_name: &str) -> Result<String, JsValue>;

#[wasm_bindgen]
pub fn list_dmn_models(domain_id: &str) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn delete_dmn_model(domain_id: &str, model_name: &str) -> Result<(), JsValue>;
```

## Error Types

### ImportError (DMN-specific)

- `InvalidFormat(String)` - XML is not well-formed
- `ValidationFailed(String)` - XSD validation failed (includes line/column)
- `FileTooLarge(u64)` - File exceeds 10MB limit
- `InvalidName(String)` - Model name invalid
- `DuplicateName(String)` - Model name already exists

### ExportError (DMN-specific)

- `ModelNotFound(Uuid)` - Model doesn't exist
- `IoError(String)` - File I/O error
- `SerializationError(String)` - XML serialization failed

## Usage Examples

### Import DMN Model

```rust
use data_modelling_sdk::import::dmn::DMNImporter;

let importer = DMNImporter::new();
let xml_content = std::fs::read_to_string("decision.dmn")?;
let model = importer.import(&xml_content, Some("pricing-rules"))?;

// Save to domain
let saver = ModelSaver::new(storage);
saver.save_dmn_model(workspace_path, "orders", &model, &xml_content).await?;
```

### Export DMN Model

```rust
use data_modelling_sdk::export::dmn::DMNExporter;

let exporter = DMNExporter::new();
let loader = ModelLoader::new(storage);
let model = loader.load_dmn_model(workspace_path, "orders", "pricing-rules").await?;
let xml_content = exporter.export(&model, storage).await?;
```

### WASM Usage

```javascript
import { importDmnModel, exportDmnModel } from 'data-modelling-sdk';

// Import
const xmlContent = await fetch('decision.dmn').then(r => r.text());
const result = importDmnModel('domain-uuid', xmlContent, 'pricing-rules');
if (result.error) {
    console.error('Import failed:', result.error);
} else {
    console.log('Model imported:', result.model);
}

// Export
const exported = exportDmnModel('domain-uuid', 'pricing-rules');
if (exported.error) {
    console.error('Export failed:', exported.error);
} else {
    console.log('DMN XML:', exported.xml);
}
```
