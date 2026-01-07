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

# With full features (including database support)
cargo build --release --bin data-modelling-cli --features cli-full
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

**Installation with full features (including database):**
```bash
cargo install --path . --bin data-modelling-cli --features cli-full
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

### Import ODPS (Open Data Product Standard)

```bash
# Import ODPS YAML file (with validation if odps-validation feature enabled)
data-modelling-cli import odps product.odps.yaml

# Import with pretty output
data-modelling-cli import odps product.odps.yaml --pretty

# Skip validation (if odps-validation feature enabled)
data-modelling-cli import odps product.odps.yaml --no-validate

# Import from stdin
cat product.odps.yaml | data-modelling-cli import odps -
```

**Note**: ODPS import validates against the ODPS JSON Schema when the `odps-validation` feature is enabled. ODPS files are standalone and do not automatically generate `.odcs.yaml` files (unlike other import formats).

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

### Export to ODPS (Open Data Product Standard)

```bash
# Export ODPS file (round-trip: import and re-export)
data-modelling-cli export odps input.odps.yaml output.odps.yaml

# Export with force overwrite
data-modelling-cli export odps input.odps.yaml output.odps.yaml --force
```

**Note**: ODPS export only accepts ODPS input files. ODCS and ODPS are separate native formats and cannot be converted between each other. The exported ODPS file is validated against the ODPS JSON Schema when the `odps-validation` feature is enabled.

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
  odps         - ODPS (Open Data Product Standard) YAML files

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

---

## Database Commands

The CLI includes database commands for high-performance queries on large workspaces. These commands require the `cli-full` or `duckdb-backend` feature.

### Database Initialization

Initialize a database for a workspace:

```bash
# Initialize with DuckDB (default, embedded database)
data-modelling-cli db init --workspace ./my-workspace --backend duckdb

# Initialize with PostgreSQL (requires postgres-backend feature)
data-modelling-cli db init --workspace ./my-workspace --backend postgres \
  --connection-string "postgresql://user:pass@localhost/datamodel"
```

This creates:
- `.data-model.toml`: Configuration file
- `.data-model.duckdb`: DuckDB database file (for DuckDB backend)
- Git hooks (if in a Git repository and hooks are enabled)

### Database Sync

Sync YAML files to the database:

```bash
# Incremental sync (only changed files)
data-modelling-cli db sync --workspace ./my-workspace

# Force full resync
data-modelling-cli db sync --workspace ./my-workspace --force
```

The sync engine:
- Detects changed files using SHA256 hashes
- Parses ODCS/ODPS/CADS YAML files
- Updates database tables, columns, and relationships

### Database Status

Check database status and statistics:

```bash
data-modelling-cli db status --workspace ./my-workspace
```

Output includes:
- Backend type (DuckDB/PostgreSQL)
- Database file path
- Workspace count
- Table, column, and relationship counts
- Health check status

### Database Export

Export database contents back to YAML files:

```bash
# Export to workspace directory
data-modelling-cli db export --workspace ./my-workspace

# Export to custom output directory
data-modelling-cli db export --workspace ./my-workspace --output ./export
```

### Query Command

Execute SQL queries directly against the workspace database:

```bash
# Basic query (table output format)
data-modelling-cli query "SELECT name, data_type FROM columns LIMIT 10" \
  --workspace ./my-workspace

# JSON output format
data-modelling-cli query "SELECT * FROM tables" \
  --workspace ./my-workspace --format json

# CSV output format
data-modelling-cli query "SELECT name, nullable FROM columns WHERE primary_key = true" \
  --workspace ./my-workspace --format csv
```

**Output Formats:**
- `table` (default): Human-readable table format
- `json`: JSON array of objects
- `csv`: Comma-separated values

**Available Tables:**
- `workspaces`: Workspace metadata
- `domains`: Business domain definitions
- `tables`: Table/data contract definitions
- `columns`: Column definitions
- `relationships`: Table relationships
- `file_hashes`: File sync tracking

**Example Queries:**

```bash
# Find all primary key columns
data-modelling-cli query \
  "SELECT t.name as table_name, c.name as column_name, c.data_type
   FROM columns c
   JOIN tables t ON c.table_id = t.id
   WHERE c.primary_key = true" \
  --workspace ./my-workspace

# Count tables per domain
data-modelling-cli query \
  "SELECT d.name as domain, COUNT(t.id) as table_count
   FROM domains d
   LEFT JOIN tables t ON t.domain_id = d.id
   GROUP BY d.name" \
  --workspace ./my-workspace

# Find nullable columns without descriptions
data-modelling-cli query \
  "SELECT name, data_type FROM columns
   WHERE nullable = true AND (description IS NULL OR description = '')" \
  --workspace ./my-workspace
```

### Database Command Reference

```
data-modelling-cli db <subcommand> [options]

Subcommands:
  init      Initialize database for a workspace
  sync      Sync YAML files to database
  status    Show database status
  export    Export database to YAML files

db init Options:
  --workspace <path>           Workspace directory (required)
  --backend <type>             Backend type: duckdb or postgres (default: duckdb)
  --connection-string <url>    PostgreSQL connection string (for postgres backend)

db sync Options:
  --workspace <path>           Workspace directory (required)
  --force                      Force full resync (ignore file hashes)

db status Options:
  --workspace <path>           Workspace directory (required)

db export Options:
  --workspace <path>           Workspace directory (required)
  --output <path>              Output directory (default: workspace directory)
```

```
data-modelling-cli query <sql> [options]

Arguments:
  <sql>                        SQL query to execute

Options:
  --workspace <path>           Workspace directory (required)
  --format <format>            Output format: table, json, csv (default: table)
```

---

## Git Hooks

When database is initialized in a Git repository, hooks are automatically installed:

### Pre-commit Hook

Located at `.git/hooks/pre-commit`:
- Exports database changes to YAML files before commit
- Ensures YAML files reflect the current database state
- Prevents committing stale YAML files

### Post-checkout Hook

Located at `.git/hooks/post-checkout`:
- Syncs YAML files to database after checkout
- Keeps database in sync when switching branches
- Runs automatically on `git checkout` and `git switch`

### Disabling Hooks

To disable Git hooks, edit `.data-model.toml`:

```toml
[git]
hooks_enabled = false
```

Or remove the hooks manually:
```bash
rm .git/hooks/pre-commit .git/hooks/post-checkout
```
