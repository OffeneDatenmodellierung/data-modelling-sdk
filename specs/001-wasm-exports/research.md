# Research: WASM Bindings and Missing Function Exports

**Date**: 2026-01-02
**Feature**: WASM Module Parsing Function Exports
**Purpose**: Document current WASM binding state, identify missing exports, and research best practices

## Current State Analysis

### Existing WASM Bindings

**Currently Exposed Functions** (from `pkg/data_modelling_sdk.d.ts`):
- `initSync(module)`: Synchronous WASM module initialization
- `default()` / `__wbg_init()`: Asynchronous WASM module initialization

**Conclusion**: Only initialization functions are exposed. No parsing or export functions are available.

### Missing WASM Exports - Import Functions

Based on codebase analysis, the following import functions exist but are NOT exposed via WASM:

1. **ODCSImporter** (`src/import/odcs.rs`):
   - `import(&mut self, yaml_content: &str) -> Result<ImportResult, ImportError>`
   - `parse_table(&mut self, yaml_content: &str) -> Result<(Table, Vec<ParserError>)>`

2. **SQLImporter** (`src/import/sql.rs`):
   - `parse(&self, sql: &str) -> Result<ImportResult>`
   - `parse_liquibase(&self, sql: &str) -> Result<ImportResult>`

3. **AvroImporter** (`src/import/avro.rs`):
   - `import(&self, avro_content: &str) -> Result<ImportResult, ImportError>`

4. **JSONSchemaImporter** (`src/import/json_schema.rs`):
   - `import(&self, json_content: &str) -> Result<ImportResult, ImportError>`

5. **ProtobufImporter** (`src/import/protobuf.rs`):
   - `import(&self, proto_content: &str) -> Result<ImportResult, ImportError>`

### Missing WASM Exports - Export Functions

The following export functions exist but are NOT exposed via WASM:

1. **ODCSExporter** (`src/export/odcs.rs`):
   - `export_table(table: &Table, format: &str) -> String`
   - `export(&self, tables: &[Table], format: &str) -> Result<ExportResult, ExportError>`
   - `export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> String`

2. **SQLExporter** (`src/export/sql.rs`):
   - `export_table(table: &Table, dialect: Option<&str>) -> String`
   - `export(&self, tables: &[Table], dialect: Option<&str>) -> Result<ExportResult, ExportError>`
   - `export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> String`

3. **AvroExporter** (`src/export/avro.rs`):
   - `export(&self, tables: &[Table]) -> Result<ExportResult, ExportError>`
   - `export_table(table: &Table) -> Value`
   - `export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> Value`

4. **JSONSchemaExporter** (`src/export/json_schema.rs`):
   - `export(&self, tables: &[Table]) -> Result<ExportResult, ExportError>`
   - `export_table(table: &Table) -> Value`
   - `export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> Value`

5. **ProtobufExporter** (`src/export/protobuf.rs`):
   - `export(&self, tables: &[Table]) -> Result<ExportResult, ExportError>`
   - `export_table(table: &Table, field_number: &mut u32) -> String`
   - `export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> String`

### Additional Functions Not Exposed

**Validation Functions** (`src/validation/`):
- `TableValidator::detect_naming_conflicts()`
- `RelationshipValidator::check_circular_dependency()`
- These may be useful for WASM but are not in scope for this feature

**Model Operations** (`src/model/`):
- `ModelLoader::load_model()`
- `ModelSaver::save_model()`
- These require storage backends and are out of scope (storage operations are already WASM-compatible via BrowserStorageBackend)

## WASM Binding Best Practices Research

### Decision: Use wasm-bindgen for Function Exports

**Rationale**:
- Project already uses `wasm-bindgen` 0.2 for WASM support
- `wasm-bindgen` provides automatic TypeScript definition generation
- Supports both sync and async operations via `wasm-bindgen-futures`
- Handles serialization/deserialization via serde integration

**Alternatives Considered**:
- Manual WASM exports: Too low-level, requires manual TypeScript definitions
- `wasm-pack`: Already used for building, but bindings still use wasm-bindgen

### Decision: Serialization Strategy

**Rationale**:
- Use `serde` with `Serialize`/`Deserialize` for all data structures
- Convert Rust types to `JsValue` using `wasm-bindgen::JsValue`
- Use `serde-wasm-bindgen` crate for seamless conversion (if needed)
- Return JSON strings for complex types, parse in JavaScript (simpler than deep JsValue conversion)

**Alternatives Considered**:
- Deep JsValue conversion: More complex, harder to maintain
- JSON string serialization: Simpler, more flexible, easier to debug

### Decision: Error Handling Strategy

**Rationale**:
- Convert `ImportError` and `ExportError` to JavaScript Error objects
- Include error message and error type in thrown exceptions
- Use `wasm-bindgen`'s built-in error conversion for `Result<T, E>` where `E: std::error::Error`
- Wrap errors in `JsValue` for consistent JavaScript error handling

**Alternatives Considered**:
- Return error objects: More complex, requires custom error types in TypeScript
- Return error codes: Less informative, requires error code mapping

### Decision: Async vs Sync Functions

**Rationale**:
- Import/export operations are CPU-bound and should be synchronous
- No I/O operations required (all data passed as strings)
- Synchronous functions are simpler to call from JavaScript
- Use `#[wasm_bindgen]` without async for all parsing/export functions

**Alternatives Considered**:
- Async functions: Unnecessary complexity for CPU-bound operations
- Web Workers: Overkill for parsing operations, adds complexity

### Decision: Function Naming Convention

**Rationale**:
- Use snake_case for Rust functions (Rust convention)
- `wasm-bindgen` automatically converts to camelCase in JavaScript
- Example: `parse_odcs_yaml()` → `parseOdcsYaml()` in JavaScript
- Keep names descriptive and consistent with existing SDK API

**Alternatives Considered**:
- camelCase in Rust: Violates Rust naming conventions
- Custom JavaScript names: Requires manual mapping, more maintenance

### Decision: TypeScript Definition Generation

**Rationale**:
- `wasm-pack` automatically generates TypeScript definitions from `#[wasm_bindgen]` attributes
- Definitions are written to `pkg/data_modelling_sdk.d.ts`
- Manual TypeScript definitions not needed if bindings are properly annotated
- Use `#[wasm_bindgen(typescript_custom_section)]` for complex type documentation

**Alternatives Considered**:
- Manual TypeScript definitions: Error-prone, requires manual sync with Rust code
- Separate TypeScript package: Unnecessary complexity for this use case

## Implementation Approach

### Module Structure

Create a new module `src/wasm.rs` (or add to `src/lib.rs`) with:

```rust
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
mod wasm {
    use wasm_bindgen::prelude::*;

    // Import functions
    #[wasm_bindgen]
    pub fn parse_odcs_yaml(yaml_content: &str) -> Result<JsValue, JsValue> { ... }

    #[wasm_bindgen]
    pub fn import_from_sql(sql_content: &str, dialect: &str) -> Result<JsValue, JsValue> { ... }

    // Export functions
    #[wasm_bindgen]
    pub fn export_to_odcs_yaml(workspace: JsValue) -> Result<String, JsValue> { ... }

    // ... etc
}
```

### Data Conversion Strategy

1. **Input**: JavaScript strings → Rust `&str` (automatic via wasm-bindgen)
2. **Output**: Rust types → JSON string → JavaScript (parse in JS)
3. **Errors**: Rust errors → JsValue → JavaScript Error objects

### Performance Considerations

- Parsing operations are CPU-bound, suitable for synchronous execution
- Large files (5MB+) may block the main thread - acceptable for offline mode
- Consider Web Workers for very large files if performance becomes an issue (future enhancement)

## Open Questions Resolved

1. **Q**: Should functions be async?
   **A**: No, synchronous is sufficient for CPU-bound parsing operations.

2. **Q**: How to handle complex data structures?
   **A**: Serialize to JSON strings, parse in JavaScript.

3. **Q**: Should we expose validation functions?
   **A**: Not in scope for this feature, but can be added later if needed.

4. **Q**: How to handle errors?
   **A**: Convert to JsValue and throw JavaScript errors.

5. **Q**: Should we expose model loading/saving?
   **A**: No, those require storage backends which are already WASM-compatible via BrowserStorageBackend.

## References

- [wasm-bindgen documentation](https://rustwasm.github.io/wasm-bindgen/)
- [wasm-pack documentation](https://rustwasm.github.io/wasm-pack/)
- [serde-wasm-bindgen crate](https://crates.io/crates/serde-wasm-bindgen)
- Existing SDK import/export modules in `src/import/` and `src/export/`
