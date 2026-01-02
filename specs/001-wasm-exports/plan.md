# Implementation Plan: WASM Module Parsing Function Exports

**Branch**: `001-wasm-exports` | **Date**: 2026-01-02 | **Spec**: [specs/001-wasm-exports/spec.md](specs/001-wasm-exports/spec.md)
**Input**: Feature specification from `/specs/001-wasm-exports/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Expose SDK import/export functions as WASM bindings to enable offline parsing and export operations in browser environments. Currently, only initialization functions (`initSync`, `default`) are exposed. This plan adds WASM bindings for ODCS YAML parsing/export and import/export functions for SQL, AVRO, JSON Schema, and Protobuf formats, enabling the web application to eliminate JavaScript fallback parsers and provide consistent SDK-based functionality in offline mode.

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2024)
**Primary Dependencies**: wasm-bindgen 0.2, wasm-bindgen-futures 0.4, serde 1.0, serde_json 1.0, serde_yaml 0.9
**Storage**: N/A (this feature focuses on in-memory parsing/export operations)
**Testing**: cargo test with wasm32-unknown-unknown target, wasm-pack test, browser-based integration tests
**Target Platform**: WebAssembly (wasm32-unknown-unknown), browser environments (Chrome, Firefox, Safari, Edge)
**Project Type**: Single project (Rust library with WASM bindings)
**Performance Goals**: Parse ODCS YAML files up to 5MB within 3 seconds, export operations complete within 2 seconds for typical data models
**Constraints**: Functions must be synchronous or properly handle async via wasm-bindgen-futures, all data structures must be serializable to/from JavaScript via serde, error handling must convert Rust errors to JavaScript-compatible error types
**Scale/Scope**: Support parsing/export of data models with up to 1000 tables, handle YAML files up to 10MB, support concurrent calls from multiple browser contexts

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Verify compliance with Data Modelling SDK Constitution principles:

- **Commit Requirements**: Code MUST build successfully before commit. All commits MUST be GPG signed.
- **Code Quality & Security**: Plan MUST include security audit, formatting, linting checks. Dependencies MUST use latest stable versions.
- **Storage Abstraction**: If adding storage operations, MUST use `StorageBackend` trait. MUST be async and feature-gated appropriately.
- **Feature Flags**: If adding optional functionality, MUST be behind Cargo features with clear documentation.
- **Testing Requirements**: Plan MUST include unit tests, integration tests, and doctests where applicable.
- **Import/Export Patterns**: If adding format support, MUST follow importer/exporter trait patterns.
- **Error Handling**: MUST use structured error types (`thiserror` for library errors, `anyhow` for convenience where appropriate).

**Constitution Compliance**: ✅ All principles satisfied
- WASM bindings will be feature-gated behind existing `wasm` feature
- Functions follow existing import/export trait patterns
- Error handling uses existing `ImportError` and `ExportError` types
- Testing will include unit tests, WASM-specific integration tests, and browser tests

Any violations or exceptions MUST be documented in the Complexity Tracking section below.

## Project Structure

### Documentation (this feature)

```text
specs/001-wasm-exports/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   └── wasm-bindings.md # WASM function signatures and TypeScript definitions
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Main library entry point (add WASM module here)
├── import/              # Import modules (ODCS, SQL, AVRO, JSON Schema, Protobuf)
│   ├── mod.rs
│   ├── odcs.rs
│   ├── sql.rs
│   ├── avro.rs
│   ├── json_schema.rs
│   └── protobuf.rs
├── export/              # Export modules (ODCS, SQL, AVRO, JSON Schema, Protobuf)
│   ├── mod.rs
│   ├── odcs.rs
│   ├── sql.rs
│   ├── avro.rs
│   ├── json_schema.rs
│   └── protobuf.rs
└── models/              # Data model structures (Table, Column, DataModel, etc.)

tests/
├── wasm/                # WASM-specific tests
│   ├── wasm_import_tests.rs
│   └── wasm_export_tests.rs
└── integration/        # Existing integration tests
```

**Structure Decision**: Single project structure maintained. WASM bindings will be added to `src/lib.rs` in a new `#[cfg(all(target_arch = "wasm32", feature = "wasm"))]` module. Tests will be added to `tests/wasm/` directory. All existing import/export modules remain unchanged - WASM bindings will wrap existing functionality.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations identified. All WASM bindings follow existing patterns and are feature-gated appropriately.
