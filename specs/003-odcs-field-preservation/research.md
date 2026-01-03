# Research: Complete ODCS/ODCL Field Preservation & Universal Format Conversion

**Date**: 2026-01-27
**Feature**: Complete ODCS/ODCL Field Preservation & Universal Format Conversion
**Purpose**: Document design decisions and research findings for field preservation and universal format conversion

## Phase 0: Research Findings

### ODCS v3.1.0 Schema Coverage Analysis

**Decision**: Audit and ensure 100% coverage of ODCS v3.1.0 specification fields

**Rationale**:
- ODCS v3.1.0 is the primary format and must support complete field preservation
- Current implementation may be missing specification fields beyond description, quality, and $ref
- Need to verify all specification-compliant fields are parsed and preserved
- Full example from [official ODCS repository](https://raw.githubusercontent.com/bitol-io/open-data-contract-standard/refs/heads/main/docs/examples/all/full-example.odcs.yaml) shows extensive field structure

**ODCS v3.1.0 Key Fields** (from specification and full example):

**Root Level Fields**:
- `apiVersion`, `kind`, `id`, `version`, `name`, `domain`, `dataProduct`, `status`, `description` (with nested: purpose, limitations, usage, authoritativeDefinitions), `tenant`, `authoritativeDefinitions`, `servers`, `schema` (array), `price`, `team`, `roles`, `slaProperties`, `support`, `tags`, `customProperties`, `contractCreatedTs`

**Schema Level Fields** (each schema object in array):
- `id`, `name`, `physicalName`, `physicalType`, `businessName`, `description`, `authoritativeDefinitions`, `tags`, `dataGranularityDescription`, `relationships` (array), `properties` (array), `quality` (array), `customProperties`

**Property Level Fields** (each property in properties array):
- `id`, `name`, `physicalName`, `primaryKey`, `primaryKeyPosition`, `businessName`, `logicalType`, `physicalType`, `required`, `description`, `partitioned`, `partitionKeyPosition`, `criticalDataElement`, `tags`, `classification`, `transformSourceObjects`, `transformLogic`, `transformDescription`, `examples` (array), `customProperties`, `relationships` (array), `authoritativeDefinitions`, `encryptedName`, `quality` (array), `unique`, `minLength`, `maxLength`, `format`, `pattern`, `$ref`

**Current Coverage**:
- ✅ Parsed: `name`, `type` (from `logicalType`/`physicalType`), `nullable` (from `required`), `primaryKey`
- ❌ Missing: `description`, `quality`, `$ref`, `examples`, `format`, `pattern`, `minLength`, `maxLength`, `classification`, `tags`, `customProperties`, `relationships`, `authoritativeDefinitions`, and many more

**Action Required**:
- **Phase 1 (Critical)**: Extend parsing to include `description`, `quality`, `$ref` (addresses issue #9)
- **Phase 2 (Future)**: Extend parsing to include all other specification fields for complete compliance

**Test Fixtures**:
- Full ODCS example saved to `test-fixtures/full-example.odcs.yaml`
- ODCL example saved to `test-fixtures/example.odcl.yaml`

**Alternatives Considered**:
- Only fix critical fields (description, quality, $ref): Rejected - incomplete compliance
- Parse all fields but only preserve in Column struct: Rejected - need to expose via ColumnData

### ODCL Schema Coverage Analysis

**Decision**: Ensure complete ODCL (Data Contract Specification) field coverage

**Rationale**:
- ODCL is a legacy format but still widely used
- Current implementation converts ODCL to ODCS internally, but may lose fields during conversion
- Need to preserve all ODCL-specific fields during conversion

**ODCL Key Fields** (from specification and example):

**Root Level Fields**:
- `dataContractSpecification`, `id`, `info` (with nested: title, version, description, owner, status, contact), `servers`, `terms` (with nested: usage, limitations, policies, billing, noticePeriod), `models`, `definitions`, `servicelevels`, `tags`, `links`

**Model Level Fields** (each model in models):
- `description`, `type`, `fields` (object with field definitions), `quality` (array), `examples` (array), `primaryKey` (array)

**Field Level Fields** (each field in fields object):
- `$ref`, `required`, `unique`, `primaryKey`, `description`, `type`, `examples` (array), `tags` (array), `quality` (array), `minLength`, `maxLength`, `format`, `pii`, `classification`, `lineage`, `config`, `references`, `pattern`

**Current Coverage**:
- ✅ Parsed: `name`, `type`, `required` (mapped to nullable)
- ❌ Missing: `description`, `quality`, `$ref`, `examples`, `format`, `pattern`, `minLength`, `maxLength`, `tags`, `pii`, `classification`, `lineage`, `config`, `references`, and more

**Action Required**:
- **Phase 1 (Critical)**: Extend parsing to include `description`, `quality`, `$ref` (addresses issue #9)
- **Phase 2 (Future)**: Extend parsing to include all other ODCL fields for complete compliance

**Action Required**: Extend ODCL parsing to preserve all fields before conversion to ODCS

**Alternatives Considered**:
- Only preserve in Column struct, not ColumnData: Rejected - frontend needs access
- Create separate ODCL-specific structures: Rejected - adds complexity, ODCS is target format

### Other Import Format Coverage Analysis

**Decision**: Audit field coverage for SQL, JSON Schema, AVRO, and Protobuf importers

**Rationale**:
- Each format has different capabilities and metadata support
- Need to understand what fields can be extracted from each format
- Universal converter must handle format-specific limitations gracefully

**SQL Format Coverage**:
- **Capabilities**: Table names, column names, data types, nullable, primary keys, foreign keys, constraints, indexes
- **Limitations**: No description, quality rules, or $ref support in standard SQL DDL
- **Action**: Document limitations, preserve what can be extracted

**JSON Schema Coverage**:
- **Capabilities**: `description`, `type`, `required`, `enum`, `format`, `pattern`, `minLength`, `maxLength`, `minimum`, `maximum`, `default`, `examples`, `$ref`
- **Limitations**: No native quality rules support (would need custom extensions)
- **Action**: Extract all JSON Schema fields, map to ODCS equivalents

**AVRO Coverage**:
- **Capabilities**: Field names, types, `doc` (description), `default`, `aliases`, `logicalType`
- **Limitations**: No quality rules, limited constraint support
- **Action**: Map `doc` to `description`, preserve `default` and `logicalType` as custom properties

**Protobuf Coverage**:
- **Capabilities**: Field names, types, `optional`/`required`, field numbers, `repeated`, `oneof`, `map`, comments (via `//`)
- **Limitations**: No description field (only comments), no quality rules, no $ref
- **Action**: Extract comments as description, preserve field options as custom properties

**Alternatives Considered**:
- Skip formats with limited metadata: Rejected - universal converter must handle all formats
- Create format-specific metadata structures: Rejected - ODCS is the target format, convert everything to ODCS

### Universal Converter Design

**Decision**: Create `convert_to_odcs()` function that accepts any import format and returns ODCS v3.1.0 YAML

**Rationale**:
- ODCS v3.1.0 is closest to the SDK's native format (Table/Column structures)
- Provides a unified conversion path for all formats
- Enables format normalization for downstream processing

**Design Approach**:
1. Accept input as string with format identifier (or auto-detect)
2. Use existing importers to parse input to `Table` structures
3. Use existing `ODCSExporter` to convert `Table` structures to ODCS YAML
4. Handle format-specific limitations gracefully (document missing fields)

**Function Signature**:
```rust
pub fn convert_to_odcs(
    input: &str,
    format: Option<&str>, // "sql", "json_schema", "avro", "protobuf", "odcl", "odcs", or None for auto-detect
) -> Result<String, ConversionError>
```

**Error Handling**:
- Use existing `ImportError` for parse failures
- Create `ConversionError` for conversion-specific issues (e.g., unsupported format)

**Alternatives Considered**:
- Convert to DataModel first, then to ODCS: Accepted - uses existing patterns
- Direct format-to-format conversion: Rejected - adds complexity, ODCS is intermediate format
- Return ImportResult instead of YAML: Rejected - user wants ODCS YAML string

### ColumnData Struct Extension

**Decision**: Extend `ColumnData` struct to include `description`, `quality`, and `$ref` fields

**Rationale**:
- Current `ColumnData` only includes basic fields (name, data_type, nullable, primary_key)
- Frontend needs access to all column metadata
- Maintains backward compatibility (new fields are optional)

**Proposed Structure**:
```rust
pub struct ColumnData {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Vec<HashMap<String, serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_path: Option<String>, // $ref field
}
```

**Backward Compatibility**:
- New fields are `Option` types, default to `None` if not present
- Existing code using `ColumnData` continues to work
- Serialization skips `None` values to maintain JSON compatibility

**Alternatives Considered**:
- Create new `ExtendedColumnData` struct: Rejected - breaks existing code
- Store in separate metadata field: Rejected - adds indirection, less convenient

### Column Struct $ref Field Addition

**Decision**: Add `ref_path: Option<String>` field to `Column` struct

**Rationale**:
- `$ref` references are part of JSON Schema and ODCS specifications
- Need to preserve references during import/export
- Resolution can happen separately (preservation only)

**Implementation**:
- Add `ref_path: Option<String>` field to `Column` struct
- Parse `$ref` from YAML/JSON during import
- Export `$ref` during export (if present)
- Default to `None` for backward compatibility

**Alternatives Considered**:
- Store in `odcl_metadata` HashMap: Rejected - $ref is a first-class concept, not metadata
- Create separate `Reference` struct: Rejected - over-engineering for a string path

### WASM Bindings Design

**Decision**: Add WASM bindings for universal converter and extended ColumnData

**Rationale**:
- User requirement: "All must be available via WASM to JavaScript"
- Existing WASM bindings pattern already established
- Converter function fits existing synchronous WASM API pattern

**Proposed Bindings**:
```rust
#[wasm_bindgen]
pub fn convert_to_odcs(input: &str, format: Option<String>) -> Result<String, JsValue>
```

**Error Handling**:
- Convert Rust errors to `JsValue` (JavaScript Error)
- Return error messages as strings for JavaScript error handling

**Alternatives Considered**:
- Async bindings: Rejected - conversion is CPU-bound, synchronous is appropriate
- Return ImportResult JSON: Rejected - user wants ODCS YAML string directly

## Summary of Design Decisions

1. **ODCS/ODCL Coverage**: Extend parsing to include all specification fields (description, quality, $ref, enum, format, pattern, constraints, etc.)
   - **Phase 1 Focus**: Critical fields (description, quality, $ref) identified in issue #9
   - **Test Fixtures**: Full ODCS and ODCL examples saved to `test-fixtures/` for comprehensive testing
   - **Future Phases**: Can extend to include all specification fields (examples, format, pattern, classification, tags, customProperties, relationships, authoritativeDefinitions, etc.)
2. **ColumnData Extension**: Add optional `description`, `quality`, and `ref_path` fields to `ColumnData` struct
3. **Column Extension**: Add optional `ref_path` field to `Column` struct
4. **Universal Converter**: Create `convert_to_odcs()` function that converts any import format to ODCS v3.1.0 YAML
5. **Format Coverage**: Document format-specific capabilities and limitations
6. **WASM Bindings**: Add `convert_to_odcs()` WASM binding for JavaScript access
7. **Backward Compatibility**: All new fields are optional, existing code continues to work

## Test Fixtures

**Full ODCS Example**:
- Source: [Official ODCS Repository](https://raw.githubusercontent.com/bitol-io/open-data-contract-standard/refs/heads/main/docs/examples/all/full-example.odcs.yaml)
- Saved to: `test-fixtures/full-example.odcs.yaml`
- Contains: Complete ODCS v3.1.0 structure with all field types (description, quality, relationships, authoritativeDefinitions, customProperties, examples, etc.)

**ODCL Example**:
- Saved to: `test-fixtures/example.odcl.yaml`
- Contains: Complete ODCL structure with description, quality arrays, $ref references, definitions, servicelevels

**Usage**: These fixtures can be used for:
- Comprehensive parsing tests
- Field preservation verification
- Round-trip import/export tests
- Format coverage validation

## JSON Schema Definitions

**ODCS JSON Schema v3.1.0**:
- Source: [Official ODCS Schema](https://github.com/bitol-io/open-data-contract-standard/blob/main/schema/odcs-json-schema-v3.1.0.json)
- Saved to: `schemas/odcs-json-schema-v3.1.0.json`
- Purpose: Authoritative JSON Schema definition for ODCS v3.1.0 format validation
- Usage: Can be used to validate parsed ODCS YAML/JSON files, verify field structure, and ensure compliance

**ODCL JSON Schema v1.2.1** (Last Supported Version):
- Source: [Official ODCL Schema](https://github.com/datacontract/datacontract-specification/blob/main/versions/1.2.1/datacontract.schema.json)
- Saved to: `schemas/odcl-json-schema-1.2.1.json`
- Purpose: Authoritative JSON Schema definition for ODCL v1.2.1 format validation
- Usage: Can be used to validate parsed ODCL YAML/JSON files, verify field structure, and ensure compliance

**Usage**: These schemas can be used for:
- Schema validation of imported YAML/JSON files
- Verification of field structure and types
- Ensuring compliance with official specifications
- Generating validation errors for malformed files
- Documentation and reference for field definitions

## Open Questions Resolved

1. **Q**: Should we preserve $ref references even if they don't resolve?
   **A**: Yes - preservation only, resolution happens separately

2. **Q**: How to handle format-specific limitations (e.g., SQL has no description)?
   **A**: Document limitations, preserve what can be extracted, set missing fields to None

3. **Q**: Should universal converter return YAML or ImportResult?
   **A**: YAML string - user requirement is for ODCS YAML output

4. **Q**: Should new ColumnData fields be required or optional?
   **A**: Optional - maintains backward compatibility

## CADS (Compute Asset Description Specification) Analysis

**Decision**: Full import/export support for CADS v1.0 schema

**Rationale**:
- CADS enables governance, discoverability, and risk management for AI/ML models, applications, and pipelines
- Required to record AI Nodes, ML Nodes, and Applications in the SDK
- Used as internal storage format for defining these resources
- Defines transformations that happen to data by Applications or AI/ML

**CADS v1.0 Key Fields** (from schema):
- **Root Level**: `apiVersion`, `kind` (AIModel|MLPipeline|Application|ETLPipeline|SourceSystem|DestinationSystem), `id`, `name`, `version`, `status`, `domain`, `tags`
- **Description**: `purpose`, `usage`, `limitations`, `externalLinks`
- **Runtime**: `environment`, `endpoints`, `container`, `resources` (cpu, memory, gpu)
- **SLA**: `properties` array with `element`, `value`, `unit`, `driver`
- **Pricing**: `model`, `currency`, `unitCost`, `billingUnit`, `notes`
- **Team**: Array with `role`, `name`, `contact`
- **Risk**: `classification`, `impactAreas`, `intendedUse`, `outOfScopeUse`, `assessment`, `mitigations`
- **Compliance**: `frameworks`, `controls`
- **Validation Profiles**: `name`, `appliesTo`, `requiredChecks`
- **BPMN Models**: `name`, `reference`, `format`, `description`
- **Custom Properties**: Free-form object

**Schema Location**: `schemas/cads.schema.json`

**Implementation Approach**:
- Create `src/models/cads.rs` with structs for each asset kind
- Create `src/import/cads.rs` following existing importer patterns
- Create `src/export/cads.rs` following existing exporter patterns
- Add CADS to universal converter (CADS → ODCS conversion)
- Add WASM bindings for CADS import/export

**Alternatives Considered**:
- Minimal CADS support: Rejected - need full specification compliance
- Separate SDK for CADS: Rejected - better integration with existing SDK

## ODPS (Open Data Product Standard) Analysis

**Decision**: Full import/export support for ODPS schema

**Rationale**:
- ODPS enables defining data products that link to ODCS Tables
- Required for Data Product management in the SDK
- Used as internal storage format for Data Products
- Products link to ODCS Tables via contractId references

**ODPS Key Fields** (from schema):
- **Root Level**: `apiVersion`, `kind` (DataProduct), `id`, `name`, `version`, `status`, `domain`, `tenant`
- **Description**: `purpose`, `limitations`, `usage`, `authoritativeDefinitions`, `customProperties`
- **Input Ports**: `name`, `version`, `contractId` (links to ODCS Table), `tags`, `customProperties`, `authoritativeDefinitions`
- **Output Ports**: `name`, `description`, `type`, `version`, `contractId` (links to ODCS Table), `sbom`, `inputContracts`, `tags`, `customProperties`, `authoritativeDefinitions`
- **Management Ports**: `name`, `content`, `type`, `url`, `channel`, `description`, `tags`, `customProperties`, `authoritativeDefinitions`
- **Support**: `channel`, `url`, `description`, `tool`, `scope`, `invitationUrl`, `tags`, `customProperties`, `authoritativeDefinitions`
- **Team**: `name`, `description`, `members` (with `username`, `name`, `description`, `role`, `dateIn`, `dateOut`, `replacedByUsername`)

**Schema Location**: `schemas/odps-json-schema-latest.json` (from [official repository](https://github.com/bitol-io/open-data-product-standard/blob/main/schema/odps-json-schema-latest.json))

**Implementation Approach**:
- Create `src/models/odps.rs` with DataProduct, InputPort, OutputPort, ManagementPort, Support, Team structs
- Create `src/import/odps.rs` following existing importer patterns
- Create `src/export/odps.rs` following existing exporter patterns
- Implement ODCS Table linking validation (contractId references must exist)
- Add ODPS to universal converter (ODPS → ODCS conversion)
- Add WASM bindings for ODPS import/export

**ODCS Table Linking**:
- InputPorts and OutputPorts reference ODCS Tables via `contractId`
- Validation required: contractId must reference existing ODCS Table ID
- Links are bidirectional: ODCS Tables can reference Data Products

**Alternatives Considered**:
- Minimal ODPS support: Rejected - need full specification compliance
- Separate SDK for ODPS: Rejected - better integration with existing SDK

## Business Domain Schema Analysis

**Decision**: New top-level schema for business domains with systems, CADS nodes, and ODCS nodes

**Rationale**:
- Business domains organize systems, applications, and data contracts
- Systems are physical entities (Kafka, Cassandra, EKS, EC2, etc.)
- Systems contain CADS nodes (AI/ML models, applications, pipelines) and ODCS nodes (data contracts)
- Different relationship notations required: ERD-style for systems/CADS, Crowsfeet for ODCS nodes

**Business Domain Key Entities**:

**Domain**:
- `id`: UUID
- `name`: String
- `description`: Option<String>
- `systems`: Vec<System>
- `cads_nodes`: Vec<CADSNode>
- `odcs_nodes`: Vec<ODCSNode>
- `system_connections`: Vec<SystemConnection>
- `node_connections`: Vec<NodeConnection>

**System**:
- `id`: UUID
- `name`: String
- `infrastructure_type`: InfrastructureType (Kafka, Cassandra, EKS, EC2, etc.)
- `domain_id`: UUID (parent domain)
- `description`: Option<String>
- `endpoints`: Vec<String>
- `metadata`: HashMap<String, serde_json::Value>

**SystemConnection** (ERD-style):
- `id`: UUID
- `source_system_id`: UUID
- `target_system_id`: UUID
- `connection_type`: String (e.g., "data_flow", "api_call", "message_queue")
- `bidirectional`: bool
- `metadata`: HashMap<String, serde_json::Value>

**CADSNode**:
- `id`: UUID
- `system_id`: UUID (parent system)
- `cads_asset_id`: UUID (reference to CADS asset)
- `kind`: CADSKind (AIModel, MLPipeline, Application, etc.)

**ODCSNode**:
- `id`: UUID
- `system_id`: UUID (parent system)
- `table_id`: UUID (reference to ODCS Table)
- `role`: String (e.g., "source", "destination", "intermediate")

**NodeConnection** (Crowsfeet notation):
- `id`: UUID
- `source_node_id`: UUID
- `target_node_id`: UUID
- `cardinality`: Cardinality (OneToOne, OneToMany, ZeroOrOne, ZeroOrMany)
- `relationship_type`: String (e.g., "foreign_key", "data_flow", "derived_from")

**Relationship Notation**:
- **ERD-style** (System ↔ System, System ↔ CADS): Bidirectional connections with connection metadata
- **Crowsfeet** (ODCS ↔ ODCS): Cardinality-based relationships (1:1, 1:N, 0:1, 0:N)

**Implementation Approach**:
- Create `src/models/domain.rs` with Domain, System, SystemConnection, CADSNode, ODCSNode, NodeConnection structs
- Create `Cardinality` enum for Crowsfeet notation
- Add domain operations to DataModel (add_system, add_cads_node, add_odcs_node, etc.)
- Create domain import/export (YAML/JSON)
- Add WASM bindings for domain operations

**Alternatives Considered**:
- Single relationship type: Rejected - ERD and Crowsfeet serve different purposes
- Separate domain SDK: Rejected - better integration with existing SDK
