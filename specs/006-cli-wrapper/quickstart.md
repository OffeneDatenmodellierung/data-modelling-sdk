# Quickstart: CLI Wrapper

**Date**: 2026-01-05
**Feature**: CLI Wrapper for Data Modelling SDK

## Installation

The CLI will be built as a binary from the SDK repository:

```bash
# Build the CLI binary
cargo build --release --bin data-modelling-cli

# Or install globally (when implemented)
cargo install --path . --bin data-modelling-cli
```

The binary will be available at `target/release/data-modelling-cli` (or `target/debug/data-modelling-cli` for debug builds).

## Basic Usage

### Import SQL Schema

```bash
# Import PostgreSQL SQL from file
data-modelling-cli import sql schema.sql --dialect postgres

# Import Databricks SQL from stdin
cat schema.sql | data-modelling-cli import sql - --dialect databricks

# Import with pretty output
data-modelling-cli import sql schema.sql --dialect postgres --pretty
```

### Import AVRO Schema

```bash
# Basic import
data-modelling-cli import avro schema.avsc

# Import with UUID override (for schema versioning)
data-modelling-cli import avro schema-v1.0.avsc --uuid 550e8400-e29b-41d4-a716-446655440000

# Import with pretty output showing mappings
data-modelling-cli import avro schema.avsc --pretty
```

### Import JSON Schema

```bash
# Import JSON Schema
data-modelling-cli import json-schema schema.json

# Import with external references resolved
data-modelling-cli import json-schema schema.json  # References resolved by default

# Skip reference resolution
data-modelling-cli import json-schema schema.json --no-resolve-references
```

### Import Protobuf

```bash
# Import from .proto file
data-modelling-cli import protobuf schema.proto

# Import from JAR file
data-modelling-cli import protobuf --jar library.jar

# Import specific message type from JAR
data-modelling-cli import protobuf --jar library.jar --message-type com.example.Person
```

### Import OpenAPI

```bash
# Import OpenAPI spec
data-modelling-cli import openapi api.yaml

# Import with validation disabled (faster, but less safe)
data-modelling-cli import openapi api.yaml --no-validate
```

### Import ODCS

```bash
# Import and validate ODCS YAML
data-modelling-cli import odcs table.odcs.yaml

# Import without validation
data-modelling-cli import odcs table.odcs.yaml --no-validate
```

## Export Operations

### Export to ODCS YAML

```bash
# Export workspace to ODCS YAML
data-modelling-cli export odcs workspace.json users.odcs.yaml

# Export with force overwrite
data-modelling-cli export odcs workspace.json output.odcs.yaml --force
```

### Export to AVRO

```bash
# Export ODCS schema to AVRO
data-modelling-cli export avro schema.odcs.yaml schema.avsc
```

### Export to JSON Schema

```bash
# Export ODCS schema to JSON Schema
data-modelling-cli export json-schema schema.odcs.yaml schema.json
```

### Export to Protobuf

```bash
# Export ODCS schema to Protobuf
data-modelling-cli export protobuf schema.odcs.yaml schema.proto
```

### Export Protobuf Descriptor

```bash
# Export .proto file to binary descriptor (requires protoc)
data-modelling-cli export protobuf-descriptor schema.proto schema.pb

# Use custom protoc path
data-modelling-cli export protobuf-descriptor schema.proto schema.pb --protoc-path /usr/local/bin/protoc
```

## Common Workflows

### Schema Versioning with UUID Override

Track schema evolution by maintaining consistent UUIDs:

```bash
# Import version 1.0
data-modelling-cli import avro schema-v1.0.avsc --uuid 550e8400-e29b-41d4-a716-446655440000 > v1.0.odcs.yaml

# Import version 1.1 (same UUID)
data-modelling-cli import avro schema-v1.1.avsc --uuid 550e8400-e29b-41d4-a716-446655440000 > v1.1.odcs.yaml

# Import version 1.2 (same UUID)
data-modelling-cli import avro schema-v1.2.avsc --uuid 550e8400-e29b-41d4-a716-446655440000 > v1.2.odcs.yaml
```

### Convert SQL to ODCS

```bash
# Import SQL and export to ODCS
data-modelling-cli import sql schema.sql --dialect postgres > workspace.json
data-modelling-cli export odcs workspace.json schema.odcs.yaml
```

### Validate Schema Before Import

```bash
# Import with validation (default)
data-modelling-cli import odcs schema.odcs.yaml

# Validation errors will be reported before import proceeds
```

### Handle External References

```bash
# Import JSON Schema with external references
# References in same directory are automatically resolved
data-modelling-cli import json-schema schema.json

# References from HTTP/HTTPS URLs are automatically fetched
# (non-authenticated URLs only)
```

## Error Handling

### Common Errors and Solutions

**File not found**:
```bash
Error: File not found: schema.avsc
  Hint: Check the file path and ensure the file exists
```
Solution: Verify the file path is correct.

**Invalid UUID**:
```bash
Error: Invalid UUID format: 'not-a-uuid'
  Expected format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```
Solution: Use a valid UUID format.

**Multiple tables with UUID override**:
```bash
Error: UUID override is only supported when importing a single table. Found 3 tables.
  Hint: Import tables separately or remove --uuid flag
```
Solution: Import tables one at a time, or remove the `--uuid` flag.

**protoc not found**:
```bash
Error: protoc not found
  Install from: https://protobuf.dev/downloads
```
Solution: Install `protoc` or use `--protoc-path` to specify custom location.

## Tips

1. **Use `--pretty` for detailed inspection**: Shows column details, type mappings, and validation information
2. **Validate before import**: Keep validation enabled (default) to catch schema issues early
3. **Resolve references automatically**: External reference resolution is enabled by default and works for most cases
4. **Use UUID override for versioning**: Maintain consistent UUIDs across schema versions for change tracking
5. **Force overwrite in scripts**: Use `--force` flag when scripting to avoid prompts

## Next Steps

- See [contracts/cli-api.md](contracts/cli-api.md) for complete API reference
- See [data-model.md](data-model.md) for data structure details
- See [research.md](research.md) for implementation decisions
