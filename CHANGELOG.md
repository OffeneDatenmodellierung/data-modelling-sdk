# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.7.0] - 2026-01-05

### Added

- **feat(cli)**: Full CLI wrapper implementation (`data-modelling-cli`)
  - Comprehensive import/export commands for all supported formats (SQL, AVRO, JSON Schema, Protobuf, OpenAPI, ODCS)
  - Support for importing SQL files with various dialects (PostgreSQL, MySQL, SQLite, Generic, Databricks)
  - Support for `CREATE VIEW` and `CREATE MATERIALIZED VIEW` statements
  - Import from AVRO, JSON Schema, Protobuf, OpenAPI, and ODCS formats
  - Export to ODCS, AVRO, JSON Schema, Protobuf (proto2/proto3), and Protobuf descriptor formats
  - UUID override support for imported tables (`--uuid` flag)
  - Automatic ODCS file generation on import (can be disabled with `--no-odcs`)
  - External reference resolution for JSON Schema and OpenAPI (`$ref`, `import`)
  - Schema validation before import (optional `--validate` flag)
  - Protobuf descriptor export using `protoc` compiler
  - Comprehensive error messages with platform-specific installation guidance
  - GitHub Actions workflow for building and releasing CLI binaries (Linux, macOS, Windows)
  - Full test coverage for all import/export operations

- **feat(protobuf)**: Proto2 and Proto3 format support
  - `--protobuf-version` flag to select proto2 or proto3 (default: proto3)
  - Correct field labeling: proto2 uses `required`/`optional`, proto3 uses optional by default
  - Proper handling of `repeated` fields in both versions
  - Protobuf descriptor export supports both proto2 and proto3

- **feat(cli)**: OpenAPI support enabled by default
  - CLI builds now include OpenAPI feature by default in GitHub releases
  - CI/CD workflows updated to build with `openapi` feature enabled
  - Documentation updated with OpenAPI build instructions

### Changed

- **refactor(cli)**: Export operations now accept `.odcs.yaml` files directly as input
  - Removed requirement for `workspace.json` file
  - Simplified export workflow: `data-modelling-cli export <format> input.odcs.yaml output.<ext>`

### Fixed

- **fix(protobuf)**: Fixed invalid `optional repeated` syntax in proto3 exports
  - `repeated` fields no longer include `optional` keyword (proto3 compliant)
  - Proto2 correctly uses `required` for non-nullable fields

## [1.6.2] - 2026-01-04

### Added

- **feat(json-schema)**: Full JSON Schema validation support
  - Import: Extract all JSON Schema validation keywords (pattern, minimum, maximum, minLength, maxLength, enum, const, multipleOf, minItems, maxItems, uniqueItems, minProperties, maxProperties, allOf, anyOf, oneOf, not)
  - Export: Export validation conditions from Column quality rules and enum_values back to JSON Schema format
  - Added enum_values field to ColumnData structure to preserve enumeration values through import pipeline
  - Validation keywords stored as quality rules with source="json_schema" for proper round-trip preservation
  - Comprehensive integration test for validation conditions round-trip

## [1.6.1] - 2026-01-04

### Fixed

- **fix(sql)**: Fixed Databricks SQL parser issues with COMMENT clauses
  - Fixed multiline COMMENT clause parsing by converting newlines to spaces within quoted strings
  - Fixed escaped quotes in COMMENT clauses (`\'` converted to SQL standard `''`)
  - Added support for `MAP<...>` complex types (extracted and restored like STRUCT/ARRAY)
  - Added preprocessing to remove unsupported table-level `COMMENT` clauses
  - Added preprocessing to remove unsupported `CLUSTER BY AUTO` clauses
  - Improved SQL normalization to preserve quoted strings correctly

### Added

- **feat(sql)**: Added SQL parser test tool (`cargo run --example test_sql`)
  - Command-line tool for manually testing SQL parsing with different dialects
  - Supports reading from file, stdin, or command-line argument
  - Pretty-print option for detailed column information
  - Convenience script (`test_sql.sh`) for quick testing

## [1.6.0] - 2026-01-03

### Added

- **feat(sql)**: Databricks SQL dialect support
  - `DatabricksDialect` implementation for parsing Databricks-specific SQL syntax
  - Support for `IDENTIFIER()` function calls in table/view names with variable references and string concatenation
  - Support for variable references (`:variable_name`) in `STRUCT` and `ARRAY` type definitions
  - Support for variable references in column definitions (`column_name :variable TYPE`)
  - Support for variable references in `COMMENT` clauses
  - Support for `CREATE VIEW` and `CREATE MATERIALIZED VIEW` statements
  - Complex type extraction and restoration for `STRUCT<...>` and `ARRAY<...>` types
  - SQL normalization for handling multiline SQL statements
  - Comprehensive test coverage with unit and integration tests
  - Backward compatibility maintained with existing SQL dialects (PostgreSQL, MySQL, SQLite, Generic)

### Changed

- **docs**: Updated documentation to list all supported SQL dialects (PostgreSQL, MySQL, SQLite, Generic, Databricks)
- **docs**: Enhanced `SQLImporter` documentation with Databricks dialect examples and usage

## [1.5.0] - 2026-01-03

### Added

- **feat(bpmn)**: BPMN 2.0 model support
  - `BPMNImporter` for importing BPMN XML files with validation
  - `BPMNExporter` for exporting BPMN models in native XML format
  - `BPMNModel` struct for representing BPMN models
  - WASM bindings: `importBpmnModel()`, `exportBpmnModel()`
  - Storage integration: Models saved to `{domain_name}/{model_name}.bpmn.xml`
  - CADS asset references: CADS assets can reference BPMN models via `bpmn_models` field

- **feat(dmn)**: DMN 1.3 model support
  - `DMNImporter` for importing DMN XML files with validation
  - `DMNExporter` for exporting DMN models in native XML format
  - `DMNModel` struct for representing DMN models
  - WASM bindings: `importDmnModel()`, `exportDmnModel()`
  - Storage integration: Models saved to `{domain_name}/{model_name}.dmn.xml`
  - CADS asset references: CADS assets can reference DMN models via `dmn_models` field

- **feat(openapi)**: OpenAPI 3.1.1 specification support
  - `OpenAPIImporter` for importing OpenAPI YAML/JSON files with validation
  - `OpenAPIExporter` for exporting OpenAPI specs with YAML ↔ JSON conversion
  - `OpenAPIModel` struct for representing OpenAPI specifications
  - WASM bindings: `importOpenapiSpec()`, `exportOpenapiSpec()`
  - Storage integration: Specs saved to `{domain_name}/{api_name}.openapi.yaml` or `.openapi.json`
  - CADS asset references: CADS assets can reference OpenAPI specs via `openapi_specs` field

- **feat(converter)**: OpenAPI to ODCS converter
  - `OpenAPIToODCSConverter` for converting OpenAPI schema components to ODCS tables
  - Type mapping: OpenAPI types (string, integer, number, boolean) mapped to ODCS types
  - Constraint preservation: minLength, maxLength, pattern, minimum, maximum, enum converted to ODCS quality rules
  - Format support: date, date-time, email, uri, uuid, password formats handled
  - WASM bindings: `convertOpenapiToOdcs()`, `analyzeOpenapiConversion()`
  - Conversion reports with warnings and field mappings

- **feat(cads)**: Extended CADS asset references
  - `CADSDMNModel` struct for DMN model references
  - `CADSOpenAPISpec` struct for OpenAPI spec references
  - `CADSDMNFormat` and `CADSOpenAPIFormat` enums
  - CADS importer/exporter support for `dmn_models` and `openapi_specs` fields

- **feat(schemas)**: Schema reference files
  - BPMN 2.0 XSD schema placeholder in `schemas/bpmn-2.0.xsd`
  - DMN 1.3 XSD schema placeholder in `schemas/dmn-1.3.xsd`
  - OpenAPI 3.1.1 JSON Schema placeholder in `schemas/openapi-3.1.1.json`

### Changed

- **refactor(features)**: Feature flags for optional formats
  - `bpmn` feature flag for BPMN support (requires `quick-xml`)
  - `dmn` feature flag for DMN support (requires `quick-xml`)
  - `openapi` feature flag for OpenAPI support (uses existing `jsonschema`)

## [1.4.0] - 2026-01-03

### Added

- **feat(storage)**: Domain-based file organization
  - Domain directories with `domain.yaml` files
  - ODCS tables saved as `{name}.odcs.yaml` within domain directories
  - ODPS products saved as `{name}.odps.yaml` within domain directories
  - CADS assets saved as `{name}.cads.yaml` within domain directories
  - `ModelSaver::save_domain()` for saving complete domains with all associated files
  - `ModelLoader::load_domains()` for loading domains from domain directories
  - `ModelLoader::load_domains_from_list()` for loading specific domains
  - Backward compatibility maintained with legacy `tables/` directory structure

- **feat(schemas)**: Schema reference directory
  - `schemas/` directory with JSON Schema definitions for all supported formats
  - ODCS v3.1.0, ODCL v1.2.1, ODPS, and CADS schemas maintained for validation and reference
  - Comprehensive schema documentation in `schemas/README.md`

- **feat(docs)**: Architecture guide
  - Comprehensive Architecture Guide (`docs/ARCHITECTURE.md`) covering project decisions, use cases, and integration patterns
  - Updated README.md with domain-based file structure documentation
  - Updated LLM.txt with new architecture details

### Changed

- **refactor(storage)**: File organization now uses domain-based structure
  - Files organized by business domain instead of flat `tables/` directory
  - Each domain contains its definition and all associated ODCS/ODPS/CADS files
  - Legacy `tables/` directory still supported for backward compatibility

## [1.3.0] - 2026-01-03

### Added

- **feat(odcs)**: Complete ODCS v3.1.0 and ODCL v1.2.1 field preservation
  - Preserve `description`, `quality` arrays (with nested structures), and `$ref` references during import/export
  - Full schema coverage for ODCS v3.1.0 and ODCL v1.2.1 formats
  - Round-trip import/export preserves all fields

- **feat(tags)**: Enhanced tag support with Simple, Pair, and List formats
  - `Tag` enum supporting three formats: `Simple(String)`, `Pair(String, String)`, `List(String, Vec<String>)`
  - Auto-detection parsing with graceful degradation for malformed tags
  - Backward compatible with existing `Vec<String>` tags
  - Tag support in ODCS, ODCL, JSON Schema, AVRO, and Protobuf importers/exporters

- **feat(cads)**: Full CADS v1.0 support
  - CADS importer (`CADSImporter`) for AI/ML models, applications, pipelines
  - CADS exporter (`CADSExporter`) for serializing CADS assets
  - Support for all CADS asset kinds: AIModel, MLPipeline, Application, ETLPipeline, SourceSystem, DestinationSystem
  - Full metadata support: runtime, SLA, pricing, team, risk, compliance, validation profiles

- **feat(odps)**: Full ODPS (Open Data Product Standard) support
  - ODPS importer (`ODPSImporter`) for data products
  - ODPS exporter (`ODPSExporter`) for serializing data products
  - Contract ID validation against ODCS Tables
  - Support for input/output ports, support, team, and custom properties

- **feat(domain)**: Business Domain schema support
  - `Domain` struct for organizing systems, CADS nodes, and ODCS nodes
  - `System` struct representing physical infrastructure with DataFlow metadata
  - `CADSNode` and `ODCSNode` structs with shared reference support
  - ERD-style `SystemConnection` for system-to-system relationships
  - Crowsfeet notation `NodeConnection` for ODCS node relationships
  - Domain operations in `DataModel`: `add_domain()`, `add_system_to_domain()`, `add_cads_node_to_domain()`, `add_odcs_node_to_domain()`, etc.
  - Domain YAML import/export: `Domain::from_yaml()`, `Domain::to_yaml()`

- **feat(converter)**: Universal format converter
  - `convert_to_odcs()` function converts any format to ODCS v3.1.0
  - Auto-detection for SQL, ODCS, ODCL, JSON Schema, AVRO, Protobuf, CADS, ODPS, Domain formats
  - Format-specific conversion logic with informative error messages

- **feat(migration)**: DataFlow to Domain migration utility
  - `migrate_dataflow_to_domain()` converts DataFlow YAML to Domain schema
  - Preserves all metadata during migration
  - Maps DataFlow nodes to Systems and relationships to SystemConnections

- **feat(wasm)**: Comprehensive WASM bindings for new features
  - CADS import/export: `importFromCads()`, `exportToCads()`
  - ODPS import/export: `importFromOdps()`, `exportToOdps()`
  - Domain operations: `createDomain()`, `importFromDomain()`, `exportToDomain()`, `migrateDataflowToDomain()`
  - Domain management: `addSystemToDomain()`, `addCadsNodeToDomain()`, `addOdcsNodeToDomain()`
  - Tag operations: `parseTag()`, `serializeTag()`
  - Universal converter: `convertToOdcs()`

### Changed

- **refactor(dataflow)**: DataFlow format removed and replaced with Domain schema
  - DataFlow import/export modules removed
  - DataFlow WASM bindings removed
  - Migration utility provided for existing DataFlow files

- **refactor(tags)**: Enhanced tag support across all formats
  - `Table.tags` and `Relationship.tags` now use `Vec<Tag>` instead of `Vec<String>`
  - All importers parse enhanced tags using `Tag::from_str()`
  - All exporters serialize tags using `Tag::to_string()`

### Documentation

- Added comprehensive Schema Overview Guide (`docs/SCHEMA_OVERVIEW.md`)
- Updated README.md with Domain schema and new format support
- Updated LLM.txt with new modules and architecture details

## [1.2.0] - 2026-01-27

### Added

- **feat(metadata)**: Enhanced Data Flow node and relationship metadata support
  - Add `owner`, `sla`, `contact_details`, `infrastructure_type`, and `notes` fields to `Table` struct (for Data Flow nodes)
  - Add `owner`, `sla`, `contact_details`, `infrastructure_type` fields to `Relationship` struct (for Data Flow relationships)
  - Add `SlaProperty` struct with ODCS-inspired structure (property, value, unit, element, driver, description, scheduler, schedule)
  - Add `ContactDetails` struct with email, phone, name, role, and other fields
  - Add `InfrastructureType` enum with 70+ infrastructure types covering major cloud databases, container platforms, data warehouses, message queues, and BI/analytics tools
  - Add filter methods to `DataModel`: `filter_nodes_by_owner()`, `filter_relationships_by_owner()`, `filter_nodes_by_infrastructure_type()`, `filter_relationships_by_infrastructure_type()`, `filter_by_tags()`

### Changed

- **refactor(metadata)**: All new metadata fields are optional and maintain backward compatibility
- **refactor(filter)**: Updated `filter_by_tags()` to return tuple `(Vec<&Table>, Vec<&Relationship>)` for both nodes and relationships

## [1.1.0] - 2026-01-02

### Added

- **feat(wasm)**: Add comprehensive WASM bindings for validation, export, and model management
  - Add WASM bindings for input validation functions (`validate_table_name`, `validate_column_name`, `validate_uuid`, `validate_data_type`, `validate_description`)
  - Add WASM bindings for sanitization functions (`sanitize_sql_identifier`, `sanitize_description`)
  - Add WASM bindings for table validation (`detect_naming_conflicts`, `validate_pattern_exclusivity`)
  - Add WASM bindings for relationship validation (`check_circular_dependency`, `validate_no_self_reference`)
  - Add WASM binding for PNG export (`export_to_png`) - feature-gated behind `png-export`
  - Add async WASM bindings for model loading/saving (`load_model`, `save_model`) using browser storage backend
  - Add `Serialize`/`Deserialize` to validation result types for WASM interop
  - Add `Serialize`/`Deserialize` to `ModelLoadResult` and related types for WASM interop

### Changed

- **refactor(wasm)**: Extend WASM module with 14 new binding functions
  - All validation functions return JSON strings with structured results
  - Model loading/saving functions use `js_sys::Promise` for async operations
  - Error handling converts Rust errors to JavaScript-compatible errors

## [1.0.2] - 2025-12-31

### Fixed

- **fix(publish)**: Fix publish workflow and Cargo.toml metadata
  - Add required `description` field to Cargo.toml for crates.io publishing
  - Update publish workflow to use `cargo login` with environment variables instead of deprecated `--token` flag
  - Fix cargo publish warnings

## [1.0.1] - 2025-12-31

### Changed

- **refactor(ci)**: Update GitHub workflows
  - CI workflow now only runs on PR open/update events (not on main branch)
  - Removed publish job from CI workflow
  - Build/test only on Linux by default (macOS/Windows optional)
  - Publish workflow runs on manual trigger or merge to main
  - Publish workflow creates git tag from Cargo.toml version
  - Publish workflow verifies version matches CHANGELOG.md

- **fix(auth)**: Use derive Default for AuthMode enum
  - Replace manual Default impl with derive macro and #[default] attribute
  - Fixes clippy::derivable_impls warning

## [1.0.0] - 2024-12-19

### Added

#### Features

- **feat(models)**: Add comprehensive data model structures for tables, columns, and relationships
  - Add `Table` struct with support for database types, medallion layers, SCD patterns, and Data Vault classifications
  - Add `Column` struct with support for foreign keys, composite keys, constraints, and enum values
  - Add `Relationship` struct with cardinality, relationship types, foreign key details, and ETL job metadata
  - Add `DataModel` struct for managing collections of tables and relationships
  - Add `Position` struct for 2D canvas positioning
  - Add `CrossDomainConfig` for cross-domain table and relationship references

- **feat(import)**: Add comprehensive import functionality for multiple formats
  - Add `ODCSImporter` for importing ODCS v3.1.0 YAML format (legacy ODCL formats supported for import)
  - Add `SQLImporter` for parsing CREATE TABLE statements (PostgreSQL, MySQL, SQL Server dialects)
  - Add `JSONSchemaImporter` for importing JSON Schema definitions with nested object support
  - Add `AvroImporter` for importing AVRO schema files
  - Add `ProtobufImporter` for runtime parsing of proto3 syntax files
  - Add support for deeply nested structures and references in all importers
  - Add validation for imported table and column names

- **feat(export)**: Add comprehensive export functionality for multiple formats
  - Add `ODCSExporter` for exporting to ODCS v3.1.0 YAML format
  - Add `SQLExporter` for generating CREATE TABLE statements (PostgreSQL, MySQL, SQL Server dialects)
  - Add `JSONSchemaExporter` for exporting JSON Schema definitions
  - Add `AvroExporter` for exporting AVRO schema files
  - Add `ProtobufExporter` for exporting Protocol Buffer message definitions
  - Add `ODCLExporter` for legacy ODCL format support (deprecated)
  - Add `PNGExporter` for exporting ER diagrams as PNG images (feature-gated)

- **feat(storage)**: Add storage backend abstraction layer
  - Add `StorageBackend` trait for platform-agnostic file operations
  - Add `FileSystemStorageBackend` for native file system operations (feature-gated)
  - Add `BrowserStorageBackend` for WASM browser storage APIs (feature-gated)
  - Add `ApiStorageBackend` for HTTP API-based storage (feature-gated)
  - Add path traversal protection in all storage backends
  - Add domain slug validation for API backend

- **feat(model)**: Add model loading and saving functionality
  - Add `ModelLoader` for loading tables and relationships from storage backends
  - Add `ModelSaver` for saving tables and relationships to storage backends
  - Add `ApiModelLoader` for loading models from API endpoints
  - Add support for orphaned relationship detection
  - Add YAML file path tracking for tables

- **feat(validation)**: Add comprehensive validation functionality
  - Add `TableValidator` for detecting naming conflicts and pattern exclusivity violations
  - Add `RelationshipValidator` for detecting circular dependencies and self-references
  - Add input validation functions for table names, column names, data types, and UUIDs
  - Add SQL identifier sanitization for multiple database dialects
  - Add reserved word checking for SQL identifiers

- **feat(git)**: Add Git integration support
  - Add `GitService` for Git repository operations (feature-gated)
  - Add support for staging files, committing changes, and checking repository status
  - Add error handling for Git operations

- **feat(workspace)**: Add workspace management types
  - Add `WorkspaceInfo` for workspace metadata
  - Add `ProfileInfo` for user profile management
  - Add request/response types for workspace operations

- **feat(auth)**: Add authentication types
  - Add `AuthMode` enum for different authentication modes (Web, Local, Online)
  - Add `AuthState` for tracking authentication status
  - Add `GitHubEmail` for GitHub OAuth email selection
  - Add OAuth request/response types

- **feat(cross-domain)**: Add cross-domain table and relationship sharing
  - Add `CrossDomainTableRef` for referencing tables from other domains
  - Add `CrossDomainRelationshipRef` for referencing relationships from other domains
  - Add `CrossDomainConfig` for managing imported references

#### Documentation

- **docs**: Add comprehensive documentation coverage (~85%+)
  - Add module-level documentation for all public modules
  - Add function-level documentation with examples for all public functions
  - Add field-level documentation for all model structs
  - Add usage examples in documentation comments
  - Add security considerations documentation

#### Tests

- **test**: Add comprehensive test suite (~65+ tests)
  - Add unit tests for all validation functions
  - Add unit tests for model loading and saving
  - Add unit tests for Git service operations
  - Add unit tests for storage backend operations
  - Add unit tests for import/export functionality
  - Add unit tests for workspace and auth types
  - Add integration tests for ODCS import/export roundtrips
  - Add tests for deeply nested structures and references
  - Add tests for edge cases and error conditions

### Changed

- **refactor(import/odcs)**: Improve ODCS importer to fully parse table structure
  - Replace basic field parsing with full `ODCSImporter::parse_table()` usage
  - Implement complete STRUCT expansion with recursive helper function
  - Improve handling of complex union types in AVRO import

- **refactor(import/protobuf)**: Enhance Protobuf parser for complete proto3 support
  - Add support for nested messages with dot notation
  - Add support for repeated and optional fields
  - Add support for enum definitions
  - Add support for oneof and map types

- **refactor(export/odcs)**: Standardize on ODCS v3.1.0 format
  - Remove support for legacy ODCS formats in export (import still supported)
  - Ensure all exports conform to ODCS v3.1.0 schema

- **refactor(models)**: Improve model structure and methods
  - Change table ID generation from deterministic UUIDv5 to random UUIDv4
  - Add `get_unique_key()` method to `Table` for conflict detection
  - Add helper methods to `DataModel` for table and relationship lookup

- **refactor(validation)**: Improve validation error messages
  - Add detailed error messages with context
  - Add support for multiple validation errors per operation

### Fixed

- **fix(import/odcs)**: Fix comments to clarify ODCS single-table-per-file format
  - Update comments to reflect that ODCS only specifies one table per file
  - Fix STRUCT expansion to handle deeply nested structures correctly

- **fix(import/avro)**: Improve union type handling
  - Use first non-null type from union instead of defaulting to string

- **fix(model/loader)**: Fix table loading to use full ODCS parser
  - Replace basic field parsing with complete table structure parsing
  - Ensure all columns and metadata are properly loaded

- **fix(storage)**: Fix path traversal vulnerabilities
  - Add comprehensive path traversal protection in all storage backends
  - Add validation for domain slugs in API backend

- **fix(export)**: Fix clippy warnings and improve code quality
  - Replace `or_insert_with(Vec::new)` with `or_default()`
  - Fix redundant closure warnings
  - Fix unused enumerate index warnings

### Security

- **security(storage)**: Add path traversal protection
  - Block all path traversal attempts (../, ..\, etc.)
  - Validate domain slugs to prevent injection attacks
  - Sanitize SQL identifiers to prevent SQL injection

- **security(validation)**: Add input validation
  - Validate all user inputs before processing
  - Prevent SQL injection via malicious table/column names
  - Enforce maximum length limits on identifiers

### Performance

- **perf(validation)**: Optimize relationship validation
  - Use efficient graph algorithms for cycle detection
  - Cache validation results where appropriate

### Build

- **build**: Add feature flags for optional functionality
  - Add `api-backend` feature for HTTP API support
  - Add `native-fs` feature for native file system operations
  - Add `wasm` feature for WebAssembly support
  - Add `png-export` feature for PNG diagram export
  - Add `git` feature for Git integration
  - Add `databricks-dialect` feature for Databricks SQL support

### Removed

- **remove**: Remove legacy ODCS format support from export
  - ODCL exporter remains for import compatibility only
  - All exports now use ODCS v3.1.0 format

### Deprecated

- **deprecate(export/odcl)**: Mark ODCL exporter as deprecated
  - ODCL is End-of-Life standard
  - Use ODCS v3.1.0 format for new exports
  - ODCL import still supported for backward compatibility

---

## Version History

### [1.0.0] - 2024-12-19

Initial stable release with comprehensive feature set:

- Complete data modeling SDK with support for tables, columns, and relationships
- Multi-format import/export (ODCS, SQL, JSON Schema, AVRO, Protobuf)
- Storage backend abstraction for multiple platforms
- Comprehensive validation and security features
- Cross-domain sharing capabilities
- Git integration support
- Extensive test coverage (~65+ tests)
- Comprehensive documentation (~85%+ coverage)

---

## Types of Changes

- **Added** for new features
- **Changed** for changes in existing functionality
- **Deprecated** for soon-to-be removed features
- **Removed** for now removed features
- **Fixed** for any bug fixes
- **Security** for vulnerability fixes

[1.0.0]: https://github.com/your-org/data-modelling-sdk/releases/tag/v1.0.0
