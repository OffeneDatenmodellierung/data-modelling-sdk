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

## Status

The SDK structure is in place. The actual implementation of import/export/validation logic is being migrated incrementally from the parent crate. Currently, the SDK provides:

- ✅ Storage backend abstraction and implementations
- ✅ Model loader/saver structure
- ✅ Import/export module structure
- ✅ Validation module structure
- ⏳ Full implementation of parsers/exporters (in progress)
