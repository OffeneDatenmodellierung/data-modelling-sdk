# Quickstart: WASM Module Parsing Function Exports

**Date**: 2026-01-02
**Feature**: WASM Module Parsing Function Exports

## Overview

This quickstart guide demonstrates how to use the WASM module parsing and export functions in a browser environment.

## Prerequisites

- Built WASM module (`pkg/data_modelling_sdk.wasm` and `pkg/data_modelling_sdk.js`)
- Modern browser with WebAssembly support
- Basic knowledge of JavaScript/TypeScript

## Setup

### 1. Build the WASM Module

```bash
cd data-modelling-sdk
wasm-pack build --target web --out-dir pkg --features wasm
```

This generates:
- `pkg/data_modelling_sdk.wasm` - WebAssembly binary
- `pkg/data_modelling_sdk.js` - JavaScript bindings
- `pkg/data_modelling_sdk.d.ts` - TypeScript definitions

### 2. Copy Files to Your Project

Copy the generated files to your web application's public directory:

```bash
cp -r pkg/* frontend/public/wasm/
```

### 3. Load the Module

```javascript
// Load the WASM module
import init, { parseOdcsYaml, exportToOdcsYaml } from '/wasm/data_modelling_sdk.js';

// Initialize the module
await init();
// or use initSync() for synchronous initialization
// initSync(wasmModule);
```

## Basic Usage

### Parse ODCS YAML

```javascript
const yamlContent = `
apiVersion: v3.1.0
kind: DataContract
id: 550e8400-e29b-41d4-a716-446655440000
version: 1.0.0
name: users
schema:
  fields:
    - name: id
      type: bigint
      nullable: false
    - name: name
      type: varchar
      nullable: false
`;

try {
  const resultJson = parseOdcsYaml(yamlContent);
  const result = JSON.parse(resultJson);

  console.log('Parsed tables:', result.tables);
  console.log('Errors:', result.errors);

  // Access parsed table data
  result.tables.forEach(table => {
    console.log(`Table: ${table.name}`);
    table.columns.forEach(column => {
      console.log(`  Column: ${column.name} (${column.data_type})`);
    });
  });
} catch (error) {
  console.error('Parse error:', error.message);
}
```

### Export to ODCS YAML

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
    }, {
      name: "name",
      data_type: "varchar(255)",
      nullable: false,
      primary_key: false
    }]
  }],
  relationships: []
};

try {
  const yaml = exportToOdcsYaml(JSON.stringify(workspace));
  console.log('Exported YAML:', yaml);

  // Save to file or use in application
  downloadFile(yaml, 'workspace.yaml', 'text/yaml');
} catch (error) {
  console.error('Export error:', error.message);
}
```

### Import from SQL

```javascript
const sqlContent = `
CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE
);

CREATE TABLE orders (
    id BIGINT PRIMARY KEY,
    user_id BIGINT REFERENCES users(id),
    total DECIMAL(10, 2)
);
`;

try {
  const resultJson = importFromSql(sqlContent, "postgresql");
  const result = JSON.parse(resultJson);

  console.log('Imported tables:', result.tables);

  // Handle tables requiring names
  if (result.tables_requiring_name.length > 0) {
    console.log('Tables needing names:', result.tables_requiring_name);
  }
} catch (error) {
  console.error('Import error:', error.message);
}
```

### Export to SQL

```javascript
const workspace = {
  tables: [{
    id: "550e8400-e29b-41d4-a716-446655440000",
    name: "users",
    columns: [{
      name: "id",
      data_type: "BIGINT",
      nullable: false,
      primary_key: true
    }]
  }],
  relationships: []
};

try {
  const sql = exportToSql(JSON.stringify(workspace), "postgresql");
  console.log('Generated SQL:', sql);
} catch (error) {
  console.error('Export error:', error.message);
}
```

## Advanced Usage

### Error Handling

```javascript
function safeParseOdcsYaml(yamlContent) {
  try {
    const resultJson = parseOdcsYaml(yamlContent);
    const result = JSON.parse(resultJson);

    if (result.errors.length > 0) {
      // Handle parse warnings/errors
      console.warn('Parse warnings:', result.errors);
    }

    return {
      success: true,
      data: result,
      errors: result.errors
    };
  } catch (error) {
    return {
      success: false,
      error: error.message,
      data: null
    };
  }
}
```

### Batch Processing

```javascript
async function processMultipleFiles(files) {
  const results = [];

  for (const file of files) {
    const content = await file.text();
    const result = safeParseOdcsYaml(content);
    results.push({
      filename: file.name,
      ...result
    });
  }

  return results;
}
```

### Format Conversion

```javascript
// Convert SQL to ODCS YAML
function sqlToOdcsYaml(sqlContent, dialect = "postgresql") {
  // Import from SQL
  const importResult = JSON.parse(importFromSql(sqlContent, dialect));

  // Build workspace structure
  const workspace = {
    tables: importResult.tables.map(tableData => ({
      id: generateUuid(),
      name: tableData.name || `table_${tableData.table_index}`,
      columns: tableData.columns.map(col => ({
        name: col.name,
        data_type: col.data_type,
        nullable: col.nullable,
        primary_key: col.primary_key
      }))
    })),
    relationships: []
  };

  // Export to ODCS YAML
  return exportToOdcsYaml(JSON.stringify(workspace));
}
```

## TypeScript Usage

```typescript
import init, {
  parseOdcsYaml,
  exportToOdcsYaml,
  importFromSql,
  exportToSql
} from '/wasm/data_modelling_sdk';

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

// Initialize
await init();

// Use with type safety
const resultJson: string = parseOdcsYaml(yamlContent);
const result: ImportResult = JSON.parse(resultJson);
```

## Performance Tips

1. **Initialize Once**: Call `init()` or `initSync()` only once per page load
2. **Reuse Module**: Store the initialized module reference
3. **Large Files**: For files >5MB, consider processing in chunks or using Web Workers
4. **Error Handling**: Always wrap calls in try-catch blocks

## Troubleshooting

### Module Not Loading

```javascript
// Check if module is loaded
if (typeof parseOdcsYaml === 'undefined') {
  console.error('WASM module not initialized. Call init() first.');
}
```

### Memory Issues

```javascript
// For very large files, process in chunks
function processLargeYaml(yamlContent) {
  // Split into multiple documents if possible
  const documents = yamlContent.split('---\n');
  return documents.map(doc => parseOdcsYaml(doc));
}
```

### Type Errors

Ensure TypeScript definitions are included:

```typescript
/// <reference path="./wasm/data_modelling_sdk.d.ts" />
```

## Next Steps

- Review the [data model documentation](data-model.md) for structure details
- Check [WASM bindings contract](contracts/wasm-bindings.md) for complete API reference
- See [implementation plan](plan.md) for technical details
