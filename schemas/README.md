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

## File Structure

```
schemas/
├── README.md                      # This file
├── odcs-json-schema-v3.1.0.json  # ODCS v3.1.0 schema
├── odcl-json-schema-1.2.1.json   # ODCL v1.2.1 schema (legacy)
├── odps-json-schema-latest.json  # ODPS latest schema
└── cads.schema.json               # CADS v1.0 schema
```

## Related Documentation

- [Schema Overview Guide](../docs/SCHEMA_OVERVIEW.md) - Comprehensive guide to all schemas
- [ODCS Field Preservation Spec](../specs/003-odcs-field-preservation/spec.md) - Implementation details
- [Universal Converter](../src/convert/converter.rs) - Format conversion utilities
