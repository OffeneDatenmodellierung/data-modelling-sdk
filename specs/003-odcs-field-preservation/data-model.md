# Data Model: Complete ODCS/ODCL Field Preservation & Universal Format Conversion

**Date**: 2026-01-27
**Feature**: Complete ODCS/ODCL Field Preservation & Universal Format Conversion

## Enhanced ColumnData Structure

### Current Structure

```rust
pub struct ColumnData {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}
```

### Enhanced Structure

```rust
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

**Changes**:
- Added `description: Option<String>` - Preserves column descriptions from source formats
- Added `quality: Option<Vec<HashMap<String, serde_json::Value>>>` - Preserves quality rules with nested structures
- Added `ref_path: Option<String>` - Preserves JSON Schema $ref references

**Backward Compatibility**: All new fields are optional and default to `None`. Existing code continues to work.

## Enhanced Column Structure

### Current Structure

The `Column` struct already includes `description` and `quality` fields. We need to add `$ref` support.

### Enhanced Structure

```rust
pub struct Column {
    // ... existing fields ...
    pub description: String,
    pub quality: Vec<HashMap<String, serde_json::Value>>,
    /// JSON Schema $ref reference path
    #[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]
    pub ref_path: Option<String>,
    // ... other existing fields ...
}
```

**Changes**:
- Added `ref_path: Option<String>` - Stores $ref references from source formats

## Universal Converter Structure

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

### Converter Function

```rust
pub fn convert_to_odcs(
    input: &str,
    format: Option<&str>, // "sql", "json_schema", "avro", "protobuf", "odcl", "odcs", or None for auto-detect
) -> Result<String, ConversionError>
```

**Behavior**:
1. If `format` is `None`, attempt auto-detection based on input content
2. Use appropriate importer to parse input to `Table` structures
3. Use `ODCSExporter` to convert `Table` structures to ODCS v3.1.0 YAML
4. Return ODCS YAML string

## Format Coverage Matrix

| Format | Description | Quality | $ref | Enum | Format/Pattern | Constraints | Default |
|--------|-------------|---------|------|------|----------------|-------------|---------|
| ODCS v3.1.0 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| ODCL | ✅ | ✅ | ✅ | ✅ | ✅ | Partial | ✅ |
| SQL | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| JSON Schema | ✅ | ❌* | ✅ | ✅ | ✅ | ✅ | ✅ |
| AVRO | ✅ (doc) | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Protobuf | ✅ (comments) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

*JSON Schema doesn't have native quality rules, but custom extensions could be supported

## Data Flow

### Import Flow (Enhanced)

1. **Input**: Format-specific string (ODCS, ODCL, SQL, JSON Schema, AVRO, Protobuf)
2. **Parsing**: Format-specific importer extracts all available fields
3. **Mapping**: Fields mapped to `Column` struct (including description, quality, ref_path)
4. **Conversion**: `Column` converted to `ColumnData` with all fields preserved
5. **Output**: `ImportResult` with complete `ColumnData` structures

### Universal Conversion Flow

1. **Input**: Format-specific string + optional format identifier
2. **Detection**: Auto-detect format if not specified
3. **Import**: Use format-specific importer to create `Table` structures
4. **Export**: Use `ODCSExporter` to convert `Table` structures to ODCS YAML
5. **Output**: ODCS v3.1.0 YAML string

### WASM Conversion Flow

1. **Input**: JavaScript string (format-specific content) + optional format string
2. **Processing**: Rust `convert_to_odcs()` function
3. **Output**: ODCS YAML string (or JavaScript Error)

## Validation Rules

### Field Preservation

- **Description**: Must preserve empty strings (not omit field)
- **Quality**: Must preserve empty arrays (not omit field)
- **$ref**: Must preserve reference path even if definition doesn't exist
- **Nested Structures**: Must preserve all nesting levels in quality arrays

### Format-Specific Handling

- **SQL**: Extract available fields, set missing fields to `None`
- **JSON Schema**: Map all JSON Schema fields to ODCS equivalents
- **AVRO**: Map `doc` to `description`, preserve `default` and `logicalType`
- **Protobuf**: Extract comments as `description`, preserve field options

## State Transitions

### ColumnData Creation

1. **From ODCS/ODCL**: All fields populated from source
2. **From SQL**: Only basic fields (name, data_type, nullable, primary_key)
3. **From JSON Schema**: Description, $ref, enum, format/pattern populated
4. **From AVRO**: Description (from doc), default populated
5. **From Protobuf**: Description (from comments) populated

### Universal Conversion

1. **Any Format → ODCS**: All available fields preserved, missing fields omitted in ODCS output
2. **ODCS → ODCS**: Round-trip preserves 100% of fields
3. **Format → ODCS → Format**: May lose format-specific fields not supported by ODCS
