# Next Steps for SDK Migration

## Immediate Actions Needed

### 1. Decide on Type Strategy

Choose one approach for handling model types:

- **Option A**: Copy models to SDK (independence, some duplication)
- **Option B**: Create shared `canvas-models` crate (best long-term)
- **Option C**: SDK depends on parent crate (quickest, creates coupling)

**Recommendation**: Option B (shared models crate) for long-term maintainability.

### 2. Complete Storage Backends

#### API Backend (`src/storage/api.rs`)
- Implement actual HTTP calls using `reqwest`
- Map API endpoints:
  - `GET /tables` → `load_model()`
  - `POST /tables` → `save_table()`
  - `GET /relationships` → load relationships
  - `POST /relationships` → save relationships
- Handle authentication headers (`x-session-id`)
- Map API errors to `StorageError`

#### Browser Backend (`src/storage/browser.rs`)
- Fix IndexedDB async handling (use proper Promise-based API)
- Implement proper error handling
- Add localStorage fallback for small files
- Handle database versioning/upgrades

### 3. Migrate One Parser (Proof of Concept)

Start with SQL parser as it's the most commonly used:

1. Copy `SQLParser` implementation from `rust/src/api/services/sql_parser.rs`
2. Update to use SDK types (or create conversion layer)
3. Update API route to use SDK importer
4. Test end-to-end

### 4. Migrate One Exporter (Proof of Concept)

Start with SQL exporter:

1. Copy `SQLExporter` implementation from `rust/src/export/sql.rs`
2. Update to use SDK types
3. Update API route to use SDK exporter
4. Test end-to-end

### 5. Integrate into WASM App

Add SDK usage to `data-modeller`:

```rust
// In data-modeller/src/app.rs
use data_modelling_sdk::storage::{StorageBackend, BrowserStorageBackend, ApiStorageBackend};
use data_modelling_sdk::model::ModelLoader;

// Add mode switching
enum StorageMode {
    Online(ApiStorageBackend),
    Offline(BrowserStorageBackend),
}

impl DataModeller {
    fn load_model(&mut self, mode: StorageMode) {
        let loader = match mode {
            StorageMode::Online(backend) => ModelLoader::new(backend),
            StorageMode::Offline(backend) => ModelLoader::new(backend),
        };
        // Use loader...
    }
}
```

### 6. Integrate into Native App

Add file picker and SDK usage:

```rust
// In data-modeller/src/app.rs (native build)
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;
use data_modelling_sdk::storage::FileSystemStorageBackend;

#[cfg(not(target_arch = "wasm32"))]
fn open_folder() -> Option<PathBuf> {
    FileDialog::new().pick_folder()
}
```

## Files to Update

### High Priority
- `rust/data-modelling-sdk/src/storage/api.rs` - Complete API backend
- `rust/data-modelling-sdk/src/storage/browser.rs` - Fix async handling
- `rust/data-modelling-sdk/src/import/sql.rs` - Migrate SQL parser logic
- `rust/data-modelling-sdk/src/export/sql.rs` - Migrate SQL exporter logic

### Medium Priority
- `rust/src/api/routes/import.rs` - Use SDK importers
- `rust/src/api/routes/models.rs` - Use SDK exporters
- `rust/data-modeller/src/app.rs` - Add SDK integration

### Low Priority (After core migration)
- Other parsers (ODCL, JSON Schema, AVRO, Protobuf)
- Other exporters
- Validation logic migration

## Testing Checklist

- [ ] SDK compiles with all features
- [ ] File system backend works with temp directories
- [ ] API backend can load/save models
- [ ] Browser backend works in WASM
- [ ] SQL import via SDK matches existing behavior
- [ ] SQL export via SDK matches existing behavior
- [ ] API routes still work after migration
- [ ] WASM app can switch between online/offline
- [ ] Native app can open folders and load models
