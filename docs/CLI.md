# Data Modelling CLI

Command-line interface for the Data Modelling SDK.

## Building

### Prerequisites

- Rust toolchain (stable or later)
- Cargo (comes with Rust)

### Build Commands

**Debug build:**
```bash
cargo build --bin data-modelling-cli --features cli
```

**Release build (optimized):**
```bash
# Without OpenAPI support
cargo build --release --bin data-modelling-cli --features cli

# With OpenAPI support
cargo build --release --bin data-modelling-cli --features cli,openapi
```

The binary will be located at:
- Debug: `target/debug/data-modelling-cli`
- Release: `target/release/data-modelling-cli`

### Build with OpenAPI Support

If you need OpenAPI import/export support:
```bash
cargo build --release --bin data-modelling-cli --features cli,openapi
```

## Running

### Direct Execution

**Using cargo run (development):**
```bash
cargo run --bin data-modelling-cli --features cli -- <command> [options]
```

**Using the built binary:**
```bash
./target/release/data-modelling-cli <command> [options]
```

### Installation

**Install to Cargo bin directory:**

**Basic installation (without OpenAPI support):**
```bash
cargo install --path . --bin data-modelling-cli --features cli
```

**Installation with OpenAPI support:**
```bash
cargo install --path . --bin data-modelling-cli --features cli,openapi
```

This installs the binary to `~/.cargo/bin/data-modelling-cli` (or `%USERPROFILE%\.cargo\bin\data-modelling-cli` on Windows).

## Usage Examples

### Import SQL Schema

```bash
# From file (creates users.odcs.yaml automatically)
data-modelling-cli import sql schema.sql --dialect postgres

# From stdin
cat schema.sql | data-modelling-cli import sql - --dialect postgres

# Direct SQL string
data-modelling-cli import sql "CREATE TABLE users (id INT);" --dialect postgres

# Skip ODCS file creation
data-modelling-cli import sql schema.sql --dialect postgres --no-odcs
```

### Import AVRO Schema

```bash
data-modelling-cli import avro schema.avsc
```

### Import JSON Schema

```bash
data-modelling-cli import json-schema schema.json
```

### Import Protobuf

```bash
# From .proto file
data-modelling-cli import protobuf schema.proto

# From JAR file
data-modelling-cli import protobuf --jar schema.jar --message-type User
```

### Import OpenAPI

**⚠️ Note:** OpenAPI support requires building the CLI with the `openapi` feature enabled.

**Build with OpenAPI support:**
```bash
cargo build --release --bin data-modelling-cli --features cli,openapi
```

**Then use it:**
```bash
data-modelling-cli import openapi api.yaml
```

### Import ODCS

```bash
data-modelling-cli import odcs table.odcs.yaml
# This will create table.odcs.yaml (or update if it exists)
# Use --no-odcs to skip writing the ODCS file
```

### Export to ODCS

```bash
data-modelling-cli export odcs input.odcs.yaml output.odcs.yaml
```

### Export to AVRO

```bash
data-modelling-cli export avro input.odcs.yaml output.avsc
```

### Export Protobuf

```bash
# Export to proto3 format (default)
data-modelling-cli export protobuf input.odcs.yaml output.proto

# Export to proto2 format
data-modelling-cli export protobuf input.odcs.yaml output.proto --protobuf-version proto2
```

### Export Protobuf Descriptor

```bash
# Requires protoc to be installed (uses proto3 by default)
data-modelling-cli export protobuf-descriptor input.odcs.yaml output.pb

# Export proto2 descriptor
data-modelling-cli export protobuf-descriptor input.odcs.yaml output.pb --protobuf-version proto2

# If protoc is not in PATH, specify custom path
data-modelling-cli export protobuf-descriptor input.odcs.yaml output.pb --protoc-path /usr/local/bin/protoc
```

**Installing protoc:**
- **macOS**: `brew install protobuf`
- **Linux (Debian/Ubuntu)**: `sudo apt-get install protobuf-compiler`
- **Linux (RHEL/CentOS)**: `sudo yum install protobuf-compiler`
- **Windows**: Download from https://protobuf.dev/downloads/ or `choco install protoc`

## Command Reference

### Import Command

```
data-modelling-cli import <format> <input> [options]

Formats:
  sql          - SQL CREATE TABLE/VIEW statements
  avro         - AVRO schema files
  json-schema  - JSON Schema files
  protobuf     - Protocol Buffer .proto files
  openapi      - OpenAPI 3.1.1 YAML/JSON files
  odcs         - ODCS v3.1.0 YAML files

Options:
  --dialect <dialect>           SQL dialect (postgres|mysql|sqlite|generic|databricks)
  --uuid <uuid>                 Override table UUID (single-table imports only)
  --no-resolve-references       Disable external reference resolution
  --no-validate                 Skip schema validation before import
  --no-odcs                     Don't write .odcs.yaml file after import
  --pretty                      Pretty-print output with detailed information
  --jar <path>                  JAR file path (for Protobuf imports)
  --message-type <type>         Filter by message type (for Protobuf JAR imports)
```

### Export Command

```
data-modelling-cli export <format> <input> <output> [options]

Formats:
  odcs                  - ODCS v3.1.0 YAML
  avro                  - AVRO schema
  json-schema           - JSON Schema
  protobuf              - Protocol Buffer .proto
  protobuf-descriptor   - Binary Protobuf descriptor (.pb)

Input:
  <input>               ODCS YAML file (.odcs.yaml)

Options:
  --force                      Overwrite existing files without prompting
  --protoc-path <path>         Custom path to protoc binary (for protobuf-descriptor)
  --protobuf-version <version> Protobuf syntax version: proto2 or proto3 (default: proto3)
```

## Getting Help

```bash
# General help
data-modelling-cli --help

# Command-specific help
data-modelling-cli import --help
data-modelling-cli export --help
```

## Platform-Specific Notes

### Linux

No special requirements. The binary should work on most Linux distributions.

### macOS

No special requirements. The binary is a standard macOS executable.

### Windows

The binary is a `.exe` file. Ensure you have the necessary Visual C++ runtime libraries if you encounter runtime errors.

## Troubleshooting

### "CLI feature is not enabled"

Make sure you're building with the `cli` feature:
```bash
cargo build --bin data-modelling-cli --features cli
```

### "protoc not found" (for Protobuf descriptor export)

Install Protocol Buffers compiler:
- **Linux**: `sudo apt-get install protobuf-compiler` (Debian/Ubuntu) or `sudo yum install protobuf-compiler` (RHEL/CentOS)
- **macOS**: `brew install protobuf`
- **Windows**: Download from https://protobuf.dev/downloads/

### External Reference Resolution Fails

- Ensure referenced files are accessible
- For HTTP/HTTPS references, ensure the URL is publicly accessible (no authentication required)
- Check file paths are relative to the source file's directory
