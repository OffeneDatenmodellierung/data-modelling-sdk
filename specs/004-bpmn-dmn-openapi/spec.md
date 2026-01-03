# Feature Specification: BPMN, DMN, and OpenAPI Schema Support

**Feature Branch**: `004-bpmn-dmn-openapi`
**Created**: 2026-01-03
**Status**: Draft
**Input**: User description: "I would like to now extend support to BPMN models, DMN Models and OpenAPI Schemas. We should store these in their native formats, Allowing CADS to reference them. We must support import and export along with file storage following the domain model and enable WASM methods for all. We must also offer an OpenAPI to ODCS convertor so that if needed we can convert an OpenAPI data elements to an ODCS table for example where we have an API that writes to a table - this will save duplication but these are then two seperate nodes. The frontend for BPMN and DMN will use the node modules from bpmn-js and dmn-js. Make sure we also store the current schema XFD's for BPMN/DMN long with the OpenAPI 3.1.1 JSON spec to the schema dir in the base directory"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Import and Store BPMN Models (Priority: P1)

A data architect needs to import BPMN process models into the system to document business processes that interact with data systems. The BPMN files should be stored in their native XML format within the domain structure, allowing them to be referenced by CADS assets.

**Why this priority**: BPMN models are fundamental for documenting business processes and their relationship to data systems. This is the foundation for linking process models to compute assets.

**Independent Test**: Can be fully tested by importing a valid BPMN 2.0 XML file and verifying it is stored correctly in the domain directory structure. Delivers immediate value by enabling process documentation.

**Acceptance Scenarios**:

1. **Given** a valid BPMN 2.0 XML file, **When** a user imports it into a domain, **Then** the file is stored as `{domain_name}/{model_name}.bpmn.xml` and can be referenced by CADS assets
2. **Given** an invalid BPMN XML file, **When** a user attempts to import it, **Then** the system returns a clear error message indicating what is wrong with the file
3. **Given** a BPMN file with the same name already exists in the domain, **When** a user imports a new file, **Then** the system handles the conflict appropriately (either overwrites with confirmation or creates a unique filename)

---

### User Story 2 - Import and Store DMN Models (Priority: P1)

A business analyst needs to import DMN decision models to document business rules and decisions that affect data processing. DMN files should be stored in their native XML format and be referenceable by CADS assets.

**Why this priority**: DMN models document business rules and decisions, which are critical for understanding how data is processed and transformed. This is equally foundational as BPMN support.

**Independent Test**: Can be fully tested by importing a valid DMN 1.3 XML file and verifying it is stored correctly. Delivers value by enabling decision rule documentation.

**Acceptance Scenarios**:

1. **Given** a valid DMN 1.3 XML file, **When** a user imports it into a domain, **Then** the file is stored as `{domain_name}/{model_name}.dmn.xml` and can be referenced by CADS assets
2. **Given** an invalid DMN XML file, **When** a user attempts to import it, **Then** the system returns a clear error message indicating validation failures
3. **Given** a DMN file is imported, **When** a CADS asset references it, **Then** the reference is validated and the file can be located

---

### User Story 3 - Import and Store OpenAPI Schemas (Priority: P1)

An API developer needs to import OpenAPI 3.1.1 specifications to document APIs that interact with data systems. OpenAPI files should be stored in their native YAML or JSON format within the domain structure.

**Why this priority**: OpenAPI schemas document APIs that often read from or write to data systems. This is essential for understanding the complete data flow including API interactions.

**Independent Test**: Can be fully tested by importing a valid OpenAPI 3.1.1 YAML or JSON file and verifying it is stored correctly. Delivers value by enabling API documentation within the data modeling context.

**Acceptance Scenarios**:

1. **Given** a valid OpenAPI 3.1.1 YAML file, **When** a user imports it into a domain, **Then** the file is stored as `{domain_name}/{api_name}.openapi.yaml` and can be referenced by CADS assets
2. **Given** a valid OpenAPI 3.1.1 JSON file, **When** a user imports it, **Then** the file is stored as `{domain_name}/{api_name}.openapi.json` and format is preserved
3. **Given** an invalid OpenAPI file, **When** a user attempts to import it, **Then** the system validates against the OpenAPI 3.1.1 schema and returns specific validation errors

---

### User Story 4 - CADS Asset References to BPMN/DMN/OpenAPI (Priority: P2)

A data architect needs to link CADS assets (applications, ETL pipelines, etc.) to their associated BPMN process models, DMN decision models, or OpenAPI specifications to create a complete picture of how systems interact.

**Why this priority**: Linking CADS assets to process models, decision models, and API specs creates a comprehensive view of the system architecture. This enables better documentation and understanding.

**Independent Test**: Can be fully tested by creating a CADS asset and adding references to BPMN, DMN, or OpenAPI files. Delivers value by enabling cross-referencing between different model types.

**Acceptance Scenarios**:

1. **Given** a CADS asset exists in a domain, **When** a user adds a reference to a BPMN file in the same domain, **Then** the reference is stored and validated to ensure the file exists
2. **Given** a CADS asset references a BPMN file, **When** the BPMN file is deleted, **Then** the system warns about broken references or handles it gracefully
3. **Given** a CADS asset references an OpenAPI spec, **When** viewing the asset, **Then** users can navigate to the referenced OpenAPI specification

---

### User Story 5 - Export BPMN/DMN/OpenAPI Models (Priority: P2)

A user needs to export BPMN, DMN, or OpenAPI models from the system to share with other tools or team members, maintaining the original format and content.

**Why this priority**: Export functionality enables interoperability with other tools and allows users to work with models outside the system. This is essential for practical workflow integration.

**Independent Test**: Can be fully tested by exporting a previously imported model and verifying the exported file matches the original format and content. Delivers value by enabling tool interoperability.

**Acceptance Scenarios**:

1. **Given** a BPMN model is stored in a domain, **When** a user exports it, **Then** the exported XML file matches the original format and can be opened in standard BPMN tools
2. **Given** an OpenAPI model is stored, **When** a user exports it, **Then** the exported file maintains the original format (YAML or JSON) and is valid OpenAPI 3.1.1
3. **Given** a model is exported, **When** it is imported again, **Then** it can be successfully imported without data loss

---

### User Story 6 - OpenAPI to ODCS Converter (Priority: P2)

A data architect needs to convert OpenAPI schema definitions to ODCS table definitions when an API writes to a database table, avoiding duplication while maintaining separate nodes for the API and the table.

**Why this priority**: This enables reuse of schema definitions between APIs and data tables, reducing duplication while maintaining clear separation between API contracts and data contracts. This is valuable for APIs that directly interact with databases.

**Independent Test**: Can be fully tested by converting an OpenAPI schema component to an ODCS table and verifying the fields are correctly mapped. Delivers value by reducing schema duplication and enabling better traceability.

**Acceptance Scenarios**:

1. **Given** an OpenAPI schema with a component definition, **When** a user converts it to ODCS format, **Then** the component fields are mapped to ODCS table columns with appropriate data types
2. **Given** an OpenAPI schema with nested objects, **When** converting to ODCS, **Then** the system handles nested structures appropriately (either flattening or creating related tables)
3. **Given** an OpenAPI schema is converted to ODCS, **When** both are stored, **Then** they remain as separate nodes (API node and Table node) but share schema definitions
4. **Given** an OpenAPI schema with validation rules (min/max, patterns, etc.), **When** converting to ODCS, **Then** these constraints are preserved in the ODCS quality rules where applicable

---

### User Story 7 - WASM Methods for BPMN/DMN/OpenAPI Operations (Priority: P3)

A frontend developer needs to use JavaScript/TypeScript to import, export, and manage BPMN, DMN, and OpenAPI models through WASM bindings, enabling web-based tooling integration.

**Why this priority**: WASM bindings enable frontend integration and allow the use of bpmn-js and dmn-js libraries. This is important for user experience but depends on the core import/export functionality being complete first.

**Independent Test**: Can be fully tested by calling WASM methods from JavaScript to import a model and verify it is stored correctly. Delivers value by enabling web-based tooling.

**Acceptance Scenarios**:

1. **Given** a JavaScript application loads the WASM module, **When** calling `importBpmnModel(domainId, bpmnXml)`, **Then** the model is imported and stored in the specified domain
2. **Given** a BPMN model exists in a domain, **When** calling `exportBpmnModel(domainId, modelName)`, **Then** the model XML is returned as a string
3. **Given** an OpenAPI model exists, **When** calling `convertOpenApiToOdcs(openApiYaml)`, **Then** the converted ODCS YAML is returned

---

### Edge Cases

- What happens when a BPMN file references external resources (e.g., DMN files, other BPMN files) that are not yet imported?
- How does the system handle BPMN files with embedded DMN decision tables?
- What happens when an OpenAPI schema references external schema files ($ref to external URLs)?
- How does the system handle very large BPMN/DMN files (e.g., >10MB)?
- What happens when importing a BPMN file with a different BPMN version (e.g., 2.0.2 vs 2.0.3)?
- How does the system handle OpenAPI files with circular references in schema definitions?
- What happens when converting an OpenAPI schema that uses advanced features not supported in ODCS (e.g., discriminator, oneOf)?
- How does the system handle BPMN/DMN files with custom extensions or non-standard elements?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support importing BPMN 2.0 XML files and storing them in native XML format within domain directories
- **FR-002**: System MUST support importing DMN 1.3 XML files and storing them in native XML format within domain directories
- **FR-003**: System MUST support importing OpenAPI 3.1.1 specifications in both YAML and JSON formats, preserving the original format
- **FR-004**: System MUST validate BPMN files against BPMN 2.0 XSD schema before storing
- **FR-005**: System MUST validate DMN files against DMN 1.3 XSD schema before storing
- **FR-006**: System MUST validate OpenAPI files against OpenAPI 3.1.1 JSON Schema before storing
- **FR-007**: System MUST store BPMN files as `{domain_name}/{model_name}.bpmn.xml` following the domain-based file structure
- **FR-008**: System MUST store DMN files as `{domain_name}/{model_name}.dmn.xml` following the domain-based file structure
- **FR-009**: System MUST store OpenAPI files as `{domain_name}/{api_name}.openapi.yaml` or `{domain_name}/{api_name}.openapi.json` following the domain-based file structure
- **FR-010**: System MUST allow CADS assets to reference BPMN, DMN, and OpenAPI files within the same domain or across domains
- **FR-011**: System MUST validate that referenced BPMN/DMN/OpenAPI files exist when creating or updating CADS asset references
- **FR-012**: System MUST support exporting BPMN models in their original XML format
- **FR-013**: System MUST support exporting DMN models in their original XML format
- **FR-014**: System MUST support exporting OpenAPI specifications in their original format (YAML or JSON)
- **FR-015**: System MUST provide a converter function to transform OpenAPI schema components to ODCS table definitions
- **FR-016**: System MUST map OpenAPI data types to ODCS field types appropriately (e.g., string → text, integer → long, number → double)
- **FR-017**: System MUST preserve OpenAPI validation constraints (min/max, patterns, enums) in ODCS quality rules where applicable
- **FR-018**: System MUST handle OpenAPI nested objects appropriately during conversion (either flattening or creating related tables)
- **FR-019**: System MUST maintain separate nodes for OpenAPI APIs and ODCS tables even when they share schema definitions
- **FR-020**: System MUST provide WASM bindings for importing BPMN models (`importBpmnModel`)
- **FR-021**: System MUST provide WASM bindings for exporting BPMN models (`exportBpmnModel`)
- **FR-022**: System MUST provide WASM bindings for importing DMN models (`importDmnModel`)
- **FR-023**: System MUST provide WASM bindings for exporting DMN models (`exportDmnModel`)
- **FR-024**: System MUST provide WASM bindings for importing OpenAPI specifications (`importOpenApiSpec`)
- **FR-025**: System MUST provide WASM bindings for exporting OpenAPI specifications (`exportOpenApiSpec`)
- **FR-026**: System MUST provide WASM bindings for OpenAPI to ODCS conversion (`convertOpenApiToOdcs`)
- **FR-027**: System MUST store BPMN 2.0 XSD schema files in the `schemas/` directory
- **FR-028**: System MUST store DMN 1.3 XSD schema files in the `schemas/` directory
- **FR-029**: System MUST store OpenAPI 3.1.1 JSON Schema specification in the `schemas/` directory
- **FR-030**: System MUST provide clear error messages when validation fails, indicating the specific validation error and location in the file
- **FR-031**: System MUST handle file naming conflicts when importing models with duplicate names (either overwrite with confirmation or generate unique names)
- **FR-032**: System MUST support listing all BPMN, DMN, and OpenAPI models within a domain
- **FR-033**: System MUST support deleting BPMN, DMN, and OpenAPI models, with appropriate handling of broken references in CADS assets

### Key Entities *(include if feature involves data)*

- **BPMN Model**: A business process model stored in BPMN 2.0 XML format. Contains process definitions, tasks, gateways, and flow elements. Stored within domain directories and referenceable by CADS assets.

- **DMN Model**: A decision model stored in DMN 1.3 XML format. Contains decision tables, business rules, and decision logic. Stored within domain directories and referenceable by CADS assets.

- **OpenAPI Specification**: An API specification stored in OpenAPI 3.1.1 YAML or JSON format. Contains API endpoints, request/response schemas, and API metadata. Stored within domain directories and referenceable by CADS assets.

- **CADS Asset Reference**: A reference from a CADS asset to a BPMN, DMN, or OpenAPI model. Includes the model type, file path, and optional description. Validated to ensure the referenced file exists.

- **OpenAPI to ODCS Conversion**: A conversion process that transforms OpenAPI schema component definitions into ODCS table definitions, mapping data types and preserving constraints where applicable. Results in separate nodes for the API and the table.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can import a BPMN 2.0 XML file and have it stored correctly within 2 seconds for files up to 1MB
- **SC-002**: Users can import a DMN 1.3 XML file and have it validated and stored correctly within 2 seconds for files up to 1MB
- **SC-003**: Users can import an OpenAPI 3.1.1 specification and have it validated and stored correctly within 3 seconds for files up to 500KB
- **SC-004**: System validates BPMN files with 100% accuracy against BPMN 2.0 XSD schema, rejecting invalid files with specific error messages
- **SC-005**: System validates DMN files with 100% accuracy against DMN 1.3 XSD schema, rejecting invalid files with specific error messages
- **SC-006**: System validates OpenAPI files with 100% accuracy against OpenAPI 3.1.1 JSON Schema, rejecting invalid files with specific error messages
- **SC-007**: Users can successfully convert OpenAPI schema components to ODCS tables with 95% of common data types correctly mapped
- **SC-008**: CADS asset references to BPMN/DMN/OpenAPI files are validated within 100ms, ensuring referenced files exist
- **SC-009**: WASM methods for BPMN/DMN/OpenAPI operations complete within 500ms for typical file sizes (up to 500KB)
- **SC-010**: Exported BPMN/DMN/OpenAPI files can be successfully opened in standard tools (bpmn-js, dmn-js, Swagger UI) without format corruption
- **SC-011**: System handles file naming conflicts gracefully, either generating unique names or prompting for confirmation, within 1 second
- **SC-012**: Users can list all BPMN, DMN, and OpenAPI models in a domain within 200ms for domains with up to 100 models

## Assumptions

- BPMN files will be in BPMN 2.0 format (XSD 2.0.2 or later)
- DMN files will be in DMN 1.3 format
- OpenAPI files will be in OpenAPI 3.1.1 format
- Frontend will use bpmn-js and dmn-js libraries for visualization (not part of SDK scope)
- BPMN/DMN files are typically under 10MB in size
- OpenAPI files are typically under 5MB in size
- Users understand that OpenAPI to ODCS conversion creates separate nodes (API node and Table node) even when sharing schema definitions
- Schema XSD files for BPMN/DMN and OpenAPI JSON Schema will be obtained from official sources (OMG for BPMN/DMN, OpenAPI Initiative for OpenAPI)
- CADS assets can reference models across domains (not just within the same domain)
- File format preservation is important (YAML vs JSON for OpenAPI, XML structure for BPMN/DMN)

## Dependencies

- BPMN 2.0 XSD schema files (to be stored in `schemas/` directory)
- DMN 1.3 XSD schema files (to be stored in `schemas/` directory)
- OpenAPI 3.1.1 JSON Schema specification (to be stored in `schemas/` directory)
- XML parsing and validation libraries (for BPMN/DMN)
- YAML/JSON parsing libraries (for OpenAPI, already available)
- JSON Schema validation library (for OpenAPI validation, already available via jsonschema crate)
- Frontend libraries: bpmn-js and dmn-js (external dependencies, not part of SDK)

## Out of Scope

- Visual editing of BPMN/DMN models within the SDK (handled by frontend with bpmn-js/dmn-js)
- Execution or simulation of BPMN/DMN models
- API code generation from OpenAPI specs
- Automatic discovery of BPMN/DMN/OpenAPI files from external sources
- Version control integration for model files
- Collaborative editing of models
- Model transformation or migration between versions
- Integration with BPMN/DMN execution engines
- OpenAPI server code generation
