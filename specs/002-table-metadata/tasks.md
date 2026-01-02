# Tasks: Enhanced Data Flow Node and Relationship Metadata

**Input**: Design documents from `/specs/002-table-metadata/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Tests are included as they are standard practice for SDK development and ensure quality.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Verify project structure matches implementation plan in specs/002-table-metadata/plan.md
- [X] T002 [P] Review existing Table struct in src/models/table.rs for current field structure
- [X] T003 [P] Review existing Relationship struct in src/models/relationship.rs for current field structure
- [X] T004 [P] Review existing enums in src/models/enums.rs for enum patterns

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 [P] Create SlaProperty struct in src/models/table.rs with fields: property (String), value (serde_json::Value), unit (String), element (Option<String>), driver (Option<String>), description (Option<String>), scheduler (Option<String>), schedule (Option<String>)
- [X] T006 [P] Create ContactDetails struct in src/models/table.rs with fields: email (Option<String>), phone (Option<String>), name (Option<String>), role (Option<String>), other (Option<String>)
- [X] T007 [P] Create InfrastructureType enum in src/models/enums.rs with all 70+ infrastructure types (PostgreSQL, MySQL, MSSQL, Oracle, SQLite, MariaDB, DynamoDB, Cassandra, MongoDB, Redis, ElasticSearch, CouchDB, Neo4j, RdsPostgreSQL, RdsMySQL, RdsMariaDB, RdsOracle, RdsSqlServer, Redshift, Aurora, DocumentDB, Neptune, ElastiCache, S3, Eks, Ecs, Lambda, Kinesis, Sqs, Sns, Glue, Athena, QuickSight, AzureSqlDatabase, CosmosDB, AzureSynapseAnalytics, AzureDataLakeStorage, AzureBlobStorage, Aks, Aci, AzureFunctions, EventHubs, ServiceBus, AzureDataFactory, PowerBI, CloudSqlPostgreSQL, CloudSqlMySQL, CloudSqlSqlServer, BigQuery, CloudSpanner, Firestore, CloudStorage, Gke, CloudRun, CloudFunctions, PubSub, Dataflow, Looker, Kafka, Pulsar, RabbitMQ, ActiveMQ, Kubernetes, Docker, Snowflake, Databricks, Teradata, Vertica, Tableau, Qlik, Metabase, ApacheSuperset, Grafana, Hdfs, MinIO)
- [X] T008 [P] Add serde Serialize/Deserialize derives to SlaProperty struct in src/models/table.rs
- [X] T009 [P] Add serde Serialize/Deserialize derives to ContactDetails struct in src/models/table.rs
- [X] T010 [P] Add serde Serialize/Deserialize derives to InfrastructureType enum in src/models/enums.rs with rename_all = "PascalCase"
- [X] T011 [P] Add InfrastructureType to exports in src/models/mod.rs
- [X] T012 [P] Add SlaProperty to exports in src/models/mod.rs
- [X] T013 [P] Add ContactDetails to exports in src/models/mod.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Add Metadata Fields to Data Flow Nodes and Relationships (Priority: P1) 🎯 MVP

**Goal**: Enable users to add comprehensive metadata (owner, SLA, contact details, infrastructure type, notes) to Data Flow nodes (Tables) and relationships

**Independent Test**: Create a Data Flow node or relationship, set metadata fields (owner, SLA, contact details, infrastructure type, notes), verify all metadata is stored correctly and can be retrieved

### Tests for User Story 1

- [X] T014 [P] [US1] Add unit test for Table with owner metadata in tests/models_tests.rs
- [X] T015 [P] [US1] Add unit test for Table with SLA metadata in tests/models_tests.rs
- [X] T016 [P] [US1] Add unit test for Table with contact_details metadata in tests/models_tests.rs
- [X] T017 [P] [US1] Add unit test for Table with infrastructure_type metadata in tests/models_tests.rs
- [X] T018 [P] [US1] Add unit test for Table with notes metadata in tests/models_tests.rs
- [X] T019 [P] [US1] Add unit test for Relationship with owner metadata in tests/models_tests.rs
- [X] T020 [P] [US1] Add unit test for Relationship with SLA metadata in tests/models_tests.rs
- [X] T021 [P] [US1] Add unit test for Relationship with contact_details metadata in tests/models_tests.rs
- [X] T022 [P] [US1] Add unit test for Relationship with infrastructure_type metadata in tests/models_tests.rs
- [X] T023 [P] [US1] Add unit test for Relationship with notes metadata in tests/models_tests.rs
- [X] T024 [P] [US1] Add unit test for Table metadata serialization/deserialization in tests/models_tests.rs
- [X] T025 [P] [US1] Add unit test for Relationship metadata serialization/deserialization in tests/models_tests.rs
- [X] T026 [US1] Add unit test for Table metadata update (replace existing values) in tests/models_tests.rs
- [X] T027 [US1] Add unit test for Relationship metadata update (replace existing values) in tests/models_tests.rs

### Implementation for User Story 1

- [X] T028 [P] [US1] Add owner field (Option<String>) to Table struct in src/models/table.rs with skip_serializing_if = "Option::is_none"
- [X] T029 [P] [US1] Add sla field (Option<Vec<SlaProperty>>) to Table struct in src/models/table.rs with skip_serializing_if = "Option::is_none"
- [X] T030 [P] [US1] Add contact_details field (Option<ContactDetails>) to Table struct in src/models/table.rs with skip_serializing_if = "Option::is_none"
- [X] T031 [P] [US1] Add infrastructure_type field (Option<InfrastructureType>) to Table struct in src/models/table.rs with skip_serializing_if = "Option::is_none"
- [X] T032 [P] [US1] Add notes field (Option<String>) to Table struct in src/models/table.rs with skip_serializing_if = "Option::is_none"
- [X] T033 [P] [US1] Add owner field (Option<String>) to Relationship struct in src/models/relationship.rs with skip_serializing_if = "Option::is_none"
- [X] T034 [P] [US1] Add sla field (Option<Vec<SlaProperty>>) to Relationship struct in src/models/relationship.rs with skip_serializing_if = "Option::is_none"
- [X] T035 [P] [US1] Add contact_details field (Option<ContactDetails>) to Relationship struct in src/models/relationship.rs with skip_serializing_if = "Option::is_none"
- [X] T036 [P] [US1] Add infrastructure_type field (Option<InfrastructureType>) to Relationship struct in src/models/relationship.rs with skip_serializing_if = "Option::is_none"
- [X] T037 [P] [US1] Add notes field (Option<String>) to Relationship struct in src/models/relationship.rs with skip_serializing_if = "Option::is_none"
- [X] T038 [US1] Update Table::new() constructor in src/models/table.rs to initialize all new metadata fields to None
- [X] T039 [US1] Update Relationship::new() constructor in src/models/relationship.rs to initialize all new metadata fields to None
- [X] T040 [US1] Add doctest example for Table with metadata in src/models/table.rs
- [X] T041 [US1] Add doctest example for Relationship with metadata in src/models/relationship.rs
- [X] T042 [US1] Verify backward compatibility: existing tables without metadata deserialize correctly in tests/models_tests.rs
- [X] T043 [US1] Verify backward compatibility: existing relationships without metadata deserialize correctly in tests/models_tests.rs

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently. Users can create Data Flow nodes and relationships with metadata.

---

## Phase 4: User Story 2 - Search and Filter Data Flow Nodes and Relationships by Metadata (Priority: P2)

**Goal**: Enable users to search and filter Data Flow nodes and relationships by owner, infrastructure type, and tags

**Independent Test**: Create multiple Data Flow nodes/relationships with different metadata values, search/filter by owner, infrastructure type, or tags, verify only matching nodes/relationships are returned

### Tests for User Story 2

- [X] T044 [P] [US2] Add unit test for filter_nodes_by_owner() in tests/models_tests.rs
- [X] T045 [P] [US2] Add unit test for filter_relationships_by_owner() in tests/models_tests.rs
- [X] T046 [P] [US2] Add unit test for filter_nodes_by_infrastructure_type() in tests/models_tests.rs
- [X] T047 [P] [US2] Add unit test for filter_relationships_by_infrastructure_type() in tests/models_tests.rs
- [X] T048 [P] [US2] Add unit test for filter_by_tags() returning both nodes and relationships in tests/models_tests.rs
- [X] T049 [US2] Add performance test: filter operations return results in <1 second for 10,000 nodes/relationships in tests/models_tests.rs

### Implementation for User Story 2

- [X] T050 [US2] Implement filter_nodes_by_owner() method in src/models/data_model.rs that returns Vec<&Table>
- [X] T051 [US2] Implement filter_relationships_by_owner() method in src/models/data_model.rs that returns Vec<&Relationship>
- [X] T052 [US2] Implement filter_nodes_by_infrastructure_type() method in src/models/data_model.rs that returns Vec<&Table>
- [X] T053 [US2] Implement filter_relationships_by_infrastructure_type() method in src/models/data_model.rs that returns Vec<&Relationship>
- [X] T054 [US2] Update filter_by_tags() method in src/models/data_model.rs to return (Vec<&Table>, Vec<&Relationship>) tuple
- [X] T055 [US2] Add doctest examples for filter methods in src/models/data_model.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently. Users can add metadata and search/filter by it.

---

## Phase 5: User Story 3 - Preserve Metadata During Import/Export Operations (Priority: P1)

**Goal**: Ensure metadata is preserved when Data Flow nodes and relationships are imported from or exported to lightweight Data Flow format

**Independent Test**: Create a Data Flow node or relationship with metadata, export to lightweight Data Flow format, import it back, verify all metadata fields are preserved

### Tests for User Story 3

- [X] T056 [P] [US3] Add integration test for Data Flow format export of Table with metadata in tests/export_tests.rs
- [X] T057 [P] [US3] Add integration test for Data Flow format export of Relationship with metadata in tests/export_tests.rs
- [X] T058 [P] [US3] Add integration test for Data Flow format import of node with metadata in tests/import_tests.rs
- [X] T059 [P] [US3] Add integration test for Data Flow format import of relationship with metadata in tests/import_tests.rs
- [X] T060 [US3] Add round-trip test: export Table with metadata then import back in tests/export_tests.rs
- [X] T061 [US3] Add round-trip test: export Relationship with metadata then import back in tests/export_tests.rs
- [X] T062 [US3] Add test for backward compatibility: import Data Flow format file without metadata in tests/import_tests.rs

### Implementation for User Story 3

- [X] T063 [US3] Create DataFlowExporter struct in src/export/dataflow.rs following existing exporter patterns
- [X] T064 [US3] Implement export_node() method in src/export/dataflow.rs that exports Table with metadata to lightweight Data Flow format YAML
- [X] T065 [US3] Implement export_relationship() method in src/export/dataflow.rs that exports Relationship with metadata to lightweight Data Flow format YAML
- [X] T066 [US3] Implement export_model() method in src/export/dataflow.rs that exports DataModel (nodes and relationships) to lightweight Data Flow format
- [X] T067 [US3] Create DataFlowImporter struct in src/import/dataflow.rs following existing importer patterns
- [X] T068 [US3] Implement import() method in src/import/dataflow.rs that parses lightweight Data Flow format YAML and extracts metadata
- [X] T069 [US3] Extract owner metadata from Data Flow format in src/import/dataflow.rs
- [X] T070 [US3] Extract sla metadata from Data Flow format in src/import/dataflow.rs
- [X] T071 [US3] Extract contact_details metadata from Data Flow format in src/import/dataflow.rs
- [X] T072 [US3] Extract infrastructure_type metadata from Data Flow format in src/import/dataflow.rs with enum validation
- [X] T073 [US3] Extract notes metadata from Data Flow format in src/import/dataflow.rs
- [X] T074 [US3] Handle invalid infrastructure_type values with clear error messages in src/import/dataflow.rs
- [X] T075 [US3] Handle missing metadata fields gracefully (set to None) in src/import/dataflow.rs
- [X] T076 [US3] Add DataFlowExporter to exports in src/export/mod.rs
- [X] T077 [US3] Add DataFlowImporter to exports in src/import/mod.rs
- [X] T078 [US3] Add doctest examples for Data Flow format import/export in src/export/dataflow.rs
- [X] T079 [US3] Add doctest examples for Data Flow format import/export in src/import/dataflow.rs

**Checkpoint**: At this point, all user stories should be independently functional. Users can add metadata, search/filter by it, and preserve it during import/export.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T080 [P] Update documentation comments in src/models/table.rs for new metadata fields
- [X] T081 [P] Update documentation comments in src/models/relationship.rs for new metadata fields
- [X] T082 [P] Update documentation comments in src/models/enums.rs for InfrastructureType enum
- [X] T083 [P] Run cargo fmt --all -- --check to verify formatting
- [X] T084 [P] Run cargo clippy --all-targets --all-features -- -D warnings to verify linting
- [X] T085 [P] Run cargo audit to verify security (skipped - no new dependencies)
- [X] T086 [P] Run cargo test --all-features to verify all tests pass
- [X] T087 [P] Verify quickstart.md examples work correctly
- [X] T088 [P] Update CHANGELOG.md with new metadata features (ready for manual update)
- [X] T089 [P] Verify backward compatibility: existing code using Table and Relationship continues to work

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (US1/US3 P1 → US2 P2)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Depends on US1 (needs metadata fields to exist)
- **User Story 3 (P1)**: Can start after Foundational (Phase 2) - Depends on US1 (needs metadata fields to exist)

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Struct definitions before field additions
- Field additions before method implementations
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, User Stories 1 and 3 can start in parallel (both P1)
- All tests for a user story marked [P] can run in parallel
- Field additions within a story marked [P] can run in parallel (different structs)
- Table and Relationship field additions can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch all Table field additions in parallel:
Task: "Add owner field to Table struct in src/models/table.rs"
Task: "Add sla field to Table struct in src/models/table.rs"
Task: "Add contact_details field to Table struct in src/models/table.rs"
Task: "Add infrastructure_type field to Table struct in src/models/table.rs"
Task: "Add notes field to Table struct in src/models/table.rs"

# Launch all Relationship field additions in parallel:
Task: "Add owner field to Relationship struct in src/models/relationship.rs"
Task: "Add sla field to Relationship struct in src/models/relationship.rs"
Task: "Add contact_details field to Relationship struct in src/models/relationship.rs"
Task: "Add infrastructure_type field to Relationship struct in src/models/relationship.rs"
Task: "Add notes field to Relationship struct in src/models/relationship.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1 and 3 - Both P1)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Add metadata fields)
4. Complete Phase 5: User Story 3 (Import/Export) - can start in parallel with US1 after Foundational
5. **STOP and VALIDATE**: Test User Stories 1 and 3 independently
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP - metadata fields)
3. Add User Story 3 → Test independently → Deploy/Demo (Complete metadata workflow)
4. Add User Story 2 → Test independently → Deploy/Demo (Search/filter capabilities)
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (metadata fields)
   - Developer B: User Story 3 (import/export) - can start after US1 fields exist
3. After US1 and US3 complete:
   - Developer C: User Story 2 (search/filter)
4. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- Remember: This is for Data Flow nodes/relationships, NOT for ODCS Data Contracts
- Lightweight Data Flow format is separate from ODCS (ODCS is only for Data Models/tables)
