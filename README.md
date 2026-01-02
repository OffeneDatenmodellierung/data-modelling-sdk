# Data Modelling SDK

Shared SDK for model operations across platforms (API, WASM, Native).

Copyright (c) 2025 Mark Olliver - Licensed under MIT

## Features

- **Storage Backends**: File system, browser storage (IndexedDB/localStorage), and HTTP API
- **Model Loading/Saving**: Load and save models from various storage backends
- **Import/Export**: Import from SQL, ODCL, JSON Schema, AVRO, Protobuf; Export to various formats
- **Validation**: Table and relationship validation (naming conflicts, circular dependencies)

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
- `importFromDataflow(yamlContent: string): string` - Import Data Flow format YAML (lightweight format for nodes/relationships)
- `exportToSql(workspaceJson: string, dialect: string): string` - Export to SQL
- `exportToAvro(workspaceJson: string): string` - Export to AVRO schema
- `exportToJsonSchema(workspaceJson: string): string` - Export to JSON Schema
- `exportToProtobuf(workspaceJson: string): string` - Export to Protobuf
- `exportToDataflow(workspaceJson: string): string` - Export to Data Flow format YAML (lightweight format for nodes/relationships)

**Filtering**:
- `filterNodesByOwner(workspaceJson: string, owner: string): string` - Filter Data Flow nodes by owner
- `filterRelationshipsByOwner(workspaceJson: string, owner: string): string` - Filter Data Flow relationships by owner
- `filterNodesByInfrastructureType(workspaceJson: string, infrastructureType: string): string` - Filter Data Flow nodes by infrastructure type
- `filterRelationshipsByInfrastructureType(workspaceJson: string, infrastructureType: string): string` - Filter Data Flow relationships by infrastructure type
- `filterByTags(workspaceJson: string, tag: string): string` - Filter nodes and relationships by tag

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

## Status

The SDK structure is in place. The actual implementation of import/export/validation logic is being migrated incrementally from the parent crate. Currently, the SDK provides:

- ✅ Storage backend abstraction and implementations
- ✅ Model loader/saver structure
- ✅ Import/export module structure
- ✅ Validation module structure
- ⏳ Full implementation of parsers/exporters (in progress)
