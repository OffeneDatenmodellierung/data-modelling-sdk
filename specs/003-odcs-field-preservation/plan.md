# Implementation Plan: Complete ODCS/ODCL Field Preservation & Universal Format Conversion + CADS/ODPS/Business Domain Support

**Branch**: `003-odcs-field-preservation` | **Date**: 2026-01-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-odcs-field-preservation/spec.md` + expanded requirements:
- Ensure full schema coverage for ODCS and ODCL
- Ensure coverage for parsing of other import/export types
- Offer a function to convert all import types to ODCS (as closest to native format)
- All must be available via WASM to JavaScript
- **NEW**: Full support for CADS (Compute Asset Description Specification) - import/export for AI/ML models, applications, pipelines
- **NEW**: Full support for ODPS (Open Data Product Standard) - import/export for Data Products
- **NEW**: Business Domain super schema - top-level schema for business domains with systems, CADS nodes, and ODCS nodes
- **NEW**: Enhanced tag support (Simple, Pair, List) across all schemas
- **NEW**: DataFlow format refactoring into Domain schema

## Summary

This feature addresses critical data loss issues in ODCS/ODCL parsing and extends format coverage across all import/export types. The implementation will:

1. **Fix Field Preservation**: Extend `ColumnData` struct to include `description`, `quality`, and `$ref` fields, ensuring 100% field preservation from ODCS/ODCL formats
2. **Schema Coverage Analysis**: Audit and ensure complete coverage of ODCS v3.1.0 and ODCL specification fields across all importers
3. **Universal Converter**: Create a unified conversion function that converts any import format (SQL, JSON Schema, AVRO, Protobuf, ODCL, Data Flow, **CADS**, **ODPS**) to ODCS v3.1.0 format
4. **Format Coverage**: Ensure all import formats (SQL, ODCS, ODCL, JSON Schema, AVRO, Protobuf, Data Flow, **CADS**, **ODPS**) have comprehensive field extraction and mapping
5. **WASM Bindings**: Expose all new functionality via WASM bindings for JavaScript consumption
6. **CADS Support**: Full import/export support for CADS v1.0 schema (AI/ML models, applications, pipelines, source/destination systems)
7. **ODPS Support**: Full import/export support for ODPS schema (Data Products linking to ODCS Tables)
8. **Business Domain Schema**: New top-level schema for business domains with systems, relationships, and contained CADS/ODCS nodes
9. **Enhanced Tag Support**: Tag enum supporting Simple, Pair, and List formats across all schemas
10. **DataFlow Refactoring**: Remove DataFlow format and refactor into Domain schema (Systems and SystemConnections)

## Technical Context

**Language/Version**: Rust 2024 edition
**Primary Dependencies**:
- `serde`, `serde_json`, `serde_yaml` - Serialization
- `jsonschema` (optional) - JSON Schema validation for ODCS/ODCL/ODPS/CADS schemas
- `wasm-bindgen` - WASM bindings (feature-gated)
- Existing import/export modules: `odcs`, `odcl`, `sql`, `json_schema`, `avro`, `protobuf`
- **NEW**: `cads` - CADS importer/exporter module
- **NEW**: `odps` - ODPS importer/exporter module
- **NEW**: `domain` - Business Domain schema module
- **NEW**: `tag` - Enhanced tag enum module
**Storage**: N/A (in-memory processing only)
**Testing**: `cargo test` with unit tests, integration tests, and doctests
**Target Platform**: Multi-platform (native Rust + WASM for JavaScript)
**Project Type**: Library/SDK
**Performance Goals**:
- Import/export operations should complete in <1s for typical files (<100 tables, <1000 columns)
- Universal converter should handle files up to 10MB efficiently
- Business Domain operations should handle domains with 100+ systems efficiently
- Tag parsing should be O(n) where n is number of tags
**Constraints**:
- Must maintain backward compatibility (all new fields optional)
- Must support all existing import/export formats
- WASM binary size should remain reasonable (<5MB)
- CADS and ODPS must preserve all specification fields
- Business Domain schema must support ERD-style connections between systems and CADS, and Crowsfeet notation for ODCS node relationships
- Enhanced tags must maintain backward compatibility with existing `Vec<String>` tags
**Scale/Scope**:
- Support for 9 import formats (SQL, ODCS, ODCL, JSON Schema, AVRO, Protobuf, Data Flow → Domain, **CADS**, **ODPS**)
- Full ODCS v3.1.0 schema coverage (2,928 lines of JSON schema)
- Full ODCL v1.2.1 schema coverage (2,046 lines of JSON schema)
- Full ODPS schema coverage (latest version from [official repository](https://github.com/bitol-io/open-data-product-standard/blob/main/schema/odps-json-schema-latest.json))
- Full CADS v1.0 schema coverage (from `schemas/cads.schema.json`)
- Universal converter supporting all formats
- Business Domain schema with systems, CADS nodes, ODCS nodes, and relationship types
- Enhanced tag support across all schemas

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Verify compliance with Data Modelling SDK Constitution principles:

- **Commit Requirements**: Code MUST build successfully before commit. All commits MUST be GPG signed. ✅
- **Code Quality & Security**: Plan MUST include security audit, formatting, linting checks. Dependencies MUST use latest stable versions. ✅
- **Storage Abstraction**: If adding storage operations, MUST use `StorageBackend` trait. MUST be async and feature-gated appropriately. ✅ (N/A - in-memory only)
- **Feature Flags**: If adding optional functionality, MUST be behind Cargo features with clear documentation. ✅ (jsonschema validation optional)
- **Testing Requirements**: Plan MUST include unit tests, integration tests, and doctests where applicable. ✅
- **Import/Export Patterns**: If adding format support, MUST follow importer/exporter trait patterns. ✅
- **Error Handling**: MUST use structured error types (`thiserror` for library errors, `anyhow` for convenience where appropriate). ✅

**Compliance Status**: ✅ All principles satisfied

## Project Structure

### Documentation (this feature)

```text
specs/003-odcs-field-preservation/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md         # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
├── schemas/             # JSON Schema files for validation
│   ├── odcs-json-schema-v3.1.0.json
│   ├── odcl-json-schema-1.2.1.json
│   ├── odps-json-schema-latest.json
│   └── cads.schema.json
├── test-fixtures/       # Test YAML/JSON files
│   ├── full-example.odcs.yaml
│   └── example.odcl.yaml
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── models/
│   ├── column.rs        # Column struct with description, quality, ref_path
│   ├── table.rs         # Table struct (enhanced with tags: Vec<Tag>)
│   ├── relationship.rs  # Relationship struct (enhanced with tags: Vec<Tag>)
│   ├── data_model.rs    # DataModel struct
│   ├── enums.rs         # InfrastructureType enum
│   ├── tag.rs           # NEW: Tag enum (Simple, Pair, List)
│   ├── domain.rs        # NEW: Business Domain models (Domain, System, SystemConnection, NodeConnection)
│   ├── cads.rs          # NEW: CADS model structs (AIModel, MLPipeline, Application, etc.)
│   └── odps.rs          # NEW: ODPS model structs (DataProduct, InputPort, OutputPort, etc.)
├── import/
│   ├── mod.rs           # ColumnData struct with description, quality, ref_path
│   ├── odcs.rs          # ODCS/ODCL importer (enhanced tag parsing)
│   ├── sql.rs           # SQL importer
│   ├── json_schema.rs   # JSON Schema importer (enhanced tag parsing)
│   ├── avro.rs          # AVRO importer (enhanced tag parsing)
│   ├── protobuf.rs      # Protobuf importer (enhanced tag parsing)
│   ├── cads.rs          # NEW: CADS importer
│   └── odps.rs          # NEW: ODPS importer
├── export/
│   ├── mod.rs           # ExportError, ExportResult
│   ├── odcs.rs          # ODCS exporter (enhanced tag serialization)
│   ├── odcl.rs          # ODCL exporter (enhanced tag serialization)
│   ├── sql.rs           # SQL exporter
│   ├── json_schema.rs   # JSON Schema exporter (enhanced tag serialization)
│   ├── avro.rs          # AVRO exporter (enhanced tag serialization)
│   ├── protobuf.rs      # Protobuf exporter (enhanced tag serialization)
│   ├── cads.rs          # NEW: CADS exporter
│   └── odps.rs          # NEW: ODPS exporter
├── convert/
│   ├── mod.rs           # Converter module exports
│   └── converter.rs     # Universal converter (all formats → ODCS)
└── lib.rs               # Public API and WASM bindings

tests/
├── import_tests.rs      # Import format tests
├── export_tests.rs      # Export format tests
├── integration_tests.rs # Round-trip tests
├── odcs_comprehensive_tests.rs # ODCS field preservation tests
├── tag_tests.rs         # NEW: Enhanced tag parsing/serialization tests
├── cads_tests.rs        # NEW: CADS import/export tests
├── odps_tests.rs        # NEW: ODPS import/export tests
└── domain_tests.rs      # NEW: Business Domain tests
```

## Phase 0: Research & Design Decisions

### Format Coverage Summary

**ODCS v3.1.0**:
- **Status**: Primary format, full schema coverage required
- **Schema**: `schemas/odcs-json-schema-v3.1.0.json` (2,928 lines)
- **Test Fixture**: `test-fixtures/full-example.odcs.yaml`
- **Coverage**: Root, schema, property level fields (see research.md)

**ODCL v1.2.1** (Last Supported Version):
- **Status**: Legacy format, full schema coverage required
- **Schema**: `schemas/odcl-json-schema-1.2.1.json` (2,046 lines)
- **Test Fixture**: `test-fixtures/example.odcl.yaml`
- **Coverage**: Root, model, field level fields (see research.md)

**ODPS** (Open Data Product Standard):
- **Status**: NEW - Full import/export support required
- **Schema**: `schemas/odps-json-schema-latest.json` (from [official repository](https://github.com/bitol-io/open-data-product-standard/blob/main/schema/odps-json-schema-latest.json))
- **Purpose**: Data Products linking to ODCS Tables
- **Storage**: Used as internal storage format for Data Products
- **Key Fields**: apiVersion, kind, id, name, version, status, domain, tenant, inputPorts, outputPorts, managementPorts, support, team, description
- **Relationships**: Products link to ODCS Tables via contractId references

**CADS v1.0** (Compute Asset Description Specification):
- **Status**: NEW - Full import/export support required
- **Schema**: `schemas/cads.schema.json`
- **Purpose**: AI/ML models, applications, pipelines, source/destination systems
- **Storage**: Used as internal storage format for defining these resources
- **Asset Kinds**: AIModel, MLPipeline, Application, ETLPipeline, SourceSystem, DestinationSystem
- **Key Fields**: apiVersion, kind, id, name, version, status, domain, description (purpose/usage/limitations), runtime, sla, team, risk, compliance, validationProfiles
- **Relationships**: CADS assets define transformations that happen to data

**Business Domain Schema**:
- **Status**: NEW - Top-level schema for business domains
- **Purpose**: Organize systems, CADS nodes, and ODCS nodes within business domains
- **Key Entities**:
  - **Domain**: Top-level container for a business domain
  - **System**: Physical system entities (Kafka, Cassandra, EKS, EC2, etc.) - inherits DataFlow node metadata (owner, SLA, contact_details, infrastructure_type, notes), has version field for sharing
  - **SystemConnection**: ERD-style connections between systems (within domain or cross-domain)
  - **CADSNode**: Reference to CADS asset (AI/ML model, application, pipeline) - can be shared from other domains
  - **ODCSNode**: Reference to ODCS Table - can be shared from other domains
  - **NodeConnection**: Crowsfeet notation relationships between ODCS nodes (one-to-one, one-to-many, zero-or-one, zero-or-many)
- **Relationship Types**:
  - **System ↔ System**: ERD-style (bidirectional, with connection metadata)
  - **System ↔ CADS**: ERD-style (system contains CADS nodes)
  - **ODCS ↔ ODCS**: Crowsfeet notation (cardinality: 1:1, 1:N, 0:1, 0:N)
- **Shared Node References**: Systems, CADS nodes, and ODCS nodes can be shared from other domains via `domain_id` + `node_id` + `node_version` (read-only core data, local metadata overrides in `custom_metadata` array)

**Enhanced Tag Support**:
- **Status**: NEW - Enhanced tag format across all schemas
- **Tag Enum**: `Simple(String)`, `Pair(String, String)`, `List(String, Vec<String>)`
- **Serialization**: String-based - Simple="finance", Pair="Environment:Dev", List="SecondaryDomains:[XXXXX, PPPP]"
- **Parsing**: Auto-detect format (no colon = Simple, single colon = Pair, colon + brackets = List)
- **Backward Compatibility**: Existing `Vec<String>` tags work without migration
- **Error Handling**: Malformed tags treated as Simple tags with warning logged

**DataFlow Refactoring**:
- **Status**: REMOVAL - DataFlow format will be completely removed
- **Migration**: DataFlow nodes → Systems, DataFlow relationships → SystemConnections
- **Removal**: Delete `src/import/dataflow.rs` and `src/export/dataflow.rs`
- **Migration Utility**: Provide utility to convert existing DataFlow YAML files to Domain schema format

### Design Decisions

1. **ColumnData Extension**: Add `description`, `quality`, and `ref_path` fields to `ColumnData` struct
2. **Column Extension**: Add `ref_path` field to `Column` struct
3. **Tag Enum**: Create `Tag` enum with `Simple`, `Pair`, `List` variants - replace `Vec<String>` with `Vec<Tag>` across all models
4. **Universal Converter**: Create `convert_to_odcs()` function supporting all formats including CADS and ODPS
5. **CADS Importer/Exporter**: New modules following existing import/export patterns
6. **ODPS Importer/Exporter**: New modules following existing import/export patterns
7. **Business Domain Models**: New `domain.rs` module with Domain, System, SystemConnection, NodeConnection structs
8. **System Entity**: Systems inherit DataFlow node metadata (owner, SLA, contact_details, infrastructure_type, notes), have version field for sharing
9. **Shared Node References**: Reference structure: `domain_id` + `node_id` + `node_version` with read-only core data, local metadata overrides
10. **Relationship Notation**: ERD-style for systems/CADS, Crowsfeet for ODCS nodes
11. **WASM Bindings**: Expose CADS, ODPS, Domain operations, and enhanced tag support via WASM
12. **Backward Compatibility**: All new fields optional, existing code continues to work
13. **DataFlow Removal**: Complete removal with migration utility

## Phase 1: Implementation Phases

### Phase 1: Foundation (COMPLETED)
- ✅ Extend `ColumnData` struct with `description`, `quality`, `ref_path`
- ✅ Extend `Column` struct with `ref_path`
- ✅ Update all Column instantiations
- ✅ Create `src/convert/` module structure

### Phase 2: ODCL/ODCS Field Preservation (IN PROGRESS)
- ✅ Update `parse_data_contract_field()` to extract description, quality, $ref
- ⏳ Update ODCS property parsing
- ⏳ Update Column to ColumnData mapping
- ⏳ Add comprehensive tests

### Phase 3: Enhanced Tag Support (NEW)
- ⏳ Create `src/models/tag.rs` - Tag enum (Simple, Pair, List)
- ⏳ Update `Table` struct to use `Vec<Tag>` instead of `Vec<String>`
- ⏳ Update `Relationship` struct to use `Vec<Tag>` instead of `Vec<String>`
- ⏳ Create tag parsing utility (auto-detect format, handle malformed tags)
- ⏳ Create tag serialization utility (convert Tag enum to string)
- ⏳ Update all importers to parse enhanced tags (ODCS, ODCL, ODPS, CADS, JSON Schema, AVRO, Protobuf)
- ⏳ Update all exporters to serialize enhanced tags
- ⏳ Update `filter_by_tags()` method to work with Tag enum
- ⏳ Add comprehensive tests for tag parsing/serialization
- ⏳ Add WASM bindings for tag operations

### Phase 4: CADS Support (NEW)
- ⏳ Create `src/models/cads.rs` - CADS model structs (AIModel, MLPipeline, Application, etc.)
- ⏳ Create `src/import/cads.rs` - CADS importer
- ⏳ Create `src/export/cads.rs` - CADS exporter
- ⏳ Add CADS to universal converter
- ⏳ Add WASM bindings for CADS import/export
- ⏳ Add comprehensive tests

### Phase 5: ODPS Support (NEW)
- ⏳ Create `src/models/odps.rs` - ODPS model structs (DataProduct, InputPort, OutputPort, etc.)
- ⏳ Create `src/import/odps.rs` - ODPS importer
- ⏳ Create `src/export/odps.rs` - ODPS exporter
- ⏳ Implement ODCS Table linking (contractId references)
- ⏳ Add ODPS to universal converter
- ⏳ Add WASM bindings for ODPS import/export
- ⏳ Add comprehensive tests

### Phase 6: Business Domain Schema (NEW)
- ⏳ Create `src/models/domain.rs` - Domain, System, SystemConnection, NodeConnection structs
- ⏳ Implement System entity with DataFlow metadata (owner, SLA, contact_details, infrastructure_type, notes, version)
- ⏳ Implement shared node reference structure (domain_id, node_id, node_version, custom_metadata)
- ⏳ Implement ERD-style system connections
- ⏳ Implement Crowsfeet notation for ODCS node relationships
- ⏳ Create domain import/export (YAML/JSON)
- ⏳ Add domain operations to DataModel
- ⏳ Add WASM bindings for domain operations
- ⏳ Add comprehensive tests

### Phase 7: DataFlow Refactoring & Removal (NEW)
- ⏳ Create migration utility to convert DataFlow YAML to Domain schema format
- ⏳ Update documentation to reflect Domain schema usage
- ⏳ Remove `src/import/dataflow.rs` module
- ⏳ Remove `src/export/dataflow.rs` module
- ⏳ Remove DataFlow-related WASM bindings
- ⏳ Update universal converter to remove DataFlow format
- ⏳ Add migration tests

### Phase 8: Universal Converter Enhancement
- ⏳ Add CADS → ODCS conversion
- ⏳ Add ODPS → ODCS conversion
- ⏳ Add Domain → ODCS conversion (extract ODCS nodes)
- ⏳ Update converter to handle all 9 formats (excluding DataFlow)
- ⏳ Add comprehensive tests

### Phase 9: WASM Bindings
- ⏳ Add CADS import/export WASM functions
- ⏳ Add ODPS import/export WASM functions
- ⏳ Add domain operations WASM functions
- ⏳ Add enhanced tag WASM functions
- ⏳ Update universal converter WASM binding
- ⏳ Add comprehensive JavaScript examples

### Phase 10: Polish & Documentation
- ⏳ Update CHANGELOG.md
- ⏳ Update README.md with new formats
- ⏳ Update LLM.txt with new modules
- ⏳ Add quickstart examples for CADS, ODPS, Domain, enhanced tags
- ⏳ Version bump (minor version)

## Complexity Tracking

### High Complexity Areas

1. **Tag Enum Migration**: Converting `Vec<String>` to `Vec<Tag>` across all models requires careful serialization/deserialization handling and backward compatibility
2. **CADS Schema Complexity**: CADS includes risk management, compliance, validation profiles - requires careful mapping to SDK models
3. **ODPS Relationships**: Linking Data Products to ODCS Tables requires reference resolution and validation
4. **Business Domain Relationships**: ERD-style vs Crowsfeet notation requires different relationship models
5. **Shared Node References**: Implementing read-only references with local metadata overrides requires careful design
6. **DataFlow Migration**: Converting existing DataFlow YAML files to Domain schema format requires migration utility
7. **Universal Converter**: Converting CADS/ODPS/Domain to ODCS requires semantic mapping, not just field copying

### Risk Mitigation

- Use JSON Schema validation for CADS/ODPS to ensure specification compliance
- Create comprehensive test fixtures for each format
- Implement incremental validation (parse → validate → convert)
- Document relationship mapping rules clearly
- Provide migration utility with validation and rollback capabilities
- Maintain backward compatibility for tags through auto-detection parsing

## Test Fixtures

**ODCS**: `test-fixtures/full-example.odcs.yaml` - Complete ODCS v3.1.0 example
**ODCL**: `test-fixtures/example.odcl.yaml` - Complete ODCL example
**ODPS**: Need to create test fixture from ODPS schema
**CADS**: Need to create test fixture from CADS schema
**Domain**: Need to create test fixture for business domain with systems and nodes
**Tags**: Need to create test fixtures with Simple, Pair, and List tag examples

## Success Criteria

- ✅ 100% field preservation for ODCS/ODCL (description, quality, $ref)
- ⏳ 100% schema coverage for ODPS (all fields from JSON schema)
- ⏳ 100% schema coverage for CADS (all fields from JSON schema)
- ⏳ Business Domain schema supports all relationship types
- ⏳ Universal converter supports all 9 formats (excluding DataFlow)
- ⏳ All functionality available via WASM
- ⏳ Enhanced tag format supported across all schemas with backward compatibility
- ⏳ DataFlow format successfully migrated to Domain schema
- ⏳ All tests pass with >90% coverage

## Dependencies

- JSON Schema files: `schemas/odcs-json-schema-v3.1.0.json`, `schemas/odcl-json-schema-1.2.1.json`, `schemas/odps-json-schema-latest.json`, `schemas/cads.schema.json`
- Test fixtures: Full examples for each format
- `jsonschema` crate (optional, feature-gated) for schema validation

## Future Enhancements (Out of Scope)

- **OpenAPI Specification Support**: Import/export and viewing of OpenAPI Specs following OpenAPI Version 3.1.1
- **BPMN Specification Support**: Import/export and viewing of BPMN XML Specs following BPMN XFD 2.0.2
  - Note: CADS schema already includes `bpmnModels` field for referencing BPMN process definitions, which will be integrated with future BPMN import/export functionality

## Notes

- CADS and ODPS are new formats - full specification compliance required
- Business Domain is a new top-level concept - requires careful design to avoid circular dependencies
- Relationship notation differences (ERD vs Crowsfeet) require separate relationship types
- ODPS Data Products must link to existing ODCS Tables - validation required
- Enhanced tags maintain backward compatibility through auto-detection parsing
- DataFlow format removal requires migration utility for existing users
