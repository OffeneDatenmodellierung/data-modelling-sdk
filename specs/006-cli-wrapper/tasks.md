# Tasks: CLI Wrapper for Data Modelling SDK

**Input**: Design documents from `/specs/006-cli-wrapper/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅, quickstart.md ✅

**Tests**: Tests are included per Constitution requirements (all features MUST include appropriate test coverage).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths shown below assume single project structure per plan.md

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependencies, and basic structure

- [X] T001 Add CLI dependencies to Cargo.toml: clap 4.x (with derive feature), zip crate (for JAR extraction)
- [X] T002 [P] Add `cli` feature flag to Cargo.toml (optional, default false) to gate CLI binary dependencies
- [X] T003 [P] Add [[bin]] section to Cargo.toml for data-modelling-cli binary with path src/cli/main.rs
- [X] T004 Create src/cli/ module directory structure
- [X] T005 [P] Create src/cli/mod.rs module entry point
- [X] T006 [P] Create src/cli/commands/ directory structure
- [X] T007 [P] Create src/cli/commands/mod.rs
- [X] T008 [P] Create src/cli/error.rs for CLI-specific error types
- [X] T009 [P] Create src/cli/reference.rs for external reference resolution
- [X] T010 [P] Create src/cli/validation.rs for schema validation helpers
- [X] T011 [P] Create src/cli/output.rs for output formatting (pretty/compact)
- [X] T012 Create tests/cli/ directory structure
- [X] T013 [P] Create tests/cli/mod.rs
- [X] T014 [P] Create tests/cli/import_tests.rs (8 tests implemented)
- [X] T015 [P] Create tests/cli/export_tests.rs (5 tests implemented)
- [X] T016 [P] Create tests/cli/integration_tests.rs (1 test implemented)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T017 [US1-US9] Implement CliError enum in src/cli/error.rs using thiserror with variants: FileNotFound, FileReadError, FileWriteError, InvalidUuid, MultipleTablesWithUuid, ProtocNotFound, ProtocError, NetworkError, ReferenceResolutionError, ValidationError, ImportError, ExportError, InvalidArgument
- [X] T018 [US1-US9] Implement InputSource enum in src/cli/commands/import.rs with variants: File(PathBuf), Stdin, String(String)
- [X] T019 [US1-US9] Implement ImportArgs struct in src/cli/commands/import.rs with clap derive attributes for all import command arguments
- [X] T020 [US1-US9] Implement ExportArgs struct in src/cli/commands/export.rs with clap derive attributes for all export command arguments
- [X] T021 [US1-US9] Implement top-level Cli struct in src/cli/main.rs using clap Parser with Commands enum (Import, Export)
- [X] T022 [US4] Implement resolve_local_reference() function in src/cli/reference.rs to resolve file paths relative to source file directory
- [X] T023 [US4] Implement resolve_http_reference() blocking function in src/cli/reference.rs to fetch HTTP/HTTPS URLs with timeout
- [X] T024 [US4] Implement resolve_reference() function in src/cli/reference.rs that handles both local and HTTP references
- [X] T025 [US5] Implement validate_odcs() function in src/cli/validation.rs using SDK validation modules
- [X] T026 [US5] Implement validate_openapi() function in src/cli/validation.rs using SDK validation modules
- [X] T027 [US5] Implement validate_protobuf() function in src/cli/validation.rs for proto syntax validation
- [X] T028 [US5] Implement validate_avro() function in src/cli/validation.rs for AVRO specification validation
- [X] T029 [US5] Implement validate_json_schema() function in src/cli/validation.rs for JSON Schema validation
- [X] T030 [US1-US9] Implement format_compact_output() function in src/cli/output.rs for compact table/column display
- [X] T031 [US1-US9] Implement format_pretty_output() function in src/cli/output.rs for detailed table/column display with mappings
- [X] T032 [US1-US9] Implement format_type_mappings() function in src/cli/output.rs to display format-to-ODCS type mappings
- [X] T033 [US1-US9] Implement main() function in src/cli/main.rs with command routing to import/export handlers

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Import SQL Schema with Dialect Selection (Priority: P1)

**Purpose**: Extend existing test_sql.rs functionality into full CLI command

### Tests for User Story 1

- [ ] T034 [P] [US1] Add unit test test_cli_import_sql_from_file() in tests/cli/import_tests.rs for SQL file import
- [ ] T035 [P] [US1] Add unit test test_cli_import_sql_from_stdin() in tests/cli/import_tests.rs for stdin import
- [ ] T036 [P] [US1] Add unit test test_cli_import_sql_from_string() in tests/cli/import_tests.rs for command-line SQL string
- [ ] T037 [P] [US1] Add unit test test_cli_import_sql_with_views() in tests/cli/import_tests.rs for CREATE VIEW statements
- [ ] T038 [P] [US1] Add unit test test_cli_import_sql_with_materialized_views() in tests/cli/import_tests.rs for CREATE MATERIALIZED VIEW statements
- [ ] T039 [P] [US1] Add unit test test_cli_import_sql_pretty_output() in tests/cli/import_tests.rs for --pretty flag
- [ ] T040 [P] [US1] Add integration test test_cli_sql_import_all_dialects() in tests/cli/integration_tests.rs for all SQL dialects

### Implementation for User Story 1

- [X] T041 [US1] Implement handle_import_sql() function in src/cli/commands/import.rs to process SQL import command
- [X] T042 [US1] Implement load_input() function in src/cli/commands/import.rs to handle file/stdin/string input sources
- [X] T043 [US1] Integrate SQLImporter::parse() call in handle_import_sql() with dialect parameter
- [X] T044 [US1] Implement display_import_result() function via format_compact_output/format_pretty_output in src/cli/output.rs
- [X] T045 [US1] Add SQL import command handler routing in src/cli/main.rs

**Checkpoint**: User Story 1 complete - SQL import working with all dialects and input methods

---

## Phase 4: User Story 2 - Import Schema Formats (AVRO, JSON Schema, Protobuf, OpenAPI) with Mappings Display (Priority: P1)

**Purpose**: Core import functionality for all schema formats with mappings display

### Tests for User Story 2

- [ ] T046 [P] [US2] Add unit test test_cli_import_avro() in tests/cli/import_tests.rs for AVRO import
- [ ] T047 [P] [US2] Add unit test test_cli_import_avro_with_uuid_override() in tests/cli/import_tests.rs for UUID override
- [ ] T048 [P] [US2] Add unit test test_cli_import_json_schema() in tests/cli/import_tests.rs for JSON Schema import
- [ ] T049 [P] [US2] Add unit test test_cli_import_protobuf() in tests/cli/import_tests.rs for Protobuf import
- [ ] T050 [P] [US2] Add unit test test_cli_import_openapi() in tests/cli/import_tests.rs for OpenAPI import
- [ ] T051 [P] [US2] Add unit test test_cli_import_multiple_tables_with_uuid_error() in tests/cli/import_tests.rs for UUID override error with multiple tables
- [ ] T052 [P] [US2] Add unit test test_cli_import_display_mappings() in tests/cli/import_tests.rs for type mapping display

### Implementation for User Story 2

- [X] T053 [US2] Implement handle_import_avro() function in src/cli/commands/import.rs
- [X] T054 [US2] Implement handle_import_json_schema() function in src/cli/commands/import.rs
- [X] T055 [US2] Implement handle_import_protobuf() function in src/cli/commands/import.rs
- [X] T056 [US2] Implement handle_import_openapi() function in src/cli/commands/import.rs (feature-gated)
- [X] T057 [US2] Implement apply_uuid_override() function in src/cli/commands/import.rs to override table UUID (with single-table validation)
- [X] T058 [US2] Implement collect_type_mappings() function in src/cli/output.rs to extract format-to-ODCS type mappings from import results
- [X] T059 [US2] Integrate AvroImporter, JSONSchemaImporter, ProtobufImporter, OpenAPIImporter calls in respective handlers
- [X] T060 [US2] Add format import command handler routing in src/cli/main.rs

**Checkpoint**: User Story 2 complete - All schema formats importable with mappings display and UUID override

---

## Phase 5: User Story 3 - Import ODCS Schemas with Validation (Priority: P1)

**Purpose**: ODCS import with validation support

### Tests for User Story 3

- [ ] T061 [P] [US3] Add unit test test_cli_import_odcs_valid() in tests/cli/import_tests.rs for valid ODCS import
- [ ] T062 [P] [US3] Add unit test test_cli_import_odcs_invalid() in tests/cli/import_tests.rs for invalid ODCS with validation errors
- [ ] T063 [P] [US3] Add unit test test_cli_import_odcs_yaml_syntax_error() in tests/cli/import_tests.rs for YAML syntax errors

### Implementation for User Story 3

- [X] T064 [US3] Implement handle_import_odcs() function in src/cli/commands/import.rs
- [X] T065 [US3] Integrate ODCS validation before import in handle_import_odcs() using validate_odcs()
- [X] T066 [US3] Integrate ODCSImporter call in handle_import_odcs()
- [X] T067 [US3] Add ODCS import command handler routing in src/cli/main.rs

**Checkpoint**: User Story 3 complete - ODCS import with validation working

---

## Phase 6: User Story 4 - Resolve External References in Schemas (Priority: P2)

**Purpose**: Automatic external reference resolution for local files and HTTP/HTTPS URLs

### Tests for User Story 4

- [ ] T068 [P] [US4] Add unit test test_resolve_local_reference() in tests/cli/import_tests.rs for local file reference resolution
- [ ] T069 [P] [US4] Add unit test test_resolve_http_reference() in tests/cli/import_tests.rs for HTTP URL reference resolution
- [ ] T070 [P] [US4] Add unit test test_resolve_missing_reference_error() in tests/cli/import_tests.rs for missing reference error handling
- [ ] T071 [P] [US4] Add unit test test_resolve_authenticated_url_error() in tests/cli/import_tests.rs for authenticated URL error

### Implementation for User Story 4

- [ ] T072 [US4] Implement resolve_all_references() function in src/cli/reference.rs to resolve all references in a schema
- [ ] T073 [US4] Integrate reference resolution into import handlers (AVRO, JSON Schema, Protobuf, OpenAPI) before import
- [ ] T074 [US4] Implement reference caching in resolve_all_references() to avoid duplicate fetches
- [ ] T075 [US4] Add --no-resolve-references flag handling in ImportArgs and skip resolution when flag is set

**Checkpoint**: User Story 4 complete - External references automatically resolved

---

## Phase 7: User Story 5 - Validate All Formats Before Import (Priority: P2)

**Purpose**: Schema validation before import for all formats

### Tests for User Story 5

- [ ] T076 [P] [US5] Add unit test test_validate_before_import_odcs() in tests/cli/import_tests.rs for ODCS validation
- [ ] T077 [P] [US5] Add unit test test_validate_before_import_openapi() in tests/cli/import_tests.rs for OpenAPI validation
- [ ] T078 [P] [US5] Add unit test test_validate_before_import_protobuf() in tests/cli/import_tests.rs for Protobuf validation
- [ ] T079 [P] [US5] Add unit test test_validate_before_import_avro() in tests/cli/import_tests.rs for AVRO validation
- [ ] T080 [P] [US5] Add unit test test_validate_before_import_json_schema() in tests/cli/import_tests.rs for JSON Schema validation
- [ ] T081 [P] [US5] Add unit test test_skip_validation_flag() in tests/cli/import_tests.rs for --no-validate flag

### Implementation for User Story 5

- [ ] T082 [US5] Integrate validation calls into all import handlers before import (using --no-validate flag to skip)
- [ ] T083 [US5] Implement display_validation_errors() function in src/cli/output.rs to format validation errors clearly
- [ ] T084 [US5] Add --no-validate flag handling in ImportArgs and skip validation when flag is set

**Checkpoint**: User Story 5 complete - All formats validated before import

---

## Phase 8: User Story 6 - Export ODCS Schemas to YAML Files (Priority: P2)

**Purpose**: Export to ODCS YAML format with .odcs.yaml extension

### Tests for User Story 6

- [ ] T085 [P] [US6] Add unit test test_cli_export_odcs_single_table() in tests/cli/export_tests.rs for single table export
- [ ] T086 [P] [US6] Add unit test test_cli_export_odcs_multiple_tables() in tests/cli/export_tests.rs for multiple tables export
- [ ] T087 [P] [US6] Add unit test test_cli_export_odcs_file_overwrite_prompt() in tests/cli/export_tests.rs for overwrite handling
- [ ] T088 [P] [US6] Add unit test test_cli_export_odcs_force_flag() in tests/cli/export_tests.rs for --force flag

### Implementation for User Story 6

- [X] T089 [US6] Implement handle_export_odcs() function in src/cli/commands/export.rs
- [X] T090 [US6] Implement check_file_overwrite() function in src/cli/commands/export.rs to handle file overwrite scenarios
- [X] T091 [US6] Implement load_workspace_from_input() function in src/cli/commands/export.rs to load ODCS YAML or JSON workspace
- [X] T092 [US6] Integrate ODCSExporter::export() call in handle_export_odcs()
- [X] T093 [US6] Implement write_export_output() function in src/cli/commands/export.rs to write exported content to file
- [X] T094 [US6] Add ODCS export command handler routing in src/cli/main.rs

**Checkpoint**: User Story 6 complete - ODCS export working with file overwrite handling

---

## Phase 9: User Story 7 - Export to AVRO, JSON Schema, and Protobuf Formats (Priority: P2)

**Purpose**: Export to multiple formats with validation logic preservation

### Tests for User Story 7

- [ ] T095 [P] [US7] Add unit test test_cli_export_avro() in tests/cli/export_tests.rs for AVRO export
- [ ] T096 [P] [US7] Add unit test test_cli_export_json_schema() in tests/cli/export_tests.rs for JSON Schema export
- [ ] T097 [P] [US7] Add unit test test_cli_export_protobuf() in tests/cli/export_tests.rs for Protobuf export
- [ ] T098 [P] [US7] Add unit test test_cli_export_preserves_validation_logic() in tests/cli/export_tests.rs for validation logic preservation

### Implementation for User Story 7

- [X] T099 [US7] Implement handle_export_avro() function in src/cli/commands/export.rs
- [X] T100 [US7] Implement handle_export_json_schema() function in src/cli/commands/export.rs
- [X] T101 [US7] Implement handle_export_protobuf() function in src/cli/commands/export.rs
- [X] T102 [US7] Integrate AvroExporter, JSONSchemaExporter, ProtobufExporter calls in respective handlers
- [ ] T103 [US7] Implement collect_export_warnings() function in src/cli/commands/export.rs to track unsupported features (deferred - can be added later)
- [X] T104 [US7] Add format export command handler routing in src/cli/main.rs

**Checkpoint**: User Story 7 complete - All export formats working with validation logic preservation

---

## Phase 10: User Story 8 - Import Protobuf from JAR Files (Priority: P3)

**Purpose**: Extract and import Protobuf schemas from JAR archives

### Tests for User Story 8

- [ ] T105 [P] [US8] Add unit test test_cli_import_protobuf_from_jar() in tests/cli/import_tests.rs for JAR import
- [ ] T106 [P] [US8] Add unit test test_cli_import_protobuf_jar_message_type_filter() in tests/cli/import_tests.rs for message type filtering
- [ ] T107 [P] [US8] Add unit test test_cli_import_protobuf_jar_no_proto_files() in tests/cli/import_tests.rs for JAR with no proto files
- [ ] T108 [P] [US8] Add unit test test_cli_import_protobuf_jar_malformed_proto() in tests/cli/import_tests.rs for malformed proto files

### Implementation for User Story 8

- [X] T109 [US8] Implement extract_proto_from_jar() function in src/cli/commands/import.rs to extract .proto files from JAR (integrated in handle_import_protobuf_from_jar)
- [X] T110 [US8] Implement merge_proto_files() function in src/cli/commands/import.rs to merge multiple proto files from JAR (integrated in handle_import_protobuf_from_jar)
- [ ] T111 [US8] Implement filter_proto_by_message_type() function in src/cli/commands/import.rs to filter proto files by message type (deferred - basic JAR extraction working)
- [X] T112 [US8] Integrate JAR extraction into handle_import_protobuf() when --jar flag is provided
- [X] T113 [US8] Add --jar and --message-type flag handling in ImportArgs

**Checkpoint**: User Story 8 complete - Protobuf JAR import working

---

## Phase 11: User Story 9 - Export Protobuf Descriptor Files (Priority: P3)

**Purpose**: Generate binary Protobuf descriptor files using protoc

### Tests for User Story 9

- [ ] T114 [P] [US9] Add unit test test_cli_export_protobuf_descriptor() in tests/cli/export_tests.rs for descriptor export
- [ ] T115 [P] [US9] Add unit test test_cli_export_protobuf_descriptor_with_imports() in tests/cli/export_tests.rs for descriptor with imports
- [ ] T116 [P] [US9] Add unit test test_cli_export_protobuf_descriptor_protoc_not_found() in tests/cli/export_tests.rs for protoc not found error
- [ ] T117 [P] [US9] Add unit test test_cli_export_protobuf_descriptor_syntax_error() in tests/cli/export_tests.rs for protoc compilation errors

### Implementation for User Story 9

- [X] T118 [US9] Implement check_protoc_available() function in src/cli/commands/export.rs to check protoc availability
- [X] T119 [US9] Implement generate_protobuf_descriptor() function in src/cli/commands/export.rs to call protoc with --include_imports flag
- [X] T120 [US9] Implement handle_export_protobuf_descriptor() function in src/cli/commands/export.rs
- [X] T121 [US9] Add protobuf-descriptor export command handler routing in src/cli/main.rs
- [X] T122 [US9] Add --protoc-path flag handling in ExportArgs

**Checkpoint**: User Story 9 complete - Protobuf descriptor export working

---

## Phase 12: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T123 [US1-US9] Implement UUID validation in apply_uuid_override() using uuid::Uuid::parse_str()
- [ ] T124 [US1-US9] Implement clear error messages for all error types with actionable hints
- [ ] T125 [US1-US9] Add colored output support (optional enhancement) using colored or termcolor crate
- [ ] T126 [US1-US9] Update CLI help text and usage examples in clap derive attributes
- [ ] T127 [US1-US9] Add version information display using clap version flag
- [ ] T128 [US1-US9] Ensure all file I/O errors include file paths and clear messages
- [ ] T129 [US1-US9] Add integration tests for end-to-end CLI workflows in tests/cli/integration_tests.rs
- [ ] T130 [US1-US9] Verify CLI binary builds successfully with cargo build --bin data-modelling-cli
- [ ] T131 [US1-US9] Run cargo fmt, cargo clippy, cargo audit on CLI module
- [ ] T132 [US1-US9] Update README.md or examples/README.md with CLI usage examples

**Checkpoint**: All polish tasks complete - CLI ready for use
