# Functions Not Available to WASM

This document lists all SDK functions that are **NOT** currently exposed via WASM bindings.

## Summary

Currently, **10 functions** are exposed via WASM (5 import, 5 export). The following categories of functions are **NOT** available:

## 1. Validation Functions

All validation functions are **NOT** exposed via WASM:

### Input Validation (`src/validation/input.rs`)

- `validate_table_name(name: &str) -> ValidationResult<()>`
  - Validates table name format, length, reserved words
- `validate_column_name(name: &str) -> ValidationResult<()>`
  - Validates column name format, length, reserved words
- `validate_uuid(id: &str) -> ValidationResult<Uuid>`
  - Validates UUID string format
- `validate_data_type(data_type: &str) -> ValidationResult<()>`
  - Validates data type string format
- `validate_description(desc: &str) -> ValidationResult<()>`
  - Validates description length and content
- `sanitize_sql_identifier(name: &str, dialect: &str) -> String`
  - Sanitizes SQL identifiers by quoting them
- `sanitize_description(desc: &str) -> String`
  - Sanitizes description strings

### Table Validation (`src/validation/tables.rs`)

- `TableValidator::detect_naming_conflicts(existing_tables: &[Table], new_tables: &[Table]) -> Vec<NamingConflict>`
  - Detects naming conflicts between tables
- `TableValidator::validate_pattern_exclusivity(table: &Table) -> Result<(), PatternViolation>`
  - Validates that SCD pattern and Data Vault classification are mutually exclusive

### Relationship Validation (`src/validation/relationships.rs`)

- `RelationshipValidator::check_circular_dependency(relationships: &[Relationship], source_table_id: Uuid, target_table_id: Uuid) -> Result<(bool, Option<Vec<Uuid>>), RelationshipValidationError>`
  - Checks for circular dependencies in relationship graph
- `RelationshipValidator::validate_no_self_reference(source_table_id: Uuid, target_table_id: Uuid) -> Result<(), SelfReference>`
  - Validates that source and target tables are different

## 2. Export Functions

### PNG Export (`src/export/png.rs`)

- `PNGExporter::export(tables: &[Table], width: u32, height: u32) -> Result<ExportResult, ExportError>`
  - Exports data model to PNG image format (base64-encoded)
  - **Note**: Feature-gated behind `png-export` feature flag
  - **Note**: Currently generates diagram-only PNG (no text rendering)

## 3. Model Loading/Saving (`src/model/`)

All model loading and saving operations are **NOT** exposed via WASM:

### Model Loader (`src/model/loader.rs`)

- `ModelLoader::load_model(workspace_path: &str) -> Result<ModelLoadResult, StorageError>`
  - Loads model from storage backend (async)
  - Requires `StorageBackend` implementation
  - **Note**: Async operations are challenging in WASM without `wasm-bindgen-futures`

### Model Saver (`src/model/saver.rs`)

- `ModelSaver::save_model(model: &DataModel, workspace_path: &str) -> Result<(), StorageError>`
  - Saves model to storage backend (async)
  - Requires `StorageBackend` implementation
  - **Note**: Async operations are challenging in WASM without `wasm-bindgen-futures`

### API Model Loader (`src/model/api_loader.rs`)

- `ApiModelLoader::load_model_from_api(workspace_id: &str) -> Result<ModelLoadResult, StorageError>`
  - Loads model from API backend (async)
  - Feature-gated behind `api-backend` feature flag
  - **Note**: Async operations are challenging in WASM without `wasm-bindgen-futures`

## 4. Storage Backend Operations (`src/storage/`)

All storage backend operations are **NOT** exposed via WASM:

### StorageBackend Trait (`src/storage/mod.rs`)

All methods are async and require trait implementation:
- `file_exists(path: &str) -> Result<bool, StorageError>`
- `read_file(path: &str) -> Result<String, StorageError>`
- `write_file(path: &str, content: &str) -> Result<(), StorageError>`
- `delete_file(path: &str) -> Result<(), StorageError>`
- `list_files(path: &str) -> Result<Vec<String>, StorageError>`
- `dir_exists(path: &str) -> Result<bool, StorageError>`
- `create_dir(path: &str) -> Result<(), StorageError>`
- `delete_dir(path: &str) -> Result<(), StorageError>`

**Note**: Storage operations are async and require backend-specific implementations (FileSystem, Browser, API).

## 5. Git Operations (`src/git/`)

All Git operations are **NOT** exposed via WASM:

- `GitService` methods (if `git` feature is enabled)
  - Feature-gated behind `git` feature flag
  - Requires file system access (not available in browser WASM)

## 6. Workspace Management (`src/workspace/`)

Workspace types are available, but **NO operations** are exposed:
- Types: `WorkspaceInfo`, `ProfileInfo`, `CreateWorkspaceRequest`, etc.
- No functions for creating, listing, or managing workspaces

## 7. Authentication (`src/auth/`)

Authentication types are available, but **NO operations** are exposed:
- Types: `AuthMode`, `AuthState`, `GitHubEmail`, etc.
- No functions for authentication flows

## Rationale for Missing Functions

### Validation Functions
- **Why not exposed**: These are utility functions that can be called from JavaScript/TypeScript directly if needed, or validation can be performed client-side.
- **Potential value**: Could be useful for client-side validation before sending data to backend.

### PNG Export
- **Why not exposed**: Feature-gated, and PNG generation may have large binary size impact.
- **Potential value**: Could be useful for generating visual diagrams in browser.

### Model Loading/Saving
- **Why not exposed**: These require async operations and storage backends. WASM bindings would need `wasm-bindgen-futures` and JavaScript Promise integration.
- **Potential value**: High - would enable full offline model management in browser.

### Storage Backend Operations
- **Why not exposed**: Async operations, requires backend-specific implementations. Browser storage backend exists but operations are async.
- **Potential value**: High - would enable full offline storage in browser using IndexedDB/localStorage.

### Git Operations
- **Why not exposed**: Requires file system access, not available in browser WASM context.
- **Potential value**: Low - Git operations don't make sense in browser context.

## Recommendations

### High Priority (Consider Adding)

1. **Validation Functions** - Useful for client-side validation:
   - `validate_table_name`
   - `validate_column_name`
   - `validate_data_type`
   - `TableValidator::detect_naming_conflicts`
   - `RelationshipValidator::check_circular_dependency`

2. **Model Loading/Saving** - Would enable full offline functionality:
   - `ModelLoader::load_model` (with BrowserStorageBackend)
   - `ModelSaver::save_model` (with BrowserStorageBackend)
   - Requires `wasm-bindgen-futures` for async support

### Medium Priority

3. **PNG Export** - Could be useful for visual diagrams:
   - `PNGExporter::export`
   - Requires `png-export` feature flag

### Low Priority

4. **Storage Backend Operations** - Would require async WASM bindings
5. **Git Operations** - Not applicable to browser context
6. **Workspace Management** - Could be handled by application layer

## Implementation Notes

To add missing functions:

1. **Validation functions**: Straightforward - synchronous, simple input/output
2. **Async operations**: Require `wasm-bindgen-futures` and JavaScript Promise integration
3. **Storage operations**: Require BrowserStorageBackend integration with WASM
4. **PNG export**: Requires feature flag handling and base64 encoding

---

**Last Updated**: 2026-01-02
**Current WASM Functions**: 10 (5 import, 5 export)
**Missing Functions**: ~20+ validation, storage, and model management functions
