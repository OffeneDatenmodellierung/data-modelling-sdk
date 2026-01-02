# Implementation Plan: Enhanced Data Flow Node and Relationship Metadata

**Branch**: `002-table-metadata` | **Date**: 2026-01-27 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-table-metadata/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Enhance Table and Relationship models to support comprehensive operational and governance metadata for Data Flow nodes and relationships. Metadata includes owner, SLA (inspired by ODCS servicelevels format but lightweight), contact details (structured object), infrastructure type (strict enumeration with 70+ types), and notes. All metadata fields must be optional, preserve backward compatibility, and use a lightweight format separate from ODCS (which is only for Data Models/tables). The implementation will extend existing Table and Relationship structs with new fields while maintaining compatibility with existing structures.

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: Rust 2024 edition (current project standard)
**Primary Dependencies**: serde, serde_json, serde_yaml, uuid, chrono (existing SDK dependencies)
**Storage**: N/A (metadata stored in Table and Relationship structs, persisted via existing storage backends)
**Testing**: cargo test with unit tests, integration tests, and doctests
**Target Platform**: Multi-platform (Native, WASM, API) - SDK must work across all targets
**Project Type**: Library/SDK (single project structure)
**Performance Goals**: Search/filter operations return results in under 1 second for data models with up to 10,000 nodes/relationships (SC-003)
**Constraints**: All metadata fields support text values up to 10,000 characters without performance degradation (SC-004), must maintain backward compatibility with existing nodes/relationships (SC-005), format must be lightweight and separate from ODCS
**Scale/Scope**: Support data models with up to 10,000 Data Flow nodes and relationships, metadata fields are optional and lightweight

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Verify compliance with Data Modelling SDK Constitution principles:

- ✅ **Commit Requirements**: Code MUST build successfully before commit. All commits MUST be GPG signed. *Compliance: Standard development workflow applies.*
- ✅ **Code Quality & Security**: Plan MUST include security audit, formatting, linting checks. Dependencies MUST use latest stable versions. *Compliance: No new dependencies required, using existing SDK dependencies. All code will pass cargo fmt, clippy, and audit.*
- ✅ **Storage Abstraction**: If adding storage operations, MUST use `StorageBackend` trait. MUST be async and feature-gated appropriately. *Compliance: No new storage operations - metadata stored in Table and Relationship structs, persisted via existing storage backends.*
- ✅ **Feature Flags**: If adding optional functionality, MUST be behind Cargo features with clear documentation. *Compliance: No new features required - metadata fields are core functionality.*
- ✅ **Testing Requirements**: Plan MUST include unit tests, integration tests, and doctests where applicable. *Compliance: Will add unit tests for new metadata fields, integration tests for Data Flow format import/export roundtrips, and doctests for new structs.*
- ✅ **Import/Export Patterns**: If adding format support, MUST follow importer/exporter trait patterns. *Compliance: Creating lightweight Data Flow format import/export (separate from ODCS), following established importer/exporter trait patterns.*
- ✅ **Error Handling**: MUST use structured error types (`thiserror` for library errors, `anyhow` for convenience where appropriate). *Compliance: Using existing error handling patterns, no new error types required.*

**Status**: ✅ All constitution principles satisfied. No violations or exceptions.

## Project Structure

### Documentation (this feature)

```text
specs/002-table-metadata/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
src/
├── models/
│   ├── table.rs          # Enhanced with new metadata fields for Data Flow nodes
│   ├── relationship.rs   # Enhanced with new metadata fields for Data Flow relationships
│   ├── enums.rs          # InfrastructureType enum added
│   └── mod.rs
├── import/
│   └── dataflow.rs       # New lightweight Data Flow format importer (separate from ODCS)
└── export/
    └── dataflow.rs       # New lightweight Data Flow format exporter (separate from ODCS)

tests/
├── models_tests.rs       # Unit tests for new metadata fields
├── import_tests.rs       # Integration tests for Data Flow format import
└── export_tests.rs       # Integration tests for Data Flow format export
```

**Structure Decision**: Single project structure (library/SDK). New metadata fields added to existing `Table` struct (for Data Flow nodes) and `Relationship` struct (for Data Flow relationships) in `src/models/`. New `InfrastructureType` enum added to `src/models/enums.rs`. New lightweight Data Flow format import/export modules created (separate from ODCS). Tests added to existing test files.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations - all constitution principles satisfied.

## Phase Completion Status

### Phase 0: Research ✅ Complete

- **research.md**: Updated with design decisions for Data Flow nodes/relationships metadata, lightweight format (separate from ODCS)
- All clarifications resolved
- Design decisions documented with rationale

### Phase 1: Design & Contracts ⚠️ Needs Update

- **data-model.md**: Needs update for Data Flow nodes/relationships (currently references ODCS tables)
- **contracts/**: Needs update for lightweight Data Flow format (currently references ODCS)
- **quickstart.md**: Needs update for Data Flow examples (currently references ODCS)
- **Agent context**: Will be updated after Phase 1 completion

**Note**: Previous Phase 1 artifacts were created for ODCS scope. They need to be updated to reflect the corrected scope: Data Flow nodes/relationships with lightweight format separate from ODCS.

### Phase 2: Task Breakdown

**Status**: Ready for `/speckit.tasks` command after Phase 1 artifacts are updated

## Important Scope Clarification

**CRITICAL**: This feature is for Data Flow nodes (Tables) and relationships, NOT for ODCS Data Contracts. ODCS format is ONLY for Data Models (tables). We are creating a lightweight, cut-down specification format for Data Flow separate from ODCS.

## Next Steps

1. Update Phase 1 artifacts (data-model.md, contracts/, quickstart.md) to reflect Data Flow scope
2. Run `/speckit.tasks` to generate detailed implementation tasks
3. Begin implementation following the updated data model and API contracts
