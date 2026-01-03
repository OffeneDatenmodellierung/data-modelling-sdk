# Universal Converter API Contracts

**Date**: 2026-01-27
**Feature**: Universal Format Conversion to ODCS

## Overview

This document defines the Rust API contracts for the universal converter function that converts any import format (SQL, JSON Schema, AVRO, Protobuf, ODCL, ODCS) to ODCS v3.1.0 format.

## Converter Module

### Module Location

`src/import/converter.rs`

### ConversionError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Import error: {0}")]
    ImportError(#[from] ImportError),
    #[error("Export error: {0}")]
    ExportError(#[from] ExportError),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Auto-detection failed: {0}")]
    AutoDetectionFailed(String),
}
```

## Converter Function

### Function Signature

```rust
/// Convert any import format to ODCS v3.1.0 YAML format.
///
/// # Arguments
///
/// * `input` - Format-specific content as a string
/// * `format` - Optional format identifier. If None, attempts auto-detection.
///   Supported formats: "sql", "json_schema", "avro", "protobuf", "odcl", "odcs"
///
/// # Returns
///
/// ODCS v3.1.0 YAML string, or ConversionError
///
/// # Example
///
/// ```rust
/// use data_modelling_sdk::import::converter::convert_to_odcs;
///
/// let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));";
/// let odcs_yaml = convert_to_odcs(sql, Some("sql"))?;
/// ```
pub fn convert_to_odcs(
    input: &str,
    format: Option<&str>,
) -> Result<String, ConversionError>
```

### Format Detection

When `format` is `None`, the function attempts auto-detection:

1. **ODCS**: Checks for `apiVersion: v3.1.0` or `apiVersion: v3.0.x`
2. **ODCL**: Checks for `dataContractSpecification` field
3. **SQL**: Checks for `CREATE TABLE` keywords
4. **JSON Schema**: Checks for `"$schema"` or `"type"` fields in JSON
5. **AVRO**: Checks for `"type"` and `"fields"` in JSON (AVRO schema format)
6. **Protobuf**: Checks for `syntax`, `message`, `service` keywords

### Supported Formats

| Format | Identifier | Auto-Detect | Notes |
|--------|------------|-------------|-------|
| ODCS v3.1.0 | `"odcs"` | ✅ | Primary format, passes through |
| ODCL | `"odcl"` | ✅ | Converted to ODCS v3.1.0 |
| SQL | `"sql"` | ✅ | Requires dialect (defaults to "postgresql") |
| JSON Schema | `"json_schema"` | ✅ | Full field mapping |
| AVRO | `"avro"` | ✅ | Maps doc to description |
| Protobuf | `"protobuf"` | ✅ | Extracts comments as description |

## Usage Examples

### Example 1: Convert SQL to ODCS

```rust
use data_modelling_sdk::import::converter::convert_to_odcs;

let sql = r#"
CREATE TABLE users (
    id INT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255)
);
"#;

let odcs_yaml = convert_to_odcs(sql, Some("sql"))?;
println!("{}", odcs_yaml);
// Output: ODCS v3.1.0 YAML with users table
```

### Example 2: Convert JSON Schema to ODCS

```rust
let json_schema = r#"
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "id": {
      "type": "integer",
      "description": "User identifier"
    },
    "email": {
      "type": "string",
      "format": "email",
      "$ref": "#/definitions/emailFormat"
    }
  }
}
"#;

let odcs_yaml = convert_to_odcs(json_schema, Some("json_schema"))?;
// Output: ODCS v3.1.0 YAML with description and $ref preserved
```

### Example 3: Auto-Detect Format

```rust
let input = r#"
apiVersion: v3.1.0
kind: DataContract
name: products
schema:
  fields:
    - name: id
      type: bigint
"#;

let odcs_yaml = convert_to_odcs(input, None)?; // Auto-detects ODCS format
```

### Example 4: Convert AVRO to ODCS

```rust
let avro_schema = r#"
{
  "type": "record",
  "name": "User",
  "fields": [
    {
      "name": "id",
      "type": "long",
      "doc": "User identifier"
    },
    {
      "name": "email",
      "type": "string",
      "default": ""
    }
  ]
}
"#;

let odcs_yaml = convert_to_odcs(avro_schema, Some("avro"))?;
// Output: ODCS v3.1.0 YAML with description from "doc" field
```

## Error Handling

### Conversion Errors

```rust
use data_modelling_sdk::import::converter::{convert_to_odcs, ConversionError};

match convert_to_odcs(input, format) {
    Ok(odcs_yaml) => {
        // Success - use ODCS YAML
        println!("{}", odcs_yaml);
    }
    Err(ConversionError::ImportError(e)) => {
        eprintln!("Import failed: {}", e);
    }
    Err(ConversionError::ExportError(e)) => {
        eprintln!("Export failed: {}", e);
    }
    Err(ConversionError::UnsupportedFormat(fmt)) => {
        eprintln!("Unsupported format: {}", fmt);
    }
    Err(ConversionError::AutoDetectionFailed(msg)) => {
        eprintln!("Auto-detection failed: {}", msg);
    }
}
```

## Format-Specific Behavior

### SQL Conversion

- Extracts table name, column names, data types, nullable, primary keys
- Missing fields (description, quality, $ref) are omitted in ODCS output
- Supports multiple dialects (postgresql, mysql, sqlserver, databricks)

### JSON Schema Conversion

- Maps `description` → `description`
- Maps `$ref` → `$ref`
- Maps `enum` → `enum` values
- Maps `format`, `pattern`, `minLength`, `maxLength` → ODCS constraints
- Quality rules not supported (JSON Schema doesn't have native quality)

### AVRO Conversion

- Maps `doc` → `description`
- Preserves `default` values
- Maps `logicalType` to ODCS format hints
- Quality rules not supported

### Protobuf Conversion

- Extracts comments (`//`) → `description`
- Preserves field options as custom properties
- Quality rules and $ref not supported

### ODCL Conversion

- Full field preservation (description, quality, $ref)
- Converts to ODCS v3.1.0 structure
- Maintains all metadata

### ODCS Conversion

- Pass-through (no conversion needed)
- Validates format and returns as-is

## Performance Considerations

- Conversion is CPU-bound (parsing + serialization)
- Typical schemas (<1000 columns) convert in <1 second
- Large schemas (>10,000 columns) may take 2-5 seconds
- WASM bindings are synchronous (appropriate for CPU-bound operations)
