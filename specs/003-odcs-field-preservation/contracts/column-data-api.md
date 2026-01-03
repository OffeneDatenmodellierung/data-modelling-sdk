# ColumnData API Contracts: Enhanced Field Preservation

**Date**: 2026-01-27
**Feature**: Complete ODCS/ODCL Field Preservation

## Overview

This document defines the Rust API contracts for the enhanced `ColumnData` struct that preserves all fields from ODCS/ODCL formats, including description, quality arrays, and $ref references.

## ColumnData Structure

### Enhanced ColumnData

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnData {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    /// Column description/documentation (from ODCS/ODCL description field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Quality rules and validation checks (from ODCS/ODCL quality array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Vec<HashMap<String, serde_json::Value>>>,
    /// JSON Schema $ref reference path (from ODCS/ODCL $ref field)
    #[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]
    pub ref_path: Option<String>,
}
```

### Field Descriptions

- **`name`**: Column name (required, non-empty string)
- **`data_type`**: Data type string (required, e.g., "INT", "VARCHAR(100)")
- **`nullable`**: Whether column allows NULL values (required, boolean)
- **`primary_key`**: Whether column is part of primary key (required, boolean)
- **`description`**: Column description/documentation (optional, string, preserves empty strings)
- **`quality`**: Array of quality rule objects with nested structures (optional, preserves empty arrays)
- **`ref_path`**: JSON Schema $ref reference path (optional, string, e.g., "#/definitions/orderStatus")

## ImportResult Contract

### ImportResult Structure

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub tables: Vec<TableData>,
    pub tables_requiring_name: Vec<TableRequiringName>,
    pub errors: Vec<ImportError>,
    pub ai_suggestions: Option<Vec<serde_json::Value>>,
}
```

### TableData Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub table_index: usize,
    pub name: Option<String>,
    pub columns: Vec<ColumnData>, // Enhanced with description, quality, ref_path
}
```

## Usage Examples

### Example 1: Import ODCL with Description

```rust
use data_modelling_sdk::import::ODCSImporter;

let mut importer = ODCSImporter::new();
let yaml = r#"
dataContractSpecification: "https://datacontract.com/specification/0.9.3/datacontract.schema.json"
models:
  tables:
    - name: "Order"
      columns:
        orderStatus:
          description: "Current status of the order"
          type: string
"#;

let result = importer.import(yaml)?;
let column = &result.tables[0].columns[0];
assert_eq!(column.description, Some("Current status of the order".to_string()));
```

### Example 2: Import ODCS with Quality Rules

```rust
let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
name: users
schema:
  fields:
    - name: email
      type: string
      quality:
        - type: custom
          engine: great-expectations
          implementation:
            expectation_type: expect_column_values_to_match_regex
            kwargs:
              regex: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
"#;

let result = importer.import(yaml)?;
let column = &result.tables[0].columns[0];
assert!(column.quality.is_some());
let quality_rules = column.quality.as_ref().unwrap();
assert_eq!(quality_rules.len(), 1);
```

### Example 3: Import ODCL with $ref

```rust
let yaml = r#"
models:
  tables:
    - name: "Order"
      columns:
        status:
          $ref: '#/definitions/orderStatus'
          type: string
"#;

let result = importer.import(yaml)?;
let column = &result.tables[0].columns[0];
assert_eq!(column.ref_path, Some("#/definitions/orderStatus".to_string()));
```

## Backward Compatibility

### Existing Code Compatibility

Existing code using `ColumnData` continues to work:

```rust
// Existing code - still works
let column = &result.tables[0].columns[0];
println!("Column: {}", column.name);
println!("Type: {}", column.data_type);
println!("Nullable: {}", column.nullable);

// New code - can access enhanced fields
if let Some(desc) = &column.description {
    println!("Description: {}", desc);
}
```

### Serialization Behavior

- Fields with `None` values are omitted from JSON serialization (due to `skip_serializing_if`)
- Empty strings for `description` are preserved (not omitted)
- Empty arrays for `quality` are preserved (not omitted)
- `$ref` field is serialized as `"$ref"` in JSON (due to `rename` attribute)

## Error Handling

### Import Errors

```rust
match importer.import(yaml) {
    Ok(result) => {
        // Process columns with enhanced fields
        for table in result.tables {
            for column in table.columns {
                // Access description, quality, ref_path
            }
        }
    }
    Err(ImportError::ParseError(msg)) => {
        eprintln!("Parse error: {}", msg);
    }
    Err(e) => {
        eprintln!("Import error: {}", e);
    }
}
```

## Field Preservation Guarantees

### ODCS v3.1.0 Format

- ✅ 100% of specification fields preserved
- ✅ All nested structures in quality arrays preserved
- ✅ $ref references preserved as strings

### ODCL Format

- ✅ 100% of specification fields preserved
- ✅ Quality rules with nested structures preserved
- ✅ $ref references preserved

### Other Formats

- ✅ Maximum possible field preservation based on format capabilities
- ✅ Missing fields set to `None` (not omitted from structure)
