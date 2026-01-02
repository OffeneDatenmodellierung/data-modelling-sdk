# Tasks: WASM Module Parsing Function Exports

**Input**: Design documents from `/specs/001-wasm-exports/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are OPTIONAL - not explicitly requested in feature specification, so no test tasks included.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths shown below assume single project structure per plan.md

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Create WASM test directory structure at tests/wasm/
- [x] T002 [P] Verify wasm-bindgen dependencies are configured in Cargo.toml
- [x] T003 [P] Verify wasm feature flag is properly configured in Cargo.toml

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Create WASM module structure in src/lib.rs with #[cfg(all(target_arch = "wasm32", feature = "wasm"))] module
- [x] T005 [P] Implement error conversion utility function to convert ImportError to JsValue in src/lib.rs WASM module
- [x] T006 [P] Implement error conversion utility function to convert ExportError to JsValue in src/lib.rs WASM module
- [x] T007 [P] Implement JSON serialization helper function for ImportResult in src/lib.rs WASM module
- [x] T008 [P] Implement JSON deserialization helper function for workspace structures in src/lib.rs WASM module

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Parse ODCS YAML Files in Offline Mode (Priority: P1) 🎯 MVP

**Goal**: Enable users to parse ODCS YAML files in offline mode via WASM bindings, eliminating the need for JavaScript fallback parsers.

**Independent Test**: Load WASM module in browser, call parseOdcsYaml with valid ODCS 3.1.0 YAML content, verify parsed workspace structure matches expected data model with tables, relationships, and metadata.

### Implementation for User Story 1

- [x] T009 [US1] Implement parse_odcs_yaml WASM binding function in src/lib.rs WASM module that wraps ODCSImporter::import()
- [x] T010 [US1] Add error handling to parse_odcs_yaml function converting ImportError to JsValue in src/lib.rs WASM module
- [x] T011 [US1] Add JSON serialization of ImportResult in parse_odcs_yaml function in src/lib.rs WASM module
- [x] T012 [US1] Verify parse_odcs_yaml function is properly annotated with #[wasm_bindgen] in src/lib.rs WASM module

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently. Users can parse ODCS YAML files via WASM.

---

## Phase 4: User Story 2 - Export Data Models to ODCS YAML Format (Priority: P1)

**Goal**: Enable users to export their data models to ODCS YAML format from the web application in offline mode.

**Independent Test**: Create a data model structure in JavaScript, call exportToOdcsYaml via WASM, verify output YAML matches ODCS 3.1.0 format specification and contains all expected data model elements.

### Implementation for User Story 2

- [x] T013 [US2] Implement export_to_odcs_yaml WASM binding function in src/lib.rs WASM module that accepts workspace JSON string
- [x] T014 [US2] Add JSON deserialization of workspace structure in export_to_odcs_yaml function in src/lib.rs WASM module
- [x] T015 [US2] Add ODCSExporter::export_model() call in export_to_odcs_yaml function in src/lib.rs WASM module
- [x] T016 [US2] Add error handling to export_to_odcs_yaml function converting ExportError to JsValue in src/lib.rs WASM module
- [x] T017 [US2] Verify export_to_odcs_yaml function is properly annotated with #[wasm_bindgen] in src/lib.rs WASM module

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently. Users can parse and export ODCS YAML files via WASM.

---

## Phase 5: User Story 3 - Import Data Models from Multiple Formats (Priority: P2)

**Goal**: Enable users to import data models from SQL, AVRO, JSON Schema, and Protobuf formats when working offline.

**Independent Test**: Call each import function (SQL, AVRO, JSON Schema, Protobuf) with sample content in their respective formats, verify imported data model structures match expected table and column definitions.

### Implementation for User Story 3

- [x] T018 [P] [US3] Implement import_from_sql WASM binding function in src/lib.rs WASM module that wraps SQLImporter::parse()
- [x] T019 [P] [US3] Implement import_from_avro WASM binding function in src/lib.rs WASM module that wraps AvroImporter::import()
- [x] T020 [P] [US3] Implement import_from_json_schema WASM binding function in src/lib.rs WASM module that wraps JSONSchemaImporter::import()
- [x] T021 [P] [US3] Implement import_from_protobuf WASM binding function in src/lib.rs WASM module that wraps ProtobufImporter::import()
- [x] T022 [US3] Add error handling to all import functions converting ImportError to JsValue in src/lib.rs WASM module
- [x] T023 [US3] Add JSON serialization of ImportResult to all import functions in src/lib.rs WASM module
- [x] T024 [US3] Verify all import functions are properly annotated with #[wasm_bindgen] in src/lib.rs WASM module

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently. Users can import from multiple formats via WASM.

---

## Phase 6: User Story 4 - Export Data Models to Multiple Formats (Priority: P2)

**Goal**: Enable users to export their data models to SQL, AVRO, JSON Schema, and Protobuf formats when working offline.

**Independent Test**: Create a data model in JavaScript, call each export function (SQL, AVRO, JSON Schema, Protobuf), verify output matches respective format specifications and contains all expected data model elements.

### Implementation for User Story 4

- [x] T025 [P] [US4] Implement export_to_sql WASM binding function in src/lib.rs WASM module that accepts workspace JSON and dialect string
- [x] T026 [P] [US4] Implement export_to_avro WASM binding function in src/lib.rs WASM module that accepts workspace JSON string
- [x] T027 [P] [US4] Implement export_to_json_schema WASM binding function in src/lib.rs WASM module that accepts workspace JSON string
- [x] T028 [P] [US4] Implement export_to_protobuf WASM binding function in src/lib.rs WASM module that accepts workspace JSON string
- [x] T029 [US4] Add JSON deserialization of workspace structure to all export functions in src/lib.rs WASM module
- [x] T030 [US4] Add exporter calls (SQLExporter, AvroExporter, JSONSchemaExporter, ProtobufExporter) to respective functions in src/lib.rs WASM module
- [x] T031 [US4] Add error handling to all export functions converting ExportError to JsValue in src/lib.rs WASM module
- [x] T032 [US4] Verify all export functions are properly annotated with #[wasm_bindgen] in src/lib.rs WASM module

**Checkpoint**: All user stories should now be independently functional. Users can import and export to/from all supported formats via WASM.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T033 [P] Verify all WASM functions are documented with doc comments in src/lib.rs WASM module
- [ ] T034 [P] Build WASM module with wasm-pack to verify TypeScript definitions are generated correctly
- [ ] T035 [P] Verify generated TypeScript definitions in pkg/data_modelling_sdk.d.ts include all 10 functions
- [x] T036 [P] Update README.md with WASM usage examples for parsing and export functions
- [x] T037 Code cleanup and refactoring of WASM module in src/lib.rs
- [ ] T038 Verify all functions handle large inputs (up to 10MB) without performance degradation
- [x] T039 Run cargo fmt --all -- --check to ensure formatting compliance
- [x] T040 Run cargo clippy --all-targets --all-features -- -D warnings to ensure linting compliance
- [x] T041 Run cargo audit to ensure security compliance
- [ ] T042 Verify quickstart.md examples work with generated WASM module

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2)
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories, independently testable
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - No dependencies on other stories, independently testable
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - No dependencies on other stories, independently testable

### Within Each User Story

- Error handling utilities before function implementation
- JSON serialization/deserialization helpers before function implementation
- Core function implementation before error handling integration
- Function annotation verification after implementation

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, User Stories 1 and 2 can start in parallel (both P1)
- User Stories 3 and 4 can start in parallel after Foundational (both P2)
- All import function implementations in User Story 3 marked [P] can run in parallel
- All export function implementations in User Story 4 marked [P] can run in parallel
- All Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 3

```bash
# Launch all import function implementations together:
Task: "Implement import_from_sql WASM binding function in src/lib.rs WASM module"
Task: "Implement import_from_avro WASM binding function in src/lib.rs WASM module"
Task: "Implement import_from_json_schema WASM binding function in src/lib.rs WASM module"
Task: "Implement import_from_protobuf WASM binding function in src/lib.rs WASM module"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Parse ODCS YAML)
4. Complete Phase 4: User Story 2 (Export to ODCS YAML)
5. **STOP and VALIDATE**: Test User Stories 1 & 2 independently
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (Core parsing MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo (Complete ODCS workflow!)
4. Add User Story 3 → Test independently → Deploy/Demo (Multi-format import)
5. Add User Story 4 → Test independently → Deploy/Demo (Multi-format export)
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Parse ODCS YAML)
   - Developer B: User Story 2 (Export to ODCS YAML)
3. After P1 stories complete:
   - Developer A: User Story 3 (Import from multiple formats)
   - Developer B: User Story 4 (Export to multiple formats)
4. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- All WASM functions must be feature-gated with `#[cfg(all(target_arch = "wasm32", feature = "wasm"))]`
- All functions must return `Result<String, JsValue>` for consistent error handling
- JSON serialization/deserialization is used for complex data structures
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
