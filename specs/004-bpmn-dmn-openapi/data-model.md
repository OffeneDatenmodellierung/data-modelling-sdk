# Data Model: BPMN, DMN, and OpenAPI Support

**Feature**: 004-bpmn-dmn-openapi
**Date**: 2026-01-03

## Overview

This document defines the data structures and relationships for BPMN, DMN, and OpenAPI model support. Models are stored in their native formats (XML for BPMN/DMN, YAML/JSON for OpenAPI), with minimal metadata structures for references and management.

## Core Entities

### BPMNModel

Represents a BPMN 2.0 process model stored in native XML format.

**Fields**:
- `id: Uuid` - Unique identifier for the model
- `domain_id: Uuid` - Domain this model belongs to
- `name: String` - Model name (extracted from XML or provided)
- `file_path: String` - Relative path within domain directory (e.g., `{domain_name}/{name}.bpmn.xml`)
- `file_size: u64` - File size in bytes
- `created_at: DateTime<Utc>` - Creation timestamp
- `updated_at: DateTime<Utc>` - Last update timestamp
- `metadata: HashMap<String, serde_json::Value>` - Extracted metadata (namespace, version, etc.)

**Validation Rules**:
- Name must be valid filename (alphanumeric, hyphens, underscores, max 255 chars)
- File path must be within domain directory (prevent path traversal)
- File must be valid BPMN 2.0 XML (validated against XSD)

**Relationships**:
- Belongs to one `Domain` (via `domain_id`)
- Referenced by zero or more `CADSAsset` instances (via `bpmn_references`)

### DMNModel

Represents a DMN 1.3 decision model stored in native XML format.

**Fields**:
- `id: Uuid` - Unique identifier for the model
- `domain_id: Uuid` - Domain this model belongs to
- `name: String` - Model name (extracted from XML or provided)
- `file_path: String` - Relative path within domain directory (e.g., `{domain_name}/{name}.dmn.xml`)
- `file_size: u64` - File size in bytes
- `created_at: DateTime<Utc>` - Creation timestamp
- `updated_at: DateTime<Utc>` - Last update timestamp
- `metadata: HashMap<String, serde_json::Value>` - Extracted metadata (namespace, version, etc.)

**Validation Rules**:
- Name must be valid filename (alphanumeric, hyphens, underscores, max 255 chars)
- File path must be within domain directory (prevent path traversal)
- File must be valid DMN 1.3 XML (validated against XSD)

**Relationships**:
- Belongs to one `Domain` (via `domain_id`)
- Referenced by zero or more `CADSAsset` instances (via `dmn_references`)

### OpenAPIModel

Represents an OpenAPI 3.1.1 specification stored in native YAML or JSON format.

**Fields**:
- `id: Uuid` - Unique identifier for the model
- `domain_id: Uuid` - Domain this model belongs to
- `name: String` - API name (extracted from `info.title` or provided)
- `file_path: String` - Relative path within domain directory (e.g., `{domain_name}/{name}.openapi.yaml`)
- `format: OpenAPIFormat` - Format enum (`Yaml` or `Json`)
- `file_size: u64` - File size in bytes
- `created_at: DateTime<Utc>` - Creation timestamp
- `updated_at: DateTime<Utc>` - Last update timestamp
- `metadata: HashMap<String, serde_json::Value>` - Extracted metadata (version, description, etc.)

**Validation Rules**:
- Name must be valid filename (alphanumeric, hyphens, underscores, max 255 chars)
- File path must be within domain directory (prevent path traversal)
- File must be valid OpenAPI 3.1.1 (validated against JSON Schema)
- Format must match file extension (`.yaml`/`.yml` for YAML, `.json` for JSON)

**Relationships**:
- Belongs to one `Domain` (via `domain_id`)
- Referenced by zero or more `CADSAsset` instances (via `openapi_references`)
- Can be converted to zero or more `ODCSTable` instances (via converter)

### ModelReference

Represents a reference from a CADS asset to a BPMN, DMN, or OpenAPI model.

**Fields**:
- `model_type: ModelType` - Type enum (`Bpmn`, `Dmn`, `OpenApi`)
- `domain_id: Option<Uuid>` - Target domain (None for same domain)
- `model_name: String` - Name of the referenced model
- `description: Option<String>` - Optional description of the reference

**Validation Rules**:
- Referenced model must exist (validated on CADS asset creation/update)
- Domain must exist if `domain_id` is Some
- Model name must match existing model in target domain

**Relationships**:
- Referenced by `CADSAsset` (via `bpmn_references`, `dmn_references`, `openapi_references`)

### OpenAPIToODCSConversion

Represents a conversion from an OpenAPI schema component to an ODCS table.

**Fields**:
- `source_component_name: String` - OpenAPI component name
- `target_table_name: String` - Generated ODCS table name
- `mapping_rules: Vec<TypeMappingRule>` - Applied type mappings
- `warnings: Vec<String>` - Conversion warnings (unsupported types, etc.)

**Validation Rules**:
- Source component must exist in OpenAPI spec
- Target table name must be valid ODCS table name
- All mapped fields must have valid ODCS types

**Relationships**:
- Created from `OpenAPIModel` (via converter)
- Results in `ODCSTable` (separate node, not stored in conversion)

## Enums

### ModelType

```rust
pub enum ModelType {
    Bpmn,
    Dmn,
    OpenApi,
}
```

### OpenAPIFormat

```rust
pub enum OpenAPIFormat {
    Yaml,
    Json,
}
```

### TypeMappingRule

Represents how an OpenAPI type was mapped to ODCS.

```rust
pub struct TypeMappingRule {
    pub openapi_type: String,      // e.g., "string", "integer", "object"
    pub openapi_format: Option<String>,  // e.g., "date-time", "email"
    pub odcs_type: String,         // e.g., "text", "long", "timestamp"
    pub quality_rules: Vec<QualityRule>,  // Preserved constraints
}
```

## Extended Entities

### CADSAsset (Extended)

Adds new optional fields for model references:

```rust
pub struct CADSAsset {
    // ... existing fields ...

    /// References to BPMN process models
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpmn_references: Option<Vec<ModelReference>>,

    /// References to DMN decision models
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dmn_references: Option<Vec<ModelReference>>,

    /// References to OpenAPI specifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openapi_references: Option<Vec<ModelReference>>,
}
```

## State Transitions

### Model Lifecycle

1. **Import**: File uploaded → Validated → Stored → Metadata extracted → Model created
2. **Reference**: CADS asset references model → Reference validated → Reference stored
3. **Export**: Model retrieved → File read → Returned to caller
4. **Delete**: Model deletion requested → References checked → Warnings if referenced → Model deleted

### Conversion Lifecycle

1. **Convert**: OpenAPI component selected → Type mapping applied → ODCS table created → Separate nodes maintained
2. **Update**: OpenAPI model updated → Conversion can be re-run → New ODCS table version created

## Validation Rules

### File Naming

- BPMN: `{sanitized_name}.bpmn.xml`
- DMN: `{sanitized_name}.dmn.xml`
- OpenAPI: `{sanitized_name}.openapi.yaml` or `.openapi.json`

Sanitization rules:
- Replace invalid characters with underscores
- Limit to 255 characters
- Ensure uniqueness within domain (append counter if needed)

### Path Validation

- All paths must be relative to domain directory
- No path traversal (`..` components rejected)
- Paths validated using existing `StorageBackend` path resolution

### Reference Validation

- References validated synchronously on CADS asset creation/update
- Cross-domain references checked via `StorageBackend::file_exists()`
- Broken references detected and reported (warnings, not errors, to allow forward references)

## Error Types

### ImportError

```rust
pub enum ImportError {
    InvalidFormat(String),           // File format invalid
    ValidationFailed(String),         // Schema validation failed
    FileTooLarge(u64),                // File exceeds size limit
    InvalidName(String),              // Model name invalid
    DuplicateName(String),            // Name conflict
    IoError(String),                  // File I/O error
}
```

### ExportError

```rust
pub enum ExportError {
    ModelNotFound(Uuid),              // Model doesn't exist
    IoError(String),                  // File I/O error
    SerializationError(String),        // Format serialization failed
}
```

### ConversionError

```rust
pub enum ConversionError {
    ComponentNotFound(String),         // OpenAPI component not found
    UnsupportedType(String),          // OpenAPI type not mappable
    InvalidMapping(String),            // Type mapping failed
    ValidationError(String),          // Generated ODCS invalid
}
```

## Storage Structure

```
workspace/
├── schemas/
│   ├── bpmn-2.0.xsd
│   ├── dmn-1.3.xsd
│   └── openapi-3.1.1.json
└── {domain_name}/
    ├── domain.yaml
    ├── {model_name}.bpmn.xml
    ├── {model_name}.dmn.xml
    ├── {api_name}.openapi.yaml
    └── {api_name}.openapi.json
```

## Relationships Diagram

```
Domain
  ├── BPMNModel (1:N)
  ├── DMNModel (1:N)
  └── OpenAPIModel (1:N)

CADSAsset
  ├── bpmn_references (N:M) → BPMNModel
  ├── dmn_references (N:M) → DMNModel
  └── openapi_references (N:M) → OpenAPIModel

OpenAPIModel
  └── converts_to (1:N) → ODCSTable (via converter, separate nodes)
```
