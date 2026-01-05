# CLI API Contract

**Date**: 2026-01-05
**Feature**: CLI Wrapper for Data Modelling SDK

## Command Structure

### Top-Level Command

```bash
data-modelling-cli [OPTIONS] <COMMAND>
```

**Global Options**:
- `-h, --help` - Print help information
- `-V, --version` - Print version information

### Subcommands

#### Import

```bash
data-modelling-cli import [OPTIONS] <FORMAT> <INPUT>
```

**Positional Arguments**:
- `FORMAT` - Format to import from: `sql`, `avro`, `json-schema`, `protobuf`, `openapi`, `odcs`
- `INPUT` - Input source (file path, `-` for stdin, or SQL string for SQL format)

**Options**:
- `-d, --dialect <DIALECT>` - SQL dialect (postgres, mysql, sqlite, generic, databricks) [required for SQL format]
- `-u, --uuid <UUID>` - Override table UUID (only for single-table imports)
- `--no-resolve-references` - Disable automatic external reference resolution [default: resolve enabled]
- `--no-validate` - Skip schema validation before import [default: validate enabled]
- `-p, --pretty` - Pretty-print output with detailed information
- `--jar <JAR_PATH>` - JAR file path (for Protobuf JAR imports)
- `--message-type <MESSAGE_TYPE>` - Filter by message type (for Protobuf JAR imports)

**Examples**:
```bash
# Import SQL from file
data-modelling-cli import sql schema.sql --dialect postgres

# Import AVRO with UUID override
data-modelling-cli import avro schema.avsc --uuid 550e8400-e29b-41d4-a716-446655440000

# Import from stdin
cat schema.json | data-modelling-cli import json-schema -

# Import Protobuf from JAR
data-modelling-cli import protobuf --jar library.jar --message-type com.example.Person
```

#### Export

```bash
data-modelling-cli export [OPTIONS] <FORMAT> <INPUT> <OUTPUT>
```

**Positional Arguments**:
- `FORMAT` - Format to export to: `odcs`, `avro`, `json-schema`, `protobuf`, `protobuf-descriptor`
- `INPUT` - Input ODCS YAML file or JSON workspace
- `OUTPUT` - Output file path

**Options**:
- `-f, --force` - Overwrite existing files without prompting
- `--protoc-path <PROTOC_PATH>` - Custom path to `protoc` binary (for protobuf-descriptor format)

**Examples**:
```bash
# Export to ODCS YAML
data-modelling-cli export odcs workspace.json users.odcs.yaml

# Export to AVRO
data-modelling-cli export avro schema.odcs.yaml schema.avsc

# Export Protobuf descriptor
data-modelling-cli export protobuf-descriptor schema.proto schema.pb

# Force overwrite
data-modelling-cli export odcs workspace.json output.odcs.yaml --force
```

## Command Behavior

### Import Command

1. **Input Loading**:
   - If `INPUT` is a file path, read from file
   - If `INPUT` is `-`, read from stdin
   - If `INPUT` is a string (SQL format only), use directly

2. **Reference Resolution** (if enabled):
   - Resolve `$ref` references from same directory as source file
   - Fetch HTTP/HTTPS URLs (non-authenticated only)
   - Report errors for missing files or authenticated URLs

3. **Validation** (if enabled):
   - Validate schema against format specification
   - Report validation errors before proceeding
   - Continue only if validation passes (or with warnings)

4. **Import**:
   - Call appropriate SDK importer
   - Collect type mappings for display
   - Handle import errors gracefully

5. **UUID Override** (if provided):
   - Check that exactly one table was imported
   - If multiple tables, return error
   - Override table UUID with provided value

6. **Output**:
   - Display parsed tables, columns, and mappings
   - Show errors and warnings
   - Format according to `--pretty` flag

### Export Command

1. **Input Loading**:
   - Load ODCS YAML file or JSON workspace
   - Parse into `DataModel` structure

2. **Output File Check**:
   - Check if output file exists
   - If exists and `--force` not set, prompt for confirmation
   - If `--force` set, proceed without prompt

3. **Export**:
   - Call appropriate SDK exporter
   - Preserve validation logic and quality rules where possible
   - Collect warnings about unsupported features

4. **File Writing**:
   - Write exported content to output file
   - Create output directory if needed
   - Handle file I/O errors

5. **Output**:
   - Display success message
   - Show warnings about unsupported features

## Error Handling

### Error Codes

- `0` - Success
- `1` - General error (file I/O, validation, etc.)
- `2` - Invalid arguments
- `3` - External tool error (`protoc` not found, etc.)

### Error Messages

All errors must:
- Be clear and actionable
- Include relevant context (file paths, line numbers)
- Suggest solutions when possible

**Examples**:
```
Error: File not found: schema.avsc
  Hint: Check the file path and ensure the file exists

Error: Invalid UUID format: 'not-a-uuid'
  Expected format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx

Error: UUID override is only supported when importing a single table. Found 3 tables.
  Hint: Import tables separately or remove --uuid flag

Error: protoc not found
  Install from: https://protobuf.dev/downloads
  Or specify custom path with --protoc-path
```

## Output Format

### Compact Mode (default)

```
✅ Parsed 2 table(s):

Table 1:
  Name: users
  Columns: id:bigint, name:string, email:string

Table 2:
  Name: orders
  Columns: id:bigint, user_id:bigint, total:decimal
```

### Pretty Mode (`--pretty`)

```
✅ Parsed 2 table(s):

Table 1: users
  Columns: 3
  Column Details:
    - id (bigint)
      Primary Key: true
      Nullable: false
    - name (string)
      Nullable: true
      Comment: User's full name
    - email (string)
      Nullable: false
      Comment: User's email address

Type Mappings:
  - AVRO 'string' → ODCS 'STRING'
  - AVRO 'long' → ODCS 'BIGINT'
```

## Exit Codes

- `0` - Success
- `1` - Error (file I/O, validation, import/export failure)
- `2` - Invalid arguments
- `3` - External tool error (`protoc` not found)

## Environment Variables

- `DATA_MODELLING_CLI_PROTOC_PATH` - Default path to `protoc` binary (can be overridden with `--protoc-path`)

## Constraints

1. **UUID Override**: Only valid for single-table imports
2. **File Overwrite**: Requires `--force` or user confirmation
3. **External References**: Only non-authenticated HTTP/HTTPS URLs
4. **Protobuf Descriptor**: Requires `protoc` binary
5. **Input Formats**: Must match specified format type
6. **Output Formats**: File extensions should match format (warnings if mismatch)
