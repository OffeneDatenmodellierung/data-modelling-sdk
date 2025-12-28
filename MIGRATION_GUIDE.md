# SDK Migration Guide

## Current Status

The SDK structure is in place with:
- ✅ Storage backend abstraction (FileSystem, Browser, API)
- ✅ Model loader/saver structure
- ✅ Import/export/validation module structure
- ⏳ Parser/exporter implementations (wrappers created, logic migration pending)

## Migration Strategy

### Phase 1: Type Alignment (Current)

The SDK currently uses placeholder types. To enable full migration:

1. **Option A: Copy models to SDK** (Recommended for independence)
   - Copy `Table`, `Relationship`, `DataModel`, `Column` types to SDK
   - Update SDK to use its own types
   - Create conversion functions between SDK and API types

2. **Option B: Shared models crate** (Better long-term)
   - Create `canvas-models` crate
   - Both SDK and API depend on `canvas-models`
   - No duplication, single source of truth

3. **Option C: SDK depends on parent** (Quick but creates coupling)
   - SDK depends on `data-modelling-api` crate
   - Re-export types from parent
   - Faster migration but creates circular dependency risk

### Phase 2: Parser Migration

1. **SQL Parser** (`rust/src/api/services/sql_parser.rs` → `data-modelling-sdk/src/import/sql.rs`)
   - Move `SQLParser` struct and implementation
   - Update to use SDK types
   - Keep API route as thin wrapper

2. **ODCL Parser** (`rust/src/api/services/odcs_parser.rs` → `data-modelling-sdk/src/import/odcl.rs`)
   - Move `ODCSParser` struct and implementation
   - Update to use SDK types

3. **Other Parsers** (JSON Schema, AVRO, Protobuf)
   - Similar migration pattern

### Phase 3: Exporter Migration

1. **SQL Exporter** (`rust/src/export/sql.rs` → `data-modelling-sdk/src/export/sql.rs`)
2. **JSON Schema Exporter** (`rust/src/export/json_schema.rs` → `data-modelling-sdk/src/export/json_schema.rs`)
3. **Other Exporters** (AVRO, Protobuf, PNG, ODCL)

### Phase 4: Validation Migration

1. **Table Validation** (`rust/src/api/services/model_service.rs::detect_naming_conflicts` → `data-modelling-sdk/src/validation/tables.rs`)
2. **Relationship Validation** (`rust/src/api/services/relationship_service.rs::check_circular_dependency` → `data-modelling-sdk/src/validation/relationships.rs`)

### Phase 5: Storage Backend Completion

1. **API Backend** (`data-modelling-sdk/src/storage/api.rs`)
   - Implement HTTP calls to API endpoints
   - Handle authentication/session
   - Map API responses to SDK types

2. **Browser Backend** (`data-modelling-sdk/src/storage/browser.rs`)
   - Fix IndexedDB async handling
   - Implement proper error handling
   - Add localStorage fallback

### Phase 6: Integration

1. **API Routes** (`rust/src/api/routes/`)
   - Update import routes to use SDK importers
   - Update export routes to use SDK exporters
   - Keep routes as thin HTTP wrappers

2. **WASM App** (`rust/data-modeller/`)
   - Add SDK dependency
   - Implement online/offline mode switching
   - Use BrowserStorageBackend for offline mode
   - Use ApiStorageBackend for online mode

3. **Native App** (`rust/data-modeller/` native build)
   - Add file/folder picker (using `rfd` crate)
   - Use FileSystemStorageBackend for offline mode
   - Use ApiStorageBackend for online mode (default)

## Implementation Notes

### Type Conversion

During migration, conversion functions will be needed:

```rust
// SDK types
pub struct Table { ... }

// API types  
pub struct Table { ... }

// Conversion
impl From<api::Table> for sdk::Table { ... }
impl From<sdk::Table> for api::Table { ... }
```

### Backward Compatibility

- Keep existing API routes working during migration
- Use feature flags to enable SDK usage gradually
- Maintain API contract (request/response formats)

### Testing Strategy

1. Unit tests for SDK functions with mock storage backends
2. Integration tests for file system backend
3. WASM tests for browser storage backend
4. API tests to ensure routes still work

## Next Steps

1. **Choose migration approach** (Option A, B, or C for types)
2. **Migrate one parser** (start with SQL as it's most used)
3. **Migrate one exporter** (start with SQL)
4. **Complete storage backends**
5. **Integrate into one route** (test the pattern)
6. **Roll out to all routes**
7. **Add to WASM/native apps**
