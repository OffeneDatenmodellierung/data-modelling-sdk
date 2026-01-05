# Feature Specification: CLI Wrapper for Data Modelling SDK

**Feature Branch**: `006-cli-wrapper`
**Created**: 2026-01-05
**Status**: Draft
**Input**: User description: "we should extend examples/test_sql.rs into a full cli wrapper for the SDK. It should support the following in addition to what is currently supported in test_sql
* Import of SQL files with different dialects
* Import of AVRO, JSON, Protobuf and OpenAPI schemas - showing what is captured and mappings (as we do for the sql example)
* when importig schemas which contain external references support loading these by default from the same directory as the original schema or from non authenticated web urls.
* Import of ODCS
* Validation of all formats before import
* Export of ODCS Schemas to filename.odcs.yaml
* Export of AVRO, JSON, Protobuf Schemas from an ODCS.yaml with all conditions and validation logic supported by the schema.

For Protobuf we should also support importing from a JAR definition and exporting a Proto descriptor file."

## Clarifications

### Session 2026-01-05

- Q: Should SQL import support CREATE VIEW and CREATE MATERIALIZED VIEW statements in addition to CREATE TABLE? → A: Yes, SQL import MUST support CREATE VIEW and CREATE MATERIALIZED VIEW statements for all supported dialects
- Q: How should UUID override work when importing multiple tables? → A: Only support UUID override when importing a single table (error if multiple tables and UUID specified)
- Q: What should the CLI tool be called? → A: The CLI tool MUST be named `data-modelling-cli`

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Import SQL Schema with Dialect Selection (Priority: P1)

A data engineer needs to import SQL CREATE TABLE, CREATE VIEW, and CREATE MATERIALIZED VIEW statements from a file or command line, specifying the SQL dialect (PostgreSQL, MySQL, SQLite, Generic, or Databricks). The CLI should parse the SQL, display what tables, views, and columns were captured, show any errors or warnings, and optionally save the result as ODCS YAML.

**Why this priority**: This extends the existing test_sql.rs functionality and provides the foundation for all other import operations. It's the most mature feature and serves as the baseline for user experience.

**Independent Test**: Can be fully tested by running `data-modelling-cli import sql --dialect postgres --file schema.sql` and verifying that tables, columns, and metadata are correctly parsed and displayed. Delivers immediate value for SQL schema conversion.

**Acceptance Scenarios**:

1. **Given** a SQL file with CREATE TABLE statements, **When** user runs `data-modelling-cli import sql --dialect postgres --file schema.sql`, **Then** the CLI parses the SQL, displays table names, column details, data types, constraints, and any parse errors or warnings
2. **Given** a SQL file with CREATE VIEW statements, **When** user runs `data-modelling-cli import sql --dialect postgres --file schema.sql`, **Then** the CLI parses the SQL, displays view names, column details from the view definition, and treats views as table-like entities for data modeling purposes
3. **Given** a SQL file with CREATE MATERIALIZED VIEW statements, **When** user runs `data-modelling-cli import sql --dialect databricks --file schema.sql`, **Then** the CLI parses the SQL, displays materialized view names, column details, and preserves metadata indicating it's a materialized view
4. **Given** SQL content provided via stdin, **When** user pipes SQL to `data-modelling-cli import sql --dialect databricks`, **Then** the CLI processes the input and displays parsed results (tables, views, and materialized views)
5. **Given** SQL with unnamed tables (subqueries), **When** user imports the SQL, **Then** the CLI identifies tables requiring name resolution and suggests names
6. **Given** SQL with syntax errors, **When** user imports the SQL, **Then** the CLI displays clear error messages indicating what failed and where

---

### User Story 2 - Import Schema Formats (AVRO, JSON Schema, Protobuf, OpenAPI) with Mappings Display (Priority: P1)

A data architect needs to import schemas from AVRO, JSON Schema, Protobuf, or OpenAPI formats. The CLI should parse each format, display what was captured (tables, columns, types, constraints), show the mapping from source format to ODCS structure, and handle any errors gracefully. When importing different versions of the same schema (e.g., AVRO schema v1.0, v1.1, v1.2), users can specify a UUID to override any UUID in the imported schema, allowing the same ODCS record to maintain the same unique ID across schema versions to track changes over time.

**Why this priority**: Core functionality that enables users to convert diverse schema formats into the unified ODCS format. The mapping display helps users understand how their source schemas are interpreted.

**Independent Test**: Can be fully tested by running `data-modelling-cli import avro --file schema.avsc` and verifying that the AVRO schema is parsed, displayed with mappings, and optionally exported. Delivers value for schema conversion workflows.

**Acceptance Scenarios**:

1. **Given** an AVRO schema file, **When** user runs `data-modelling-cli import avro --file schema.avsc`, **Then** the CLI parses the schema, displays tables and columns extracted, shows AVRO-to-ODCS type mappings, and reports any validation errors
2. **Given** an AVRO schema file with a UUID override specified, **When** user runs `data-modelling-cli import avro --file schema-v1.0.avsc --uuid 550e8400-e29b-41d4-a716-446655440000`, **Then** the CLI imports the schema and assigns the specified UUID to the table, overriding any UUID present in the source schema, enabling tracking of schema evolution with consistent table IDs
3. **Given** a JSON Schema file, **When** user runs `data-modelling-cli import json-schema --file schema.json`, **Then** the CLI parses the schema, displays extracted tables/columns, shows how JSON Schema properties map to ODCS fields, and handles nested objects appropriately
4. **Given** a Protobuf .proto file, **When** user runs `data-modelling-cli import protobuf --file schema.proto`, **Then** the CLI parses the proto file, displays messages as tables with field mappings, shows how proto types map to ODCS types, and handles nested messages
5. **Given** an OpenAPI specification file, **When** user runs `data-modelling-cli import openapi --file api.yaml`, **Then** the CLI parses the OpenAPI spec, displays schema components converted to tables, shows API-to-table mappings, and handles references
6. **Given** an import operation with multiple tables and a UUID override specified, **When** user runs the import command with `--uuid` flag, **Then** the CLI reports an error indicating that UUID override is only supported for single-table imports

---

### User Story 3 - Import ODCS Schemas with Validation (Priority: P1)

A data modeler needs to import existing ODCS YAML files, validate them against the ODCS schema, and view their structure. The CLI should validate the YAML format, check against the JSON Schema definition, and display any validation errors.

**Why this priority**: Enables users to validate and inspect ODCS files, ensuring they conform to the specification before use. Essential for quality assurance in data modeling workflows.

**Independent Test**: Can be fully tested by running `data-modelling-cli import odcs --file table.odcs.yaml` and verifying that the file is validated against the ODCS schema, structure is displayed, and validation errors are reported. Delivers value for schema validation and inspection.

**Acceptance Scenarios**:

1. **Given** a valid ODCS YAML file, **When** user runs `data-modelling-cli import odcs --file table.odcs.yaml`, **Then** the CLI validates the file against the ODCS JSON Schema, displays the table structure, and confirms successful validation
2. **Given** an invalid ODCS YAML file (missing required fields), **When** user runs the import command, **Then** the CLI reports specific validation errors indicating which fields are missing or invalid
3. **Given** an ODCS file with syntax errors, **When** user runs the import command, **Then** the CLI reports YAML parsing errors with line numbers and clear error messages

---

### User Story 4 - Resolve External References in Schemas (Priority: P2)

A developer needs to import schemas that contain external references (e.g., JSON Schema `$ref`, OpenAPI `$ref`, Protobuf `import`). The CLI should automatically resolve references from the same directory as the source file or from non-authenticated web URLs, loading referenced schemas and including them in the import result.

**Why this priority**: Many real-world schemas use external references for modularity. Supporting automatic resolution improves usability and enables importing complex, multi-file schemas without manual intervention.

**Independent Test**: Can be fully tested by running `data-modelling-cli import json-schema --file schema.json` where schema.json contains `$ref: "./definitions.json"`, and verifying that definitions.json is automatically loaded from the same directory and merged into the import. Delivers value for handling complex, modular schemas.

**Acceptance Scenarios**:

1. **Given** a JSON Schema file with `$ref` pointing to a local file in the same directory, **When** user runs the import command, **Then** the CLI automatically loads the referenced file, resolves the reference, and includes it in the import result
2. **Given** a schema file with `$ref` pointing to a non-authenticated HTTP/HTTPS URL, **When** user runs the import command, **Then** the CLI fetches the referenced schema from the URL, resolves the reference, and includes it in the import result
3. **Given** a schema file with `$ref` pointing to a file that doesn't exist, **When** user runs the import command, **Then** the CLI reports a clear error indicating the missing reference file and its expected location
4. **Given** a schema file with `$ref` pointing to a URL requiring authentication, **When** user runs the import command, **Then** the CLI reports that authenticated URLs are not supported and suggests downloading the file locally first

---

### User Story 5 - Validate All Formats Before Import (Priority: P2)

A data quality engineer needs to ensure that imported schemas are valid before processing them. The CLI should validate each format against its respective schema (ODCS JSON Schema, OpenAPI JSON Schema, AVRO specification, Protobuf syntax, JSON Schema specification) and report validation errors before attempting to parse and convert.

**Why this priority**: Prevents invalid schemas from being processed, saving time and providing clear feedback about schema issues. Validation errors are more actionable than parse errors.

**Independent Test**: Can be fully tested by running `data-modelling-cli import odcs --file invalid.odcs.yaml` and verifying that validation errors are reported before any parsing occurs. Delivers value for schema quality assurance.

**Acceptance Scenarios**:

1. **Given** an ODCS file that doesn't conform to the ODCS JSON Schema, **When** user runs the import command, **Then** the CLI validates against the schema first and reports specific validation errors before attempting to parse
2. **Given** an OpenAPI file that violates the OpenAPI specification, **When** user runs the import command, **Then** the CLI validates against the OpenAPI JSON Schema and reports validation errors
3. **Given** a Protobuf file with syntax errors, **When** user runs the import command, **Then** the CLI validates proto syntax and reports syntax errors before parsing
4. **Given** a valid schema file, **When** user runs the import command, **Then** the CLI validates successfully and proceeds with import, displaying a validation success message

---

### User Story 6 - Export ODCS Schemas to YAML Files (Priority: P2)

A data modeler needs to export imported or created schemas to ODCS YAML format, saving them to files with appropriate naming (e.g., `tablename.odcs.yaml`). The CLI should support exporting individual tables or entire workspaces to separate YAML files.

**Why this priority**: Enables users to save their work and share ODCS schemas. The `.odcs.yaml` file extension convention helps identify ODCS files in file systems and version control.

**Independent Test**: Can be fully tested by running `data-modelling-cli export odcs --input imported.json --output schema.odcs.yaml` and verifying that a valid ODCS YAML file is created with the correct structure. Delivers value for schema persistence and sharing.

**Acceptance Scenarios**:

1. **Given** an imported schema (from any format), **When** user runs `data-modelling-cli export odcs --input schema.json --output users.odcs.yaml`, **Then** the CLI exports the schema to ODCS YAML format and saves it to the specified file with `.odcs.yaml` extension
2. **Given** multiple tables in a workspace, **When** user runs the export command, **Then** the CLI exports each table to a separate YAML file named after the table (e.g., `users.odcs.yaml`, `orders.odcs.yaml`)
3. **Given** an export operation, **When** the output file already exists, **Then** the CLI prompts for confirmation or uses a `--force` flag to overwrite
4. **Given** an invalid workspace structure, **When** user runs the export command, **Then** the CLI reports errors indicating what's wrong with the structure before attempting export

---

### User Story 7 - Export to AVRO, JSON Schema, and Protobuf Formats (Priority: P2)

A developer needs to export ODCS schemas to AVRO, JSON Schema, or Protobuf formats for use in other systems. The CLI should preserve all conditions, validation logic, and quality rules from the ODCS schema in the exported format, mapping them appropriately to the target format's capabilities.

**Why this priority**: Enables interoperability with systems that use these formats. Preserving validation logic ensures data quality rules are maintained across format conversions.

**Independent Test**: Can be fully tested by running `data-modelling-cli export avro --input schema.odcs.yaml --output schema.avsc` and verifying that the AVRO schema includes validation constraints mapped from ODCS quality rules. Delivers value for format interoperability.

**Acceptance Scenarios**:

1. **Given** an ODCS schema with quality rules and validation constraints, **When** user runs `data-modelling-cli export avro --input schema.odcs.yaml --output schema.avsc`, **Then** the CLI exports to AVRO format, mapping ODCS quality rules to AVRO logical types and validation annotations where possible
2. **Given** an ODCS schema with column descriptions and constraints, **When** user runs `data-modelling-cli export json-schema --input schema.odcs.yaml --output schema.json`, **Then** the CLI exports to JSON Schema format, preserving descriptions, type constraints, and required fields
3. **Given** an ODCS schema with complex validation rules, **When** user runs `data-modelling-cli export protobuf --input schema.odcs.yaml --output schema.proto`, **Then** the CLI exports to Protobuf format, mapping validation rules to proto field options and comments where supported
4. **Given** an ODCS schema with features not supported by the target format, **When** user runs the export command, **Then** the CLI reports warnings about unsupported features but still exports what is possible

---

### User Story 8 - Import Protobuf from JAR Files (Priority: P3)

A Java developer needs to import Protobuf schemas directly from JAR files. The CLI should extract `.proto` files from JAR archives, merge multiple proto files if needed, and import them into ODCS format. This supports workflows where proto files are bundled in Java libraries.

**Why this priority**: Enables importing schemas from Java ecosystem artifacts without manual extraction. Lower priority because it's a specialized use case, but valuable for Java-based data pipelines.

**Independent Test**: Can be fully tested by running `data-modelling-cli import protobuf --jar library.jar --message-type com.example.Person` and verifying that proto files are extracted from the JAR, parsed, and imported. Delivers value for Java ecosystem integration.

**Acceptance Scenarios**:

1. **Given** a JAR file containing `.proto` files, **When** user runs `data-modelling-cli import protobuf --jar library.jar`, **Then** the CLI extracts all `.proto` files from the JAR, merges them if multiple files exist, and imports them into ODCS format
2. **Given** a JAR file with a specific message type, **When** user runs `data-modelling-cli import protobuf --jar library.jar --message-type com.example.Person`, **Then** the CLI extracts and imports only the proto files relevant to that message type
3. **Given** a JAR file with no `.proto` files, **When** user runs the import command, **Then** the CLI reports that no proto files were found in the JAR
4. **Given** a JAR file with malformed `.proto` files, **When** user runs the import command, **Then** the CLI reports parsing errors for the specific proto files that failed

---

### User Story 9 - Export Protobuf Descriptor Files (Priority: P3)

A developer needs to export Protobuf schemas as binary descriptor files (`.pb`) for use with protobuf decoders and runtime systems. The CLI should compile `.proto` files into descriptor files using `protoc`, including all imports and source information.

**Why this priority**: Enables fast protobuf decoding in runtime systems that use pre-compiled descriptors. Lower priority because it requires `protoc` installation, but valuable for performance-critical applications.

**Independent Test**: Can be fully tested by running `data-modelling-cli export protobuf-descriptor --input schema.proto --output schema.pb` and verifying that a binary descriptor file is generated. Delivers value for protobuf runtime integration.

**Acceptance Scenarios**:

1. **Given** a Protobuf `.proto` file, **When** user runs `data-modelling-cli export protobuf-descriptor --input schema.proto --output schema.pb`, **Then** the CLI compiles the proto file using `protoc`, generates a binary descriptor file, and includes all imported proto files
2. **Given** a proto file with imports, **When** user runs the export command, **Then** the CLI automatically includes all imported proto files in the descriptor using `--include_imports`
3. **Given** a system without `protoc` installed, **When** user runs the export command, **Then** the CLI reports that `protoc` is required and provides installation instructions
4. **Given** a proto file with syntax errors, **When** user runs the export command, **Then** the CLI reports `protoc` compilation errors with clear messages

---

### Edge Cases

- What happens when a schema file contains circular references?
- How does the system handle very large schema files (memory limits)?
- What happens when external reference URLs are slow or timeout?
- How does the system handle conflicting table names when importing multiple files?
- What happens when exporting to a format that doesn't support certain ODCS features (e.g., quality rules)?
- How does the system handle file encoding issues (UTF-8 vs other encodings)?
- What happens when a JAR file is corrupted or not a valid ZIP archive?
- How does the system handle proto files with mixed syntax versions (proto2 and proto3)?
- What happens when `protoc` is installed but an incompatible version?
- How does the system handle relative paths in external references when the working directory changes?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: CLI MUST support importing SQL files with dialect specification (postgres, mysql, sqlite, generic, databricks), including CREATE TABLE, CREATE VIEW, and CREATE MATERIALIZED VIEW statements
- **FR-002**: CLI MUST support importing AVRO schema files (`.avsc` or `.avro` JSON format)
- **FR-003**: CLI MUST support importing JSON Schema files (`.json` format)
- **FR-004**: CLI MUST support importing Protobuf schema files (`.proto` format)
- **FR-005**: CLI MUST support importing OpenAPI specification files (`.yaml`, `.yml`, or `.json` format)
- **FR-006**: CLI MUST support importing ODCS YAML files (`.odcs.yaml`, `.yaml`, `.yml` format)
- **FR-007**: CLI MUST display what was captured during import (tables, columns, data types, constraints, descriptions)
- **FR-008**: CLI MUST display mappings from source format to ODCS structure (e.g., "AVRO 'string' type mapped to ODCS 'STRING' type")
- **FR-009**: CLI MUST automatically resolve external references (`$ref`, `import`) from the same directory as the source schema file
- **FR-010**: CLI MUST automatically resolve external references from non-authenticated HTTP/HTTPS URLs
- **FR-011**: CLI MUST report errors when external references cannot be resolved (missing files, network errors, authenticated URLs)
- **FR-012**: CLI MUST validate ODCS files against the ODCS JSON Schema before importing
- **FR-013**: CLI MUST validate OpenAPI files against the OpenAPI JSON Schema before importing
- **FR-014**: CLI MUST validate Protobuf files for syntax correctness before importing
- **FR-015**: CLI MUST validate AVRO files against AVRO specification before importing
- **FR-016**: CLI MUST validate JSON Schema files against JSON Schema specification before importing
- **FR-017**: CLI MUST report validation errors with clear messages indicating what failed and where
- **FR-018**: CLI MUST support exporting imported schemas to ODCS YAML format with `.odcs.yaml` file extension
- **FR-019**: CLI MUST support exporting ODCS schemas to AVRO format (`.avsc` file)
- **FR-020**: CLI MUST support exporting ODCS schemas to JSON Schema format (`.json` file)
- **FR-021**: CLI MUST support exporting ODCS schemas to Protobuf format (`.proto` file)
- **FR-022**: CLI MUST preserve ODCS quality rules and validation logic when exporting to target formats (mapping to format-specific validation mechanisms)
- **FR-023**: CLI MUST support importing Protobuf schemas from JAR files (extracting `.proto` files from JAR archives)
- **FR-024**: CLI MUST merge multiple `.proto` files from a JAR into a single import when needed
- **FR-025**: CLI MUST support exporting Protobuf schemas as binary descriptor files (`.pb` format) using `protoc`
- **FR-026**: CLI MUST include all imported proto files in descriptor generation using `--include_imports` flag
- **FR-027**: CLI MUST report clear error messages when `protoc` is not installed or unavailable
- **FR-028**: CLI MUST support reading input from files (`--file` flag)
- **FR-029**: CLI MUST support reading input from stdin (piping)
- **FR-030**: CLI MUST support reading SQL from command-line arguments (`--sql` flag)
- **FR-031**: CLI MUST display parse errors, warnings, and tables requiring name resolution (for SQL imports)
- **FR-032**: CLI MUST support pretty-printed output for detailed column information (`--pretty` flag)
- **FR-033**: CLI MUST support compact output format for quick overview (default)
- **FR-034**: CLI MUST handle file overwrite scenarios (prompt for confirmation or `--force` flag)
- **FR-035**: CLI MUST report file I/O errors with clear messages (file not found, permission denied, etc.)
- **FR-036**: CLI MUST support specifying a UUID for imported tables via `--uuid` flag, overriding any UUID present in the source schema
- **FR-037**: CLI MUST validate UUID format when provided via `--uuid` flag and report errors for invalid UUIDs
- **FR-038**: CLI MUST only allow UUID override when importing a single table (report error if multiple tables are imported and UUID override is specified)

### Key Entities *(include if feature involves data)*

- **CLI Command**: Represents a user command with subcommands (import, export), options, and arguments. Supports multiple input methods (file, stdin, command-line) and output formats.
- **Import Result**: Contains parsed tables, columns, mappings, errors, and warnings from schema import operations. Displayed to users to show what was captured. Table UUIDs can be overridden during import to maintain consistent IDs across schema versions.
- **Schema Mapping**: Represents the translation from source format (SQL, AVRO, JSON Schema, Protobuf, OpenAPI) to ODCS structure. Includes type mappings, constraint mappings, and field mappings.
- **External Reference**: Represents a reference to another schema file (local file path or URL). Must be resolved before import can complete.
- **Validation Result**: Contains validation errors and warnings from schema validation against format specifications. Reported before import proceeds.
- **Export Output**: Represents the exported schema in target format (ODCS YAML, AVRO, JSON Schema, Protobuf, Protobuf Descriptor). Preserves validation logic and quality rules where possible.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can import SQL schemas from files in under 5 seconds for files up to 1MB in size
- **SC-002**: Users can import AVRO, JSON Schema, Protobuf, and OpenAPI schemas with 95% success rate for valid schemas conforming to their respective specifications
- **SC-003**: External references are automatically resolved in 90% of cases where referenced files exist in the same directory or are accessible via non-authenticated URLs
- **SC-004**: Schema validation completes in under 2 seconds for files up to 500KB, reporting all validation errors before import proceeds
- **SC-005**: Users can export ODCS schemas to target formats (AVRO, JSON Schema, Protobuf) with all supported validation logic preserved in 90% of cases
- **SC-006**: Users can import Protobuf schemas from JAR files successfully when JARs contain valid `.proto` files
- **SC-007**: Protobuf descriptor export succeeds when `protoc` is installed and proto files are valid, generating descriptor files that work with protobuf decoders
- **SC-008**: CLI provides clear, actionable error messages that enable users to fix schema issues without consulting documentation in 80% of cases
- **SC-009**: CLI handles edge cases (missing files, network errors, invalid formats) gracefully without crashing, reporting errors instead

## Assumptions

- Users have basic command-line interface knowledge
- For Protobuf descriptor export, users can install `protoc` if needed (CLI will provide instructions)
- External reference URLs are publicly accessible (no authentication required)
- Schema files are encoded in UTF-8 or ASCII
- File system permissions allow reading input files and writing output files
- Network access is available when resolving external references from URLs
- JAR files are valid ZIP archives containing `.proto` files
- Users understand the difference between import (converting external formats to ODCS) and export (converting ODCS to external formats)

## Dependencies

- Existing SDK import modules (SQLImporter, AvroImporter, JSONSchemaImporter, ProtobufImporter, OpenAPIImporter, ODCSImporter)
- Existing SDK export modules (ODCSExporter, AvroExporter, JSONSchemaExporter, ProtobufExporter)
- Existing SDK validation modules (JSON Schema validation, XML validation for BPMN/DMN, input validation)
- External tool dependency: `protoc` (Protocol Buffer compiler) for Protobuf descriptor generation (optional, CLI will check and report if missing)
- HTTP client library for fetching external references from URLs
- ZIP/JAR parsing library for extracting `.proto` files from JAR archives

## Out of Scope

- Interactive mode or REPL interface (CLI is command-based only)
- Authenticated URL access for external references (only non-authenticated URLs supported)
- Schema editing or modification (CLI is import/export only, not an editor)
- Database connectivity (CLI works with files, not live databases)
- Schema versioning or migration tools
- Graphical user interface (GUI)
- Batch processing of multiple files in a single command (users run separate commands for each file)
- Schema registry integration (CLI works with local files and URLs only)
- Real-time schema validation in watch mode
- Integration with version control systems (Git, SVN, etc.)
