# Schema Reference Directory

This directory contains JSON Schema definitions for all supported data modeling formats used by the Data Modelling SDK.

## Purpose

These schemas serve as authoritative references for:
- **Validation**: Validating imported YAML/JSON files against official specifications
- **Documentation**: Understanding the structure and fields of each format
- **Compliance**: Ensuring the SDK maintains full coverage of each specification
- **Reference**: Quick lookup for field definitions and types

## Supported Schemas

### ODCS (Open Data Contract Standard)

**File**: `odcs-json-schema-v3.1.0.json`
**Version**: v3.1.0
**Source**: [Official ODCS Repository](https://github.com/bitol-io/open-data-contract-standard/blob/main/schema/odcs-json-schema-v3.1.0.json)
**Purpose**: Primary format for data contracts (tables/schemas)
**Status**: ✅ Fully Supported

ODCS is the primary format for defining data contracts. It provides comprehensive metadata about data structures including:
- Schema definitions with properties/fields
- Quality rules and validation checks
- Service level agreements (SLAs)
- Tags and metadata
- References to external definitions

### ODCL (Open Data Contract Language)

**File**: `odcl-json-schema-1.2.1.json`
**Version**: v1.2.1 (Last Supported)
**Source**: [Official ODCL Repository](https://github.com/datacontract/datacontract-specification/blob/main/versions/1.2.1/datacontract.schema.json)
**Purpose**: Legacy data contract format
**Status**: ✅ Fully Supported (Legacy)

ODCL is the legacy format for data contracts. While ODCS v3.1.0 is preferred, ODCL v1.2.1 is still supported for backward compatibility.

### ODPS (Open Data Product Standard)

**File**: `odps-json-schema-latest.json`
**Version**: Latest
**Source**: [Official ODPS Repository](https://github.com/bitol-io/open-data-product-standard/blob/main/schema/odps-json-schema-latest.json)
**Purpose**: Data Products linking to ODCS Tables
**Status**: ✅ Fully Supported

ODPS defines data products that link multiple data contracts together. Key features:
- Links to ODCS Tables via `contractId` references
- Input/output ports for data flow
- Product metadata (name, version, status, domain, tenant)
- Support and team information

### CADS (Compute Asset Description Specification)

**File**: `cads.schema.json`
**Version**: v1.0
**Source**: Internal specification
**Purpose**: AI/ML models, applications, pipelines, source/destination systems
**Status**: ✅ Fully Supported

CADS defines compute assets including:
- **AIModel**: AI/ML models
- **MLPipeline**: Machine learning pipelines
- **Application**: Traditional applications
- **ETLPipeline**: ETL pipelines
- **SourceSystem**: Source systems
- **DestinationSystem**: Destination systems

### BPMN (Business Process Model and Notation)

**File**: `bpmn-2.0.xsd`
**Version**: 2.0
**Source**: [OMG BPMN 2.0 Specification](https://www.omg.org/spec/BPMN/2.0/)
**Purpose**: Business process models stored in native XML format
**Status**: ✅ Fully Supported

BPMN 2.0 defines business process models. The SDK stores BPMN files in their native XML format and validates them against the official XSD schema.

### DMN (Decision Model and Notation)

**File**: `dmn-1.3.xsd`
**Version**: 1.3
**Source**: [OMG DMN 1.3 Specification](https://www.omg.org/spec/DMN/1.3/)
**Purpose**: Decision models stored in native XML format
**Status**: ✅ Fully Supported

DMN 1.3 defines decision models. The SDK stores DMN files in their native XML format and validates them against the official XSD schema.

### OpenAPI

**File**: `openapi-3.1.1.json`
**Version**: 3.1.1
**Source**: [OpenAPI Initiative](https://github.com/OAI/OpenAPI-Specification/blob/main/schemas/v3.1/schema.json)
**Purpose**: API specifications stored in native YAML or JSON format
**Status**: ✅ Fully Supported

OpenAPI 3.1.1 defines API specifications. The SDK stores OpenAPI files in their native YAML or JSON format and validates them against the official JSON Schema. The SDK also provides a converter to transform OpenAPI schema components to ODCS table definitions.

### Workspace

**File**: `workspace-schema.json`
**Version**: 1.0
**Source**: Internal specification
**Purpose**: Workspace configuration for the data modelling application
**Status**: ✅ Fully Supported

Workspace schema defines top-level containers that organize domains, systems, assets, and relationships. Key features:
- Workspace metadata (id, name, owner_id, timestamps)
- Domain references with systems
- Asset references (ODCS, ODPS, CADS, BPMN, DMN, OpenAPI)
- Relationship definitions between assets
- Flat file naming convention: `{workspace}_{domain}_{system}_{resource}.{type}.yaml`

**Related Sub-schemas** (modular organization):
- `common-types-schema.json`: Shared types (ContactDetails, SlaProperty, ViewPosition, ConnectionPoint)
- `domain-schema.json`: DomainReference definition
- `system-schema.json`: SystemReference with environments and connection details

**System Features**:
- `systemType`: Infrastructure type (68 types covering databases, cloud services, message queues, etc.)
- `environments`: Multi-environment connection details (production, staging, development)
- Environment connection details include: owner, contactDetails, sla, authMethod, supportTeam, connectionString, secretLink, endpoint, port, region, status, notes

### MADR Decision Records

**File**: `decision-schema.json`
**Version**: 1.0
**Source**: Internal specification (MADR-compliant)
**Purpose**: Architecture Decision Records following MADR template
**Status**: ✅ Fully Supported

Decision schema defines MADR-compliant Architecture Decision Records. Key features:
- Decision metadata (id, number, title, status, category, domain)
- Decision context and drivers with priorities
- Options considered with pros/cons analysis
- Selected decision and consequences
- Asset linking to ODCS, ODPS, CADS resources
- Supersession chain tracking
- Compliance assessment (regulatory, privacy, security)
- Index file (`decisions.yaml`) for decision catalog

### Knowledge Base Articles

**File**: `knowledge-schema.json`
**Version**: 1.0
**Source**: Internal specification
**Purpose**: Domain-partitioned knowledge articles and documentation
**Status**: ✅ Fully Supported

Knowledge schema defines knowledge base articles. Key features:
- Article metadata (id, number, title, type, status, domain)
- Article types: guide, standard, reference, glossary, howto, troubleshooting, policy, template
- Summary and full Markdown content
- Authorship and review tracking
- Audience and skill level classification
- Asset linking to ODCS, ODPS, CADS resources
- Decision linking for context
- Related article references
- Index file (`knowledge.yaml`) for article catalog

### Decision Index

**File**: `decisions-index-schema.json`
**Version**: 1.0
**Source**: Internal specification
**Purpose**: Index file for tracking all decisions in a workspace
**Status**: ✅ Fully Supported

### Knowledge Index

**File**: `knowledge-index-schema.json`
**Version**: 1.0
**Source**: Internal specification
**Purpose**: Index file for tracking all knowledge articles in a workspace
**Status**: ✅ Fully Supported

## Other Formats

The SDK also supports importing/exporting from these formats, but they use external standards rather than our own schemas:

- **SQL**: Various SQL dialects (PostgreSQL, MySQL, SQL Server, etc.)
- **JSON Schema**: Standard JSON Schema format
- **AVRO**: Apache AVRO schema format
- **Protobuf**: Protocol Buffers schema format

These formats are parsed and converted to ODCS format internally.

## Usage

### Validation

These schemas can be used with JSON Schema validators to validate imported files:

```rust
use jsonschema::JSONSchema;
use serde_json::Value;

// Load schema
let schema: Value = serde_json::from_str(include_str!("odcs-json-schema-v3.1.0.json"))?;
let compiled = JSONSchema::compile(&schema)?;

// Validate YAML/JSON
let data: Value = serde_yaml::from_str(yaml_content)?;
let validation = compiled.validate(&data);
```

### Reference

For field definitions and structure, refer to the JSON Schema files directly. Each schema includes:
- Field names and types
- Required vs optional fields
- Enumerated values
- Default values
- Descriptions and documentation

## Maintenance

These schemas should be kept in sync with the official specifications:

- **ODCS**: Update when new versions are released
- **ODCL**: v1.2.1 is the last supported version (no updates expected)
- **ODPS**: Update when new versions are released
- **CADS**: Update when specification evolves
- **BPMN**: Update when OMG releases new versions
- **DMN**: Update when OMG releases new versions
- **OpenAPI**: Update when OpenAPI Initiative releases new versions

## File Structure

```
schemas/
├── README.md                      # This file
├── odcs-json-schema-v3.1.0.json  # ODCS v3.1.0 schema
├── odcl-json-schema-1.2.1.json   # ODCL v1.2.1 schema (legacy)
├── odps-json-schema-latest.json  # ODPS latest schema
├── cads.schema.json               # CADS v1.0 schema (with bpmnModels, dmnModels, openApiSpecs)
├── bpmn-2.0.xsd                   # BPMN 2.0 XSD schema
├── dmn-1.3.xsd                    # DMN 1.3 XSD schema
├── openapi-3.1.1.json             # OpenAPI 3.1.1 JSON Schema
├── workspace-schema.json          # Workspace configuration schema (includes domains, assets, relationships)
├── common-types-schema.json       # Shared types (ContactDetails, SlaProperty, ViewPosition)
├── domain-schema.json             # Domain reference schema
├── system-schema.json             # System reference with environments schema
├── decision-schema.json           # MADR Decision Record schema
├── knowledge-schema.json          # Knowledge Base Article schema
├── decisions-index-schema.json    # Decision index (decisions.yaml) schema
└── knowledge-index-schema.json    # Knowledge index (knowledge.yaml) schema
```

## Related Documentation

- [Schema Overview Guide](../docs/SCHEMA_OVERVIEW.md) - Comprehensive guide to all schemas
- [ODCS Field Preservation Spec](../specs/003-odcs-field-preservation/spec.md) - Implementation details
- [Universal Converter](../src/convert/converter.rs) - Format conversion utilities
