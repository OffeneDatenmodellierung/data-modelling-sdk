# WASM Function Bindings Contract

**Date**: 2026-01-02
**Feature**: WASM Module Parsing Function Exports
**Target**: WebAssembly (wasm32-unknown-unknown)

## Overview

This document defines the WASM function bindings that will be exposed to JavaScript. All functions are synchronous and use JSON strings for complex data structure exchange.

## Function Signatures

### Import Functions

#### parseOdcsYaml

Parse ODCS YAML content and return a structured workspace representation.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn parse_odcs_yaml(yaml_content: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function parseOdcsYaml(yamlContent: string): string
```

**Parameters**:
- `yaml_content: &str` - ODCS YAML content as a string

**Returns**:
- `Ok(String)` - JSON string containing `ImportResult` object
- `Err(JsValue)` - JavaScript Error with error details

**Example**:
```javascript
const yaml = `apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
name: users
schema:
  fields:
    - name: id
      type: bigint`;

const resultJson = parseOdcsYaml(yaml);
const result = JSON.parse(resultJson);
// result.tables contains parsed tables
// result.errors contains any parse errors
```

#### importFromSql

Import data model from SQL CREATE TABLE statements.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn import_from_sql(sql_content: &str, dialect: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function importFromSql(sqlContent: string, dialect: string): string
```

**Parameters**:
- `sql_content: &str` - SQL CREATE TABLE statements
- `dialect: &str` - SQL dialect ("postgresql", "mysql", "sqlserver", "databricks")

**Returns**:
- `Ok(String)` - JSON string containing `ImportResult` object
- `Err(JsValue)` - JavaScript Error with error details

**Example**:
```javascript
const sql = `CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);`;

const resultJson = importFromSql(sql, "postgresql");
const result = JSON.parse(resultJson);
```

#### importFromAvro

Import data model from AVRO schema.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn import_from_avro(avro_content: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function importFromAvro(avroContent: string): string
```

**Parameters**:
- `avro_content: &str` - AVRO schema JSON as a string

**Returns**:
- `Ok(String)` - JSON string containing `ImportResult` object
- `Err(JsValue)` - JavaScript Error with error details

#### importFromJsonSchema

Import data model from JSON Schema definition.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn import_from_json_schema(json_schema_content: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function importFromJsonSchema(jsonSchemaContent: string): string
```

**Parameters**:
- `json_schema_content: &str` - JSON Schema definition as a string

**Returns**:
- `Ok(String)` - JSON string containing `ImportResult` object
- `Err(JsValue)` - JavaScript Error with error details

#### importFromProtobuf

Import data model from Protobuf schema.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn import_from_protobuf(protobuf_content: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function importFromProtobuf(protobufContent: string): string
```

**Parameters**:
- `protobuf_content: &str` - Protobuf schema text

**Returns**:
- `Ok(String)` - JSON string containing `ImportResult` object
- `Err(JsValue)` - JavaScript Error with error details

### Export Functions

#### exportToOdcsYaml

Export a workspace structure to ODCS YAML format.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn export_to_odcs_yaml(workspace_json: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function exportToOdcsYaml(workspaceJson: string): string
```

**Parameters**:
- `workspace_json: &str` - JSON string containing workspace/data model structure

**Returns**:
- `Ok(String)` - ODCS YAML format string
- `Err(JsValue)` - JavaScript Error with error details

**Example**:
```javascript
const workspace = {
  tables: [{
    id: "550e8400-e29b-41d4-a716-446655440000",
    name: "users",
    columns: [{
      name: "id",
      data_type: "bigint",
      nullable: false,
      primary_key: true
    }]
  }],
  relationships: []
};

const yaml = exportToOdcsYaml(JSON.stringify(workspace));
// yaml contains ODCS v3.1.0 YAML
```

#### exportToSql

Export a data model to SQL CREATE TABLE statements.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn export_to_sql(workspace_json: &str, dialect: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function exportToSql(workspaceJson: string, dialect: string): string
```

**Parameters**:
- `workspace_json: &str` - JSON string containing workspace/data model structure
- `dialect: &str` - SQL dialect ("postgresql", "mysql", "sqlserver", "databricks")

**Returns**:
- `Ok(String)` - SQL CREATE TABLE statements
- `Err(JsValue)` - JavaScript Error with error details

#### exportToAvro

Export a data model to AVRO schema.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn export_to_avro(workspace_json: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function exportToAvro(workspaceJson: string): string
```

**Parameters**:
- `workspace_json: &str` - JSON string containing workspace/data model structure

**Returns**:
- `Ok(String)` - AVRO schema JSON string
- `Err(JsValue)` - JavaScript Error with error details

#### exportToJsonSchema

Export a data model to JSON Schema definition.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn export_to_json_schema(workspace_json: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function exportToJsonSchema(workspaceJson: string): string
```

**Parameters**:
- `workspace_json: &str` - JSON string containing workspace/data model structure

**Returns**:
- `Ok(String)` - JSON Schema definition string
- `Err(JsValue)` - JavaScript Error with error details

#### exportToProtobuf

Export a data model to Protobuf schema.

**Rust Signature**:
```rust
#[wasm_bindgen]
pub fn export_to_protobuf(workspace_json: &str) -> Result<String, JsValue>
```

**JavaScript Signature**:
```typescript
export function exportToProtobuf(workspaceJson: string): string
```

**Parameters**:
- `workspace_json: &str` - JSON string containing workspace/data model structure

**Returns**:
- `Ok(String)` - Protobuf schema text
- `Err(JsValue)` - JavaScript Error with error details

## Type Definitions

### ImportResult

```typescript
interface ImportResult {
  tables: TableData[];
  tables_requiring_name: TableRequiringName[];
  errors: ImportError[];
  ai_suggestions?: any[];
}

interface TableData {
  table_index: number;
  name: string | null;
  columns: ColumnData[];
}

interface ColumnData {
  name: string;
  data_type: string;
  nullable: boolean;
  primary_key: boolean;
}

interface TableRequiringName {
  table_index: number;
  suggested_name: string | null;
}

interface ImportError {
  // Error type: "ParseError" | "ValidationError" | "IoError"
  message: string;
}
```

### Workspace Structure

```typescript
interface Workspace {
  tables: Table[];
  relationships: Relationship[];
}

interface Table {
  id: string; // UUID string
  name: string;
  columns: Column[];
  // ... additional metadata fields
}

interface Column {
  name: string;
  data_type: string;
  nullable: boolean;
  primary_key: boolean;
  // ... additional constraint fields
}

interface Relationship {
  id: string; // UUID string
  source_table_id: string; // UUID string
  target_table_id: string; // UUID string
  // ... additional relationship fields
}
```

## Error Handling

All functions return `Result<String, JsValue>`. Errors are converted to JavaScript Error objects:

```typescript
try {
  const result = parseOdcsYaml(yamlContent);
  const importResult = JSON.parse(result);
} catch (error) {
  // error is a JavaScript Error object
  // error.message contains the error message
  // error.stack contains the stack trace
}
```

## Performance Characteristics

- **Synchronous**: All functions are synchronous (blocking)
- **Memory**: Functions operate on in-memory data structures
- **Size Limits**: Designed to handle files up to 10MB
- **Performance**: Parse operations complete within 3 seconds for 5MB files

## Build Configuration

Functions are only compiled when:
- `target_arch = "wasm32"`
- `feature = "wasm"`

Build command:
```bash
wasm-pack build --target web --out-dir pkg --features wasm
```

## TypeScript Definitions

TypeScript definitions are automatically generated by `wasm-pack` and written to `pkg/data_modelling_sdk.d.ts`. The definitions include:

- Function signatures
- Type definitions for complex types (via JSDoc comments)
- Error types

Manual TypeScript definitions are not required if bindings are properly annotated with `#[wasm_bindgen]`.
