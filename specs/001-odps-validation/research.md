# Research: ODPS Schema Validation

**Feature**: ODPS Schema Validation and Manual Test Script
**Date**: 2026-01-05
**Phase**: 0 - Outline & Research

## Research Questions

### 1. JSON Schema Validation Library Choice

**Question**: Which JSON Schema validation library should be used for ODPS validation?

**Research**:
- Existing codebase uses `jsonschema` crate (v0.20) for ODCS and OpenAPI validation
- Pattern established in `src/cli/validation.rs` uses `jsonschema::Validator`
- ODPS JSON Schema uses Draft 2019-09 format (compatible with `jsonschema` crate)

**Decision**: Use `jsonschema` crate (v0.20) - already in Cargo.toml as optional dependency under `schema-validation` feature

**Rationale**:
- Consistency with existing validation patterns
- No new dependencies required
- Proven compatibility with JSON Schema Draft 2019-09
- Follows existing feature flag pattern

**Alternatives considered**:
- `valico` - Not used in codebase, would require new dependency
- `schemars` - Different purpose (schema generation, not validation)
- Custom validation - Too complex, reinventing the wheel

---

### 2. Validation Integration Pattern

**Question**: How should validation be integrated into ODPS import/export operations?

**Research**:
- Existing pattern in `src/cli/validation.rs`:
  - Validation functions return `Result<(), CliError>`
  - Feature-gated with `#[cfg(feature = "schema-validation")]`
  - Uses `include_str!` to load schema at compile time
  - Converts YAML to JSON for validation

**Decision**: Follow existing validation pattern:
- Create `validate_odps()` function in `src/cli/validation.rs`
- Add validation calls in `ODPSImporter::import()` and `ODPSExporter::export()`
- Feature-gate with `odps-validation` feature (which depends on `schema-validation`)
- Load schema using `include_str!("../../schemas/odps-json-schema-latest.json")`

**Rationale**:
- Consistency with existing codebase patterns
- Reuses proven validation infrastructure
- Feature flag allows backward compatibility
- Compile-time schema loading ensures schema is always available

**Alternatives considered**:
- Runtime schema loading - Less reliable, adds I/O complexity
- Separate validation module - Unnecessary abstraction for single format
- Validation only in CLI - Too limiting, library users need validation too

---

### 3. CLI Command Integration

**Question**: How should ODPS CLI commands be integrated into the existing CLI structure?

**Research**:
- Existing CLI structure:
  - `src/cli/main.rs` defines `ImportFormatArg` and `ExportFormatArg` enums
  - `src/cli/commands/import.rs` has handlers like `handle_import_odcs()`
  - `src/cli/commands/export.rs` has handlers like `handle_export_odcs()`
  - ODCS is already supported as a format option

**Decision**: Add ODPS to CLI following ODCS pattern:
- Add `Odps` variant to `ImportFormatArg` enum in `src/cli/main.rs`
- Add `Odps` variant to `ExportFormatArg` enum in `src/cli/main.rs`
- Create `handle_import_odps()` in `src/cli/commands/import.rs`
- Create `handle_export_odps()` in `src/cli/commands/export.rs`
- Map enum variants to handler functions in `main.rs`

**Rationale**:
- Consistent with existing CLI architecture
- Minimal changes required
- Follows established patterns
- ODPS treated as first-class format like ODCS

**Alternatives considered**:
- Separate CLI binary - Unnecessary complexity, violates single binary principle
- Subcommand approach - Over-engineered for simple import/export operations

---

### 4. Field Preservation Verification

**Question**: How should field preservation be verified during round-trip testing?

**Research**:
- ODPS schema defines many optional fields (tags, customProperties, authoritativeDefinitions, etc.)
- Current exporter may omit empty optional fields
- Need to ensure all fields (including empty ones) are preserved

**Decision**: Implement field-by-field comparison in test script:
- Parse original ODPS YAML to JSON
- Parse exported ODPS YAML to JSON
- Compare JSON structures recursively
- Report any missing or changed fields
- Handle empty arrays/objects explicitly (preserve structure)

**Rationale**:
- JSON comparison is reliable and language-agnostic
- Recursive comparison catches nested field issues
- Explicit empty structure handling ensures fidelity
- Can be implemented in test script without changing core library

**Alternatives considered**:
- Deep equality on Rust structs - Too implementation-specific, doesn't verify YAML output
- Schema-based validation only - Doesn't verify field preservation, only schema compliance
- Manual inspection - Not scalable, error-prone

---

### 5. Test Script Implementation

**Question**: What should the test script do and how should it be implemented?

**Research**:
- Existing codebase has CLI infrastructure (`clap` for argument parsing)
- Test scripts typically in `scripts/` directory
- Need to import ODPS, display data, export ODPS, validate both

**Decision**: Create standalone Rust binary or shell script:
- Option A: Rust binary using CLI infrastructure (recommended)
- Option B: Shell script calling CLI binary
- Display imported data in human-readable format
- Perform round-trip export
- Validate both import and export
- Report field preservation results

**Rationale**:
- Rust binary provides better error handling and cross-platform support
- Can reuse existing CLI infrastructure
- Better integration with validation functions
- More maintainable than shell script

**Alternatives considered**:
- Shell script - Simpler but less portable, harder error handling
- Python script - Adds Python dependency, not consistent with Rust codebase
- Integrated into CLI - Too complex, test script should be separate tool

---

### 6. Feature Flag Strategy

**Question**: How should the `odps-validation` feature be structured?

**Research**:
- Existing `schema-validation` feature gates `jsonschema` dependency
- ODPS validation depends on `schema-validation` functionality
- Need backward compatibility when feature disabled

**Decision**: Create `odps-validation` feature that depends on `schema-validation`:
- `odps-validation` feature enables ODPS-specific validation
- Depends on `schema-validation` feature (which provides `jsonschema`)
- When disabled, import/export proceed without validation (backward compatible)
- Validation functions use `#[cfg(feature = "odps-validation")]` guards

**Rationale**:
- Follows existing feature flag patterns
- Allows granular control (can enable schema-validation without ODPS validation)
- Maintains backward compatibility
- Clear dependency relationship

**Alternatives considered**:
- Single `schema-validation` feature - Less granular, forces all validation or none
- No feature flag - Breaks backward compatibility, forces dependency on jsonschema

---

## Technical Decisions Summary

1. **JSON Schema Library**: `jsonschema` crate v0.20 (existing dependency)
2. **Validation Pattern**: Follow existing `validate_odcs()` pattern in `src/cli/validation.rs`
3. **CLI Integration**: Add ODPS to existing CLI enums and handlers (like ODCS)
4. **Field Preservation**: JSON comparison in test script with explicit empty structure handling
5. **Test Script**: Rust binary using CLI infrastructure (recommended) or shell script
6. **Feature Flag**: `odps-validation` feature depending on `schema-validation`

## Open Questions Resolved

All technical questions resolved. No remaining ambiguities blocking implementation.
