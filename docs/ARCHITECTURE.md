# Architecture Guide

## Table of Contents

1. [What is the Data Modelling SDK?](#what-is-the-data-modelling-sdk)
2. [Project Decisions](#project-decisions)
3. [When to Use the SDK](#when-to-use-the-sdk)
4. [Architecture Overview](#architecture-overview)
5. [Design Principles](#design-principles)
6. [Component Architecture](#component-architecture)
7. [Storage Architecture](#storage-architecture)
8. [File Organization](#file-organization)
9. [Integration Patterns](#integration-patterns)
10. [Use Cases](#use-cases)

---

## What is the Data Modelling SDK?

The **Data Modelling SDK** is a Rust library that provides unified interfaces for data modeling operations across multiple platforms. It serves as the foundation for data governance, schema management, and data contract operations in modern data platforms.

### Core Purpose

The SDK enables:

- **Multi-format Support**: Import from and export to various data contract formats (ODCS, ODCL, SQL, JSON Schema, AVRO, Protobuf, CADS, ODPS, BPMN, DMN, OpenAPI)
- **Cross-platform Compatibility**: Works seamlessly in native applications, web applications (WASM), and API backends
- **Domain Organization**: Organize data contracts, compute assets, and data products within business domains
- **Validation & Governance**: Validate schemas, detect conflicts, and enforce data governance rules
- **Storage Abstraction**: Abstract storage operations across different environments (file system, browser storage, HTTP API)

### Key Characteristics

- **Language**: Rust (Edition 2024)
- **License**: MIT
- **Platform Support**: Native (Rust), Web (WASM), API (HTTP)
- **Primary Format**: ODCS v3.1.0 (Open Data Contract Standard)
- **Architecture**: Modular, trait-based, feature-gated

---

## Project Decisions

### 1. Rust as the Foundation

**Decision**: Build the SDK in Rust

**Rationale**:
- **Performance**: Rust provides near-native performance with memory safety
- **Cross-platform**: Single codebase compiles to native binaries and WASM
- **Type Safety**: Strong type system prevents common errors
- **Ecosystem**: Excellent serialization, async, and web support
- **WASM Support**: First-class WASM compilation enables web deployment

**Trade-offs**:
- Learning curve for teams unfamiliar with Rust
- Longer compile times compared to interpreted languages
- Mitigated by excellent tooling and documentation

### 2. Storage Backend Abstraction

**Decision**: Abstract storage operations behind a trait (`StorageBackend`)

**Rationale**:
- **Platform Independence**: Same code works on file system, browser storage, and HTTP API
- **Testability**: Easy to mock storage for testing
- **Flexibility**: Applications can choose storage backend based on environment
- **Future-proof**: Easy to add new storage backends (S3, Azure Blob, etc.)

**Implementation**:
```rust
#[async_trait(?Send)]
pub trait StorageBackend: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError>;
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError>;
    // ... more operations
}
```

### 3. ODCS as Primary Format

**Decision**: Use ODCS v3.1.0 as the primary internal format

**Rationale**:
- **Comprehensive**: ODCS provides the most complete metadata model
- **Standard**: Industry-standard format with broad adoption
- **Extensible**: Supports custom properties and extensions
- **Field Preservation**: Maintains all metadata during conversions

**Trade-offs**:
- More verbose than simpler formats
- Requires conversion layer for other formats
- Mitigated by universal converter and format-specific exporters

### 4. Domain-Based File Organization

**Decision**: Organize files by business domain

**Rationale**:
- **Logical Grouping**: Related assets (tables, products, compute) grouped together
- **Scalability**: Easy to manage large numbers of assets
- **Ownership**: Clear ownership boundaries per domain
- **Version Control**: Better Git history and collaboration

**Structure**:
```
domain1/
├── domain.yaml          # Domain definition
├── table1.odcs.yaml     # Data contracts
├── product1.odps.yaml   # Data products
└── model1.cads.yaml     # Compute assets
```

### 5. Feature-Gated Functionality

**Decision**: Gate optional functionality behind Cargo features

**Rationale**:
- **Minimal Dependencies**: Applications only include what they need
- **WASM Compatibility**: Some features (file system, Git) don't work in WASM
- **Build Performance**: Faster builds with fewer dependencies
- **Binary Size**: Smaller binaries for web deployment

**Features**:
- `default`: API backend (HTTP)
- `native-fs`: File system operations
- `wasm`: Browser storage (IndexedDB/localStorage)
- `git`: Git operations
- `png-export`: PNG diagram generation
- `databricks-dialect`: Databricks SQL support

### 6. UUID Strategy

**Decision**: Use UUIDv5 (deterministic) for model/table IDs

**Rationale**:
- **Deterministic**: Same inputs produce same ID (important for WASM without RNG)
- **Collision-resistant**: Very low probability of collisions
- **Reproducible**: Same model always gets same ID
- **No Random Number Generation**: Works in constrained environments

**Trade-offs**:
- Less privacy-friendly than random UUIDs
- Requires namespace and name for generation
- Acceptable for internal model IDs

### 7. Async/Await Architecture

**Decision**: Use async traits for storage operations

**Rationale**:
- **Non-blocking**: Better performance for I/O operations
- **WASM Compatibility**: Browser APIs are async
- **Consistency**: Same API across all platforms
- **Future-proof**: Aligns with Rust async ecosystem

**Trade-offs**:
- More complex than synchronous APIs
- Requires async runtime (Tokio for native, WASM runtime for web)
- Mitigated by excellent async/await syntax

### 8. Enhanced Tag Support

**Decision**: Support three tag formats (Simple, Pair, List)

**Rationale**:
- **Flexibility**: Supports various tagging strategies
- **Backward Compatible**: Simple tags work with existing systems
- **Rich Metadata**: Pair and List tags enable structured metadata
- **Auto-detection**: Automatically detects tag format during parsing

**Formats**:
- **Simple**: `"finance"` - Single word tags
- **Pair**: `"Environment:Dev"` - Key-value pairs
- **List**: `"SecondaryDomains: [XXXXX, PPPP]"` - Key with multiple values

---

## When to Use the SDK

### ✅ Use the SDK When:

1. **Building Data Governance Tools**
   - Data catalog applications
   - Schema registry systems
   - Data contract management platforms
   - Data lineage tools

2. **Cross-platform Applications**
   - Desktop applications (native)
   - Web applications (WASM)
   - Mobile applications (via API backend)
   - CLI tools

3. **Multi-format Support Required**
   - Need to import from multiple formats (SQL, JSON Schema, AVRO, etc.)
   - Need to export to multiple formats
   - Format conversion workflows

4. **Domain-Driven Data Organization**
   - Organizing data by business domains
   - Managing data products
   - Tracking compute assets (AI/ML models, applications)

5. **Validation & Quality Assurance**
   - Schema validation
   - Conflict detection
   - Circular dependency detection
   - Naming convention enforcement

6. **Storage Abstraction Needed**
   - Applications that need to work across different storage backends
   - Offline-first applications (browser storage)
   - Cloud-native applications (API backend)

### ❌ Don't Use the SDK When:

1. **Simple Single-format Use Cases**
   - If you only need to work with one format and don't need conversion
   - Consider format-specific libraries instead

2. **Non-Rust Applications**
   - The SDK is Rust-only
   - For other languages, consider the HTTP API backend or WASM bindings

3. **Real-time Streaming**
   - The SDK focuses on batch operations
   - Not designed for streaming data processing

4. **Direct Database Operations**
   - The SDK works with schema definitions, not live databases
   - Use database drivers for direct database access

---

## Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                          │
│  (Native App / Web App / API Server / CLI Tool)             │
└──────────────────────┬──────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    Data Modelling SDK                         │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Import     │  │   Export     │  │   Convert    │      │
│  │  (Formats)   │  │  (Formats)   │  │ (Universal)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Models     │  │  Validation  │  │   Domain    │      │
│  │  (Core)      │  │  (Rules)     │  │ (Org)       │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         Storage Backend Abstraction                   │   │
│  │  (Trait-based, platform-agnostic)                    │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────┬──────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ File System │  │   Browser   │  │  HTTP API   │
│  Backend    │  │   Backend   │  │   Backend   │
└─────────────┘  └─────────────┘  └─────────────┘
```

### Component Layers

1. **Application Layer**: Your application code
2. **SDK Public API**: High-level operations (import, export, validation)
3. **Core Models**: Data structures (Table, Column, Domain, etc.)
4. **Storage Abstraction**: Platform-independent storage operations
5. **Platform Implementations**: Specific storage backends

---

## Design Principles

### 1. Platform Independence

The SDK abstracts platform-specific operations behind traits, enabling the same code to run on:
- **Native**: File system operations via Tokio
- **Web**: Browser storage (IndexedDB/localStorage) via WASM
- **API**: HTTP operations via Reqwest

### 2. Format Agnosticism

The SDK supports multiple formats but maintains ODCS as the canonical internal format:
- **Import**: Convert any format → ODCS models
- **Export**: Convert ODCS models → any format
- **Universal Converter**: Direct format-to-format conversion

### 3. Domain-Driven Organization

Files and models are organized by business domain:
- **Domain**: Top-level container for related assets
- **Systems**: Physical infrastructure (Kafka, databases, etc.)
- **ODCS Nodes**: Data contracts (tables)
- **CADS Nodes**: Compute assets (AI/ML models, applications)
- **ODPS Products**: Data products linking multiple contracts

### 4. Backward Compatibility

The SDK maintains backward compatibility:
- Legacy `tables/` directory structure still supported
- ODCL v1.2.1 format supported (last version)
- Migration utilities for DataFlow → Domain

### 5. Extensibility

The SDK is designed for extension:
- **Custom Properties**: All models support custom metadata
- **Feature Flags**: Optional functionality gated behind features
- **Trait-based**: Easy to add new storage backends or exporters

### 6. Validation First

Validation is built into the core:
- **Input Validation**: Table/column names, UUIDs
- **Schema Validation**: Naming conflicts, circular dependencies
- **Format Validation**: JSON Schema validation against official schemas

---

## Component Architecture

### Core Components

#### 1. Models (`src/models/`)

Core data structures representing data contracts and domain organization:

- **`Table`**: Data contract with columns, metadata, relationships
- **`Column`**: Column definition with type, constraints, quality rules
- **`Relationship`**: Relationship between tables
- **`Domain`**: Business domain container
- **`System`**: Physical infrastructure entity
- **`CADSAsset`**: Compute asset (AI/ML model, application)
- **`ODPSDataProduct`**: Data product linking contracts
- **`DataModel`**: Container for tables, relationships, domains

#### 2. Import (`src/import/`)

Format-specific importers converting external formats to SDK models:

- **`ODCSImporter`**: ODCS v3.1.0 (primary format)
- **`ODCLImporter`**: ODCL v1.2.1 (legacy, via ODCSImporter)
- **`CADSImporter`**: CADS v1.0 (compute assets, supports BPMN/DMN/OpenAPI references)
- **`ODPSImporter`**: ODPS (data products)
- **`BPMNImporter`**: BPMN 2.0 XML (process models, requires `bpmn` feature)
- **`DMNImporter`**: DMN 1.3 XML (decision models, requires `dmn` feature)
- **`OpenAPIImporter`**: OpenAPI 3.1.1 YAML/JSON (API specs, requires `openapi` feature)
- **`SQLImporter`**: SQL DDL parsing
- **`JSONSchemaImporter`**: JSON Schema conversion
- **`AvroImporter`**: AVRO schema conversion
- **`ProtobufImporter`**: Protobuf .proto parsing

#### 3. Export (`src/export/`)

Format-specific exporters converting SDK models to external formats:

- **`ODCSExporter`**: ODCS v3.1.0 export
- **`ODCLExporter`**: ODCL v1.2.1 export (legacy)
- **`CADSExporter`**: CADS v1.0 export (supports BPMN/DMN/OpenAPI references)
- **`ODPSExporter`**: ODPS export
- **`BPMNExporter`**: BPMN 2.0 XML export (requires `bpmn` feature)
- **`DMNExporter`**: DMN 1.3 XML export (requires `dmn` feature)
- **`OpenAPIExporter`**: OpenAPI 3.1.1 YAML/JSON export with format conversion (requires `openapi` feature)
- **`SQLExporter`**: SQL DDL generation
- **`JSONSchemaExporter`**: JSON Schema generation
- **`AvroExporter`**: AVRO schema generation
- **`ProtobufExporter`**: Protobuf .proto generation

#### 4. Convert (`src/convert/`)

Universal format conversion:

- **`convert_to_odcs()`**: Convert any format → ODCS YAML
- **`auto_detect_format()`**: Detect input format automatically
- **`migrate_dataflow_to_domain()`**: Migrate legacy DataFlow → Domain

#### 5. Validation (`src/validation/`)

Validation logic:

- **`TableValidator`**: Table name validation, conflict detection
- **`RelationshipValidator`**: Circular dependency detection
- **`InputValidator`**: Input sanitization and validation

#### 6. Model Management (`src/model/`)

Loading and saving models:

- **`ModelLoader`**: Load models from storage
- **`ModelSaver`**: Save models to storage
- **`ApiModelLoader`**: Load via HTTP API

#### 7. Storage (`src/storage/`)

Storage backend abstraction:

- **`StorageBackend`**: Trait defining storage operations
- **`FileSystemStorageBackend`**: Native file system (feature-gated)
- **`BrowserStorageBackend`**: Browser storage (WASM, feature-gated)
- **`ApiStorageBackend`**: HTTP API (default)

---

## Storage Architecture

### Storage Backend Trait

All storage operations go through the `StorageBackend` trait:

```rust
#[async_trait(?Send)]
pub trait StorageBackend: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError>;
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError>;
    async fn list_files(&self, dir: &str) -> Result<Vec<String>, StorageError>;
    async fn file_exists(&self, path: &str) -> Result<bool, StorageError>;
    async fn delete_file(&self, path: &str) -> Result<(), StorageError>;
    async fn create_dir(&self, path: &str) -> Result<(), StorageError>;
    async fn dir_exists(&self, path: &str) -> Result<bool, StorageError>;
}
```

### Platform Implementations

#### File System Backend (`native-fs` feature)

- **Platform**: Native applications (desktop, CLI, server)
- **Storage**: Local file system
- **Runtime**: Tokio async runtime
- **Use Case**: Desktop applications, CLI tools, server applications

#### Browser Storage Backend (`wasm` feature)

- **Platform**: Web applications (WASM)
- **Storage**: IndexedDB or localStorage
- **Runtime**: WASM runtime (browser-provided)
- **Use Case**: Web applications, offline-first apps

#### API Backend (`api-backend` feature, default)

- **Platform**: Any (HTTP client)
- **Storage**: Remote HTTP API
- **Runtime**: Reqwest HTTP client
- **Use Case**: Cloud-native applications, mobile apps, microservices

---

## File Organization

### Domain-Based Structure

Files are organized by business domain:

```
workspace/
├── schemas/                    # Schema reference (JSON Schema files)
│   ├── odcs-json-schema-v3.1.0.json
│   ├── odcl-json-schema-1.2.1.json
│   ├── odps-json-schema-latest.json
│   └── cads.schema.json
├── customer-service/           # Domain directory
│   ├── domain.yaml            # Domain definition
│   ├── customers.odcs.yaml    # ODCS table
│   ├── orders.odcs.yaml       # ODCS table
│   ├── customer-product.odps.yaml  # ODPS product
│   └── recommendation-model.cads.yaml  # CADS asset
├── order-processing/           # Another domain
│   ├── domain.yaml
│   └── ...
└── tables/                     # Legacy: tables not in domains
    └── legacy-table.yaml
```

### File Naming Conventions

- **Domain files**: `domain.yaml`
- **ODCS tables**: `{table-name}.odcs.yaml`
- **ODPS products**: `{product-name}.odps.yaml`
- **CADS assets**: `{asset-name}.cads.yaml`

### Benefits

1. **Logical Grouping**: Related assets grouped together
2. **Scalability**: Easy to manage large numbers of assets
3. **Ownership**: Clear ownership per domain
4. **Version Control**: Better Git history
5. **Discovery**: Easy to find assets by domain

---

## Integration Patterns

### Pattern 1: Native Application

```rust
use data_modelling_sdk::storage::filesystem::FileSystemStorageBackend;
use data_modelling_sdk::model::{ModelLoader, ModelSaver};
use data_modelling_sdk::import::ODCSImporter;

// Initialize storage
let storage = FileSystemStorageBackend::new("/path/to/workspace");
let loader = ModelLoader::new(storage.clone());
let saver = ModelSaver::new(storage);

// Load domains
let result = loader.load_domains("workspace").await?;

// Import new table
let mut importer = ODCSImporter::new();
let (table, _) = importer.parse_table(odcs_yaml)?;

// Save to domain
saver.save_domain("workspace", &domain, &tables, &products, &assets).await?;
```

### Pattern 2: Web Application (WASM)

```rust
use data_modelling_sdk::storage::browser::BrowserStorageBackend;
use data_modelling_sdk::model::ModelLoader;

// Initialize browser storage
let storage = BrowserStorageBackend::new("db_name", "store_name");
let loader = ModelLoader::new(storage);

// Load domains
let result = loader.load_domains("workspace").await?;

// Use in JavaScript
#[wasm_bindgen]
pub fn load_domains() -> Promise {
    // WASM bindings handle async
}
```

### Pattern 3: API Server

```rust
use data_modelling_sdk::storage::api::ApiStorageBackend;
use data_modelling_sdk::model::ApiModelLoader;

// Initialize API backend
let storage = ApiStorageBackend::new("https://api.example.com", Some("session_id"));
let loader = ApiModelLoader::new(storage);

// Load via API
let result = loader.load_model("domain-name").await?;
```

### Pattern 4: Format Conversion

```rust
use data_modelling_sdk::convert::convert_to_odcs;

// Convert any format to ODCS
let odcs_yaml = convert_to_odcs(input_yaml, None)?;
```

---

## Use Cases

### Use Case 1: Data Catalog Application

**Scenario**: Build a data catalog that allows users to discover, document, and manage data assets.

**SDK Usage**:
- Import schemas from various sources (SQL, JSON Schema, AVRO)
- Organize by business domain
- Export to multiple formats for different consumers
- Validate schemas and detect conflicts

**Architecture**:
```
Web Frontend (WASM)
    ↓
Data Modelling SDK (Browser Storage)
    ↓
IndexedDB (Browser)
```

### Use Case 2: Schema Registry

**Scenario**: Centralized schema registry for microservices.

**SDK Usage**:
- Store schemas in domain-based structure
- Validate schemas against JSON Schema definitions
- Provide format conversion (AVRO → JSON Schema → ODCS)
- Track schema versions and relationships

**Architecture**:
```
Microservices
    ↓
HTTP API
    ↓
Data Modelling SDK (API Backend)
    ↓
File System / Object Storage
```

### Use Case 3: Data Governance Platform

**Scenario**: Platform for data governance, lineage, and quality.

**SDK Usage**:
- Import data contracts from various sources
- Organize by business domain
- Track compute assets (AI/ML models, ETL pipelines)
- Validate schemas and detect conflicts
- Export governance reports

**Architecture**:
```
Governance UI
    ↓
Data Modelling SDK (File System)
    ↓
Git Repository (Version Control)
```

### Use Case 4: CLI Tool

**Scenario**: Command-line tool for schema management.

**SDK Usage**:
- Import schemas from files
- Convert between formats
- Validate schemas
- Generate documentation

**Architecture**:
```
CLI Tool
    ↓
Data Modelling SDK (File System)
    ↓
Local File System
```

### Use Case 5: Data Product Management

**Scenario**: Manage data products linking multiple data contracts.

**SDK Usage**:
- Define ODPS data products
- Link to ODCS tables
- Organize by domain
- Track product versions and status

**Architecture**:
```
Product Management UI
    ↓
Data Modelling SDK (API Backend)
    ↓
Cloud Storage (S3, Azure Blob)
```

---

## Summary

The Data Modelling SDK provides a robust, cross-platform foundation for data modeling operations. Key strengths:

- **Multi-platform**: Works in native, web, and API environments
- **Multi-format**: Supports all major data contract formats
- **Domain-driven**: Organizes assets by business domain
- **Validation**: Built-in validation and conflict detection
- **Extensible**: Easy to extend with new formats or backends

Use the SDK when building data governance tools, schema registries, data catalogs, or any application that needs to work with data contracts across multiple formats and platforms.

For more information:
- [Schema Overview Guide](SCHEMA_OVERVIEW.md) - Detailed schema documentation
- [README](../README.md) - Quick start and usage examples
- [LLM.txt](../LLM.txt) - Technical reference
