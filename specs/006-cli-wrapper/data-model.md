# Data Model: CLI Wrapper

**Date**: 2026-01-05
**Feature**: CLI Wrapper for Data Modelling SDK

## Overview

The CLI wrapper operates on files and in-memory data structures. It does not maintain persistent state or database connections. The data model focuses on command structures, import/export operations, and error handling.

## Core Entities

### CliCommand

Represents the top-level CLI command structure.

**Fields**:
- `command`: Enum of `Import` or `Export` subcommands

**Relationships**:
- Contains one `ImportArgs` or `ExportArgs` instance

### ImportArgs

Arguments for import operations.

**Fields**:
- `format`: Enum (`Sql`, `Avro`, `JsonSchema`, `Protobuf`, `OpenApi`, `Odcs`)
- `input`: `InputSource` (file path, stdin, or command-line string)
- `dialect`: `Option<String>` (for SQL imports: postgres, mysql, sqlite, generic, databricks)
- `uuid_override`: `Option<Uuid>` (override table UUID, only for single-table imports)
- `resolve_references`: `bool` (default: true, resolve external references)
- `validate`: `bool` (default: true, validate schema before import)
- `pretty`: `bool` (default: false, pretty-print output)
- `jar_path`: `Option<PathBuf>` (for Protobuf JAR imports)
- `message_type`: `Option<String>` (for Protobuf JAR imports, filter by message type)

**Validation Rules**:
- `uuid_override` can only be set when importing a single table (validated after import)
- `dialect` must be provided for SQL imports
- `jar_path` must be provided for Protobuf JAR imports
- `message_type` only valid with `jar_path`

### ExportArgs

Arguments for export operations.

**Fields**:
- `format`: Enum (`Odcs`, `Avro`, `JsonSchema`, `Protobuf`, `ProtobufDescriptor`)
- `input`: `InputSource` (ODCS YAML file or JSON workspace)
- `output`: `PathBuf` (output file path)
- `force`: `bool` (default: false, overwrite existing files without prompt)
- `protoc_path`: `Option<PathBuf>` (custom path to `protoc` binary, default: system PATH)

**Validation Rules**:
- `output` file extension should match format (e.g., `.odcs.yaml` for ODCS, `.avsc` for AVRO)
- `protoc_path` only valid for `ProtobufDescriptor` format

### InputSource

Represents the source of input data.

**Variants**:
- `File(PathBuf)` - Read from file
- `Stdin` - Read from stdin
- `String(String)` - Provided as command-line argument (SQL only)

**Validation Rules**:
- File path must exist and be readable
- Stdin must have data available
- String input only valid for SQL format

### ImportResult

Result of an import operation (reuses SDK's `ImportResult`).

**Fields**:
- `tables`: `Vec<TableData>` - Parsed tables
- `tables_requiring_name`: `Vec<TableRequiringName>` - Tables needing name resolution
- `errors`: `Vec<ImportError>` - Parse errors and warnings
- `mappings`: `Vec<TypeMapping>` - Format-to-ODCS type mappings (for display)

**Relationships**:
- Contains multiple `TableData` instances
- Contains multiple `TypeMapping` instances

### ExportResult

Result of an export operation.

**Fields**:
- `output_file`: `PathBuf` - Path to exported file
- `tables_exported`: `usize` - Number of tables exported
- `warnings`: `Vec<String>` - Warnings about unsupported features

### TypeMapping

Represents a type mapping from source format to ODCS.

**Fields**:
- `source_type`: `String` - Source format type (e.g., "AVRO string", "JSON Schema integer")
- `odcs_type`: `String` - ODCS type (e.g., "STRING", "BIGINT")
- `table_name`: `Option<String>` - Table this mapping applies to
- `column_name`: `Option<String>` - Column this mapping applies to

### ExternalReference

Represents an external reference that needs resolution.

**Fields**:
- `reference`: `String` - Reference path (`$ref` value or `import` path)
- `source_file`: `PathBuf` - File containing the reference
- `resolved_content`: `Option<String>` - Resolved content (after fetching)
- `resolved_from`: `Option<ReferenceSource>` - Where reference was resolved from

**Relationships**:
- Belongs to a source file
- May reference another schema file

### ReferenceSource

Source of resolved external reference.

**Variants**:
- `LocalFile(PathBuf)` - Resolved from local file system
- `HttpUrl(Url)` - Resolved from HTTP/HTTPS URL

## Error Types

### CliError

CLI-specific error type (using `thiserror`).

**Variants**:
- `FileNotFound(PathBuf)` - Input file not found
- `FileReadError(PathBuf, String)` - Error reading file
- `FileWriteError(PathBuf, String)` - Error writing file
- `InvalidUuid(String)` - Invalid UUID format
- `MultipleTablesWithUuid(usize)` - UUID override with multiple tables
- `ProtocNotFound` - `protoc` binary not found
- `ProtocError(String)` - `protoc` execution error
- `NetworkError(String)` - Network error fetching external reference
- `ReferenceResolutionError(String)` - Error resolving external reference
- `ValidationError(String)` - Schema validation error
- `ImportError(ImportError)` - Error from SDK import module
- `ExportError(ExportError)` - Error from SDK export module
- `InvalidArgument(String)` - Invalid command-line argument

**Validation Rules**:
- All errors must provide clear, actionable messages
- Errors should include context (file paths, line numbers where applicable)

## State Transitions

### Import Workflow

1. **Parse Arguments** → `ImportArgs`
2. **Load Input** → `InputSource` → `String` (content)
3. **Resolve References** (if enabled) → `Vec<ExternalReference>`
4. **Validate Schema** (if enabled) → `ValidationResult`
5. **Import** → `ImportResult`
6. **Apply UUID Override** (if provided) → Validate single table → Override UUID
7. **Display Results** → Format and print `ImportResult`

### Export Workflow

1. **Parse Arguments** → `ExportArgs`
2. **Load Input** → `InputSource` → `DataModel` (workspace)
3. **Check Output File** → Exists? → Prompt or use `--force`
4. **Export** → `ExportResult`
5. **Write File** → Save exported content
6. **Display Results** → Print success message and warnings

## Data Flow

### Import Flow

```
CLI Args → ImportArgs → InputSource → Content String
                                    ↓
                          Resolve References (if enabled)
                                    ↓
                          Validate Schema (if enabled)
                                    ↓
                          SDK Importer → ImportResult
                                    ↓
                          Apply UUID Override (if provided)
                                    ↓
                          Format & Display
```

### Export Flow

```
CLI Args → ExportArgs → InputSource → DataModel
                                    ↓
                          Check Output File
                                    ↓
                          SDK Exporter → Export String
                                    ↓
                          Write File
                                    ↓
                          Display Success
```

## Constraints

1. **UUID Override**: Only valid when importing a single table
2. **File Overwrite**: Requires `--force` flag or user confirmation
3. **External References**: Only non-authenticated HTTP/HTTPS URLs supported
4. **Protobuf Descriptor**: Requires `protoc` binary availability
5. **Input Validation**: All file paths must exist and be readable
6. **Output Validation**: Output directories must exist or be creatable

## Relationships Summary

- `CliCommand` → `ImportArgs` | `ExportArgs`
- `ImportArgs` → `InputSource`, `Option<Uuid>`
- `ExportArgs` → `InputSource`, `PathBuf`
- `ImportResult` → `Vec<TableData>`, `Vec<TypeMapping>`
- `ExternalReference` → `ReferenceSource`
- `CliError` → Various error contexts
