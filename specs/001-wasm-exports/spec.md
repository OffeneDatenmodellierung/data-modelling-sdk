# Feature Specification: WASM Module Parsing Function Exports

**Feature Branch**: `001-wasm-exports`
**Created**: 2026-01-02
**Status**: Draft
**Input**: User description: "https://github.com/pixie79/data-modelling-sdk/issues/5"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Parse ODCS YAML Files in Offline Mode (Priority: P1)

Users working with the data modelling web application need to parse ODCS YAML files when operating in offline mode. Currently, the application falls back to a JavaScript YAML parser that may not handle all ODCS 3.1.0 Data Contract format variations correctly and lacks SDK-specific validation and error handling.

**Why this priority**: This is the core functionality blocking offline mode operation. Without proper ODCS parsing, users cannot reliably work with data contracts when disconnected from the network, forcing them to rely on a less robust fallback parser that may produce inconsistent results.

**Independent Test**: Can be fully tested by loading a WASM module in a browser environment, calling the parse function with valid ODCS YAML content, and verifying that the parsed workspace structure matches expected data model structures. This delivers immediate value by enabling reliable offline ODCS file parsing.

**Acceptance Scenarios**:

1. **Given** a WASM module is loaded in a browser environment, **When** a user calls the parse function with valid ODCS 3.1.0 YAML content, **Then** the function returns a parsed workspace structure containing tables, relationships, and metadata
2. **Given** a WASM module is loaded, **When** a user calls the parse function with invalid or malformed YAML content, **Then** the function returns a structured error message indicating the specific parsing issue
3. **Given** a WASM module is loaded, **When** a user calls the parse function with legacy ODCL format content, **Then** the function successfully parses and converts the content to the current format

---

### User Story 2 - Export Data Models to ODCS YAML Format (Priority: P1)

Users need to export their data models to ODCS YAML format from the web application in offline mode. This allows users to save their work locally and share data contracts in the standard format.

**Why this priority**: Export functionality is essential for completing the core workflow - users must be able to save their work. Without WASM-based export, users cannot reliably export data models when offline.

**Independent Test**: Can be fully tested by creating a data model structure in JavaScript, calling the export function via WASM, and verifying that the output YAML matches the ODCS 3.1.0 format specification and contains all expected data model elements. This delivers value by enabling reliable offline data model export.

**Acceptance Scenarios**:

1. **Given** a WASM module is loaded with a complete data model workspace, **When** a user calls the export function, **Then** the function returns valid ODCS 3.1.0 YAML content containing all tables, relationships, and metadata
2. **Given** a WASM module is loaded with a data model containing validation errors, **When** a user calls the export function, **Then** the function either exports with warnings or returns a structured error indicating validation issues
3. **Given** a WASM module is loaded, **When** a user calls the export function with an empty or minimal workspace, **Then** the function returns valid ODCS YAML with appropriate empty structures

---

### User Story 3 - Import Data Models from Multiple Formats (Priority: P2)

Users need to import data models from various formats (SQL, AVRO, JSON Schema, Protobuf) when working offline. This enables users to convert existing data definitions into the SDK's data model format without requiring network connectivity.

**Why this priority**: While not blocking core functionality, import from multiple formats significantly enhances the application's utility by allowing users to work with diverse data sources. This enables migration and conversion workflows that are valuable for data modeling tasks.

**Independent Test**: Can be fully tested independently by calling each import function (SQL, AVRO, JSON Schema, Protobuf) with sample content in their respective formats and verifying that the imported data model structures match expected table and column definitions. This delivers value by enabling format conversion workflows in offline mode.

**Acceptance Scenarios**:

1. **Given** a WASM module is loaded, **When** a user calls the SQL import function with CREATE TABLE statements, **Then** the function returns a data model with tables and columns matching the SQL schema
2. **Given** a WASM module is loaded, **When** a user calls the AVRO import function with AVRO schema content, **Then** the function returns a data model with tables representing the AVRO record structures
3. **Given** a WASM module is loaded, **When** a user calls the JSON Schema import function with a JSON Schema definition, **Then** the function returns a data model with tables representing the schema objects
4. **Given** a WASM module is loaded, **When** a user calls the Protobuf import function with Protobuf schema content, **Then** the function returns a data model with tables representing the Protobuf message structures

---

### User Story 4 - Export Data Models to Multiple Formats (Priority: P2)

Users need to export their data models to various formats (SQL, AVRO, JSON Schema, Protobuf) when working offline. This enables users to generate format-specific outputs for integration with other systems without requiring network connectivity.

**Why this priority**: Export to multiple formats enhances interoperability and enables users to generate outputs for different target systems. While not blocking core functionality, this significantly expands the application's utility for data modeling workflows.

**Independent Test**: Can be fully tested independently by creating a data model in JavaScript, calling each export function (SQL, AVRO, JSON Schema, Protobuf), and verifying that the output matches the respective format specifications and contains all expected data model elements. This delivers value by enabling format conversion and integration workflows in offline mode.

**Acceptance Scenarios**:

1. **Given** a WASM module is loaded with a complete data model, **When** a user calls the SQL export function with a dialect parameter, **Then** the function returns valid SQL CREATE TABLE statements matching the specified dialect (PostgreSQL, MySQL, SQL Server)
2. **Given** a WASM module is loaded with a data model, **When** a user calls the AVRO export function, **Then** the function returns valid AVRO schema content representing the data model tables
3. **Given** a WASM module is loaded with a data model, **When** a user calls the JSON Schema export function, **Then** the function returns valid JSON Schema definitions representing the data model structure
4. **Given** a WASM module is loaded with a data model, **When** a user calls the Protobuf export function, **Then** the function returns valid Protobuf schema content representing the data model tables

---

### Edge Cases

- What happens when a user calls a parsing function with extremely large YAML files (e.g., >10MB)?
- How does the system handle malformed YAML that is syntactically valid but semantically invalid for ODCS format?
- What happens when import functions receive content in an unsupported dialect or format version?
- How does the system handle circular references or deeply nested structures in imported formats?
- What happens when export functions are called with data models containing unsupported features for the target format?
- How does the system handle concurrent calls to parsing/export functions from multiple browser tabs or workers?
- What happens when WASM module initialization fails but parse/export functions are still called?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The WASM module MUST expose a function to parse ODCS YAML content and return a structured workspace representation
- **FR-002**: The WASM module MUST expose a function to export a workspace structure to ODCS YAML format
- **FR-003**: The WASM module MUST expose functions to import data models from SQL, AVRO, JSON Schema, and Protobuf formats
- **FR-004**: The WASM module MUST expose functions to export data models to SQL, AVRO, JSON Schema, and Protobuf formats
- **FR-005**: All parsing and export functions MUST return structured error information when operations fail
- **FR-006**: All parsing and export functions MUST be callable from JavaScript/TypeScript without requiring additional setup beyond module initialization
- **FR-007**: All exported functions MUST be documented in the generated TypeScript definition file (`data_modelling_sdk.d.ts`)
- **FR-008**: The WASM module MUST handle async operations appropriately for browser environments
- **FR-009**: All functions MUST validate input data and return appropriate errors for invalid inputs
- **FR-010**: Export functions MUST support dialect-specific options where applicable (e.g., SQL dialect selection)

### Key Entities *(include if feature involves data)*

- **ODCS Workspace**: Represents a complete data model containing tables, relationships, metadata, and configuration. This is the primary data structure exchanged between JavaScript and WASM.
- **Data Model**: Collection of tables and relationships that define the structure of data. Tables contain columns with types, constraints, and metadata.
- **Import Result**: Contains parsed tables, tables requiring name input, errors/warnings, and optional AI suggestions from import operations.
- **Export Result**: Contains exported content as a string and format identifier for export operations.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can successfully parse ODCS YAML files in offline mode with 100% accuracy compared to the native SDK implementation (no fallback parser needed)
- **SC-002**: All parsing and export functions are accessible from JavaScript within 2 seconds of WASM module initialization
- **SC-003**: Parsing functions handle ODCS 3.1.0 format files up to 5MB in size without performance degradation (parse completes within 3 seconds)
- **SC-004**: Export functions generate format-compliant output that can be successfully imported by the corresponding import functions (100% round-trip compatibility)
- **SC-005**: Error messages from parsing/export functions are clear and actionable, enabling users to resolve issues without consulting documentation in 90% of cases
- **SC-006**: The WASM module reduces reliance on JavaScript fallback parsers by 100% for ODCS format operations (eliminating the need for js-yaml fallback)
