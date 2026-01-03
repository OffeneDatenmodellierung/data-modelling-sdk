# OpenAPI to ODCS Converter API Contract

**Feature**: 004-bpmn-dmn-openapi
**Date**: 2026-01-03

## Overview

API contract for converting OpenAPI 3.1.1 schema components to ODCS table definitions.

## Converter API

### OpenAPIToODCSConverter

```rust
pub struct OpenAPIToODCSConverter {
    /// Strategy for handling nested objects
    pub nested_object_strategy: NestedObjectStrategy,
    /// Whether to flatten simple nested objects
    pub flatten_simple_objects: bool,
}

pub enum NestedObjectStrategy {
    /// Create separate related tables for nested objects
    SeparateTables,
    /// Flatten nested objects into parent table
    Flatten,
    /// Create separate tables but allow flattening for simple cases
    Hybrid,
}

impl OpenAPIToODCSConverter {
    /// Create a new converter with default settings
    pub fn new() -> Self {
        Self {
            nested_object_strategy: NestedObjectStrategy::Hybrid,
            flatten_simple_objects: true,
        }
    }

    /// Convert an OpenAPI schema component to an ODCS table
    ///
    /// # Arguments
    /// * `openapi_content` - OpenAPI 3.1.1 YAML or JSON content
    /// * `component_name` - Name of the schema component to convert (e.g., "Order", "Customer")
    /// * `table_name` - Desired ODCS table name (if None, uses component_name)
    ///
    /// # Returns
    /// * `Ok(Table)` - Converted ODCS table
    /// * `Err(ConversionError)` - Conversion failed
    pub fn convert_component(
        &self,
        openapi_content: &str,
        component_name: &str,
        table_name: Option<&str>,
    ) -> Result<Table, ConversionError>;

    /// Convert multiple OpenAPI schema components to ODCS tables
    ///
    /// # Arguments
    /// * `openapi_content` - OpenAPI 3.1.1 YAML or JSON content
    /// * `component_names` - Names of schema components to convert
    ///
    /// # Returns
    /// * `Ok(Vec<Table>)` - Converted ODCS tables
    /// * `Err(ConversionError)` - Conversion failed for any component
    pub fn convert_components(
        &self,
        openapi_content: &str,
        component_names: &[&str],
    ) -> Result<Vec<Table>, ConversionError>;

    /// Get conversion warnings and applied mappings
    ///
    /// # Arguments
    /// * `openapi_content` - OpenAPI 3.1.1 content
    /// * `component_name` - Schema component name
    ///
    /// # Returns
    /// * `ConversionReport` - Detailed conversion report with warnings and mappings
    pub fn analyze_conversion(
        &self,
        openapi_content: &str,
        component_name: &str,
    ) -> Result<ConversionReport, ConversionError>;
}

/// Detailed conversion report
pub struct ConversionReport {
    /// Component name
    pub component_name: String,
    /// Target table name
    pub table_name: String,
    /// Applied type mappings
    pub mappings: Vec<TypeMappingRule>,
    /// Conversion warnings
    pub warnings: Vec<String>,
    /// Fields that couldn't be converted
    pub skipped_fields: Vec<SkippedField>,
    /// Estimated ODCS table structure (without actually converting)
    pub estimated_structure: Vec<EstimatedColumn>,
}

pub struct SkippedField {
    pub field_name: String,
    pub openapi_type: String,
    pub reason: String,
}

pub struct EstimatedColumn {
    pub name: String,
    pub odcs_type: String,
    pub nullable: bool,
    pub quality_rules: Vec<String>,
}
```

## Type Mapping

### TypeMappingRule

```rust
pub struct TypeMappingRule {
    /// OpenAPI type (e.g., "string", "integer", "object")
    pub openapi_type: String,
    /// OpenAPI format (e.g., "date-time", "email", "uuid")
    pub openapi_format: Option<String>,
    /// Mapped ODCS type (e.g., "text", "long", "timestamp")
    pub odcs_type: String,
    /// Preserved constraints as quality rules
    pub quality_rules: Vec<QualityRule>,
    /// Field name this mapping applies to
    pub field_name: String,
}
```

### Type Mapping Table

| OpenAPI Type | OpenAPI Format | ODCS Type | Quality Rules |
|--------------|----------------|-----------|---------------|
| `string` | `date` | `date` | - |
| `string` | `date-time` | `timestamp` | - |
| `string` | `email` | `text` | `format: email` |
| `string` | `uri` | `text` | `format: uri` |
| `string` | `uuid` | `text` | `format: uuid` |
| `string` | `password` | `text` | `pii: true` |
| `string` | - | `text` | `minLength`, `maxLength`, `pattern` |
| `integer` | `int32` | `long` | `minimum`, `maximum` |
| `integer` | `int64` | `long` | `minimum`, `maximum` |
| `number` | `float` | `double` | `minimum`, `maximum` |
| `number` | `double` | `double` | `minimum`, `maximum` |
| `boolean` | - | `boolean` | - |
| `array` | - | (via nested table or flattening) | `minItems`, `maxItems` |
| `object` | - | (via nested table or flattening) | - |
| `null` | - | `text` | `nullable: true` |

## Error Types

### ConversionError

```rust
pub enum ConversionError {
    /// OpenAPI component not found
    ComponentNotFound(String),
    /// OpenAPI type cannot be mapped to ODCS
    UnsupportedType {
        field_name: String,
        openapi_type: String,
        reason: String,
    },
    /// Type mapping failed
    InvalidMapping {
        field_name: String,
        openapi_type: String,
        error: String,
    },
    /// Generated ODCS table failed validation
    ValidationError(String),
    /// OpenAPI content invalid
    InvalidOpenAPI(String),
    /// Nested object handling failed
    NestedObjectError {
        field_name: String,
        error: String,
    },
}
```

## WASM Bindings

```rust
#[wasm_bindgen]
pub fn convert_openapi_to_odcs(
    openapi_content: &str,
    component_name: &str,
    table_name: Option<String>,
) -> Result<JsValue, JsValue>;

#[wasm_bindgen]
pub fn analyze_openapi_conversion(
    openapi_content: &str,
    component_name: &str,
) -> Result<JsValue, JsValue>;
```

## Usage Examples

### Convert OpenAPI Component to ODCS

```rust
use data_modelling_sdk::convert::openapi_to_odcs::OpenAPIToODCSConverter;

let converter = OpenAPIToODCSConverter::new();
let openapi_yaml = std::fs::read_to_string("api.yaml")?;

// Convert "Order" component to ODCS table
let table = converter.convert_component(&openapi_yaml, "Order", Some("orders"))?;

// Save as separate ODCS node (not replacing OpenAPI model)
let saver = ModelSaver::new(storage);
saver.save_table(workspace_path, "orders", &table).await?;
```

### Analyze Conversion Before Converting

```rust
use data_modelling_sdk::convert::openapi_to_odcs::OpenAPIToODCSConverter;

let converter = OpenAPIToODCSConverter::new();
let openapi_yaml = std::fs::read_to_string("api.yaml")?;

// Analyze conversion to see warnings and mappings
let report = converter.analyze_conversion(&openapi_yaml, "Order")?;

println!("Will create table: {}", report.table_name);
println!("Warnings: {:?}", report.warnings);
println!("Mappings: {:?}", report.mappings);

// Proceed with conversion if acceptable
if report.warnings.is_empty() {
    let table = converter.convert_component(&openapi_yaml, "Order", None)?;
    // Save table...
}
```

### WASM Usage

```javascript
import { convertOpenApiToOdcs } from 'data-modelling-sdk';

const openApiYaml = await fetch('api.yaml').then(r => r.text());
const result = convertOpenApiToOdcs(openApiYaml, 'Order', 'orders');

if (result.error) {
    console.error('Conversion failed:', result.error);
} else {
    console.log('ODCS Table:', result.table);
    // Save table as separate node from OpenAPI model
}
```

## Conversion Strategy

### Nested Objects

**SeparateTables Strategy**: Creates related tables for nested objects
- `Order` table with `customer_id` reference
- `Customer` table (separate)

**Flatten Strategy**: Flattens nested objects into parent
- `Order` table with `customer_name`, `customer_email`, etc.

**Hybrid Strategy** (default):
- Simple nested objects (< 3 fields, no nested objects) → Flatten
- Complex nested objects → Separate tables

### Arrays

- Arrays of primitives → Quality rule with `array` type hint
- Arrays of objects → Separate table with foreign key reference

### Constraints Preservation

- `minimum`/`maximum` → ODCS quality rules with `mustBeBetween`
- `minLength`/`maxLength` → ODCS quality rules with `minLength`/`maxLength`
- `pattern` → ODCS quality rules with `pattern`
- `enum` → ODCS quality rules with `allowedValues`
- `required` → ODCS column `required: true`
