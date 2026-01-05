# CLI API Contract: ODPS Import/Export

**Feature**: ODPS Schema Validation and Manual Test Script
**Date**: 2026-01-05
**Phase**: 1 - Design & Contracts

## Overview

This document defines the CLI API contract for ODPS import and export commands, following the existing CLI patterns established for ODCS and other formats.

## Import Command

### Command Syntax

```bash
data-modelling-cli import odps <file> [OPTIONS]
```

### Arguments

- `<file>` (required): Path to ODPS YAML file to import, or `-` for stdin

### Options

- `--uuid <UUID>`: Override table UUID (not applicable for ODPS, but kept for consistency)
- `--no-resolve-references`: Disable external reference resolution (not applicable for ODPS)
- `--no-validate`: Skip schema validation (only if `odps-validation` feature is enabled)
- `--no-odcs`: Don't write `.odcs.yaml` file after import (ODPS is standalone, so this is default behavior)
- `--pretty`: Pretty-print output with detailed information

### Behavior

1. **Read Input**: Load ODPS YAML from file or stdin
2. **Validate** (if `odps-validation` feature enabled and `--no-validate` not set):
   - Validate YAML against ODPS JSON Schema
   - Report validation errors with field paths and expected/actual values
   - Exit with error if validation fails
3. **Parse**: Parse validated YAML to `ODPSDataProduct` model
4. **Display**: Print imported data in compact or pretty format
5. **Exit**: Return success (0) or error (non-zero)

### Output Format

**Compact** (default):
```
Imported ODPS Data Product:
  ID: <id>
  Name: <name>
  Version: <version>
  Status: <status>
  Input Ports: <count>
  Output Ports: <count>
```

**Pretty** (`--pretty`):
```
ODPS Data Product
=================
ID:              <id>
Name:            <name>
Version:         <version>
Status:          <status>
Domain:          <domain>
Tenant:          <tenant>

Input Ports (<count>):
  - <port-name> v<version> (contract: <contract-id>)
  ...

Output Ports (<count>):
  - <port-name> v<version>
  ...

Support Channels (<count>):
  - <channel>: <url>
  ...

Team:
  Name: <team-name>
  Members: <count>
    - <username> (<role>)
    ...
```

### Error Handling

- **File Not Found**: `Error: File not found: <path>`
- **Invalid YAML**: `Error: Failed to parse YAML: <error>`
- **Validation Error**: `Error: ODPS validation failed:\n<field-path>: <error-details>`
- **Parse Error**: `Error: Failed to parse ODPS: <error>`

### Exit Codes

- `0`: Success
- `1`: General error (file not found, parse error, etc.)
- `2`: Validation error

---

## Export Command

### Command Syntax

```bash
data-modelling-cli export odps <input> <output> [OPTIONS]
```

### Arguments

- `<input>` (required): Path to input ODCS YAML file (`.odcs.yaml`)
- `<output>` (required): Path to output ODPS YAML file

### Options

- `--force`: Overwrite existing output file without prompting
- `--no-validate`: Skip schema validation (only if `odps-validation` feature is enabled)

### Behavior

1. **Read Input**: Load ODCS YAML from input file
2. **Convert**: Convert ODCS tables to ODPS data product (note: this is a conversion, not direct ODPS-to-ODPS)
3. **Export**: Serialize ODPS data product to YAML
4. **Validate** (if `odps-validation` feature enabled and `--no-validate` not set):
   - Validate exported YAML against ODPS JSON Schema
   - Report validation errors with field paths and expected/actual values
   - Exit with error if validation fails
5. **Write**: Write validated YAML to output file (prompt if file exists and `--force` not set)
6. **Exit**: Return success (0) or error (non-zero)

### Output Format

ODPS YAML file written to `<output>` path.

### Error Handling

- **Input File Not Found**: `Error: Input file not found: <path>`
- **Invalid ODCS**: `Error: Failed to parse ODCS: <error>`
- **Conversion Error**: `Error: Failed to convert ODCS to ODPS: <error>`
- **Validation Error**: `Error: ODPS validation failed:\n<field-path>: <error-details>`
- **Write Error**: `Error: Failed to write output file: <error>`
- **File Exists**: Prompt user: `File <path> already exists. Overwrite? [y/N]`

### Exit Codes

- `0`: Success
- `1`: General error (file not found, parse error, write error, etc.)
- `2`: Validation error
- `3`: User declined overwrite

---

## Library API

### Validation Function

```rust
// In src/cli/validation.rs

#[cfg(feature = "odps-validation")]
pub fn validate_odps(content: &str) -> Result<(), CliError> {
    // Validates ODPS YAML content against ODPS JSON Schema
    // Returns Ok(()) if valid, Err(CliError::ValidationError(...)) if invalid
}

#[cfg(not(feature = "odps-validation"))]
pub fn validate_odps(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}
```

### Import Handler

```rust
// In src/cli/commands/import.rs

pub fn handle_import_odps(args: &ImportArgs) -> Result<(), CliError> {
    // 1. Load ODPS YAML content
    // 2. Validate against schema (if feature enabled)
    // 3. Import using ODPSImporter
    // 4. Display results
    // 5. Return success/error
}
```

### Export Handler

```rust
// In src/cli/commands/export.rs

pub fn handle_export_odps(args: &ExportArgs) -> Result<(), CliError> {
    // 1. Load ODCS input (ODPS export uses ODCS as source)
    // 2. Convert to ODPS data product
    // 3. Export using ODPSExporter
    // 4. Validate exported YAML (if feature enabled)
    // 5. Write to output file
    // 6. Return success/error
}
```

---

## Test Script API

### Command Syntax

```bash
scripts/test-odps.sh <odps-file> [OPTIONS]
```

Or if implemented as Rust binary:

```bash
cargo run --bin test-odps -- <odps-file> [OPTIONS]
```

### Arguments

- `<odps-file>` (required): Path to ODPS YAML file to test

### Options

- `--output <path>`: Path to write exported ODPS file (default: `<input>.exported.yaml`)
- `--no-validate`: Skip schema validation
- `--verbose`: Display detailed field-by-field comparison results
- `--help`: Display usage instructions

### Behavior

1. **Import**: Import ODPS file and validate
2. **Display**: Show imported data in human-readable format
3. **Export**: Export imported data back to ODPS YAML
4. **Validate**: Validate exported YAML against schema
5. **Compare**: Compare original and exported files field-by-field
6. **Report**: Display validation and comparison results

### Output Format

```
ODPS Round-Trip Test
====================

Importing: <file>
✓ Import successful
✓ Schema validation passed

Imported Data:
  ID: <id>
  Name: <name>
  Status: <status>
  ...

Exporting to: <output-file>
✓ Export successful
✓ Schema validation passed

Field Preservation:
✓ All fields preserved
✓ Required fields: 4/4
✓ Optional fields: 12/12
✓ Nested structures: 5/5

Round-trip test: PASSED
```

### Error Handling

- **File Not Found**: `Error: File not found: <path>`
- **Import Error**: `Error: Import failed: <error>`
- **Validation Error**: `Error: Validation failed: <error>`
- **Export Error**: `Error: Export failed: <error>`
- **Field Mismatch**: `Warning: Field <path> differs: <details>`

### Exit Codes

- `0`: All tests passed
- `1`: Import/export/validation error
- `2`: Field preservation failure

---

## Feature Flag Behavior

### With `odps-validation` Feature Enabled

- Validation is performed by default
- `--no-validate` flag can disable validation
- Validation errors cause command to fail

### Without `odps-validation` Feature Enabled

- Validation is not performed (no-op)
- `--no-validate` flag has no effect
- Commands proceed without validation (backward compatible)

---

## Examples

### Import ODPS File

```bash
# Import and validate
data-modelling-cli import odps product.odps.yaml

# Import with pretty output
data-modelling-cli import odps product.odps.yaml --pretty

# Import without validation (if feature enabled)
data-modelling-cli import odps product.odps.yaml --no-validate
```

### Export to ODPS Format

```bash
# Export ODCS to ODPS (with validation)
data-modelling-cli export odps input.odcs.yaml output.odps.yaml

# Export with force overwrite
data-modelling-cli export odps input.odcs.yaml output.odps.yaml --force

# Export without validation (if feature enabled)
data-modelling-cli export odps input.odcs.yaml output.odps.yaml --no-validate
```

### Test Script Usage

```bash
# Basic round-trip test
scripts/test-odps.sh product.odps.yaml

# Test with custom output path
scripts/test-odps.sh product.odps.yaml --output test-output.odps.yaml

# Verbose comparison
scripts/test-odps.sh product.odps.yaml --verbose
```
