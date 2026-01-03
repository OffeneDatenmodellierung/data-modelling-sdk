# Feature Specification: Complete ODCS/ODCL Field Preservation

**Feature Branch**: `003-odcs-field-preservation`
**Created**: 2026-01-27
**Status**: Draft
**Input**: User description: "Fix ODCS/ODCL parsing to preserve all fields including description, quality arrays, and $ref references"
**Related Issue**: [#9](https://github.com/pixie79/data-modelling-sdk/issues/9)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Import ODCL YAML with Complete Field Preservation (Priority: P1)

As a data engineer, I want to import ODCL YAML files containing column descriptions, quality rules, and $ref references, so that all metadata is preserved and available for validation, documentation, and data quality checks.

**Why this priority**: This is a critical data integrity issue. Missing fields cause data loss, break compliance with ODCS/ODCL specifications, and require error-prone workarounds in frontend applications.

**Independent Test**: Import an ODCL YAML file containing columns with `description`, `quality` arrays, and `$ref` references. Verify that all fields are present in the parsed result and match the source YAML exactly.

**Acceptance Scenarios**:

1. **Given** an ODCL YAML file with a column containing a `description` field, **When** I call `parse_odcs_yaml()`, **Then** the returned column includes the `description` field with the exact value from the source YAML
2. **Given** an ODCL YAML file with a column containing a `quality` array with nested structures (e.g., `implementation.kwargs.value_set`), **When** I call `parse_odcs_yaml()`, **Then** the returned column includes the complete `quality` array with all nested structures preserved
3. **Given** an ODCL YAML file with a column containing a `$ref` reference (e.g., `$ref: '#/definitions/orderStatus'`), **When** I call `parse_odcs_yaml()`, **Then** the returned column includes the `$ref` field with the exact reference path
4. **Given** an ODCL YAML file with columns containing all three field types (description, quality, $ref), **When** I call `parse_odcs_yaml()`, **Then** all fields are preserved in the returned columns

---

### User Story 2 - Import ODCS v3.1.0 Format with Complete Field Preservation (Priority: P1)

As a data engineer, I want to import ODCS v3.1.0 YAML files with complete field preservation, so that all specification-compliant metadata is available for use.

**Why this priority**: ODCS v3.1.0 is the primary format and must support 100% field preservation to maintain specification compliance.

**Independent Test**: Import an ODCS v3.1.0 YAML file with columns containing description, quality, and $ref fields. Verify complete field preservation.

**Acceptance Scenarios**:

1. **Given** an ODCS v3.1.0 YAML file with column-level `description` fields, **When** I call `parse_odcs_yaml()`, **Then** all descriptions are preserved in the parsed result
2. **Given** an ODCS v3.1.0 YAML file with column-level `quality` arrays, **When** I call `parse_odcs_yaml()`, **Then** all quality rules with nested structures are preserved
3. **Given** an ODCS v3.1.0 YAML file with column-level `$ref` references, **When** I call `parse_odcs_yaml()`, **Then** all $ref paths are preserved

---

### User Story 3 - Round-Trip Import/Export Preserves All Fields (Priority: P2)

As a data engineer, I want to import an ODCL/ODCS file and export it back, so that no data is lost during the round-trip process.

**Why this priority**: Round-trip preservation ensures data integrity and validates that the SDK correctly handles all fields in both directions.

**Independent Test**: Import an ODCL/ODCS YAML file, then export it back to YAML. Compare the exported YAML with the original to verify no fields are lost.

**Acceptance Scenarios**:

1. **Given** an ODCL YAML file with complete column metadata, **When** I import it and export it back, **Then** the exported YAML contains all original fields (description, quality, $ref)
2. **Given** an ODCS v3.1.0 YAML file with complete column metadata, **When** I import it and export it back, **Then** the exported YAML contains all original fields

---

### Edge Cases

- What happens when a column has an empty `description` string? (Should preserve empty string, not omit field)
- What happens when a column has an empty `quality` array? (Should preserve empty array, not omit field)
- What happens when a column has multiple `quality` rules with deeply nested structures? (Should preserve all nesting levels)
- What happens when a `$ref` reference points to a non-existent definition? (Should preserve the reference path, validation can happen separately)
- What happens when `quality` contains custom engines (e.g., great-expectations) with complex `implementation.kwargs` structures? (Should preserve complete structure)
- How does system handle malformed `quality` arrays? (Should preserve what can be parsed, log errors for malformed parts)
- What happens when both table-level and column-level `quality` rules exist? (Should preserve both, column-level takes precedence for that column)
- What happens when a tag string contains multiple colons (e.g., "Key:Value1:Value2")? (Should treat as Simple tag, log warning)
- What happens when a List tag has unclosed brackets (e.g., "Key:[Value1, Value2")? (Should treat as Simple tag, log warning)
- What happens when a tag string contains whitespace around colons (e.g., "Key : Value")? (Should trim whitespace, parse as Pair)
- What happens when a List tag has empty values (e.g., "Key:[]")? (Should parse as List with empty Vec<String>)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: SDK MUST parse and preserve column-level `description` fields from ODCL format YAML files
- **FR-002**: SDK MUST parse and preserve column-level `description` fields from ODCS v3.1.0 format YAML files
- **FR-003**: SDK MUST parse and preserve column-level `quality` arrays from ODCL format YAML files, including all nested structures (e.g., `implementation.kwargs.value_set`)
- **FR-004**: SDK MUST parse and preserve column-level `quality` arrays from ODCS v3.1.0 format YAML files, including all nested structures
- **FR-005**: SDK MUST parse and preserve column-level `$ref` references from ODCL format YAML files
- **FR-006**: SDK MUST parse and preserve column-level `$ref` references from ODCS v3.1.0 format YAML files
- **FR-007**: SDK MUST preserve empty `description` strings (not omit the field when empty)
- **FR-008**: SDK MUST preserve empty `quality` arrays (not omit the field when empty)
- **FR-009**: SDK MUST preserve `$ref` references even when the referenced definition does not exist in the current document
- **FR-010**: SDK MUST preserve all nested structures within `quality` arrays, including custom engine configurations (e.g., great-expectations with `implementation.kwargs.value_set`)
- **FR-011**: SDK MUST return 100% of all fields present in source ODCL/ODCS YAML files in the parsed result
- **FR-012**: SDK MUST support round-trip import/export without data loss for description, quality, and $ref fields
- **FR-013**: SDK MUST support enhanced tag format across all schemas (ODCS, ODCL, ODPS, CADS, Domain): Simple tags (single word), Pair tags (key:value format), List tags (key:[value1, value2] format)
- **FR-014**: SDK MUST auto-detect tag format during parsing (no colon = Simple, single colon = Pair, colon + brackets = List)
- **FR-015**: SDK MUST preserve backward compatibility with existing simple string tags (no migration required)
- **FR-016**: SDK MUST handle malformed tags gracefully (treat as Simple tags, log warning, preserve original string)

### Key Entities *(include if feature involves data)*

- **Column**: Represents a field in a table/data contract. Must include `description` (string), `quality` (array of quality rule objects), and `$ref` (string reference path) fields
- **Quality Rule**: Represents a data quality check or validation rule. Contains type, engine, description, and implementation details with nested structures (e.g., `implementation.kwargs.value_set`)
- **ImportResult**: Contains parsed tables and columns. Must include all column fields (description, quality, $ref) in the returned structure
- **Tag**: Enhanced tag structure supporting three formats:
  - **Simple**: Single word tag (e.g., "finance", "sensitive")
  - **Pair**: Key-value pair tag (e.g., "Environment:Dev", "Status:Active")
  - **List**: Key with multiple values (e.g., "SecondaryDomains:[XXXXX, PPPP]", "Regions:[US, EU, APAC]")
  - Stored as `Tag` enum: `Simple(String)`, `Pair(String, String)`, `List(String, Vec<String>)`
  - Serialized as strings in YAML/JSON: Simple tags as plain strings, Pair tags as "Key:Value", List tags as "Key:[Value1, Value2]"
  - Supported across all schemas: ODCS, ODCL, ODPS, CADS, Domain (Systems, CADSNodes, ODCSNodes)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of column `description` fields from source ODCL/ODCS YAML files are preserved in parsed results (measured by comparing source YAML to parsed JSON output)
- **SC-002**: 100% of column `quality` arrays from source ODCL/ODCS YAML files are preserved in parsed results, including all nested structures (measured by deep comparison of quality rule objects)
- **SC-003**: 100% of column `$ref` references from source ODCL/ODCS YAML files are preserved in parsed results (measured by exact string match of reference paths)
- **SC-004**: Round-trip import/export preserves 100% of description, quality, and $ref fields (measured by comparing original YAML to exported YAML after import)
- **SC-005**: SDK successfully parses ODCL files with complex quality rules (e.g., great-expectations with nested `implementation.kwargs.value_set`) without data loss (measured by verifying all nested levels are present)
- **SC-006**: Frontend applications can access all column metadata (description, quality, $ref) without requiring workaround merge logic (measured by removing workaround code and verifying functionality)
- **SC-007**: Enhanced tag format (Simple, Pair, List) is supported across all schemas (ODCS, ODCL, ODPS, CADS, Domain) with 100% round-trip preservation (measured by comparing imported/exported tag arrays)
- **SC-008**: Existing simple string tags continue to work without migration (measured by importing files with simple tags and verifying they parse correctly)

## Assumptions

- ODCL and ODCS v3.1.0 formats are the primary targets for this fix
- The `Column` struct already has `description` and `quality` fields - the issue is in the mapping from parsed `Column` to `ImportResult.ColumnData`
- `$ref` references should be preserved as strings (JSON Schema reference paths) - resolution can happen separately
- Empty strings and empty arrays should be preserved (not omitted) to maintain field presence
- Quality rules may have arbitrary nested structures based on the engine type (e.g., great-expectations) - all nesting must be preserved
- The fix should maintain backward compatibility with existing code that uses `ImportResult`

## Dependencies

- Existing `Column` struct already supports `description` and `quality` fields
- ODCS importer's `parse_column()` function already extracts description and quality - needs to also extract `$ref`
- `ImportResult.ColumnData` struct needs to be extended to include `description`, `quality`, and `$ref` fields
- Mapping logic in ODCS importer needs to include all fields when converting `Column` to `ColumnData`

## Out of Scope

- Resolving `$ref` references to actual definitions (preservation only, not resolution)
- Validating that `$ref` paths are correct or exist
- Modifying export functionality (this is an import-only fix, though round-trip should work)
- Supporting additional ODCL/ODCS field types beyond description, quality, and $ref (those are the critical missing ones identified in issue #9)
  - Note: Full ODCS/ODCL specifications include many more fields (examples, format, pattern, classification, tags, customProperties, relationships, authoritativeDefinitions, etc.)
  - These additional fields are preserved in the `Column` struct's `odcl_metadata` HashMap but not exposed via `ColumnData`
  - Future phases can extend `ColumnData` to include more fields as needed
- Performance optimization (focus is on correctness and completeness)

## Future Enhancements

The following features are planned for future implementation but are out of scope for the current feature:

- **OpenAPI Specification Support**: Import/export and viewing of OpenAPI Specs following OpenAPI Version 3.1.1
- **BPMN Specification Support**: Import/export and viewing of BPMN XML Specs following BPMN XFD 2.0.2
  - Note: CADS schema already includes `bpmnModels` field for referencing BPMN process definitions, which will be integrated with future BPMN import/export functionality

## Expanded Scope (NEW)

### CADS (Compute Asset Description Specification) Support

- **Purpose**: Support for AI/ML models, applications, pipelines, and source/destination systems
- **Schema**: `schemas/cads.schema.json` (v1.0)
- **Asset Kinds**: AIModel, MLPipeline, Application, ETLPipeline, SourceSystem, DestinationSystem
- **Storage**: Used as internal storage format for defining these resources
- **Relationships**: CADS assets define transformations that happen to data

### ODPS (Open Data Product Standard) Support

- **Purpose**: Support for Data Products linking to ODCS Tables
- **Schema**: `schemas/odps-json-schema-latest.json` (from [official repository](https://github.com/bitol-io/open-data-product-standard/blob/main/schema/odps-json-schema-latest.json))
- **Storage**: Used as internal storage format for Data Products
- **Relationships**: Products link to ODCS Tables via contractId references in inputPorts/outputPorts

### Business Domain Schema

- **Purpose**: Top-level schema for organizing systems, CADS nodes, and ODCS nodes within business domains
- **Entities**: Domain, System, SystemConnection, CADSNode, ODCSNode, NodeConnection
- **System Entity**:
  - Physical infrastructure entities (Kafka, Cassandra, EKS, EC2, etc.)
  - Inherits DataFlow node metadata: `owner`, `sla`, `contact_details`, `infrastructure_type`, `notes`
  - Has `version: Option<String>` field (semantic version) - required when sharing, optional for local systems
  - Can be shared across domains via `domain_id`, `system_id`, `system_version` reference
- **Relationship Types**:
  - **System ↔ System**: ERD-style connections (bidirectional, with connection metadata)
  - **System ↔ CADS**: ERD-style connections (system contains CADS nodes)
  - **ODCS ↔ ODCS**: Crowsfeet notation (cardinality: 1:1, 1:N, 0:1, 0:N)
- **DataFlow Refactoring**: DataFlow format will be completely removed and refactored into Domain schema:
  - DataFlow nodes → Systems (physical infrastructure entities with metadata: owner, SLA, contact_details, infrastructure_type, notes)
  - DataFlow relationships → SystemConnections (ERD-style connections between systems)
  - **System Metadata**: Systems inherit all DataFlow node metadata fields as optional fields: `owner: Option<String>`, `sla: Option<Vec<SlaProperty>>`, `contact_details: Option<ContactDetails>`, `infrastructure_type: Option<InfrastructureType>`, `notes: Option<String>`
  - **Removal**: Delete `src/import/dataflow.rs` and `src/export/dataflow.rs` modules
  - **Migration**: Provide migration utility to convert existing DataFlow YAML files to Domain schema format
  - **WASM Bindings**: Remove DataFlow-related WASM bindings (`import_from_dataflow`, `export_to_dataflow`)
- **Shared Node References**: Systems, CADS nodes, and ODCS nodes can be shared from other domains:
  - **Systems**: Shared by `domain_id`, `system_id`, and `system_version` (Systems have `version: Option<String>` field - required when sharing, optional for local systems)
  - **CADS Nodes**: Shared by `domain_id`, `cads_node_id`, and `cads_node_version` (CADS assets have version field)
  - **ODCS Nodes**: Shared by `domain_id`, `odcs_node_id`, and `odcs_node_version` (ODCS Tables have version field)
  - Core node data is read-only (original nodes remain owned by their source business domain)
  - Local metadata overrides are allowed and stored in a separate `custom_metadata` array (domain-specific annotations, tags, or overrides)
  - Shared references prevent accidental modification of nodes owned by other domains

## Clarifications

### Session 2026-01-27

- Q: How should DataFlow nodes and relationships map to Domain schema entities? → A: DataFlow nodes become Systems (physical infrastructure entities with metadata), and DataFlow relationships become SystemConnections (ERD-style connections between systems)
- Q: How should shared nodes from other domains be referenced? → A: Shared nodes referenced by `domain_id` + `node_id` + `node_version` (read-only core data, but allow local metadata overrides stored in custom metadata array)
- Q: How should DataFlow format be handled during refactoring? → A: Remove DataFlow format completely - delete `src/import/dataflow.rs` and `src/export/dataflow.rs`, provide migration utility to convert DataFlow YAML to Domain schema format
- Q: What metadata fields should Systems have? → A: Systems inherit all DataFlow node metadata fields: `owner`, `sla`, `contact_details`, `infrastructure_type`, `notes` (all optional)
- Q: Should Systems have version fields for cross-domain sharing? → A: Systems have `version: Option<String>` field (semantic version) - required when sharing, optional for local systems
- Q: What data model structure should be used for tags supporting single words, pairs, and lists? → A: Use `Tag` enum with variants: `Simple(String)`, `Pair(String, String)`, `List(String, Vec<String>)` - store as `Vec<Tag>` instead of `Vec<String>`
- Q: Which schemas should support enhanced tag format (Simple, Pair, List)? → A: All schemas support enhanced tags: ODCS, ODCL, ODPS, CADS, Domain (Systems, CADSNodes, ODCSNodes)
- Q: How should tags be serialized in YAML/JSON formats? → A: Serialize as strings: Simple="finance", Pair="Environment:Dev", List="SecondaryDomains:[XXXXX, PPPP]" - parse on import, serialize on export
- Q: How should backward compatibility be handled for existing simple string tags? → A: Auto-detect format during parsing: no colon = Simple, single colon = Pair, colon + brackets = List. Existing simple tags work without migration
- Q: How should malformed tags be handled during parsing? → A: Treat malformed tags as Simple tags - log warning, preserve original string value, continue parsing

## Technical Context

- **SDK Version**: 1.2.0+
- **Affected Module**: `src/import/odcs.rs` - ODCSImporter
- **Affected Types**: `src/import/mod.rs` - ColumnData struct
- **Current Behavior**: `ColumnData` only includes `name`, `data_type`, `nullable`, `primary_key` fields
- **Root Cause**: Mapping from parsed `Column` struct to `ColumnData` omits description, quality, and $ref fields
- **Impact**: HIGH - Data loss, compliance issues, requires frontend workarounds
