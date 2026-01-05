# Implementation Plan: ODPS Schema Validation and Manual Test Script

**Branch**: `001-odps-validation` | **Date**: 2026-01-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-odps-validation/spec.md`

## Summary

Add schema validation to ODPS import and export operations using the existing `jsonschema` crate, following the pattern established for ODCS validation. Implement CLI support for ODPS as a native format (import/export commands), and create a manual test script for round-trip verification. Validation will be feature-flagged via `odps-validation` to maintain backward compatibility.

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2024)
**Primary Dependencies**:
- `jsonschema` crate (v0.20, already in Cargo.toml as optional dependency)
- `serde_json`, `serde_yaml` (for JSON/YAML parsing)
- `clap` (for CLI commands, already available via `cli` feature)
- Existing `ODPSImporter` and `ODPSExporter` implementations

**Storage**: N/A (file-based import/export, no persistent storage)
**Testing**: `cargo test` with unit tests, integration tests, and doctests
**Target Platform**: Native (CLI), WASM (library functions), API backend
**Project Type**: Library with CLI binary (single project structure)
**Performance Goals**: Validation errors reported within 2 seconds for files up to 1MB
**Constraints**:
- Must maintain backward compatibility when `odps-validation` feature is disabled
- Must follow existing validation patterns (see `src/cli/validation.rs`)
- ODPS is standalone format (no conversion to other formats)
**Scale/Scope**:
- Single validation function for import/export
- CLI command handlers for ODPS format
- One test script executable
- Feature flag for optional validation

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Verify compliance with Data Modelling SDK Constitution principles:

- **Commit Requirements**: ✅ Code MUST build successfully before commit. All commits MUST be GPG signed.
- **Code Quality & Security**: ✅ Plan includes security audit (`cargo audit`), formatting (`cargo fmt`), linting (`cargo clippy`) checks. Dependencies use latest stable versions (`jsonschema` v0.20).
- **Storage Abstraction**: ✅ N/A - No storage operations added (file-based import/export only)
- **Feature Flags**: ✅ Validation functionality gated behind `odps-validation` feature flag, following existing `schema-validation` pattern
- **Testing Requirements**: ✅ Plan includes unit tests for validation functions, integration tests for CLI commands, and doctests for public APIs
- **Import/Export Patterns**: ✅ Follows existing importer/exporter trait patterns. Uses existing `ODPSImporter` and `ODPSExporter` implementations
- **Error Handling**: ✅ Uses structured error types (`ExportError`, `ImportError` via `thiserror`), following existing patterns

**No violations or exceptions identified.**

## Project Structure

### Documentation (this feature)

```text
specs/001-odps-validation/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── export/
│   └── odps.rs          # Add validation to ODPSExporter
├── import/
│   └── odps.rs          # Add validation to ODPSImporter
├── cli/
│   ├── commands/
│   │   ├── import.rs    # Add handle_import_odps function
│   │   └── export.rs    # Add handle_export_odps function
│   ├── validation.rs    # Add validate_odps function (following existing pattern)
│   └── main.rs          # Add ODPS to ImportFormatArg and ExportFormatArg enums
└── lib.rs               # (no changes needed - WASM bindings already exist)

tests/
├── odps_tests.rs        # Add validation tests, field preservation tests
└── cli/
    ├── import_tests.rs   # Add ODPS import CLI tests
    └── export_tests.rs   # Add ODPS export CLI tests

scripts/
└── test-odps.sh         # NEW: Manual test script for ODPS round-trip testing
```

**Structure Decision**: Single project structure (existing). New code integrates into existing modules following established patterns. Test script added to `scripts/` directory.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations identified - all requirements comply with constitution.
