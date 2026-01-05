# Quickstart: ODPS Schema Validation

**Feature**: ODPS Schema Validation and Manual Test Script
**Date**: 2026-01-05

## Overview

This quickstart guide demonstrates how to use ODPS schema validation in the Data Modelling SDK, including CLI commands and the manual test script.

## Prerequisites

1. **Build with ODPS Validation Feature**:
   ```bash
   cargo build --release --features cli,odps-validation
   ```

2. **ODPS YAML File**: Have an ODPS YAML file ready for testing

## Basic Usage

### Import ODPS File

Import and validate an ODPS YAML file:

```bash
# Basic import
data-modelling-cli import odps product.odps.yaml

# Import with pretty output
data-modelling-cli import odps product.odps.yaml --pretty
```

**Example Output**:
```
Imported ODPS Data Product:
  ID: 550e8400-e29b-41d4-a716-446655440000
  Name: customer-data-product
  Version: 1.0.0
  Status: active
  Input Ports: 2
  Output Ports: 1
```

### Export to ODPS Format

Export ODCS tables to ODPS format:

```bash
# Export ODCS to ODPS
data-modelling-cli export odps input.odcs.yaml output.odps.yaml

# Export with force overwrite
data-modelling-cli export odps input.odcs.yaml output.odps.yaml --force
```

**Note**: ODPS export uses ODCS as input (ODPS is standalone, no direct ODPS-to-ODPS conversion).

### Manual Test Script

Test ODPS import/export round-trip:

```bash
# Basic test
scripts/test-odps.sh product.odps.yaml

# Test with verbose output
scripts/test-odps.sh product.odps.yaml --verbose
```

**Example Output**:
```
ODPS Round-Trip Test
====================

Importing: product.odps.yaml
✓ Import successful
✓ Schema validation passed

Imported Data:
  ID: 550e8400-e29b-41d4-a716-446655440000
  Name: customer-data-product
  Status: active
  Input Ports: 2
  Output Ports: 1

Exporting to: product.odps.yaml.exported.yaml
✓ Export successful
✓ Schema validation passed

Field Preservation:
✓ All fields preserved
✓ Required fields: 4/4
✓ Optional fields: 12/12

Round-trip test: PASSED
```

## Validation Examples

### Valid ODPS File

```yaml
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: active
name: customer-data-product
version: 1.0.0
```

**Result**: ✓ Validation passes

### Invalid ODPS File (Missing Required Field)

```yaml
apiVersion: v1.0.0
kind: DataProduct
# Missing 'id' field
status: active
```

**Result**: ✗ Validation fails
```
Error: ODPS validation failed:
/id: missing required field
```

### Invalid ODPS File (Invalid Enum Value)

```yaml
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
status: invalid-status  # Invalid enum value
```

**Result**: ✗ Validation fails
```
Error: ODPS validation failed:
/status: invalid enum value 'invalid-status', expected one of: proposed, draft, active, deprecated, retired
```

## Library Usage

### Import with Validation

```rust
use data_modelling_sdk::import::odps::ODPSImporter;
use data_modelling_sdk::cli::validation::validate_odps;

#[cfg(feature = "odps-validation")]
fn import_odps_with_validation(yaml_content: &str) -> Result<ODPSDataProduct, Error> {
    // Validate before importing
    validate_odps(yaml_content)?;

    // Import
    let importer = ODPSImporter::new();
    let product = importer.import(yaml_content)?;

    Ok(product)
}
```

### Export with Validation

```rust
use data_modelling_sdk::export::odps::ODPSExporter;
use data_modelling_sdk::cli::validation::validate_odps;

#[cfg(feature = "odps-validation")]
fn export_odps_with_validation(product: &ODPSDataProduct) -> Result<String, Error> {
    let exporter = ODPSExporter;

    // Export
    let yaml = exporter.export(product)?;

    // Validate exported YAML
    validate_odps(&yaml)?;

    Ok(yaml)
}
```

## Field Preservation Testing

The test script verifies that all fields are preserved during round-trip:

```bash
# Test with verbose field comparison
scripts/test-odps.sh product.odps.yaml --verbose
```

**Verbose Output Example**:
```
Field Preservation (Verbose):
  ✓ apiVersion: preserved
  ✓ kind: preserved
  ✓ id: preserved
  ✓ status: preserved
  ✓ name: preserved
  ✓ version: preserved
  ✓ tags: preserved (empty array)
  ✓ inputPorts: preserved (2 items)
    ✓ inputPorts[0].name: preserved
    ✓ inputPorts[0].version: preserved
    ✓ inputPorts[0].contractId: preserved
  ✓ outputPorts: preserved (1 item)
  ...
```

## Error Handling

### Validation Errors

When validation fails, detailed error messages are provided:

```bash
$ data-modelling-cli import odps invalid.odps.yaml
Error: ODPS validation failed:
/support[0].url: invalid URI format 'not-a-valid-url'
/support[0]: missing required field 'channel'
/inputPorts[1]: missing required field 'contractId'
```

### File Errors

```bash
$ data-modelling-cli import odps missing.odps.yaml
Error: File not found: missing.odps.yaml
```

### Parse Errors

```bash
$ data-modelling-cli import odps invalid-yaml.odps.yaml
Error: Failed to parse YAML: expected key at line 3, column 1
```

## Feature Flag Behavior

### With Feature Enabled

```bash
# Build with validation
cargo build --release --features cli,odps-validation

# Validation is performed by default
data-modelling-cli import odps product.odps.yaml  # Validates

# Can disable validation if needed
data-modelling-cli import odps product.odps.yaml --no-validate  # Skips validation
```

### Without Feature Enabled

```bash
# Build without validation
cargo build --release --features cli

# Validation is not performed (backward compatible)
data-modelling-cli import odps product.odps.yaml  # No validation, works as before
```

## Next Steps

1. **Read the Full Specification**: See [spec.md](./spec.md) for complete requirements
2. **Review Implementation Plan**: See [plan.md](./plan.md) for technical details
3. **Check API Contracts**: See [contracts/cli-api.md](./contracts/cli-api.md) for API details
4. **Explore Data Model**: See [data-model.md](./data-model.md) for data structures

## Troubleshooting

### Validation Not Working

**Problem**: Validation errors not appearing

**Solution**: Ensure `odps-validation` feature is enabled:
```bash
cargo build --release --features cli,odps-validation
```

### Schema File Not Found

**Problem**: `Error: Failed to load ODPS schema`

**Solution**: Ensure `schemas/odps-json-schema-latest.json` exists in the repository

### Field Preservation Failures

**Problem**: Test script reports field preservation failures

**Solution**: Check that exporter preserves empty arrays/objects and all optional fields. Review exporter implementation in `src/export/odps.rs`
