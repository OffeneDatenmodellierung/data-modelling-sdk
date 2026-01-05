# Data Model: ODPS Schema Validation

**Feature**: ODPS Schema Validation and Manual Test Script
**Date**: 2026-01-05
**Phase**: 1 - Design & Contracts

## Overview

This document describes the data model for ODPS (Open Data Product Standard) schema validation. The validation system validates ODPS YAML files against the official ODPS JSON Schema specification, ensuring schema compliance for both import and export operations.

## Core Entities

### ODPSDataProduct

**Purpose**: Represents a complete ODPS data product specification.

**Source**: `src/models/odps.rs` (existing)

**Key Fields** (from ODPS schema):
- `api_version: String` - Required, enum: `["v0.9.0", "v1.0.0"]`
- `kind: String` - Required, must equal `"DataProduct"`
- `id: String` - Required, UUID format
- `status: ODPSStatus` - Required, enum: `["proposed", "draft", "active", "deprecated", "retired"]`
- `name: Option<String>` - Optional
- `version: Option<String>` - Optional
- `domain: Option<String>` - Optional
- `tenant: Option<String>` - Optional
- `tags: Vec<String>` - Optional array
- `description: Option<ODPSDescription>` - Optional nested object
- `authoritativeDefinitions: Option<Vec<ODPSAuthoritativeDefinition>>` - Optional array
- `customProperties: Option<Vec<ODPSCustomProperty>>` - Optional array
- `inputPorts: Option<Vec<ODPSInputPort>>` - Optional array
- `outputPorts: Option<Vec<ODPSOutputPort>>` - Optional array
- `managementPorts: Option<Vec<ODPSManagementPort>>` - Optional array
- `support: Option<Vec<ODPSSupport>>` - Optional array
- `team: Option<ODPSTeam>` - Optional nested object
- `productCreatedTs: Option<String>` - Optional, ISO 8601 date-time format

**Validation Rules**:
- All required fields must be present
- `apiVersion` must be one of allowed enum values
- `kind` must equal `"DataProduct"`
- `status` must be one of allowed enum values
- `id` must be valid UUID format
- `productCreatedTs` must be valid ISO 8601 date-time format (if present)

---

### ODPSAuthoritativeDefinition

**Purpose**: Represents an authoritative definition reference.

**Required Fields**:
- `type: String` - Type of definition
- `url: String` - URL to the authority (must be valid URI format)

**Optional Fields**:
- `description: Option<String>` - Description

**Validation Rules**:
- `type` and `url` are required when object is present
- `url` must conform to URI format requirements

---

### ODPSCustomProperty

**Purpose**: Represents a custom key-value property.

**Required Fields**:
- `property: String` - Property name
- `value: serde_json::Value` - Property value (any JSON type)

**Optional Fields**:
- `description: Option<String>` - Description

**Validation Rules**:
- `property` and `value` are required when object is present
- `value` can be any valid JSON type (string, number, boolean, object, array, null)

---

### ODPSInputPort

**Purpose**: Represents an input port for a data product.

**Required Fields**:
- `name: String` - Port name
- `version: String` - Port version
- `contract_id: String` - Contract ID reference (UUID format)

**Optional Fields**:
- `tags: Vec<String>` - Tags array
- `custom_properties: Option<Vec<ODPSCustomProperty>>` - Custom properties
- `authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>` - Authoritative definitions

**Validation Rules**:
- `name`, `version`, and `contract_id` are required when object is present
- Nested objects (customProperties, authoritativeDefinitions) must follow their own validation rules

---

### ODPSOutputPort

**Purpose**: Represents an output port for a data product.

**Required Fields**:
- `name: String` - Port name
- `version: String` - Port version

**Optional Fields**:
- `description: Option<String>` - Description
- `r#type: Option<String>` - Port type
- `contract_id: Option<String>` - Contract ID reference (UUID format, optional)
- `sbom: Option<Vec<ODPSSBOM>>` - Software Bill of Materials
- `input_contracts: Option<Vec<ODPSInputContract>>` - Input contract references
- `tags: Vec<String>` - Tags array
- `custom_properties: Option<Vec<ODPSCustomProperty>>` - Custom properties
- `authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>` - Authoritative definitions

**Validation Rules**:
- `name` and `version` are required when object is present
- Nested objects must follow their own validation rules

---

### ODPSSupport

**Purpose**: Represents a support channel.

**Required Fields**:
- `channel: String` - Support channel type
- `url: String` - Support URL (must be valid URI format)

**Optional Fields**:
- `description: Option<String>` - Description
- `tool: Option<String>` - Tool name
- `scope: Option<String>` - Scope
- `invitation_url: Option<String>` - Invitation URL
- `tags: Vec<String>` - Tags array
- `custom_properties: Option<Vec<ODPSCustomProperty>>` - Custom properties
- `authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>` - Authoritative definitions

**Validation Rules**:
- `channel` and `url` are required when object is present
- `url` must conform to URI format requirements

---

### ODPSTeamMember

**Purpose**: Represents a team member.

**Required Fields**:
- `username: String` - Username

**Optional Fields**:
- `name: Option<String>` - Full name
- `description: Option<String>` - Description
- `role: Option<String>` - Role
- `date_in: Option<String>` - Start date
- `date_out: Option<String>` - End date
- `replaced_by_username: Option<String>` - Replacement username
- `tags: Vec<String>` - Tags array
- `custom_properties: Option<Vec<ODPSCustomProperty>>` - Custom properties
- `authoritative_definitions: Option<Vec<ODPSAuthoritativeDefinition>>` - Authoritative definitions

**Validation Rules**:
- `username` is required when object is present

---

## Validation Error Model

### ValidationError

**Purpose**: Represents a schema validation error.

**Fields**:
- `field_path: String` - JSON path to the field (e.g., `support[0].url`)
- `expected_type: String` - Expected type/format (e.g., `"string"`, `"uri"`, `"enum: [proposed, draft, active, deprecated, retired]"`)
- `actual_value: serde_json::Value` - Actual value that failed validation
- `error_message: String` - Human-readable error message
- `schema_reference: Option<String>` - Reference to schema requirement (optional)

**Source**: Generated by `jsonschema::Validator` and formatted for user display

---

## State Transitions

### Import Flow

```
ODPS YAML File
    ↓
[Parse YAML] → YAML Parse Error
    ↓
[Validate against Schema] → Validation Error
    ↓
[Parse to ODPSDataProduct] → Parse Error
    ↓
ODPSDataProduct (validated)
```

### Export Flow

```
ODPSDataProduct
    ↓
[Serialize to YAML] → Serialization Error
    ↓
[Validate against Schema] → Validation Error
    ↓
ODPS YAML File (validated)
```

---

## Field Preservation Requirements

All fields defined in the ODPS schema must be preserved during import/export round-trips:

1. **Required Fields**: Always present, always preserved
2. **Optional Fields**: Preserved if present in source, including:
   - Empty arrays (`[]`) - preserved as empty arrays, not omitted
   - Empty objects (`{}`) - preserved as empty objects, not omitted
   - Null values - preserved as null, not omitted
3. **Nested Structures**: All nested fields preserved recursively
4. **Field Order**: Order within arrays/objects may vary (JSON/YAML doesn't guarantee order), but all fields must be present

---

## Relationships

- `ODPSDataProduct` contains zero or more `ODPSInputPort` objects
- `ODPSDataProduct` contains zero or more `ODPSOutputPort` objects
- `ODPSDataProduct` contains zero or more `ODPSManagementPort` objects
- `ODPSDataProduct` contains zero or more `ODPSSupport` objects
- `ODPSDataProduct` contains zero or one `ODPSTeam` object
- `ODPSTeam` contains zero or more `ODPSTeamMember` objects
- Ports, Support, and Team objects can contain `ODPSCustomProperty` and `ODPSAuthoritativeDefinition` arrays

---

## Constraints

1. **Schema Compliance**: All ODPS files must validate against `schemas/odps-json-schema-latest.json`
2. **UUID Format**: `id` and `contract_id` fields must be valid UUIDs
3. **URI Format**: All `url` fields must conform to URI format requirements
4. **Date-Time Format**: `productCreatedTs` must be ISO 8601 format if present
5. **Enum Values**: `apiVersion`, `status`, and other enum fields must match allowed values exactly
6. **Standalone Format**: ODPS cannot be converted to other formats (ODCS, JSON Schema, etc.)

---

## Validation Implementation

Validation is performed using the `jsonschema` crate:

1. Load ODPS JSON Schema from `schemas/odps-json-schema-latest.json` (compile-time via `include_str!`)
2. Parse schema to `serde_json::Value`
3. Create `jsonschema::Validator` from schema
4. Parse ODPS YAML to `serde_json::Value`
5. Validate YAML JSON value against schema
6. Format validation errors with field paths and expected/actual values

Feature-flagged via `odps-validation` feature (depends on `schema-validation` feature).
