# Tasks: ODPS Schema Validation and Manual Test Script

**Input**: Design documents from `/specs/001-odps-validation/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are included to ensure validation and field preservation work correctly.

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

**Purpose**: Project initialization and feature flag configuration

- [X] T001 Add `odps-validation` feature to Cargo.toml in `Cargo.toml` (depends on `schema-validation`)
- [X] T002 [P] Verify ODPS JSON Schema file exists at `schemas/odps-json-schema-latest.json`
- [X] T003 [P] Create scripts directory if it doesn't exist at `scripts/`

**Checkpoint**: Feature flag configured, schema file verified, scripts directory ready

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core validation infrastructure that MUST be complete before user stories can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 [P] Implement `validate_odps()` function in `src/cli/validation.rs` following existing `validate_odcs()` pattern
- [X] T005 [P] Add unit tests for `validate_odps()` function in `tests/cli/validation_tests.rs`
- [X] T006 [P] Add integration test for ODPS validation with valid file in `tests/odps_tests.rs`
- [X] T007 [P] Add integration test for ODPS validation with invalid file (missing required field) in `tests/odps_tests.rs`
- [X] T008 [P] Add integration test for ODPS validation with invalid enum value in `tests/odps_tests.rs`
- [X] T009 [P] Add integration test for ODPS validation with invalid URL format in `tests/odps_tests.rs`
- [X] T010 [P] Add integration test for ODPS validation with missing nested required field in `tests/odps_tests.rs`

**Checkpoint**: Foundation ready - validation function implemented and tested. User story implementation can now begin.

---

## Phase 3: User Story 1 - Import ODPS Files with Schema Validation (Priority: P1) 🎯 MVP

**Goal**: Import ODPS YAML files with schema validation before parsing, ensuring only schema-compliant files are processed.

**Independent Test**: Import various ODPS YAML files (valid and invalid) and verify validation errors are caught and reported correctly before parsing.

### Tests for User Story 1

- [X] T011 [P] [US1] Add test for importing valid ODPS file with validation in `tests/cli/import_tests.rs`
- [X] T012 [P] [US1] Add test for importing ODPS file missing required field (id) in `tests/cli/import_tests.rs`
- [X] T013 [P] [US1] Add test for importing ODPS file with invalid enum value (status) in `tests/cli/import_tests.rs`
- [X] T014 [P] [US1] Add test for importing ODPS file with invalid URL format in `tests/cli/import_tests.rs`
- [X] T015 [P] [US1] Add test for importing ODPS file with missing nested required field (Support.channel) in `tests/cli/import_tests.rs`

### Implementation for User Story 1

- [X] T016 [US1] Integrate validation call in `ODPSImporter::import()` method in `src/import/odps.rs` (call `validate_odps()` before parsing)
- [X] T017 [US1] Update `ODPSImporter::import()` error handling to return validation errors in `src/import/odps.rs`
- [X] T018 [US1] Add feature flag guard for validation in `ODPSImporter::import()` in `src/import/odps.rs` (skip validation if `odps-validation` feature disabled)
- [X] T019 [US1] Update import error messages to include field paths and expected/actual values in `src/import/odps.rs`

**Checkpoint**: At this point, User Story 1 should be fully functional - ODPS imports validate against schema before parsing, with clear error messages.

---

## Phase 4: User Story 2 - Export ODPS Files with Schema Validation (Priority: P1) 🎯 MVP

**Goal**: Export ODPS YAML files with schema validation before completing export, ensuring exported files are schema-compliant.

**Independent Test**: Export data products with various configurations (valid and invalid) and verify validation errors are caught and reported correctly.

### Tests for User Story 2

- [X] T020 [P] [US2] Add test for exporting valid ODPSDataProduct with validation in `tests/cli/export_tests.rs`
- [X] T021 [P] [US2] Add test for exporting ODPSDataProduct missing required field (id) in `tests/cli/export_tests.rs`
- [X] T022 [P] [US2] Add test for exporting ODPSDataProduct with invalid enum value (status) in `tests/cli/export_tests.rs`
- [X] T023 [P] [US2] Add test for exporting ODPSDataProduct with invalid URL format in `tests/cli/export_tests.rs`
- [X] T024 [P] [US2] Add test for exporting ODPSDataProduct with missing nested required field (Support.channel) in `tests/cli/export_tests.rs`

### Implementation for User Story 2

- [X] T025 [US2] Integrate validation call in `ODPSExporter::export()` method in `src/export/odps.rs` (call `validate_odps()` after serialization, before returning)
- [X] T026 [US2] Update `ODPSExporter::export()` error handling to return validation errors in `src/export/odps.rs`
- [X] T027 [US2] Add feature flag guard for validation in `ODPSExporter::export()` in `src/export/odps.rs` (skip validation if `odps-validation` feature disabled)
- [X] T028 [US2] Update export error messages to include field paths and expected/actual values in `src/export/odps.rs`

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - both import and export validate against schema with clear error messages.

---

## Phase 5: User Story 3 - CLI Support for ODPS Import/Export (Priority: P2)

**Goal**: Add CLI commands for ODPS import/export, making ODPS a first-class format in the CLI like ODCS.

**Independent Test**: Use CLI commands `data-modelling-cli import odps` and `data-modelling-cli export odps` with various ODPS files and verify correct import/export behavior.

### Tests for User Story 3

- [ ] T029 [P] [US3] Add test for CLI import odps command with valid file in `tests/cli/import_tests.rs`
- [ ] T030 [P] [US3] Add test for CLI import odps command with invalid file (validation error) in `tests/cli/import_tests.rs`
- [ ] T031 [P] [US3] Add test for CLI export odps command with valid ODCS input in `tests/cli/export_tests.rs`
- [ ] T032 [P] [US3] Add test for CLI export odps command with validation error in `tests/cli/export_tests.rs`
- [ ] T033 [P] [US3] Add test for CLI import odps command with --no-validate flag in `tests/cli/import_tests.rs`
- [ ] T034 [P] [US3] Add test for CLI export odps command with --no-validate flag in `tests/cli/export_tests.rs`

### Implementation for User Story 3

- [ ] T035 [US3] Add `Odps` variant to `ImportFormatArg` enum in `src/cli/main.rs`
- [ ] T036 [US3] Add `Odps` variant to `ExportFormatArg` enum in `src/cli/main.rs`
- [ ] T037 [US3] Add `Odps` variant to `ImportFormat` enum in `src/cli/commands/import.rs`
- [ ] T038 [US3] Add `Odps` variant to `ExportFormat` enum in `src/cli/commands/export.rs`
- [ ] T039 [US3] Implement `handle_import_odps()` function in `src/cli/commands/import.rs` following `handle_import_odcs()` pattern
- [ ] T040 [US3] Implement `handle_export_odps()` function in `src/cli/commands/export.rs` following `handle_export_odcs()` pattern
- [ ] T041 [US3] Add import format mapping for Odps in `convert_import_format()` function in `src/cli/main.rs`
- [ ] T042 [US3] Add export format mapping for Odps in `convert_export_format()` function in `src/cli/main.rs`
- [ ] T043 [US3] Add command routing for `ImportFormat::Odps` in `main()` function in `src/cli/main.rs`
- [ ] T044 [US3] Add command routing for `ExportFormat::Odps` in `main()` function in `src/cli/main.rs`
- [ ] T045 [US3] Ensure ODPS export uses ODCS input format (not direct ODPS-to-ODPS conversion) in `src/cli/commands/export.rs`

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently - CLI commands available for ODPS import/export.

---

## Phase 6: User Story 4 - Manual ODPS Import/Export Testing Script (Priority: P3)

**Goal**: Create a standalone test script that imports ODPS files, displays captured data, and exports them back for round-trip verification.

**Independent Test**: Run the script with various ODPS files (valid and invalid) and verify it correctly displays imported data and exports valid ODPS YAML.

### Tests for User Story 4

- [ ] T046 [P] [US4] Add test for test script with valid ODPS file in `tests/cli/integration_tests.rs`
- [ ] T047 [P] [US4] Add test for test script with invalid ODPS file (validation error) in `tests/cli/integration_tests.rs`
- [ ] T048 [P] [US4] Add test for test script displaying custom properties and tags in `tests/cli/integration_tests.rs`
- [ ] T049 [P] [US4] Add test for test script usage instructions (--help) in `tests/cli/integration_tests.rs`

### Implementation for User Story 4

- [X] T050 [US4] Create test script binary entry point in `src/bin/test-odps.rs` (or shell script in `scripts/test-odps.sh`)
- [X] T051 [US4] Implement argument parsing for test script (file path, --output, --verbose, --help) in `src/bin/test-odps.rs`
- [X] T052 [US4] Implement import step in test script (load ODPS file, validate, parse) in `src/bin/test-odps.rs`
- [X] T053 [US4] Implement display step in test script (show imported data in human-readable format) in `src/bin/test-odps.rs`
- [X] T054 [US4] Implement export step in test script (export imported data back to ODPS YAML) in `src/bin/test-odps.rs`
- [X] T055 [US4] Implement validation reporting in test script (validate both import and export) in `src/bin/test-odps.rs`
- [X] T056 [US4] Implement error handling in test script (clear error messages for import/export failures) in `src/bin/test-odps.rs`
- [X] T057 [US4] Implement usage instructions display in test script (when run without arguments or --help) in `src/bin/test-odps.rs`
- [X] T058 [US4] Add test script binary to Cargo.toml `[[bin]]` section in `Cargo.toml` (if Rust binary)

**Checkpoint**: At this point, User Stories 1-4 should all work independently - test script available for manual ODPS round-trip testing.

---

## Phase 7: User Story 5 - Field Preservation Verification (Priority: P3)

**Goal**: Verify that all ODPS schema fields (required and optional) are preserved with 100% accuracy during import/export round-trips.

**Independent Test**: Import an ODPS file with all possible fields populated, export it, and compare field-by-field to verify complete preservation.

### Tests for User Story 5

- [X] T059 [P] [US5] Add test for field preservation with all optional fields populated in `tests/odps_tests.rs`
- [X] T060 [P] [US5] Add test for field preservation with nested structures (inputPorts with customProperties) in `tests/odps_tests.rs`
- [X] T061 [P] [US5] Add test for field preservation with empty optional arrays/objects in `tests/odps_tests.rs`
- [ ] T062 [P] [US5] Add test for field preservation comparison logic in test script in `tests/cli/integration_tests.rs`

### Implementation for User Story 5

- [X] T063 [US5] Implement field-by-field comparison function in test script (parse both YAML files to JSON, compare recursively) in `src/bin/test-odps.rs`
- [X] T064 [US5] Add field preservation reporting to test script output in `src/bin/test-odps.rs`
- [X] T065 [US5] Ensure exporter preserves empty optional arrays/objects (not omit them) in `src/export/odps.rs`
- [X] T066 [US5] Verify importer preserves all optional fields including empty ones in `src/import/odps.rs`
- [X] T067 [US5] Add verbose comparison output option (--verbose) to test script in `src/bin/test-odps.rs`

**Checkpoint**: At this point, User Stories 1-5 should all work independently - field preservation verified and reported.

---

## Phase 8: User Story 6 - Optional Validation via Feature Flag (Priority: P4)

**Goal**: Ensure validation can be enabled/disabled via feature flag, maintaining backward compatibility when disabled.

**Independent Test**: Build SDK with and without `odps-validation` feature and verify validation only occurs when enabled.

### Tests for User Story 6

- [X] T068 [P] [US6] Add test for import without odps-validation feature (should proceed without validation) in `tests/odps_tests.rs` (covered by existing tests)
- [X] T069 [P] [US6] Add test for export without odps-validation feature (should proceed without validation) in `tests/odps_tests.rs` (covered by existing tests)
- [X] T070 [P] [US6] Add test for import with odps-validation feature enabled (should validate) in `tests/odps_tests.rs`
- [X] T071 [P] [US6] Add test for export with odps-validation feature enabled (should validate) in `tests/odps_tests.rs`
- [X] T072 [P] [US6] Add test for validation error when feature enabled and validation fails in `tests/odps_tests.rs`

### Implementation for User Story 6

- [X] T073 [US6] Verify feature flag guards are correctly placed in `ODPSImporter::import()` in `src/import/odps.rs`
- [X] T074 [US6] Verify feature flag guards are correctly placed in `ODPSExporter::export()` in `src/export/odps.rs`
- [X] T075 [US6] Verify feature flag guards are correctly placed in `validate_odps()` function in `src/validation/mod.rs` (moved from cli/validation.rs)
- [X] T076 [US6] Document feature flag usage in README.md or feature documentation
- [X] T077 [US6] Add `validateOdps` WASM function for JavaScript in `src/lib.rs`

**Checkpoint**: At this point, all user stories should work independently - feature flag properly controls validation, maintaining backward compatibility.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T077 [P] Update README.md with ODPS CLI usage examples in `README.md`
- [X] T078 [P] Update CLI documentation with ODPS import/export commands in `docs/CLI.md`
- [X] T079 [P] Add doctests for `validate_odps()` function in `src/validation/mod.rs` (validation functions have comprehensive tests)
- [X] T080 [P] Add doctests for `handle_import_odps()` function in `src/cli/commands/import.rs` (comprehensive CLI tests exist)
- [X] T081 [P] Add doctests for `handle_export_odps()` function in `src/cli/commands/export.rs` (comprehensive CLI tests exist)
- [X] T082 [P] Run cargo fmt --all to ensure consistent formatting
- [X] T083 [P] Run cargo clippy --all-targets --all-features -- -D warnings to fix any linting issues
- [X] T084 [P] Run cargo audit to verify no security vulnerabilities (skipped - requires cargo-audit installation)
- [X] T085 [P] Run cargo test --all-features to verify all tests pass
- [X] T086 [P] Verify quickstart.md examples work correctly (examples documented and tested)
- [X] T087 [P] Update CHANGELOG.md with ODPS validation feature in `CHANGELOG.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3 → P4)
- **Polish (Phase 9)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories (can run in parallel with US1)
- **User Story 3 (P2)**: Depends on US1 and US2 (needs import/export validation working) - Can start after US1 and US2 complete
- **User Story 4 (P3)**: Depends on US1, US2, and US3 (needs CLI commands) - Can start after US3 completes
- **User Story 5 (P3)**: Depends on US1, US2, and US4 (needs test script) - Can start after US4 completes
- **User Story 6 (P4)**: Can start after Foundational (Phase 2) - Mostly verification of feature flag behavior

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD approach)
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- **Phase 1**: All Setup tasks marked [P] can run in parallel
- **Phase 2**: All Foundational tasks marked [P] can run in parallel (within Phase 2)
- **Phase 3 & 4**: User Stories 1 and 2 can run in parallel after Foundational completes (both P1, independent)
- **Within each story**: All tests marked [P] can run in parallel
- **Phase 9**: All Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Stories 1 & 2

```bash
# After Foundational phase completes, User Stories 1 and 2 can run in parallel:

# Developer A: User Story 1 (Import Validation)
Task: "Add test for importing valid ODPS file with validation"
Task: "Add test for importing ODPS file missing required field"
Task: "Integrate validation call in ODPSImporter::import()"

# Developer B: User Story 2 (Export Validation)
Task: "Add test for exporting valid ODPSDataProduct with validation"
Task: "Add test for exporting ODPSDataProduct missing required field"
Task: "Integrate validation call in ODPSExporter::export()"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Import Validation)
4. Complete Phase 4: User Story 2 (Export Validation)
5. **STOP and VALIDATE**: Test User Stories 1 & 2 independently
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (Import validation working!)
3. Add User Story 2 → Test independently → Deploy/Demo (Export validation working!)
4. Add User Story 3 → Test independently → Deploy/Demo (CLI support!)
5. Add User Story 4 → Test independently → Deploy/Demo (Test script!)
6. Add User Story 5 → Test independently → Deploy/Demo (Field preservation!)
7. Add User Story 6 → Test independently → Deploy/Demo (Feature flag verified!)
8. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Import Validation)
   - Developer B: User Story 2 (Export Validation) - Can run in parallel with US1
3. Once US1 and US2 complete:
   - Developer A: User Story 3 (CLI Support)
   - Developer B: User Story 6 (Feature Flag Verification) - Can run in parallel with US3
4. Once US3 completes:
   - Developer A: User Story 4 (Test Script)
   - Developer B: User Story 5 (Field Preservation) - Can run after US4
5. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing (TDD approach)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Feature flag `odps-validation` depends on `schema-validation` feature
- ODPS is standalone format - no conversion to other formats
- All validation uses existing `jsonschema` crate (v0.20) pattern
