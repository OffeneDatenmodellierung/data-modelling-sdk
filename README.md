# Data Modelling SDK

Shared SDK for model operations across platforms (API, WASM, Native).

Copyright (c) 2025 Mark Olliver - Licensed under MIT

## Features

- **Storage Backends**: File system, browser storage (IndexedDB/localStorage), and HTTP API
- **Model Loading/Saving**: Load and save models from various storage backends
- **Import/Export**: Import from SQL, ODCS, ODCL, JSON Schema, AVRO, Protobuf, CADS, ODPS; Export to various formats
- **Business Domain Schema**: Organize systems, CADS nodes, and ODCS nodes within business domains
- **Universal Converter**: Convert any format to ODCS v3.1.0 format
- **Validation**: Table and relationship validation (naming conflicts, circular dependencies)
- **Schema Reference**: JSON Schema definitions for all supported formats in `schemas/` directory

## File Structure

The SDK organizes files using a domain-based directory structure:

```
base_directory/
├── .git/                     # Git folder (if present)
├── README.md                 # Repository files
├── domain1/                  # Domain directory
│   ├── domain.yaml          # Domain definition
│   ├── table1.odcs.yaml      # ODCS table files
│   ├── table2.odcs.yaml
│   ├── product1.odps.yaml   # ODPS product files
│   ├── model1.cads.yaml     # CADS asset files
│   └── ...                  # Future: OpenAPI/BPMN files
├── domain2/                  # Another domain directory
│   ├── domain.yaml
│   └── ...
└── tables/                    # Legacy: tables not in any domain (backward compatibility)
```

Each domain directory contains:
- `domain.yaml`: The domain definition with systems, CADS nodes, ODCS nodes, and connections
- `*.odcs.yaml`: ODCS table files referenced by ODCSNodes in the domain
- `*.odps.yaml`: ODPS product files for data products in the domain
- `*.cads.yaml`: CADS asset files referenced by CADSNodes in the domain

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
- `importFromSql(sqlContent: string, dialect: string): string` - Import from SQL
- `importFromAvro(avroContent: string): string` - Import from AVRO schema
- `importFromJsonSchema(jsonSchemaContent: string): string` - Import from JSON Schema
- `importFromProtobuf(protobufContent: string): string` - Import from Protobuf
- `importFromCads(yamlContent: string): string` - Import CADS (Compute Asset Description Specification) YAML
- `importFromOdps(yamlContent: string): string` - Import ODPS (Open Data Product Standard) YAML
- `exportToSql(workspaceJson: string, dialect: string): string` - Export to SQL
- `exportToAvro(workspaceJson: string): string` - Export to AVRO schema
- `exportToJsonSchema(workspaceJson: string): string` - Export to JSON Schema
- `exportToProtobuf(workspaceJson: string): string` - Export to Protobuf
- `exportToCads(workspaceJson: string): string` - Export to CADS YAML
- `exportToOdps(workspaceJson: string): string` - Export to ODPS YAML
- `convertToOdcs(input: string, format?: string): string` - Universal converter: convert any format to ODCS v3.1.0
- `migrateDataflowToDomain(dataflowYaml: string, domainName?: string): string` - Migrate DataFlow YAML to Domain schema format

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
- **Business Domain Schema**: Organize systems, CADS nodes, and ODCS nodes
- **Universal Converter**: Convert any format to ODCS v3.1.0

### Schema Reference Directory

The SDK maintains JSON Schema definitions for all supported formats in the `schemas/` directory:

- **ODCS v3.1.0**: `schemas/odcs-json-schema-v3.1.0.json` - Primary format for data contracts
- **ODCL v1.2.1**: `schemas/odcl-json-schema-1.2.1.json` - Legacy data contract format
- **ODPS**: `schemas/odps-json-schema-latest.json` - Data products linking to ODCS tables
- **CADS v1.0**: `schemas/cads.schema.json` - Compute assets (AI/ML models, applications, pipelines)

These schemas serve as authoritative references for validation, documentation, and compliance. See [schemas/README.md](schemas/README.md) for detailed information about each schema.

## Status

The SDK provides comprehensive support for multiple data modeling formats:

- ✅ Storage backend abstraction and implementations
- ✅ Model loader/saver structure
- ✅ Full import/export implementation for all supported formats
- ✅ Validation module structure
- ✅ Business Domain schema support
- ✅ Universal format converter
- ✅ Enhanced tag support (Simple, Pair, List)
- ✅ Full ODCS/ODCL field preservation
- ✅ Schema reference directory (`schemas/`) with JSON Schema definitions for all supported formats
