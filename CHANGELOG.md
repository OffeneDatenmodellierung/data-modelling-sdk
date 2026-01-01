# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
