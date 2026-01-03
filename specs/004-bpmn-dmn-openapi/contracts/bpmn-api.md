# BPMN API Contract

**Feature**: 004-bpmn-dmn-openapi
**Date**: 2026-01-03

## Overview

API contract for BPMN 2.0 model import, export, and management operations.

## Importer API

### BPMNImporter

```rust
pub struct BPMNImporter {
    // No configuration needed - uses default XSD schema from schemas/
}

impl BPMNImporter {
    /// Create a new BPMN importer
    pub fn new() -> Self {
        Self {}
    }

    /// Import a BPMN model from XML content
    ///
    /// # Arguments
    /// * `xml_content` - BPMN 2.0 XML content as string
    /// * `model_name` - Optional model name (if None, extracted from XML)
    ///
    /// # Returns
    /// * `Ok(BPMNModel)` - Successfully imported model
    /// * `Err(ImportError)` - Import failed (validation, parsing, etc.)
    pub fn import(&self, xml_content: &str, model_name: Option<&str>) -> Result<BPMNModel, ImportError>;

    /// Validate BPMN XML against XSD schema
    ///
    /// # Arguments
    /// * `xml_content` - BPMN 2.0 XML content
    ///
    /// # Returns
    /// * `Ok(())` - Valid BPMN XML
    /// * `Err(ImportError::ValidationFailed)` - Validation errors with details
    pub fn validate(&self, xml_content: &str) -> Result<(), ImportError>;

    /// Extract metadata from BPMN XML
    ///
    /// # Arguments
    /// * `xml_content` - BPMN 2.0 XML content
    ///
    /// # Returns
    /// * `HashMap<String, serde_json::Value>` - Extracted metadata (namespace, version, etc.)
    pub fn extract_metadata(&self, xml_content: &str) -> Result<HashMap<String, serde_json::Value>, ImportError>;
}
```

## Exporter API

### BPMNExporter

```rust
pub struct BPMNExporter {
    // No configuration needed
}

impl BPMNExporter {
    /// Create a new BPMN exporter
    pub fn new() -> Self {
        Self {}
    }

    /// Export a BPMN model to XML
    ///
    /// # Arguments
    /// * `model` - BPMNModel to export
    /// * `storage` - StorageBackend to read file from
    ///
    /// # Returns
    /// * `Ok(String)` - BPMN XML content
    /// * `Err(ExportError)` - Export failed (file not found, I/O error, etc.)
    pub async fn export(&self, model: &BPMNModel, storage: &dyn StorageBackend) -> Result<String, ExportError>;
}
```

## Model Management API

### ModelSaver (Extended)

```rust
impl ModelSaver {
    /// Save a BPMN model to domain directory
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `model` - BPMNModel to save
    /// * `xml_content` - BPMN XML content
    ///
    /// # Returns
    /// * `Ok(())` - Successfully saved
    /// * `Err(StorageError)` - Save failed
    pub async fn save_bpmn_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model: &BPMNModel,
        xml_content: &str,
    ) -> Result<(), StorageError>;
}
```

### ModelLoader (Extended)

```rust
impl ModelLoader {
    /// Load all BPMN models from a domain
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    ///
    /// # Returns
    /// * `Ok(Vec<BPMNModel>)` - List of BPMN models
    /// * `Err(StorageError)` - Load failed
    pub async fn load_bpmn_models(
        &self,
        workspace_path: &str,
        domain_name: &str,
    ) -> Result<Vec<BPMNModel>, StorageError>;

    /// Load a specific BPMN model by name
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `model_name` - Model name
    ///
    /// # Returns
    /// * `Ok(BPMNModel)` - BPMN model
    /// * `Err(StorageError)` - Model not found or load failed
    pub async fn load_bpmn_model(
        &self,
        workspace_path: &str,
        domain_name: &str,
        model_name: &str,
    ) -> Result<BPMNModel, StorageError>;

    /// Load BPMN XML content for a model
    ///
    /// # Arguments
    /// * `workspace_path` - Base workspace path
    /// * `domain_name` - Domain name
    /// * `model_name` - Model name
    ///
    /// # Returns
    /// * `Ok(String)` - BPMN XML content
    /// * `Err(StorageError)` - File not found or read failed
    pub async fn load_bpmn_xml(
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
pub fn import_bpmn_model(domain_id: &str, xml_content: &str, model_name: Option<String>) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn export_bpmn_model(domain_id: &str, model_name: &str) -> Result<String, JsValue>;

#[wasm_bindgen]
pub fn list_bpmn_models(domain_id: &str) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn delete_bpmn_model(domain_id: &str, model_name: &str) -> Result<(), JsValue>;
```

## Error Types

### ImportError (BPMN-specific)

- `InvalidFormat(String)` - XML is not well-formed
- `ValidationFailed(String)` - XSD validation failed (includes line/column)
- `FileTooLarge(u64)` - File exceeds 10MB limit
- `InvalidName(String)` - Model name invalid
- `DuplicateName(String)` - Model name already exists

### ExportError (BPMN-specific)

- `ModelNotFound(Uuid)` - Model doesn't exist
- `IoError(String)` - File I/O error
- `SerializationError(String)` - XML serialization failed

## Usage Examples

### Import BPMN Model

```rust
use data_modelling_sdk::import::bpmn::BPMNImporter;

let importer = BPMNImporter::new();
let xml_content = std::fs::read_to_string("process.bpmn")?;
let model = importer.import(&xml_content, Some("order-process"))?;

// Save to domain
let saver = ModelSaver::new(storage);
saver.save_bpmn_model(workspace_path, "orders", &model, &xml_content).await?;
```

### Export BPMN Model

```rust
use data_modelling_sdk::export::bpmn::BPMNExporter;

let exporter = BPMNExporter::new();
let loader = ModelLoader::new(storage);
let model = loader.load_bpmn_model(workspace_path, "orders", "order-process").await?;
let xml_content = exporter.export(&model, storage).await?;
```

### WASM Usage

```javascript
import { importBpmnModel, exportBpmnModel } from 'data-modelling-sdk';

// Import
const xmlContent = await fetch('process.bpmn').then(r => r.text());
const result = importBpmnModel('domain-uuid', xmlContent, 'order-process');
if (result.error) {
    console.error('Import failed:', result.error);
} else {
    console.log('Model imported:', result.model);
}

// Export
const exported = exportBpmnModel('domain-uuid', 'order-process');
if (exported.error) {
    console.error('Export failed:', exported.error);
} else {
    console.log('BPMN XML:', exported.xml);
}
```
