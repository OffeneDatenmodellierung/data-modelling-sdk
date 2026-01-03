# WASM Bindings API Contracts: Universal Converter

**Date**: 2026-01-27
**Feature**: WASM Bindings for Universal Format Conversion

## Overview

This document defines the WASM function bindings for the universal converter and enhanced field preservation, enabling JavaScript applications to convert any format to ODCS and access all column metadata.

## WASM Function Bindings

### convertToOdcs

Convert any import format to ODCS v3.1.0 YAML format.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn convert_to_odcs(input: &str, format: Option<String>) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function convertToOdcs(input: string, format?: string): string
```

**Parameters**:
- `input: string` - Format-specific content (SQL, JSON Schema, AVRO, Protobuf, ODCL, ODCS)
- `format?: string` - Optional format identifier. If omitted, attempts auto-detection.
  Supported values: `"sql"`, `"json_schema"`, `"avro"`, `"protobuf"`, `"odcl"`, `"odcs"`

**Returns**:
- `Ok(String)` - ODCS v3.1.0 YAML string
- `Err(JsValue)` - JavaScript Error with error message

**Example**:
```javascript
import init, { convertToOdcs } from './pkg/data_modelling_sdk.js';

await init();

// Convert SQL to ODCS
const sql = `CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));`;
const odcsYaml = convertToOdcs(sql, "sql");
console.log(odcsYaml);

// Auto-detect format
const jsonSchema = `{"$schema": "http://json-schema.org/draft-07/schema#", "type": "object"}`;
const odcsYaml2 = convertToOdcs(jsonSchema); // Auto-detects JSON Schema
```

## Enhanced ImportResult Structure

### ColumnData (Enhanced)

The `ColumnData` structure in `ImportResult` now includes additional fields accessible from JavaScript:

```typescript
interface ColumnData {
  name: string;
  data_type: string;
  nullable: boolean;
  primary_key: boolean;
  description?: string;        // NEW: Column description
  quality?: QualityRule[];     // NEW: Quality rules array
  $ref?: string;               // NEW: JSON Schema $ref path
}

interface QualityRule {
  [key: string]: any;          // Flexible structure for nested quality rules
}
```

### Usage Example

```javascript
import init, { parseOdcsYaml } from './pkg/data_modelling_sdk.js';

await init();

const yaml = `
apiVersion: v3.1.0
kind: DataContract
name: orders
schema:
  fields:
    - name: status
      type: string
      description: "Order status"
      quality:
        - type: custom
          engine: great-expectations
          implementation:
            expectation_type: expect_column_values_to_be_in_set
            kwargs:
              value_set: ["PENDING", "SHIPPED", "DELIVERED"]
      $ref: "#/definitions/orderStatus"
`;

const resultJson = parseOdcsYaml(yaml);
const result = JSON.parse(resultJson);

// Access enhanced fields
const column = result.tables[0].columns[0];
console.log("Description:", column.description);
console.log("Quality rules:", column.quality);
console.log("$ref:", column.$ref);
```

## Error Handling

### JavaScript Error Handling

```javascript
try {
  const odcsYaml = convertToOdcs(input, format);
  // Success
} catch (error) {
  console.error("Conversion failed:", error.message);
  // Error types:
  // - ImportError: Parse error in source format
  // - ExportError: Error converting to ODCS
  // - UnsupportedFormat: Format not recognized
  // - AutoDetectionFailed: Could not detect format
}
```

## Format Support Matrix

| Format | Auto-Detect | Description | Quality | $ref |
|--------|-------------|-------------|---------|------|
| ODCS | ✅ | ✅ | ✅ | ✅ |
| ODCL | ✅ | ✅ | ✅ | ✅ |
| SQL | ✅ | ❌ | ❌ | ❌ |
| JSON Schema | ✅ | ✅ | ❌ | ✅ |
| AVRO | ✅ | ✅ (doc) | ❌ | ❌ |
| Protobuf | ✅ | ✅ (comments) | ❌ | ❌ |

## Performance Notes

- Conversion operations are synchronous (CPU-bound)
- Typical schemas convert in <1 second
- Large schemas may take 2-5 seconds (blocks main thread)
- Consider Web Workers for very large files if performance becomes an issue

## Migration Guide

### Existing Code

Existing code using `parseOdcsYaml()` continues to work:

```javascript
// Existing code - still works
const result = JSON.parse(parseOdcsYaml(yaml));
const column = result.tables[0].columns[0];
console.log(column.name);        // ✅ Still works
console.log(column.data_type);  // ✅ Still works
```

### Enhanced Code

New code can access enhanced fields:

```javascript
// New code - access enhanced fields
const column = result.tables[0].columns[0];
if (column.description) {
  console.log("Description:", column.description);
}
if (column.quality) {
  console.log("Quality rules:", column.quality);
}
if (column.$ref) {
  console.log("Reference:", column.$ref);
}
```

## Complete Example

```javascript
import init, { convertToOdcs, parseOdcsYaml } from './pkg/data_modelling_sdk.js';

await init();

// Convert SQL to ODCS
const sql = `
CREATE TABLE users (
    id INT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255)
);
`;

try {
  // Convert to ODCS
  const odcsYaml = convertToOdcs(sql, "sql");
  console.log("Converted to ODCS:");
  console.log(odcsYaml);

  // Parse back to verify
  const resultJson = parseOdcsYaml(odcsYaml);
  const result = JSON.parse(resultJson);

  // Access all fields
  result.tables[0].columns.forEach(col => {
    console.log(`Column: ${col.name}`);
    console.log(`  Type: ${col.data_type}`);
    console.log(`  Nullable: ${col.nullable}`);
    console.log(`  Primary Key: ${col.primary_key}`);
    if (col.description) {
      console.log(`  Description: ${col.description}`);
    }
    if (col.quality) {
      console.log(`  Quality Rules: ${col.quality.length}`);
    }
    if (col.$ref) {
      console.log(`  $ref: ${col.$ref}`);
    }
  });
} catch (error) {
  console.error("Error:", error.message);
}
```
