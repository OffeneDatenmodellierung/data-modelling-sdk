# Data Model: WASM Module Parsing Function Exports

**Date**: 2026-01-02
**Feature**: WASM Module Parsing Function Exports

## Overview

This feature exposes import/export functions via WASM bindings. The data model focuses on the structures exchanged between JavaScript and WASM modules, which are primarily the existing SDK data structures serialized to/from JSON.

## Core Entities

### ODCS Workspace

Represents a complete data model containing tables, relationships, metadata, and configuration. This is the primary data structure exchanged between JavaScript and WASM.

**Structure** (from SDK `DataModel`):
- `tables: Vec<Table>` - Collection of table definitions
- `relationships: Vec<Relationship>` - Collection of relationships between tables
- Metadata and configuration (implicit in ODCS format)

**Serialization**: Serialized to/from JSON strings for JavaScript interop

### Data Model

Collection of tables and relationships that define the structure of data.

**Structure**:
- `tables: Vec<Table>` - Table definitions
- `relationships: Vec<Relationship>` - Relationship definitions

**Table Structure**:
- `id: Uuid` - Unique identifier
- `name: String` - Table name
- `columns: Vec<Column>` - Column definitions
- `database_type: Option<DatabaseType>` - Target database type
- `medallion_layer: Option<MedallionLayer>` - Data medallion layer
- `scd_pattern: Option<SCDPattern>` - Slowly Changing Dimension pattern
- `data_vault_classification: Option<DataVaultClassification>` - Data Vault classification
- Additional metadata fields

**Column Structure**:
- `name: String` - Column name
- `data_type: String` - Data type (database-specific)
- `nullable: bool` - Whether column allows NULL
- `primary_key: bool` - Whether column is part of primary key
- `foreign_key: Option<ForeignKey>` - Foreign key relationship
- Additional constraint and metadata fields

**Relationship Structure**:
- `id: Uuid` - Unique identifier
- `source_table_id: Uuid` - Source table ID
- `target_table_id: Uuid` - Target table ID
- `cardinality: RelationshipCardinality` - Relationship cardinality
- `relationship_type: RelationshipType` - Type of relationship
- Additional metadata fields

### Import Result

Contains parsed tables, tables requiring name input, errors/warnings, and optional AI suggestions from import operations.

**Structure**:
- `tables: Vec<TableData>` - Successfully parsed tables
- `tables_requiring_name: Vec<TableRequiringName>` - Tables that need name input
- `errors: Vec<ImportError>` - Parse errors and warnings
- `ai_suggestions: Option<Vec<JsonValue>>` - Optional AI-generated suggestions

**TableData**:
- `table_index: usize` - Index of table in source
- `name: Option<String>` - Table name (if available)
- `columns: Vec<ColumnData>` - Column definitions

**ColumnData**:
- `name: String` - Column name
- `data_type: String` - Data type
- `nullable: bool` - Nullable flag
- `primary_key: bool` - Primary key flag

### Export Result

Contains exported content as a string and format identifier for export operations.

**Structure**:
- `content: String` - Exported content (format-specific string)
- `format: String` - Format identifier (e.g., "odcs_v3_1_0", "sql_postgresql")

## Data Flow

### Import Flow

1. **Input**: JavaScript string (YAML, SQL, AVRO, JSON Schema, or Protobuf)
2. **Processing**: Rust import function parses content
3. **Output**: `ImportResult` serialized to JSON string
4. **JavaScript**: Parse JSON to get `ImportResult` object

### Export Flow

1. **Input**: JavaScript object (workspace/data model) serialized to JSON string
2. **Processing**: Rust export function converts to target format
3. **Output**: Format-specific string (YAML, SQL, AVRO, JSON Schema, or Protobuf)
4. **JavaScript**: Receive string directly

## Validation Rules

### Input Validation

- Table names: Max 255 chars, alphanumeric/hyphens/underscores, must start with letter/underscore
- Column names: Max 255 chars, alphanumeric/hyphens/underscores/dots, must start with letter/underscore
- YAML content: Must be valid YAML syntax
- SQL content: Must be valid SQL syntax for specified dialect
- JSON Schema: Must be valid JSON Schema specification
- AVRO: Must be valid AVRO schema JSON
- Protobuf: Must be valid Protobuf schema text

### Data Model Validation

- Table IDs must be unique UUIDs
- Column names must be unique within a table
- Relationships must reference existing tables
- Foreign keys must reference existing columns
- Circular dependency detection for relationships

## Error Handling

### Import Errors

**ImportError** enum:
- `ParseError(String)` - Parsing failure with message
- `ValidationError(String)` - Validation failure with message
- `IoError(String)` - I/O error (rare for WASM, but possible)

### Export Errors

**ExportError** enum:
- `SerializationError(String)` - Serialization failure
- `ValidationError(String)` - Validation failure
- `IoError(String)` - I/O error
- `ExportError(String)` - Generic export error

### Error Conversion

Rust errors are converted to JavaScript Error objects:
- Error type preserved in error message
- Error details included in error message
- Stack trace available in JavaScript

## Type Mapping

### Rust → JavaScript

| Rust Type | JavaScript Type | Conversion Method |
|-----------|----------------|-------------------|
| `String` | `string` | Direct (wasm-bindgen) |
| `&str` | `string` | Direct (wasm-bindgen) |
| `Vec<T>` | `Array<T>` | Serialize to JSON |
| `Option<T>` | `T \| null` | Serialize to JSON |
| `Uuid` | `string` | Serialize to JSON (string representation) |
| `ImportResult` | `object` | Serialize to JSON string, parse in JS |
| `ExportResult` | `object` | Serialize to JSON string, parse in JS |
| `Table` | `object` | Serialize to JSON string, parse in JS |
| `DataModel` | `object` | Serialize to JSON string, parse in JS |

### JavaScript → Rust

| JavaScript Type | Rust Type | Conversion Method |
|----------------|-----------|-------------------|
| `string` | `&str` | Direct (wasm-bindgen) |
| `string` (JSON) | `T` | Deserialize from JSON |
| `object` | `T` | Serialize to JSON string, deserialize in Rust |

## Performance Considerations

- Large data models (1000+ tables) may require significant memory
- YAML files up to 10MB must be handled efficiently
- JSON serialization/deserialization overhead for complex structures
- Consider streaming for very large exports (future enhancement)
