# Tasks: Complete ODCS/ODCL Field Preservation & Universal Format Conversion + CADS/ODPS/Business Domain Support

**Input**: Design documents from `/specs/003-odcs-field-preservation/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are included to ensure field preservation and round-trip functionality.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths shown below assume single project structure

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Create `src/convert/` module directory structure
- [X] T002 [P] Add `jsonschema` dependency to `Cargo.toml` with optional `schema-validation` feature gate
- [X] T003 [P] Update `src/lib.rs` to include `convert` module (when created)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data structures that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 [US1] [US2] Extend `ColumnData` struct in `src/import/mod.rs` with `description: Option<String>`, `quality: Option<Vec<HashMap<String, serde_json::Value>>>`, and `ref_path: Option<String>` fields
- [X] T005 [US1] [US2] Add `ref_path: Option<String>` field to `Column` struct in `src/models/column.rs` with serde attributes `#[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]`
- [X] T006 [US1] [US2] Update `Column::new()` constructor in `src/models/column.rs` to initialize `ref_path: None`
- [X] T007 [US1] [US2] Update all existing `Column` struct instantiations in codebase to include `ref_path: None` field (if using struct literal syntax)

**Checkpoint**: Foundation ready - data structures support field preservation. User story implementation can now begin.

---

## Phase 3: User Story 1 - Import ODCL YAML with Complete Field Preservation (Priority: P1) 🎯 MVP

**Goal**: Import ODCL YAML files containing column descriptions, quality rules, and $ref references with 100% field preservation.

**Independent Test**: Import an ODCL YAML file containing columns with `description`, `quality` arrays, and `$ref` references. Verify that all fields are present in the parsed result and match the source YAML exactly.

### Implementation for User Story 1

- [X] T008 [US1] Update `parse_data_contract_field()` function in `src/import/odcs.rs` to extract `description` field from ODCL field definition and populate Column.description
- [X] T009 [US1] Update `parse_data_contract_field()` function in `src/import/odcs.rs` to extract `quality` array from ODCL field definition and populate Column.quality with all nested structures preserved
- [X] T010 [US1] Update `parse_data_contract_field()` function in `src/import/odcs.rs` to extract `$ref` reference from ODCL field definition and populate Column.ref_path
- [X] T011 [US1] Update Column to ColumnData mapping in `src/import/odcs.rs` to include `description`, `quality`, and `ref_path` fields when converting parsed Column to ColumnData
- [X] T012 [US1] Add test in `tests/import_tests.rs` to verify ODCL YAML import preserves description field
- [X] T013 [US1] Add test in `tests/import_tests.rs` to verify ODCL YAML import preserves quality array with nested structures
- [X] T014 [US1] Add test in `tests/import_tests.rs` to verify ODCL YAML import preserves $ref references
- [X] T015 [US1] Add test in `tests/import_tests.rs` to verify ODCL YAML import preserves all three field types together

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently. ODCL YAML files with description, quality, and $ref fields are preserved.

---

## Phase 4: User Story 2 - Import ODCS v3.1.0 Format with Complete Field Preservation (Priority: P1)

**Goal**: Import ODCS v3.1.0 YAML files with complete field preservation, so that all specification-compliant metadata is available for use.

**Independent Test**: Import an ODCS v3.1.0 YAML file with columns containing description, quality, and $ref fields. Verify complete field preservation.

### Implementation for User Story 2

- [X] T016 [US2] Update ODCS property parsing in `src/import/odcs.rs` to extract `description` field from property definition
- [X] T017 [US2] Update ODCS property parsing in `src/import/odcs.rs` to extract `quality` array from property definition with all nested structures preserved
- [X] T018 [US2] Update ODCS property parsing in `src/import/odcs.rs` to extract `$ref` reference from property definition
- [X] T019 [US2] Update Column to ColumnData mapping in `src/import/odcs.rs` for ODCS format to include `description`, `quality`, and `ref_path` fields
- [X] T020 [US2] Add test in `tests/odcs_comprehensive_tests.rs` to verify ODCS v3.1.0 YAML import preserves description fields
- [X] T021 [US2] Add test in `tests/odcs_comprehensive_tests.rs` to verify ODCS v3.1.0 YAML import preserves quality arrays with nested structures
- [X] T022 [US2] Add test in `tests/odcs_comprehensive_tests.rs` to verify ODCS v3.1.0 YAML import preserves $ref references

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently. Both ODCL and ODCS v3.1.0 formats preserve description, quality, and $ref fields.

---

## Phase 5: User Story 3 - Round-Trip Import/Export Preserves All Fields (Priority: P2)

**Goal**: Import an ODCL/ODCS file and export it back, so that no data is lost during the round-trip process.

**Independent Test**: Import an ODCL/ODCS YAML file, then export it back to YAML. Compare the exported YAML with the original to verify no fields are lost.

### Implementation for User Story 3

- [X] T023 [US3] Update ODCL exporter in `src/export/odcl.rs` to serialize `description` field from Column to YAML
- [X] T024 [US3] Update ODCL exporter in `src/export/odcl.rs` to serialize `quality` array from Column to YAML with all nested structures preserved
- [X] T025 [US3] Update ODCL exporter in `src/export/odcl.rs` to serialize `$ref` field from Column to YAML
- [X] T026 [US3] Update ODCS exporter in `src/export/odcs.rs` to serialize `description` field from Column to YAML
- [X] T027 [US3] Update ODCS exporter in `src/export/odcs.rs` to serialize `quality` array from Column to YAML with all nested structures preserved
- [X] T028 [US3] Update ODCS exporter in `src/export/odcs.rs` to serialize `$ref` field from Column to YAML
- [X] T029 [US3] Add test in `tests/integration_tests.rs` to verify ODCL round-trip preserves all fields (description, quality, $ref)
- [X] T030 [US3] Add test in `tests/integration_tests.rs` to verify ODCS v3.1.0 round-trip preserves all fields (description, quality, $ref)

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently. Round-trip import/export preserves all fields.

---

## Phase 6: Enhanced Tag Support (NEW)

**Goal**: Support enhanced tag format (Simple, Pair, List) across all schemas with backward compatibility.

**Independent Test**: Import YAML files with Simple tags ("finance"), Pair tags ("Environment:Dev"), and List tags ("SecondaryDomains:[XXXXX, PPPP]"). Verify all formats parse correctly and serialize back correctly.

### Implementation for Enhanced Tag Support

- [X] T031 [P] Create `src/models/tag.rs` with Tag enum: `Simple(String)`, `Pair(String, String)`, `List(String, Vec<String>)`
- [X] T032 [P] Implement `FromStr` trait for Tag enum in `src/models/tag.rs` with auto-detection parsing (no colon = Simple, single colon = Pair, colon + brackets = List)
- [X] T033 [P] Implement `Display` trait for Tag enum in `src/models/tag.rs` for serialization (Simple="finance", Pair="Environment:Dev", List="SecondaryDomains:[XXXXX, PPPP]")
- [X] T034 Update `Table` struct in `src/models/table.rs` to use `Vec<Tag>` instead of `Vec<String>` for tags field
- [X] T035 Update `Relationship` struct in `src/models/relationship.rs` to use `Vec<Tag>` instead of `Vec<String>` for tags field (if it has tags)
- [X] T036 Update `filter_by_tags()` method in `src/models/data_model.rs` to work with Tag enum
- [X] T037 Update ODCS importer in `src/import/odcs.rs` to parse enhanced tags using Tag::from_str()
- [X] T038 Update ODCL importer in `src/import/odcs.rs` to parse enhanced tags using Tag::from_str()
- [X] T039 [P] Update JSON Schema importer in `src/import/json_schema.rs` to parse enhanced tags
- [X] T040 [P] Update AVRO importer in `src/import/avro.rs` to parse enhanced tags
- [X] T041 [P] Update Protobuf importer in `src/import/protobuf.rs` to parse enhanced tags
- [X] T042 Update ODCS exporter in `src/export/odcs.rs` to serialize enhanced tags using Tag::to_string()
- [X] T043 Update ODCL exporter in `src/export/odcl.rs` to serialize enhanced tags using Tag::to_string()
- [X] T044 [P] Update JSON Schema exporter in `src/export/json_schema.rs` to serialize enhanced tags
- [X] T045 [P] Update AVRO exporter in `src/export/avro.rs` to serialize enhanced tags
- [X] T046 [P] Update Protobuf exporter in `src/export/protobuf.rs` to serialize enhanced tags
- [X] T047 Add test in `tests/tag_tests.rs` to verify Simple tag parsing and serialization
- [X] T048 Add test in `tests/tag_tests.rs` to verify Pair tag parsing and serialization
- [X] T049 Add test in `tests/tag_tests.rs` to verify List tag parsing and serialization
- [X] T050 Add test in `tests/tag_tests.rs` to verify malformed tags are treated as Simple tags with warning
- [X] T051 Add test in `tests/tag_tests.rs` to verify backward compatibility with existing simple string tags

**Checkpoint**: Enhanced tag support is complete. All schemas support Simple, Pair, and List tag formats with backward compatibility.

---

## Phase 7: CADS Support (NEW)

**Goal**: Full import/export support for CADS v1.0 schema (AI/ML models, applications, pipelines, source/destination systems).

**Independent Test**: Import a CADS YAML file with all asset kinds (AIModel, MLPipeline, Application, etc.). Verify all fields are preserved. Export back to CADS format and verify round-trip preservation.

### Implementation for CADS Support

- [X] T052 [P] Create `src/models/cads.rs` with CADS model structs: `CADSAsset`, `CADSRuntime`, `CADSSLA`, `CADSRisk`, `CADSCompliance`, etc.
- [X] T053 [P] Implement CADS asset kinds enum in `src/models/cads.rs`: `AIModel`, `MLPipeline`, `Application`, `ETLPipeline`, `SourceSystem`, `DestinationSystem`
- [X] T054 Create `src/import/cads.rs` with `CADSImporter` struct following existing importer patterns
- [X] T055 Implement `import()` method in `src/import/cads.rs` to parse CADS YAML files
- [X] T056 Implement parsing for CADS root-level fields (apiVersion, kind, id, name, version, status, domain, tags) in `src/import/cads.rs`
- [X] T057 Implement parsing for CADS description object (purpose, usage, limitations, externalLinks) in `src/import/cads.rs`
- [X] T058 Implement parsing for CADS runtime, SLA, pricing, team, risk, compliance, validationProfiles in `src/import/cads.rs`
- [X] T059 Create `src/export/cads.rs` with `CADSExporter` struct following existing exporter patterns
- [X] T060 Implement `export()` method in `src/export/cads.rs` to serialize CADS assets to YAML
- [X] T061 Update universal converter in `src/convert/converter.rs` to support CADS → ODCS conversion
- [X] T062 Add test in `tests/cads_tests.rs` to verify CADS import preserves all fields
- [X] T063 Add test in `tests/cads_tests.rs` to verify CADS export generates valid CADS YAML
- [X] T064 Add test in `tests/cads_tests.rs` to verify CADS round-trip preservation
- [X] T065 Add test in `tests/cads_tests.rs` to verify CADS → ODCS conversion

**Checkpoint**: CADS support is complete. CADS assets can be imported, exported, and converted to ODCS format.

---

## Phase 8: ODPS Support (NEW)

**Goal**: Full import/export support for ODPS schema (Data Products linking to ODCS Tables).

**Independent Test**: Import an ODPS YAML file with DataProduct, InputPorts, OutputPorts. Verify all fields are preserved and contractId references are validated. Export back to ODPS format and verify round-trip preservation.

### Implementation for ODPS Support

- [X] T066 [P] Create `src/models/odps.rs` with ODPS model structs: `DataProduct`, `InputPort`, `OutputPort`, `ManagementPort`, `Support`, `Team`, etc.
- [X] T067 Create `src/import/odps.rs` with `ODPSImporter` struct following existing importer patterns
- [X] T068 Implement `import()` method in `src/import/odps.rs` to parse ODPS YAML files
- [X] T069 Implement parsing for ODPS root-level fields (apiVersion, kind, id, name, version, status, domain, tenant) in `src/import/odps.rs`
- [X] T070 Implement parsing for ODPS inputPorts and outputPorts with contractId references in `src/import/odps.rs`
- [X] T071 Implement ODCS Table linking validation in `src/import/odps.rs` (contractId must reference existing ODCS Table ID)
- [X] T072 Create `src/export/odps.rs` with `ODPSExporter` struct following existing exporter patterns
- [X] T073 Implement `export()` method in `src/export/odps.rs` to serialize DataProducts to YAML
- [X] T074 Update universal converter in `src/convert/converter.rs` to support ODPS → ODCS conversion
- [X] T075 Add test in `tests/odps_tests.rs` to verify ODPS import preserves all fields
- [X] T076 Add test in `tests/odps_tests.rs` to verify ODPS export generates valid ODPS YAML
- [X] T077 Add test in `tests/odps_tests.rs` to verify ODPS round-trip preservation
- [X] T078 Add test in `tests/odps_tests.rs` to verify ODPS contractId validation
- [X] T079 Add test in `tests/odps_tests.rs` to verify ODPS → ODCS conversion

**Checkpoint**: ODPS support is complete. Data Products can be imported, exported, and converted to ODCS format with contractId validation.

---

## Phase 9: Business Domain Schema (NEW)

**Goal**: New top-level schema for business domains with systems, CADS nodes, ODCS nodes, and relationship types (ERD-style and Crowsfeet notation).

**Independent Test**: Create a Domain with Systems, CADSNodes, ODCSNodes, and connections. Import/export Domain YAML. Verify shared node references work correctly.

### Implementation for Business Domain Schema

- [X] T080 [P] Create `src/models/domain.rs` with Domain struct (id, name, description, systems, cads_nodes, odcs_nodes, system_connections, node_connections)
- [X] T081 [P] Create System struct in `src/models/domain.rs` with DataFlow metadata (owner, sla, contact_details, infrastructure_type, notes, version)
- [X] T082 [P] Create SystemConnection struct in `src/models/domain.rs` for ERD-style connections (source_system_id, target_system_id, connection_type, bidirectional, metadata)
- [X] T083 [P] Create CADSNode struct in `src/models/domain.rs` with shared reference support (system_id, cads_asset_id, domain_id, node_id, node_version, custom_metadata)
- [X] T084 [P] Create ODCSNode struct in `src/models/domain.rs` with shared reference support (system_id, table_id, domain_id, node_id, node_version, custom_metadata)
- [X] T085 [P] Create NodeConnection struct in `src/models/domain.rs` for Crowsfeet notation (source_node_id, target_node_id, cardinality, relationship_type)
- [X] T086 [P] Create Cardinality enum in `src/models/domain.rs`: `OneToOne`, `OneToMany`, `ZeroOrOne`, `ZeroOrMany`
- [X] T087 Implement domain import/export functions in `src/models/domain.rs` (from_yaml, to_yaml)
- [X] T088 Add domain operations to DataModel in `src/models/data_model.rs` (add_system, add_cads_node, add_odcs_node, etc.)
- [X] T089 Add test in `tests/domain_tests.rs` to verify Domain creation and serialization
- [X] T090 Add test in `tests/domain_tests.rs` to verify System creation with DataFlow metadata
- [X] T091 Add test in `tests/domain_tests.rs` to verify SystemConnection ERD-style connections
- [X] T092 Add test in `tests/domain_tests.rs` to verify NodeConnection Crowsfeet notation
- [X] T093 Add test in `tests/domain_tests.rs` to verify shared node references (domain_id, node_id, node_version)
- [X] T094 Add test in `tests/domain_tests.rs` to verify local metadata overrides in custom_metadata array

**Checkpoint**: Business Domain schema is complete. Domains can be created, imported, exported, and support all relationship types.

---

## Phase 10: DataFlow Refactoring & Removal (NEW)

**Goal**: Remove DataFlow format completely and migrate existing DataFlow YAML files to Domain schema format.

**Independent Test**: Convert a DataFlow YAML file to Domain schema format. Verify DataFlow nodes become Systems and DataFlow relationships become SystemConnections.

### Implementation for DataFlow Refactoring

- [X] T095 Create migration utility function in `src/convert/migrate_dataflow.rs` to convert DataFlow YAML to Domain schema format
- [X] T096 Implement DataFlow node → System conversion in `src/convert/migrate_dataflow.rs` (preserve all metadata: owner, SLA, contact_details, infrastructure_type, notes)
- [X] T097 Implement DataFlow relationship → SystemConnection conversion in `src/convert/migrate_dataflow.rs`
- [X] T098 Update documentation in `README.md` to reflect Domain schema usage instead of DataFlow format
- [X] T099 Remove `src/import/dataflow.rs` module
- [X] T100 Remove `src/export/dataflow.rs` module
- [X] T101 Remove DataFlow-related WASM bindings from `src/lib.rs` (`import_from_dataflow`, `export_to_dataflow`)
- [X] T102 Update universal converter in `src/convert/converter.rs` to remove DataFlow format support
- [X] T103 Add test in `tests/integration_tests.rs` to verify DataFlow → Domain migration utility works correctly
- [X] T104 Add test in `tests/integration_tests.rs` to verify migrated Domain preserves all DataFlow metadata

**Checkpoint**: DataFlow format is removed. Migration utility successfully converts existing DataFlow files to Domain schema format.

---

## Phase 11: Universal Converter Enhancement

**Goal**: Universal converter supporting all formats (SQL, ODCS, ODCL, JSON Schema, AVRO, Protobuf, CADS, ODPS, Domain) → ODCS conversion.

**Independent Test**: Convert files from each format to ODCS YAML. Verify all formats convert correctly and preserve relevant metadata.

### Implementation for Universal Converter Enhancement

- [X] T105 Update `convert_to_odcs()` function in `src/convert/converter.rs` to support CADS format detection and conversion
- [X] T106 Update `convert_to_odcs()` function in `src/convert/converter.rs` to support ODPS format detection and conversion
- [X] T107 Update `convert_to_odcs()` function in `src/convert/converter.rs` to support Domain format detection and conversion (extract ODCS nodes)
- [X] T108 Remove DataFlow format support from `convert_to_odcs()` function in `src/convert/converter.rs`
- [X] T109 Add test in `tests/integration_tests.rs` to verify CADS → ODCS conversion
- [X] T110 Add test in `tests/integration_tests.rs` to verify ODPS → ODCS conversion
- [X] T111 Add test in `tests/integration_tests.rs` to verify Domain → ODCS conversion (extract ODCS nodes)
- [X] T112 Add test in `tests/integration_tests.rs` to verify all 9 formats (excluding DataFlow) convert to ODCS correctly

**Checkpoint**: Universal converter supports all formats (SQL, ODCS, ODCL, JSON Schema, AVRO, Protobuf, CADS, ODPS, Domain) → ODCS conversion.

---

## Phase 12: WASM Bindings

**Goal**: Expose all new functionality via WASM bindings for JavaScript consumption.

**Independent Test**: Build WASM package and verify all new functions are accessible from JavaScript.

### Implementation for WASM Bindings

- [X] T113 [P] Add WASM bindings for CADS import/export in `src/lib.rs` (`import_from_cads`, `export_to_cads`)
- [X] T114 [P] Add WASM bindings for ODPS import/export in `src/lib.rs` (`import_from_odps`, `export_to_odps`)
- [X] T115 [P] Add WASM bindings for domain operations in `src/lib.rs` (`create_domain`, `add_system`, `add_cads_node`, `add_odcs_node`, etc.)
- [X] T116 [P] Add WASM bindings for enhanced tag operations in `src/lib.rs` (`parse_tag`, `serialize_tag`, `filter_by_tags`)
- [X] T117 Update universal converter WASM binding in `src/lib.rs` to include CADS, ODPS, Domain formats
- [X] T118 Add JavaScript examples in `examples/wasm/` for CADS import/export
- [X] T119 Add JavaScript examples in `examples/wasm/` for ODPS import/export
- [X] T120 Add JavaScript examples in `examples/wasm/` for domain operations
- [X] T121 Add JavaScript examples in `examples/wasm/` for enhanced tag operations
- [X] T122 Add test in `tests/wasm_tests.rs` to verify CADS WASM bindings work correctly
- [X] T123 Add test in `tests/wasm_tests.rs` to verify ODPS WASM bindings work correctly
- [X] T124 Add test in `tests/wasm_tests.rs` to verify domain WASM bindings work correctly
- [X] T125 Add test in `tests/wasm_tests.rs` to verify enhanced tag WASM bindings work correctly

**Checkpoint**: All new functionality is available via WASM bindings. JavaScript examples demonstrate usage.

---

## Phase 13: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and final polish

- [X] T126 [P] Update `CHANGELOG.md` with all new features (ODCS/ODCL field preservation, CADS, ODPS, Domain schema, enhanced tags)
- [X] T127 [P] Update `README.md` with new format support (CADS, ODPS, Domain schema)
- [X] T128 [P] Update `LLM.txt` with new modules (tag.rs, cads.rs, odps.rs, domain.rs)
- [X] T129 [P] Add quickstart examples in `specs/003-odcs-field-preservation/quickstart.md` for CADS import/export
- [X] T130 [P] Add quickstart examples in `specs/003-odcs-field-preservation/quickstart.md` for ODPS import/export
- [X] T131 [P] Add quickstart examples in `specs/003-odcs-field-preservation/quickstart.md` for Domain schema usage
- [X] T132 [P] Add quickstart examples in `specs/003-odcs-field-preservation/quickstart.md` for enhanced tag usage
- [X] T133 Run `cargo fmt --all -- --check` to ensure code formatting
- [X] T134 Run `cargo clippy --all-targets --all-features -- -D warnings` to ensure code quality
- [X] T135 Run `cargo test --all-features` to ensure all tests pass
- [X] T136 Run `cargo audit` to ensure no security vulnerabilities (warnings only - unmaintained dependencies, no security vulnerabilities)
- [X] T137 Version bump (minor version) in `Cargo.toml` and `src/lib.rs`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can proceed sequentially in priority order (P1 → P1 → P2)
- **Enhanced Features (Phase 6-10)**: Can proceed after user stories or in parallel if team capacity allows
- **Universal Converter (Phase 11)**: Depends on CADS, ODPS, Domain support (Phase 7-9)
- **WASM Bindings (Phase 12)**: Depends on all feature implementations (Phase 3-11)
- **Polish (Phase 13)**: Depends on all desired features being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories, can run in parallel with US1
- **User Story 3 (P2)**: Depends on User Stories 1 and 2 (needs import/export functionality)

### Within Each User Story

- Core parsing/export logic before tests
- Tests verify functionality independently
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- User Stories 1 and 2 can run in parallel after Foundational phase
- Enhanced Tag Support tasks marked [P] can run in parallel
- CADS, ODPS, Domain model creation tasks marked [P] can run in parallel
- WASM binding tasks marked [P] can run in parallel
- Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# User Story 1 tasks can be worked on sequentially:
# T008 → T009 → T010 → T011 → T012-T015 (tests)
```

---

## Parallel Example: Enhanced Tag Support

```bash
# These tasks can run in parallel:
Task: "Create src/models/tag.rs with Tag enum"
Task: "Implement FromStr trait for Tag enum"
Task: "Implement Display trait for Tag enum"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (ODCL field preservation)
4. Complete Phase 4: User Story 2 (ODCS field preservation)
5. **STOP and VALIDATE**: Test User Stories 1 and 2 independently
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo
4. Add User Story 3 → Test independently → Deploy/Demo
5. Add Enhanced Tag Support → Test independently → Deploy/Demo
6. Add CADS Support → Test independently → Deploy/Demo
7. Add ODPS Support → Test independently → Deploy/Demo
8. Add Domain Schema → Test independently → Deploy/Demo
9. Add DataFlow Migration → Test independently → Deploy/Demo
10. Add Universal Converter → Test independently → Deploy/Demo
11. Add WASM Bindings → Test independently → Deploy/Demo
12. Polish → Final release

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 2
3. Once User Stories 1 & 2 are done:
   - Developer A: User Story 3
   - Developer B: Enhanced Tag Support
   - Developer C: CADS Support
4. Continue with parallel feature development
5. Final integration and WASM bindings

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing (if TDD approach)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- Enhanced tag support maintains backward compatibility with existing `Vec<String>` tags
- DataFlow format removal requires migration utility for existing users
