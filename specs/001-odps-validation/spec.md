# Feature Specification: ODPS Schema Validation and Manual Test Script

**Feature Branch**: `001-odps-validation`
**Created**: 2026-01-05
**Status**: Draft
**Input**: User description: "Fix the following issue - https://github.com/OffeneDatenmodellierung/data-modelling-sdk/issues/20 - Add schema validation to ODPS exporter and create manual test script for import/export"

## Clarifications

### Session 2026-01-05

- Q: Should ODPS import operations also validate against the schema, or only exports? → A: Validate both import and export against the schema
- Q: When you say "native format," do you mean ODPS should have CLI support like ODCS, or just library support? → A: Add CLI support: `import odps` and `export odps` commands (like ODCS), but ODPS is standalone (no conversion to other formats)
- Q: Should we add explicit requirements to verify all ODPS schema fields are preserved in round-trips? → A: Add explicit requirements to verify all ODPS schema fields (required and optional) are preserved in round-trips

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Import ODPS Files with Schema Validation (Priority: P1)

A developer imports an ODPS YAML file and receives immediate feedback if the file violates the official ODPS JSON Schema specification. The system validates all required fields, field types, formats, and enum values before parsing the file into an ODPSDataProduct model.

**Why this priority**: This ensures invalid ODPS files are rejected early, preventing downstream errors and ensuring only schema-compliant files are processed. This is critical for treating ODPS as a native format with full validation support.

**Independent Test**: Can be fully tested by importing various ODPS YAML files (valid and invalid) and verifying that validation errors are caught and reported correctly before parsing. This delivers immediate value by ensuring only valid ODPS files are imported.

**Acceptance Scenarios**:

1. **Given** a valid ODPS YAML file with all required fields present, **When** importing the file, **Then** the import succeeds and the file validates against the ODPS JSON Schema before parsing
2. **Given** an ODPS YAML file missing a required field (e.g., `id`), **When** importing the file, **Then** the import fails with a clear error message indicating which required field is missing
3. **Given** an ODPS YAML file with an invalid enum value (e.g., `status: "invalid"`), **When** importing the file, **Then** the import fails with an error message indicating the invalid enum value and valid options
4. **Given** an ODPS YAML file with an invalid URL format in a `Support` object, **When** importing the file, **Then** the import fails with an error message indicating the field path and expected format
5. **Given** an ODPS YAML file with a missing required nested field (e.g., `Support` object missing `channel`), **When** importing the file, **Then** the import fails with an error message indicating the missing nested field

---

### User Story 2 - Export ODPS Files with Schema Validation (Priority: P1)

A developer exports a data product to ODPS YAML format and receives immediate feedback if the exported file violates the official ODPS JSON Schema specification. The system validates all required fields, field types, formats, and enum values before completing the export.

**Why this priority**: This is the core functionality requested in the issue. Without schema validation, exported ODPS files may fail when used with ODPS-compliant tools, causing interoperability issues and user frustration.

**Independent Test**: Can be fully tested by exporting a data product with various configurations (valid and invalid) and verifying that validation errors are caught and reported correctly. This delivers immediate value by ensuring exported files are schema-compliant.

**Acceptance Scenarios**:

1. **Given** a valid ODPSDataProduct with all required fields present, **When** exporting to ODPS YAML format, **Then** the export succeeds and the output file validates against the ODPS JSON Schema
2. **Given** an ODPSDataProduct missing a required field (e.g., `id`), **When** exporting to ODPS YAML format, **Then** the export fails with a clear error message indicating which required field is missing
3. **Given** an ODPSDataProduct with an invalid enum value (e.g., `status: "invalid"`), **When** exporting to ODPS YAML format, **Then** the export fails with an error message indicating the invalid enum value and valid options
4. **Given** an ODPSDataProduct with an invalid URL format in a `Support` object, **When** exporting to ODPS YAML format, **Then** the export fails with an error message indicating the field path and expected format
5. **Given** an ODPSDataProduct with a missing required nested field (e.g., `Support` object missing `channel`), **When** exporting to ODPS YAML format, **Then** the export fails with an error message indicating the missing nested field

---

### User Story 3 - CLI Support for ODPS Import/Export (Priority: P2)

A developer uses the CLI to import and export ODPS files as a native format, similar to how ODCS is supported. ODPS is treated as a standalone format that does not convert to other formats.

**Why this priority**: This makes ODPS a first-class format in the CLI, enabling direct usage without requiring library integration. ODPS remains standalone (no conversion to ODCS or other formats), maintaining its distinct purpose as a data product specification format.

**Independent Test**: Can be fully tested by using CLI commands `data-modelling-cli import odps` and `data-modelling-cli export odps` with various ODPS files and verifying correct import/export behavior. This delivers value by providing command-line access to ODPS functionality.

**Acceptance Scenarios**:

1. **Given** a valid ODPS YAML file, **When** running `data-modelling-cli import odps file.odps.yaml`, **Then** the CLI imports the file, validates it against the schema, and displays imported data
2. **Given** an ODPSDataProduct in memory, **When** running `data-modelling-cli export odps input.odcs.yaml output.odps.yaml`, **Then** the CLI exports to ODPS format (note: ODPS export uses ODCS input format, not direct ODPS-to-ODPS conversion)
3. **Given** an invalid ODPS YAML file, **When** running `data-modelling-cli import odps file.odps.yaml`, **Then** the CLI reports validation errors and does not proceed with import
4. **Given** ODPS is used in CLI commands, **When** attempting to convert ODPS to other formats, **Then** the system does not support conversion (ODPS is standalone)

---

### User Story 4 - Manual ODPS Import/Export Testing Script (Priority: P3)

A developer or user can run a standalone test script that accepts an ODPS YAML file, imports it, displays what data was captured, and then exports it back to verify round-trip functionality and schema compliance.

**Why this priority**: This provides a practical tool for users to verify ODPS import/export functionality manually, helping with debugging and validation workflows. It complements the automated validation by providing a user-friendly testing interface.

**Independent Test**: Can be fully tested by running the script with various ODPS files (valid and invalid) and verifying that it correctly displays imported data and exports valid ODPS YAML. This delivers value by enabling users to test and debug ODPS workflows independently.

**Acceptance Scenarios**:

1. **Given** a valid ODPS YAML file, **When** running the test script with the file path, **Then** the script imports the file, displays captured data (tables, columns, metadata), and exports a valid ODPS YAML file
2. **Given** an invalid ODPS YAML file (missing required fields), **When** running the test script, **Then** the script reports validation errors and does not proceed with export
3. **Given** an ODPS YAML file with custom properties and tags, **When** running the test script, **Then** the script displays all captured metadata including custom properties and tags
4. **Given** the test script is run without arguments, **When** executed, **Then** it displays usage instructions and examples

---

### User Story 5 - Field Preservation Verification (Priority: P3)

A developer imports an ODPS file and exports it back, verifying that all fields defined in the ODPS schema (both required and optional) are preserved with 100% accuracy during the round-trip.

**Why this priority**: This ensures complete fidelity when handling ODPS files, guaranteeing that no data is lost during import/export operations. This is essential for treating ODPS as a native format with full field support.

**Independent Test**: Can be fully tested by importing an ODPS file with all possible fields populated, exporting it, and comparing field-by-field to verify complete preservation. This delivers value by ensuring data integrity.

**Acceptance Scenarios**:

1. **Given** an ODPS file with all optional fields populated (customProperties, tags, authoritativeDefinitions, support, team, etc.), **When** importing and exporting the file, **Then** all fields are preserved exactly as in the original
2. **Given** an ODPS file with nested structures (inputPorts with customProperties, outputPorts with tags), **When** importing and exporting the file, **Then** all nested fields are preserved
3. **Given** an ODPS file with empty optional arrays/objects, **When** importing and exporting the file, **Then** the structure is preserved (empty arrays/objects remain empty, not omitted)

---

### User Story 6 - Optional Validation via Feature Flag (Priority: P4)

Developers can enable or disable ODPS schema validation via a feature flag, allowing gradual adoption and backward compatibility during the migration period.

**Why this priority**: This ensures backward compatibility and allows teams to adopt validation at their own pace. However, it's lower priority than core validation functionality since it's primarily a deployment/configuration concern.

**Independent Test**: Can be fully tested by building the SDK with and without the validation feature flag and verifying that validation only occurs when enabled. This delivers value by maintaining backward compatibility.

**Acceptance Scenarios**:

1. **Given** the SDK is built with the `odps-validation` feature enabled, **When** exporting an ODPS file, **Then** schema validation is performed
2. **Given** the SDK is built without the `odps-validation` feature, **When** exporting an ODPS file, **Then** export proceeds without validation (backward compatible behavior)
3. **Given** validation is enabled but fails, **When** exporting an ODPS file, **Then** the export operation fails with validation error details

---

### Edge Cases

- What happens when the ODPS JSON Schema file is missing or corrupted?
- How does the system handle validation when optional nested objects are present but incomplete (e.g., `Support` object with `channel` but missing `url`)?
- What happens when an array field contains invalid items (e.g., `inputPorts` array contains an object missing required `contractId`)?
- How does validation handle date-time format validation for `productCreatedTs`?
- What happens when custom properties contain invalid value types?
- How does the test script handle very large ODPS files (performance considerations)?
- What happens when the test script is run with a non-existent file path?
- What happens when a user attempts to convert ODPS to another format via CLI (should be rejected)?
- How does field preservation handle fields with null values vs. omitted fields?
- What happens when an ODPS file contains fields not defined in the schema (should validation reject them)?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST validate imported ODPS YAML files against the official ODPS JSON Schema specification (`schemas/odps-json-schema-latest.json`) before parsing
- **FR-002**: System MUST validate exported ODPS YAML files against the official ODPS JSON Schema specification (`schemas/odps-json-schema-latest.json`) before completing export
- **FR-003**: System MUST enforce presence of all required fields during import validation: `apiVersion`, `kind`, `id`, `status`
- **FR-004**: System MUST enforce presence of all required fields during export validation: `apiVersion`, `kind`, `id`, `status`
- **FR-005**: System MUST validate that `apiVersion` is one of the allowed enum values: `["v0.9.0", "v1.0.0"]` (both import and export)
- **FR-006**: System MUST validate that `kind` equals `"DataProduct"` (both import and export)
- **FR-007**: System MUST validate that `status` is one of the allowed enum values: `["proposed", "draft", "active", "deprecated", "retired"]` (both import and export)
- **FR-008**: System MUST validate that `id` is a valid UUID format string (both import and export)
- **FR-009**: System MUST validate required nested fields in `Support` objects: `channel` and `url` must be present (both import and export)
- **FR-010**: System MUST validate required nested fields in `InputPort` objects: `name`, `version`, and `contractId` must be present (both import and export)
- **FR-011**: System MUST validate required nested fields in `OutputPort` objects: `name` and `version` must be present (both import and export)
- **FR-012**: System MUST validate required nested fields in `CustomProperty` objects: `property` and `value` must be present (both import and export)
- **FR-013**: System MUST validate required nested fields in `AuthoritativeDefinition` objects: `type` and `url` must be present (both import and export)
- **FR-014**: System MUST validate required nested fields in `TeamMember` objects: `username` must be present (both import and export)
- **FR-015**: System MUST validate URL format fields (e.g., `url` in `Support`, `AuthoritativeDefinition`) conform to URI format requirements (both import and export)
- **FR-016**: System MUST validate date-time format for `productCreatedTs` field (ISO 8601 format) (both import and export)
- **FR-017**: System MUST provide detailed validation error messages that include: field path (e.g., `support[0].url`), expected type/format, actual value, and schema requirement reference
- **FR-018**: System MUST make validation optional via feature flag (`odps-validation`) that can be enabled or disabled at build time (applies to both import and export)
- **FR-019**: System MUST provide CLI commands `data-modelling-cli import odps <file>` and `data-modelling-cli export odps <input> <output>` for ODPS format
- **FR-020**: CLI import command MUST validate imported ODPS files against the schema before parsing
- **FR-021**: CLI export command MUST validate exported ODPS files against the schema before writing
- **FR-022**: System MUST NOT support conversion of ODPS to other formats (ODPS is standalone, no ODPS→ODCS or ODPS→other conversions)
- **FR-023**: System MUST preserve all ODPS schema fields (required and optional) during import/export round-trips with 100% accuracy
- **FR-024**: System MUST preserve all optional fields including: `name`, `version`, `domain`, `tenant`, `tags`, `description` (with all sub-fields), `authoritativeDefinitions`, `customProperties`, `inputPorts`, `outputPorts`, `managementPorts`, `support`, `team`, `productCreatedTs`
- **FR-025**: System MUST preserve nested structures within optional fields (e.g., `customProperties` within `inputPorts`, `tags` within `support` objects)
- **FR-026**: System MUST preserve empty optional arrays/objects (not omit them) to maintain structural consistency
- **FR-027**: System MUST provide a manual test script that accepts an ODPS YAML file path as input
- **FR-028**: The test script MUST import the ODPS file and display captured data (tables, columns, metadata, custom properties, tags)
- **FR-029**: The test script MUST export the imported data back to ODPS YAML format
- **FR-030**: The test script MUST validate both imported and exported ODPS YAML against the schema and report validation results
- **FR-031**: The test script MUST verify field preservation by comparing original and exported files field-by-field
- **FR-032**: The test script MUST display clear error messages if import or export fails
- **FR-033**: The test script MUST display usage instructions when run without arguments or with `--help`
- **FR-034**: System MUST handle validation errors gracefully without crashing, returning structured error information instead

### Key Entities *(include if feature involves data)*

- **ODPSDataProduct**: Represents a data product in ODPS format, containing metadata, ports, support information, team details, and custom properties
- **ODPS JSON Schema**: The official schema specification that defines valid structure, required fields, types, formats, and constraints for ODPS files
- **Validation Error**: Contains field path, expected value/type/format, actual value, and error message describing the violation
- **Test Script**: A standalone executable that performs import/export round-trip testing with ODPS files, displaying captured data and validation results

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of imported ODPS files pass validation against the official ODPS JSON Schema when validation is enabled
- **SC-002**: 100% of exported ODPS files pass validation against the official ODPS JSON Schema when validation is enabled
- **SC-003**: Validation errors are reported within 2 seconds for ODPS files up to 1MB in size (both import and export)
- **SC-004**: Validation error messages include sufficient detail (field path, expected value, actual value) for developers to fix issues without consulting external documentation in 90% of cases
- **SC-005**: All ODPS schema fields (required and optional) are preserved with 100% accuracy during import/export round-trips
- **SC-006**: CLI commands for ODPS import/export are available and functional, matching the user experience of ODCS CLI commands
- **SC-007**: The manual test script successfully imports and exports valid ODPS files with 100% field preservation accuracy
- **SC-008**: The test script displays captured data in a human-readable format that allows users to verify import correctness within 30 seconds for typical ODPS files
- **SC-009**: When validation is disabled (feature flag off), import and export performance is not degraded compared to current implementation (no more than 5% overhead)
- **SC-010**: Invalid ODPS files are rejected with clear error messages in 100% of validation failure cases (both import and export)

## Assumptions

- The ODPS JSON Schema file (`schemas/odps-json-schema-latest.json`) is available at compile time or runtime and can be loaded reliably
- Users will enable the `odps-validation` feature flag when they want validation; backward compatibility is maintained when disabled
- The JSON Schema validation library (`jsonschema` crate) supports the schema format used by ODPS (Draft 2019-09)
- ODPS files imported via the test script will be valid YAML format (YAML parsing errors are separate from schema validation)
- The test script will be used primarily for development and debugging purposes, not in production workflows
- Performance requirements assume typical ODPS file sizes (under 1MB); larger files may take longer to validate

## Dependencies

- JSON Schema validation library (e.g., `jsonschema` crate version 0.18 or compatible)
- Access to the ODPS JSON Schema specification file
- Existing ODPS exporter implementation (`ODPSExporter`)
- Existing ODPS importer implementation (`ODPSImporter`)
- CLI infrastructure (for CLI commands and test script command-line interface)

## Out of Scope

- Modifying the ODPS JSON Schema specification itself
- Adding new ODPS schema features or fields
- Converting ODPS to other formats (ODPS is standalone - no ODPS→ODCS, ODPS→JSON Schema, etc. conversions)
- Performance optimization for very large ODPS files (>10MB)
- Real-time validation during data product editing (only import/export-time validation)
- Integration with external ODPS validation services
- Automatic fixing of validation errors (only reporting)
- WASM-specific validation optimizations (covered by existing WASM bindings)
