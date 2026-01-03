# Implementation Plan: BPMN, DMN, and OpenAPI Schema Support

**Branch**: `004-bpmn-dmn-openapi` | **Date**: 2026-01-03 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/004-bpmn-dmn-openapi/spec.md`

## Summary

This feature extends the Data Modelling SDK to support BPMN 2.0, DMN 1.3, and OpenAPI 3.1.1 models. Models are stored in their native formats (XML for BPMN/DMN, YAML/JSON for OpenAPI) within domain directories, following the existing domain-based file structure. CADS assets can reference these models, enabling comprehensive documentation of business processes, decisions, and APIs alongside data contracts. The feature includes import/export functionality, WASM bindings for frontend integration, and an OpenAPI-to-ODCS converter for schema reuse.

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2024)
**Primary Dependencies**:
- `xml-rs` or `quick-xml` for XML parsing and validation
- `jsonschema` crate (already available) for OpenAPI validation
- `serde_yaml` (already available) for YAML processing
- `wasm-bindgen` (already available, feature-gated) for WASM bindings
- `async-trait` (already available) for async storage operations

**Storage**: Domain-based file structure using `StorageBackend` trait (already implemented)
- BPMN: `{domain_name}/{model_name}.bpmn.xml`
- DMN: `{domain_name}/{model_name}.dmn.xml`
- OpenAPI: `{domain_name}/{api_name}.openapi.yaml` or `.openapi.json`

**Testing**: `cargo test --all-features` (unit tests, integration tests, doctests)

**Target Platform**:
- Native (via `native-fs` feature)
- WASM/browser (via `wasm` feature)
- HTTP API (via `api-backend` feature, default)

**Project Type**: Single Rust library with feature-gated functionality

**Performance Goals**:
- Import BPMN/DMN files (<1MB) within 2 seconds
- Import OpenAPI files (<500KB) within 3 seconds
- Validate references within 100ms
- WASM operations complete within 500ms for typical files

**Constraints**:
- Files must be validated against official schemas before storage
- Format preservation critical (YAML vs JSON for OpenAPI)
- Cross-domain references must be validated
- Maximum file sizes: BPMN/DMN 10MB, OpenAPI 5MB

**Scale/Scope**:
- Support for domains with up to 100 models per type
- Handle typical BPMN/DMN file sizes (under 10MB)
- Handle typical OpenAPI file sizes (under 5MB)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Verify compliance with Data Modelling SDK Constitution principles:

- ✅ **Commit Requirements**: Code MUST build successfully before commit. All commits MUST be GPG signed.
- ✅ **Code Quality & Security**: Plan includes security audit (`cargo audit`), formatting (`cargo fmt`), linting (`cargo clippy`). Dependencies will use latest stable versions compatible with Rust 1.75+.
- ✅ **Storage Abstraction**: All storage operations use `StorageBackend` trait. Operations are async and feature-gated (`native-fs` for filesystem, `wasm` for browser).
- ✅ **Feature Flags**: BPMN/DMN/OpenAPI support will be behind optional Cargo features (`bpmn`, `dmn`, `openapi`) with clear documentation in `Cargo.toml`.
- ✅ **Testing Requirements**: Plan includes unit tests for importers/exporters, integration tests for storage operations, and doctests for public APIs.
- ✅ **Import/Export Patterns**: Follows existing importer/exporter trait patterns (similar to `CADSImporter`, `ODPSImporter`).
- ✅ **Error Handling**: Uses structured error types (`thiserror` for library errors, `anyhow` for convenience where appropriate).

**No violations or exceptions identified.**

## Project Structure

### Documentation (this feature)

```text
specs/004-bpmn-dmn-openapi/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   ├── bpmn-api.md
│   ├── dmn-api.md
│   ├── openapi-api.md
│   └── openapi-to-odcs-api.md
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── import/
│   ├── bpmn.rs          # BPMN importer
│   ├── dmn.rs           # DMN importer
│   └── openapi.rs       # OpenAPI importer
├── export/
│   ├── bpmn.rs          # BPMN exporter
│   ├── dmn.rs           # DMN exporter
│   └── openapi.rs       # OpenAPI exporter
├── convert/
│   └── openapi_to_odcs.rs  # OpenAPI to ODCS converter
├── models/
│   ├── bpmn.rs          # BPMN model structures (minimal, for references)
│   ├── dmn.rs           # DMN model structures (minimal, for references)
│   └── openapi.rs       # OpenAPI model structures (minimal, for references)
└── lib.rs               # WASM bindings for BPMN/DMN/OpenAPI

tests/
├── bpmn_tests.rs        # BPMN import/export tests
├── dmn_tests.rs         # DMN import/export tests
├── openapi_tests.rs     # OpenAPI import/export tests
└── openapi_converter_tests.rs  # OpenAPI to ODCS conversion tests

schemas/
├── bpmn-2.0.xsd         # BPMN 2.0 XSD schema (to be added)
├── dmn-1.3.xsd          # DMN 1.3 XSD schema (to be added)
└── openapi-3.1.1.json   # OpenAPI 3.1.1 JSON Schema (to be added)
```

**Structure Decision**: Following existing SDK structure with import/export modules, models for reference metadata, and converters. Models are stored as-is (no parsing into Rust structs beyond minimal metadata), following the "store native formats" requirement.

## Complexity Tracking

> **No violations identified - all requirements comply with Constitution**

## Phase 0: Research & Decisions

See [research.md](./research.md) for:
- XML parsing library selection (xml-rs vs quick-xml)
- XSD validation approach
- OpenAPI schema validation patterns
- CADS reference extension approach
- OpenAPI to ODCS type mapping strategy

## Phase 1: Design & Contracts

See:
- [data-model.md](./data-model.md) - Entity definitions and relationships
- [contracts/](./contracts/) - API contracts for import/export/conversion operations
- [quickstart.md](./quickstart.md) - Usage examples and getting started guide

## Phase 2: Implementation Tasks

See [tasks.md](./tasks.md) - Detailed implementation tasks (created by `/speckit.tasks`)
