# Quickstart: BPMN, DMN, and OpenAPI Support

**Feature**: 004-bpmn-dmn-openapi
**Date**: 2026-01-03

## Overview

This guide provides quick examples for importing, exporting, and managing BPMN, DMN, and OpenAPI models in the Data Modelling SDK.

## Prerequisites

Enable the required features in `Cargo.toml`:

```toml
[features]
default = []
bpmn = ["quick-xml", "xsd"]
dmn = ["quick-xml", "xsd"]
openapi = []  # Uses existing jsonschema crate
```

Or enable all process models:

```toml
process-models = ["bpmn", "dmn", "openapi"]
```

## BPMN Models

### Import a BPMN Model

```rust
use data_modelling_sdk::import::bpmn::BPMNImporter;
use data_modelling_sdk::model::saver::ModelSaver;
use data_modelling_sdk::storage::filesystem::FileSystemStorageBackend;

// Create importer
let importer = BPMNImporter::new();

// Read BPMN XML file
let xml_content = std::fs::read_to_string("order-process.bpmn")?;

// Import and validate
let model = importer.import(&xml_content, Some("order-process"))?;

// Save to domain
let storage = FileSystemStorageBackend::new("/workspace");
let saver = ModelSaver::new(storage);
saver.save_bpmn_model("/workspace", "orders", &model, &xml_content).await?;
```

### Export a BPMN Model

```rust
use data_modelling_sdk::export::bpmn::BPMNExporter;
use data_modelling_sdk::model::loader::ModelLoader;

let storage = FileSystemStorageBackend::new("/workspace");
let loader = ModelLoader::new(storage);
let exporter = BPMNExporter::new();

// Load model
let model = loader.load_bpmn_model("/workspace", "orders", "order-process").await?;

// Export XML
let xml_content = exporter.export(&model, &storage).await?;
std::fs::write("exported-process.bpmn", xml_content)?;
```

### Reference BPMN from CADS Asset

```rust
use data_modelling_sdk::models::{CADSAsset, ModelReference, ModelType};
use uuid::Uuid;

let mut asset = CADSAsset {
    // ... existing fields ...
    bpmn_references: Some(vec![
        ModelReference {
            model_type: ModelType::Bpmn,
            domain_id: None,  // Same domain
            model_name: "order-process".to_string(),
            description: Some("Order processing workflow".to_string()),
        }
    ]),
    // ... rest of fields ...
};

// References are validated when saving the asset
```

## DMN Models

### Import a DMN Model

```rust
use data_modelling_sdk::import::dmn::DMNImporter;
use data_modelling_sdk::model::saver::ModelSaver;
use data_modelling_sdk::storage::filesystem::FileSystemStorageBackend;

let importer = DMNImporter::new();
let xml_content = std::fs::read_to_string("pricing-rules.dmn")?;
let model = importer.import(&xml_content, Some("pricing-rules"))?;

let storage = FileSystemStorageBackend::new("/workspace");
let saver = ModelSaver::new(storage);
saver.save_dmn_model("/workspace", "orders", &model, &xml_content).await?;
```

### Export a DMN Model

```rust
use data_modelling_sdk::export::dmn::DMNExporter;
use data_modelling_sdk::model::loader::ModelLoader;

let storage = FileSystemStorageBackend::new("/workspace");
let loader = ModelLoader::new(storage);
let exporter = DMNExporter::new();

let model = loader.load_dmn_model("/workspace", "orders", "pricing-rules").await?;
let xml_content = exporter.export(&model, &storage).await?;
```

## OpenAPI Specifications

### Import an OpenAPI Specification

```rust
use data_modelling_sdk::import::openapi::{OpenAPIImporter, OpenAPIFormat};
use data_modelling_sdk::model::saver::ModelSaver;
use data_modelling_sdk::storage::filesystem::FileSystemStorageBackend;

let importer = OpenAPIImporter::new();

// Import from YAML
let yaml_content = std::fs::read_to_string("api.yaml")?;
let model = importer.import(&yaml_content, Some(OpenAPIFormat::Yaml), Some("orders-api"))?;

let storage = FileSystemStorageBackend::new("/workspace");
let saver = ModelSaver::new(storage);
saver.save_openapi_model("/workspace", "orders", &model, &yaml_content).await?;
```

### Export an OpenAPI Specification

```rust
use data_modelling_sdk::export::openapi::OpenAPIExporter;
use data_modelling_sdk::model::loader::ModelLoader;

let storage = FileSystemStorageBackend::new("/workspace");
let loader = ModelLoader::new(storage);
let exporter = OpenAPIExporter::new();

let model = loader.load_openapi_model("/workspace", "orders", "orders-api").await?;

// Export in original format
let content = exporter.export(&model, &storage, None).await?;

// Or export as JSON
let json_content = exporter.export(&model, &storage, Some(OpenAPIFormat::Json)).await?;
```

## OpenAPI to ODCS Conversion

### Convert OpenAPI Component to ODCS Table

```rust
use data_modelling_sdk::convert::openapi_to_odcs::OpenAPIToODCSConverter;
use data_modelling_sdk::model::saver::ModelSaver;

let converter = OpenAPIToODCSConverter::new();
let openapi_yaml = std::fs::read_to_string("api.yaml")?;

// Convert "Order" component to ODCS table
let table = converter.convert_component(&openapi_yaml, "Order", Some("orders"))?;

// Save as separate ODCS node (OpenAPI model remains separate)
let storage = FileSystemStorageBackend::new("/workspace");
let saver = ModelSaver::new(storage);
saver.save_table("/workspace", "orders", &table).await?;
```

### Analyze Conversion Before Converting

```rust
use data_modelling_sdk::convert::openapi_to_odcs::OpenAPIToODCSConverter;

let converter = OpenAPIToODCSConverter::new();
let openapi_yaml = std::fs::read_to_string("api.yaml")?;

// Analyze conversion
let report = converter.analyze_conversion(&openapi_yaml, "Order")?;

println!("Table name: {}", report.table_name);
println!("Warnings: {:?}", report.warnings);
for mapping in &report.mappings {
    println!("  {}: {} → {}", mapping.field_name, mapping.openapi_type, mapping.odcs_type);
}

// Convert if acceptable
if report.warnings.is_empty() {
    let table = converter.convert_component(&openapi_yaml, "Order", None)?;
    // Save table...
}
```

## WASM Usage

### Import BPMN Model (JavaScript)

```javascript
import { importBpmnModel } from 'data-modelling-sdk';

const xmlContent = await fetch('order-process.bpmn').then(r => r.text());
const result = importBpmnModel('domain-uuid', xmlContent, 'order-process');

if (result.error) {
    console.error('Import failed:', result.error);
} else {
    console.log('BPMN model imported:', result.model);
    // Use with bpmn-js for visualization
    const viewer = new BpmnJS({ container: '#canvas' });
    await viewer.importXML(result.xml);
}
```

### Import OpenAPI and Convert to ODCS (JavaScript)

```javascript
import { importOpenApiSpec, convertOpenApiToOdcs } from 'data-modelling-sdk';

// Import OpenAPI
const yamlContent = await fetch('api.yaml').then(r => r.text());
const importResult = importOpenApiSpec('domain-uuid', yamlContent, 'yaml', 'orders-api');

if (importResult.error) {
    console.error('Import failed:', importResult.error);
    return;
}

// Convert component to ODCS
const conversionResult = convertOpenApiToOdcs(yamlContent, 'Order', 'orders');

if (conversionResult.error) {
    console.error('Conversion failed:', conversionResult.error);
} else {
    console.log('ODCS table created:', conversionResult.table);
    // Save table as separate node
}
```

## Error Handling

### Handling Import Errors

```rust
use data_modelling_sdk::import::bpmn::{BPMNImporter, ImportError};

let importer = BPMNImporter::new();
match importer.import(&xml_content, None) {
    Ok(model) => println!("Imported: {}", model.name),
    Err(ImportError::ValidationFailed(msg)) => {
        eprintln!("Validation failed: {}", msg);
        // Show validation errors to user
    }
    Err(ImportError::InvalidFormat(msg)) => {
        eprintln!("Invalid XML: {}", msg);
    }
    Err(ImportError::FileTooLarge(size)) => {
        eprintln!("File too large: {} bytes (max 10MB)", size);
    }
    Err(e) => eprintln!("Import error: {}", e),
}
```

### Handling Conversion Errors

```rust
use data_modelling_sdk::convert::openapi_to_odcs::{OpenAPIToODCSConverter, ConversionError};

let converter = OpenAPIToODCSConverter::new();
match converter.convert_component(&openapi_yaml, "Order", None) {
    Ok(table) => println!("Converted: {}", table.name),
    Err(ConversionError::ComponentNotFound(name)) => {
        eprintln!("Component '{}' not found in OpenAPI spec", name);
    }
    Err(ConversionError::UnsupportedType { field_name, openapi_type, reason }) => {
        eprintln!("Field '{}' has unsupported type '{}': {}", field_name, openapi_type, reason);
    }
    Err(e) => eprintln!("Conversion error: {}", e),
}
```

## Best Practices

1. **Validate Before Import**: Use `validate()` methods to check files before importing
2. **Handle Errors Gracefully**: Provide clear error messages to users
3. **Preserve Formats**: Don't convert YAML to JSON unnecessarily - preserve original format
4. **Reference Validation**: Always validate references when creating CADS assets
5. **Separate Nodes**: Remember that OpenAPI-to-ODCS conversion creates separate nodes
6. **File Naming**: Use descriptive, consistent naming for models
7. **Domain Organization**: Group related models in the same domain

## Next Steps

- See [data-model.md](./data-model.md) for detailed entity definitions
- See [contracts/](./contracts/) for complete API documentation
- See [spec.md](./spec.md) for full feature specification
