# Data Modelling SDK

Shared SDK for model operations across platforms (API, WASM, Native).

Copyright (c) 2025 Mark Olliver - Licensed under MIT

## CLI Tool

The SDK includes a command-line interface (CLI) for importing and exporting schemas. See [CLI.md](CLI.md) for detailed usage instructions.

**Quick Start:**
```bash
# Build the CLI (with OpenAPI and ODPS validation support)
cargo build --release --bin data-modelling-cli --features cli,openapi,odps-validation

# Run it
./target/release/data-modelling-cli --help
```

**Note:** The CLI now includes OpenAPI support by default in GitHub releases. For local builds, include the `openapi` feature to enable OpenAPI import/export. Include `odps-validation` to enable ODPS schema validation.

**ODPS Import/Export Examples:**
```bash
# Import ODPS YAML file
data-modelling-cli import odps product.odps.yaml

# Export ODCS to ODPS format
data-modelling-cli export odps input.odcs.yaml output.odps.yaml

# Test ODPS round-trip (requires odps-validation feature)
cargo run --bin test-odps --features odps-validation,cli -- product.odps.yaml --verbose
```

## Features

- **Storage Backends**: File system, browser storage (IndexedDB/localStorage), and HTTP API
- **Database Backends**: DuckDB (embedded) and PostgreSQL for high-performance queries
- **Model Loading/Saving**: Load and save models from various storage backends
- **Import/Export**: Import from SQL (PostgreSQL, MySQL, SQLite, Generic, Databricks), ODCS, ODCL, JSON Schema, AVRO, Protobuf (proto2/proto3), CADS, ODPS, BPMN, DMN, OpenAPI; Export to various formats
- **Decision Records (DDL)**: MADR-compliant Architecture Decision Records with full lifecycle management
- **Knowledge Base (KB)**: Domain-partitioned knowledge articles with Markdown content support
- **Sketches**: Excalidraw diagram storage with metadata, thumbnails, and cross-references
- **Business Domain Schema**: Organize systems, CADS nodes, and ODCS nodes within business domains
- **Universal Converter**: Convert any format to ODCS v3.1.0 format
- **OpenAPI to ODCS Converter**: Convert OpenAPI schema components to ODCS table definitions
- **Validation**: Table and relationship validation (naming conflicts, circular dependencies)
- **Relationship Modeling**: Crow's feet notation cardinality (zeroOrOne, exactlyOne, zeroOrMany, oneOrMany) and data flow directions
- **Schema Reference**: JSON Schema definitions for all supported formats in `schemas/` directory
- **Database Sync**: Bidirectional sync between YAML files and database with change detection
- **Git Hooks**: Automatic pre-commit and post-checkout hooks for database synchronization

## Decision Records (DDL)

The SDK includes full support for **Architecture Decision Records** following the MADR (Markdown Any Decision Records) format. Decisions are stored as YAML files and can be exported to Markdown for documentation.

### Decision File Structure

```
workspace/
├── decisions/
│   ├── index.yaml                              # Decision index with metadata
│   ├── 0001-use-postgresql-database.yaml       # Individual decision records
│   ├── 0002-adopt-microservices.yaml
│   └── ...
└── decisions-md/                               # Markdown exports (auto-generated)
    ├── 0001-use-postgresql-database.md
    └── 0002-adopt-microservices.md
```

### Decision Lifecycle

Decisions follow a defined lifecycle with these statuses:
- **Draft**: Initial proposal, open for discussion
- **Proposed**: Formal proposal awaiting decision
- **Accepted**: Approved and in effect
- **Deprecated**: No longer recommended but still valid
- **Superseded**: Replaced by a newer decision
- **Rejected**: Not approved

### Decision Categories

- **Architecture**: System design and structure decisions
- **Technology**: Technology stack and tool choices
- **Process**: Development workflow decisions
- **Security**: Security-related decisions
- **Data**: Data modeling and storage decisions
- **Integration**: External system integration decisions

### CLI Commands

```bash
# Create a new decision
data-modelling-cli decision new --title "Use PostgreSQL" --domain platform

# List all decisions
data-modelling-cli decision list --workspace ./my-workspace

# Show a specific decision
data-modelling-cli decision show 1 --workspace ./my-workspace

# Filter by status or category
data-modelling-cli decision list --status accepted --category architecture

# Export decisions to Markdown
data-modelling-cli decision export --workspace ./my-workspace
```

## Knowledge Base (KB)

The SDK provides a **Knowledge Base** system for storing domain knowledge, guides, and documentation as structured articles.

### Knowledge Base File Structure

```
workspace/
├── knowledge/
│   ├── index.yaml                              # Knowledge index with metadata
│   ├── 0001-api-authentication-guide.yaml      # Individual knowledge articles
│   ├── 0002-deployment-procedures.yaml
│   └── ...
└── knowledge-md/                               # Markdown exports (auto-generated)
    ├── 0001-api-authentication-guide.md
    └── 0002-deployment-procedures.md
```

### Article Types

- **Guide**: Step-by-step instructions and tutorials
- **Reference**: API documentation and technical references
- **Concept**: Explanations of concepts and principles
- **Tutorial**: Learning-focused content with examples
- **Troubleshooting**: Problem-solving guides
- **Runbook**: Operational procedures

### Article Status

- **Draft**: Work in progress
- **Review**: Ready for peer review
- **Published**: Approved and available
- **Archived**: No longer actively maintained
- **Deprecated**: Outdated, pending replacement

### CLI Commands

```bash
# Create a new knowledge article
data-modelling-cli knowledge new --title "API Authentication Guide" --domain platform --type guide

# List all articles
data-modelling-cli knowledge list --workspace ./my-workspace

# Show a specific article
data-modelling-cli knowledge show 1 --workspace ./my-workspace

# Filter by type, status, or domain
data-modelling-cli knowledge list --type guide --status published

# Search article content
data-modelling-cli knowledge search "authentication" --workspace ./my-workspace

# Export articles to Markdown
data-modelling-cli knowledge export --workspace ./my-workspace
```

## File Structure

The SDK organizes files using a flat file naming convention within a workspace:

```
workspace/
├── .git/                                        # Git folder (if present)
├── README.md                                    # Repository files
├── workspace.yaml                               # Workspace metadata with assets and relationships
├── myworkspace_sales_customers.odcs.yaml        # ODCS table: workspace_domain_resource.type.yaml
├── myworkspace_sales_orders.odcs.yaml           # Another ODCS table in sales domain
├── myworkspace_sales_crm_leads.odcs.yaml        # ODCS table with system: workspace_domain_system_resource.type.yaml
├── myworkspace_analytics_metrics.odps.yaml      # ODPS product file
├── myworkspace_platform_api.cads.yaml           # CADS asset file
├── myworkspace_platform_api.openapi.yaml        # OpenAPI specification file
├── myworkspace_ops_approval.bpmn.xml            # BPMN process model file
├── myworkspace_ops_routing.dmn.xml              # DMN decision model file
└── sketches/                                    # Excalidraw sketches
    ├── sketches.yaml                            # Sketch index
    ├── myworkspace_sales_sketch-0001.sketch.yaml # Individual sketch files
    └── thumbnails/                              # Sketch thumbnails
        └── sketch-0001.png
```

### File Naming Convention

Files follow the pattern: `{workspace}_{domain}_{system}_{resource}.{type}.{ext}`

- **workspace**: The workspace name (required)
- **domain**: The business domain (required)
- **system**: The system within the domain (optional)
- **resource**: The resource/asset name (required)
- **type**: The asset type (`odcs`, `odps`, `cads`, `openapi`, `bpmn`, `dmn`)
- **ext**: File extension (`yaml`, `xml`, `json`)

### Workspace-Level Files

- `workspace.yaml`: Workspace metadata including domains, systems, asset references, and relationships

### Asset Types

- `*.odcs.yaml`: ODCS table/schema definitions (Open Data Contract Standard)
- `*.odps.yaml`: ODPS data product definitions (Open Data Product Standard)
- `*.cads.yaml`: CADS asset definitions (architecture assets)
- `*.openapi.yaml` / `*.openapi.json`: OpenAPI specification files
- `*.bpmn.xml`: BPMN 2.0 process model files
- `*.dmn.xml`: DMN 1.3 decision model files
- `*.sketch.yaml`: Excalidraw sketch files with metadata

## Usage

### File System Backend (Native Apps)

```rust
use data_modelling_sdk::storage::filesystem::FileSystemStorageBackend;
use data_modelling_sdk::model::ModelLoader;

let storage = FileSystemStorageBackend::new("/path/to/workspace");
let loader = ModelLoader::new(storage);
let result = loader.load_model("workspace_path").await?;
```

### Browser Storage Backend (WASM Apps)

```rust
use data_modelling_sdk::storage::browser::BrowserStorageBackend;
use data_modelling_sdk::model::ModelLoader;

let storage = BrowserStorageBackend::new("db_name", "store_name");
let loader = ModelLoader::new(storage);
let result = loader.load_model("workspace_path").await?;
```

### API Backend (Online Mode)

```rust
use data_modelling_sdk::storage::api::ApiStorageBackend;
use data_modelling_sdk::model::ModelLoader;

let storage = ApiStorageBackend::new("http://localhost:8081/api/v1", Some("session_id"));
let loader = ModelLoader::new(storage);
let result = loader.load_model("workspace_path").await?;
```

### WASM Bindings (Browser/Offline Mode)

The SDK exposes WASM bindings for parsing and export operations, enabling offline functionality in web applications.

**Build the WASM module**:
```bash
wasm-pack build --target web --out-dir pkg --features wasm
```

**Use in JavaScript/TypeScript**:
```javascript
import init, { parseOdcsYaml, exportToOdcsYaml } from './pkg/data_modelling_sdk.js';

// Initialize the module
await init();

// Parse ODCS YAML
const yaml = `apiVersion: v3.1.0
kind: DataContract
name: users
schema:
  fields:
    - name: id
      type: bigint`;

const resultJson = parseOdcsYaml(yaml);
const result = JSON.parse(resultJson);
console.log('Parsed tables:', result.tables);

// Export to ODCS YAML
const workspace = {
  tables: [{
    id: "550e8400-e29b-41d4-a716-446655440000",
    name: "users",
    columns: [{ name: "id", data_type: "bigint", nullable: false, primary_key: true }]
  }],
  relationships: []
};

const exportedYaml = exportToOdcsYaml(JSON.stringify(workspace));
console.log('Exported YAML:', exportedYaml);
```

**Available WASM Functions**:

**Import/Export**:
- `parseOdcsYaml(yamlContent: string): string` - Parse ODCS YAML to workspace structure
- `exportToOdcsYaml(workspaceJson: string): string` - Export workspace to ODCS YAML
- `importFromSql(sqlContent: string, dialect: string): string` - Import from SQL (supported dialects: "postgres"/"postgresql", "mysql", "sqlite", "generic", "databricks")
- `importFromAvro(avroContent: string): string` - Import from AVRO schema
- `importFromJsonSchema(jsonSchemaContent: string): string` - Import from JSON Schema
- `importFromProtobuf(protobufContent: string): string` - Import from Protobuf
- `importFromCads(yamlContent: string): string` - Import CADS (Compute Asset Description Specification) YAML
- `importFromOdps(yamlContent: string): string` - Import ODPS (Open Data Product Standard) YAML
- `exportToOdps(productJson: string): string` - Export ODPS data product to YAML format
- `validateOdps(yamlContent: string): void` - Validate ODPS YAML content against ODPS JSON Schema (requires `odps-validation` feature)
- `importBpmnModel(domainId: string, xmlContent: string, modelName?: string): string` - Import BPMN 2.0 XML model
- `importDmnModel(domainId: string, xmlContent: string, modelName?: string): string` - Import DMN 1.3 XML model
- `importOpenapiSpec(domainId: string, content: string, apiName?: string): string` - Import OpenAPI 3.1.1 specification
- `exportToSql(workspaceJson: string, dialect: string): string` - Export to SQL (supported dialects: "postgres"/"postgresql", "mysql", "sqlite", "generic", "databricks")
- `exportToAvro(workspaceJson: string): string` - Export to AVRO schema
- `exportToJsonSchema(workspaceJson: string): string` - Export to JSON Schema
- `exportToProtobuf(workspaceJson: string): string` - Export to Protobuf
- `exportToCads(workspaceJson: string): string` - Export to CADS YAML
- `exportToOdps(workspaceJson: string): string` - Export to ODPS YAML
- `exportBpmnModel(xmlContent: string): string` - Export BPMN model to XML
- `exportDmnModel(xmlContent: string): string` - Export DMN model to XML
- `exportOpenapiSpec(content: string, sourceFormat: string, targetFormat?: string): string` - Export OpenAPI spec with optional format conversion
- `convertToOdcs(input: string, format?: string): string` - Universal converter: convert any format to ODCS v3.1.0
- `convertOpenapiToOdcs(openapiContent: string, componentName: string, tableName?: string): string` - Convert OpenAPI schema component to ODCS table
- `analyzeOpenapiConversion(openapiContent: string, componentName: string): string` - Analyze OpenAPI component conversion feasibility
- `migrateDataflowToDomain(dataflowYaml: string, domainName?: string): string` - Migrate DataFlow YAML to Domain schema format

**Sketch Operations**:
- `parseSketchYaml(yamlContent: string): string` - Parse sketch YAML to JSON
- `parseSketchIndexYaml(yamlContent: string): string` - Parse sketch index YAML to JSON
- `exportSketchToYaml(sketchJson: string): string` - Export sketch to YAML format
- `exportSketchIndexToYaml(indexJson: string): string` - Export sketch index to YAML format
- `createSketch(number: number, title: string, sketchType: string, excalidrawData: string): string` - Create new sketch
- `createSketchIndex(): string` - Create empty sketch index
- `addSketchToIndex(indexJson: string, sketchJson: string, filename: string): string` - Add sketch to index
- `searchSketches(sketchesJson: string, query: string): string` - Search sketches by title, description, or tags

**Domain Operations**:
- `createDomain(name: string): string` - Create a new business domain
- `addSystemToDomain(workspaceJson: string, domainId: string, systemJson: string): string` - Add a system to a domain
- `addCadsNodeToDomain(workspaceJson: string, domainId: string, nodeJson: string): string` - Add a CADS node to a domain
- `addOdcsNodeToDomain(workspaceJson: string, domainId: string, nodeJson: string): string` - Add an ODCS node to a domain

**Filtering**:
- `filterNodesByOwner(workspaceJson: string, owner: string): string` - Filter tables by owner
- `filterRelationshipsByOwner(workspaceJson: string, owner: string): string` - Filter relationships by owner
- `filterNodesByInfrastructureType(workspaceJson: string, infrastructureType: string): string` - Filter tables by infrastructure type
- `filterRelationshipsByInfrastructureType(workspaceJson: string, infrastructureType: string): string` - Filter relationships by infrastructure type
- `filterByTags(workspaceJson: string, tag: string): string` - Filter nodes and relationships by tag (supports Simple, Pair, and List tag formats)

## Database Support

The SDK includes an optional database layer for high-performance queries on large workspaces (10-100x faster than file-based operations).

### Database Backends

- **DuckDB**: Embedded analytical database, ideal for CLI tools and local development
- **PostgreSQL**: Server-based database for team environments and shared access

### Quick Start

```bash
# Build CLI with database support
cargo build --release --bin data-modelling-cli --features cli-full

# Initialize database for a workspace
./target/release/data-modelling-cli db init --workspace ./my-workspace

# Sync YAML files to database
./target/release/data-modelling-cli db sync --workspace ./my-workspace

# Query the database
./target/release/data-modelling-cli query "SELECT name FROM tables" --workspace ./my-workspace
```

### Configuration

Database settings are stored in `.data-model.toml`:

```toml
[database]
backend = "duckdb"
path = ".data-model.duckdb"

[sync]
auto_sync = true

[git]
hooks_enabled = true
```

### Git Hooks Integration

When initializing a database in a Git repository, the CLI automatically installs:

- **Pre-commit hook**: Exports database changes to YAML before commit
- **Post-checkout hook**: Syncs YAML files to database after checkout

This ensures YAML files and database stay in sync across branches and collaborators.

See [CLI.md](docs/CLI.md) for detailed database command documentation.

## Development

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality. Install them with:

```bash
# Install pre-commit (if not already installed)
pip install pre-commit

# Install the git hooks
pre-commit install

# Run hooks manually on all files
pre-commit run --all-files
```

The hooks will automatically run on `git commit` and check:
- Rust formatting (`cargo fmt`)
- Rust linting (`cargo clippy`)
- Security audit (`cargo audit`)
- File formatting (trailing whitespace, end of file, etc.)
- YAML/TOML/JSON syntax

### CI/CD

GitHub Actions workflows automatically run on push and pull requests:
- **Lint**: Format check, clippy, and security audit
- **Test**: Unit and integration tests on Linux, macOS, and Windows
- **Build**: Release build verification
- **Publish**: Automatic publishing to crates.io on main branch (after all checks pass)

## Documentation

- **[Architecture Guide](docs/ARCHITECTURE.md)**: Comprehensive guide to project architecture, design decisions, and use cases
- **[Schema Overview Guide](docs/SCHEMA_OVERVIEW.md)**: Detailed documentation of all supported schemas

The SDK supports:
- **ODCS v3.1.0**: Primary format for data contracts (tables)
- **ODCL v1.2.1**: Legacy data contract format (backward compatibility)
- **ODPS**: Data products linking to ODCS Tables
- **CADS v1.0**: Compute assets (AI/ML models, applications, pipelines)
- **BPMN 2.0**: Business Process Model and Notation (process models stored in native XML)
- **DMN 1.3**: Decision Model and Notation (decision models stored in native XML)
- **OpenAPI 3.1.1**: API specifications (stored in native YAML or JSON)
- **Business Domain Schema**: Organize systems, CADS nodes, and ODCS nodes
- **Universal Converter**: Convert any format to ODCS v3.1.0
- **OpenAPI to ODCS Converter**: Convert OpenAPI schema components to ODCS table definitions

### Schema Reference Directory

The SDK maintains JSON Schema definitions for all supported formats in the `schemas/` directory:

- **ODCS v3.1.0**: `schemas/odcs-json-schema-v3.1.0.json` - Primary format for data contracts
- **ODCL v1.2.1**: `schemas/odcl-json-schema-1.2.1.json` - Legacy data contract format
- **ODPS**: `schemas/odps-json-schema-latest.json` - Data products linking to ODCS tables
- **CADS v1.0**: `schemas/cads.schema.json` - Compute assets (AI/ML models, applications, pipelines)

These schemas serve as authoritative references for validation, documentation, and compliance. See [schemas/README.md](schemas/README.md) for detailed information about each schema.

## Data Pipeline

The SDK includes a complete data pipeline for ingesting JSON data, inferring schemas, and mapping to target formats.

### Pipeline Features

- **JSON Ingestion**: Ingest JSON/JSONL files into a staging database with deduplication
- **S3 Ingestion**: Ingest directly from AWS S3 buckets with streaming downloads (feature: `s3`)
- **Databricks Volumes**: Ingest from Databricks Unity Catalog Volumes (feature: `databricks`)
- **Progress Reporting**: Real-time progress bars with throughput metrics
- **Schema Inference**: Automatically infer types, formats, and nullability from data
- **LLM Refinement**: Optionally enhance schemas using Ollama or local LLM models
- **Schema Mapping**: Map inferred schemas to target schemas with transformation generation
- **Checkpointing**: Resume pipelines from the last successful stage
- **Secure Credentials**: Credential wrapper types preventing accidental logging

### Quick Start

```bash
# Build with pipeline support
cargo build --release -p odm --features pipeline

# Initialize staging database
odm staging init staging.duckdb

# Run full pipeline
odm pipeline run \
  --database staging.duckdb \
  --source ./json-data \
  --output-dir ./output \
  --verbose

# Check pipeline status
odm pipeline status --database staging.duckdb
```

### Schema Mapping

Map source schemas to target schemas with fuzzy matching and transformation script generation:

```bash
# Map schemas with fuzzy matching
odm map source.json target.json --fuzzy --min-similarity 0.7

# Generate SQL transformation
odm map source.json target.json \
  --transform-format sql \
  --transform-output transform.sql

# Generate Python transformation
odm map source.json target.json \
  --transform-format python \
  --transform-output transform.py
```

See [CLI.md](docs/CLI.md) for detailed pipeline and mapping documentation.

## Status

The SDK provides comprehensive support for multiple data modeling formats:

- ✅ Storage backend abstraction and implementations
- ✅ Database backend abstraction (DuckDB, PostgreSQL)
- ✅ Model loader/saver structure
- ✅ Full import/export implementation for all supported formats
- ✅ Validation module structure
- ✅ Business Domain schema support
- ✅ Universal format converter
- ✅ Enhanced tag support (Simple, Pair, List)
- ✅ Full ODCS/ODCL field preservation
- ✅ Schema reference directory (`schemas/`) with JSON Schema definitions for all supported formats
- ✅ Bidirectional YAML ↔ Database sync with change detection
- ✅ Git hooks for automatic synchronization
- ✅ Decision Records (DDL) with MADR format support
- ✅ Knowledge Base (KB) with domain partitioning
- ✅ Data Pipeline with staging, inference, and mapping
- ✅ Schema Mapping with fuzzy matching and transformation generation
- ✅ LLM-enhanced schema refinement (Ollama and local models)
- ✅ S3 ingestion with AWS SDK for Rust
- ✅ Databricks Unity Catalog Volumes ingestion
- ✅ Real-time progress reporting with indicatif
- ✅ Secure credential handling with automatic redaction
- ✅ Stable YAML export key ordering (eliminates git diff noise)
- ✅ Excalidraw sketch support with thumbnails and cross-references
