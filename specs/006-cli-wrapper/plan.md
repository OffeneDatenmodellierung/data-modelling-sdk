# Implementation Plan: CLI Wrapper for Data Modelling SDK

**Branch**: `006-cli-wrapper` | **Date**: 2026-01-05 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-cli-wrapper/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Extend the existing `examples/test_sql.rs` into a comprehensive CLI wrapper for the Data Modelling SDK. The CLI will support importing schemas from SQL (multiple dialects), AVRO, JSON Schema, Protobuf, OpenAPI, and ODCS formats with validation, external reference resolution, and UUID override capabilities. It will also support exporting to ODCS, AVRO, JSON Schema, Protobuf, and Protobuf descriptor formats. The implementation will use `clap` for argument parsing, leverage existing SDK import/export modules, and follow Rust CLI best practices.

## Technical Context

**Language/Version**: Rust 1.75+ (Rust 2024 edition)
**Primary Dependencies**:
- `clap` 4.x (CLI argument parsing - to be added)
- `reqwest` 0.12 (HTTP client for external reference resolution - already in dependencies)
- `zip` crate (JAR file extraction - to be added)
- Existing SDK modules: `SQLImporter`, `AvroImporter`, `JSONSchemaImporter`, `ProtobufImporter`, `OpenAPIImporter`, `ODCSImporter`, `ODCSExporter`, `AvroExporter`, `JSONSchemaExporter`, `ProtobufExporter`
- `jsonschema` 0.20 (validation - already in dependencies, feature-gated)
- `protoc` binary (external tool dependency for Protobuf descriptor generation)

**Storage**: N/A (CLI operates on files, no persistent storage backend needed)

**Testing**: `cargo test` with unit tests, integration tests, and CLI command tests

**Target Platform**: Native CLI binary (Linux, macOS, Windows)

**Project Type**: Single binary CLI application

**Performance Goals**:
- Import SQL schemas in under 5 seconds for files up to 1MB
- Schema validation in under 2 seconds for files up to 500KB
- External reference resolution with reasonable timeout (10 seconds default)

**Constraints**:
- Must work with existing SDK import/export modules without modification
- Must handle large schema files efficiently (streaming where possible)
- Must provide clear, actionable error messages
- Must validate UUID format when provided
- Must handle missing `protoc` gracefully with helpful error messages

**Scale/Scope**:
- Single binary CLI tool
- Support for 6 import formats (SQL, AVRO, JSON Schema, Protobuf, OpenAPI, ODCS)
- Support for 5 export formats (ODCS, AVRO, JSON Schema, Protobuf, Protobuf Descriptor)
- Multiple SQL dialects (postgres, mysql, sqlite, generic, databricks)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Verify compliance with Data Modelling SDK Constitution principles:

- **Commit Requirements**: ✅ Code MUST build successfully before commit. All commits MUST be GPG signed.
- **Code Quality & Security**: ✅ Plan includes security audit (`cargo audit`), formatting (`cargo fmt`), linting (`cargo clippy`) checks. Dependencies use latest stable versions.
- **Storage Abstraction**: ✅ N/A - CLI operates on files directly, no storage backend needed
- **Feature Flags**: ✅ CLI binary will be optional (behind `cli` feature flag) to avoid adding dependencies to core SDK
- **Testing Requirements**: ✅ Plan includes unit tests, integration tests, and CLI command execution tests
- **Import/Export Patterns**: ✅ Uses existing SDK importers/exporters following established patterns
- **Error Handling**: ✅ Uses structured error types (`thiserror` for CLI-specific errors, `anyhow` for convenience)

**No violations or exceptions identified.**

## Project Structure

### Documentation (this feature)

```text
specs/006-cli-wrapper/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   └── cli-api.md      # CLI command structure and options
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── cli/                 # New CLI module
│   ├── mod.rs          # CLI module entry point
│   ├── commands/       # Command implementations
│   │   ├── mod.rs
│   │   ├── import.rs   # Import command handlers
│   │   └── export.rs   # Export command handlers
│   ├── error.rs        # CLI-specific error types
│   ├── reference.rs    # External reference resolution
│   ├── validation.rs   # Schema validation helpers
│   └── output.rs       # Output formatting (pretty/compact)
├── [existing modules]  # All existing SDK modules unchanged

examples/
└── test_sql.rs         # Existing example (may be deprecated or kept for reference)

tests/
├── cli/                # New CLI tests
│   ├── mod.rs
│   ├── import_tests.rs
│   ├── export_tests.rs
│   └── integration_tests.rs
└── [existing tests]    # All existing tests unchanged

Cargo.toml              # Add [[bin]] section for CLI binary, add clap dependency
```

**Structure Decision**: Single binary CLI application. The CLI will be built as a separate binary (`data-modelling-cli`) that depends on the SDK library. This keeps the SDK library clean and allows the CLI to have its own dependencies (like `clap`) without affecting SDK users. The CLI module will be feature-gated behind a `cli` feature flag.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations identified.

## Phase Completion Status

### Phase 0: Research ✅ Complete

**Output**: `research.md`

All technical decisions resolved:
- CLI argument parsing: `clap` 4.x with derive API
- External reference resolution: `reqwest` for HTTP/HTTPS
- JAR extraction: `zip` crate
- Protobuf descriptor generation: external `protoc` binary
- UUID validation: `uuid` crate
- Error handling: `thiserror` for structured errors
- Output formatting: compact and pretty modes
- Schema validation: reuse existing SDK modules

### Phase 1: Design & Contracts ✅ Complete

**Outputs**:
- `data-model.md` - Data structures and relationships
- `contracts/cli-api.md` - CLI command structure and API contract
- `quickstart.md` - Usage examples and common workflows

**Agent Context Updated**: `.cursor/rules/specify-rules.mdc` updated with CLI technologies

### Phase 2: Task Breakdown

**Status**: Ready for `/speckit.tasks` command

The plan is complete and ready for task breakdown. All design artifacts have been created and technical decisions documented.
