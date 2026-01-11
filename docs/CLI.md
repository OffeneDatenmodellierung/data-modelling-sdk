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

### Export to PDF

Export decision records, knowledge articles, ODCS data contracts, ODPS data products, and CADS compute assets to PDF format with optional branding.

```bash
# Export decision to PDF
data-modelling-cli export pdf decisions/ADR-0001.madr.yaml output.pdf

# Export knowledge article to PDF
data-modelling-cli export pdf knowledge/KB-0001.kb.yaml output.pdf

# Export ODCS data contract to PDF
data-modelling-cli export pdf contracts/customer.odcs.yaml customer-contract.pdf

# Export ODPS data product to PDF
data-modelling-cli export pdf products/customer-360.odps.yaml customer-product.pdf

# Export CADS compute asset to PDF
data-modelling-cli export pdf assets/ml-pipeline.cads.yaml ml-pipeline.pdf

# Export with branding options
data-modelling-cli export pdf decisions/ADR-0001.madr.yaml output.pdf \
  --logo-url "https://example.com/logo.png" \
  --header-text "ACME Corp Architecture Decisions" \
  --footer-text "Confidential" \
  --brand-color "#0066CC" \
  --company-name "ACME Corporation"
```

### Export to Markdown

Export decision records, knowledge articles, ODCS data contracts, ODPS data products, and CADS compute assets to Markdown format.

```bash
# Export decision to Markdown
data-modelling-cli export markdown decisions/ADR-0001.madr.yaml output.md

# Export knowledge article to Markdown
data-modelling-cli export markdown knowledge/KB-0001.kb.yaml output.md

# Export ODCS data contract to Markdown
data-modelling-cli export markdown contracts/customer.odcs.yaml customer-contract.md

# Export ODPS data product to Markdown
data-modelling-cli export markdown products/customer-360.odps.yaml customer-product.md

# Export CADS compute asset to Markdown
data-modelling-cli export markdown assets/ml-pipeline.cads.yaml ml-pipeline.md
```

### Export to Branded Markdown

Export decision records and knowledge articles to Markdown with branding (logo, header, footer, table of contents).

```bash
# Export with branding
data-modelling-cli export branded-markdown decisions/ADR-0001.madr.yaml output.md \
  --logo-url "https://example.com/logo.png" \
  --header-text "Architecture Decision Records" \
  --footer-text "© 2025 ACME Corp" \
  --company-name "ACME Corporation" \
  --include-toc

# Export knowledge article with branding
data-modelling-cli export branded-markdown knowledge/KB-0001.kb.yaml output.md \
  --header-text "Knowledge Base" \
  --brand-color "#336699"
```

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
  odps                  - ODPS (Open Data Product Standard) YAML
  avro                  - AVRO schema
  json-schema           - JSON Schema
  protobuf              - Protocol Buffer .proto
  protobuf-descriptor   - Binary Protobuf descriptor (.pb)
  pdf                   - PDF document (supports all YAML file types)
  markdown              - Markdown document (supports all YAML file types)
  branded-markdown      - Branded Markdown with logo, header, footer

Input:
  <input>               ODCS YAML file (.odcs.yaml), ODPS file (.odps.yaml),
                        CADS file (.cads.yaml), decision record (.madr.yaml),
                        or knowledge article (.kb.yaml)

Options:
  --force                      Overwrite existing files without prompting
  --protoc-path <path>         Custom path to protoc binary (for protobuf-descriptor)
  --protobuf-version <version> Protobuf syntax version: proto2 or proto3 (default: proto3)

Branding Options (for pdf and branded-markdown formats):
  --logo-url <url>             Logo URL for branding
  --header-text <text>         Header text for branding
  --footer-text <text>         Footer text for branding
  --brand-color <hex>          Brand color in hex format (e.g., "#0066CC")
  --company-name <name>        Company or organization name
  --include-toc                Include table of contents (branded-markdown only)
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

## Decision Commands

The CLI includes commands for managing Architecture Decision Records (ADRs) following the MADR format. Decisions are stored as YAML files in the `decisions/` folder and can be exported to Markdown.

### Create a New Decision

```bash
# Create a new decision interactively
data-modelling-cli decision new --workspace ./my-workspace

# Create with title and domain
data-modelling-cli decision new --title "Use PostgreSQL for persistence" \
  --domain platform --workspace ./my-workspace

# Create with full details
data-modelling-cli decision new \
  --title "Adopt microservices architecture" \
  --domain architecture \
  --category architecture \
  --status proposed \
  --workspace ./my-workspace
```

### List Decisions

```bash
# List all decisions
data-modelling-cli decision list --workspace ./my-workspace

# Filter by status
data-modelling-cli decision list --status accepted --workspace ./my-workspace

# Filter by category
data-modelling-cli decision list --category architecture --workspace ./my-workspace

# Filter by domain
data-modelling-cli decision list --domain platform --workspace ./my-workspace

# Combine filters
data-modelling-cli decision list --status accepted --category technology \
  --workspace ./my-workspace
```

### Show Decision Details

```bash
# Show by number
data-modelling-cli decision show 1 --workspace ./my-workspace

# Show by number with leading zeros
data-modelling-cli decision show 0001 --workspace ./my-workspace
```

### Check Decision Status

```bash
# Show summary of all decisions by status
data-modelling-cli decision status --workspace ./my-workspace
```

### Export Decisions to Markdown

```bash
# Export all decisions to decisions-md/ folder
data-modelling-cli decision export --workspace ./my-workspace

# Export specific decision
data-modelling-cli decision export 1 --workspace ./my-workspace
```

### Decision Command Reference

```
data-modelling-cli decision <subcommand> [options]

Subcommands:
  new       Create a new decision record
  list      List all decisions
  show      Show a specific decision
  status    Show decision status summary
  export    Export decisions to Markdown

decision new Options:
  --workspace <path>      Workspace directory (required)
  --title <title>         Decision title
  --domain <domain>       Business domain
  --category <category>   Category: architecture, technology, process, security, data, integration
  --status <status>       Status: draft, proposed, accepted, deprecated, superseded, rejected

decision list Options:
  --workspace <path>      Workspace directory (required)
  --status <status>       Filter by status
  --category <category>   Filter by category
  --domain <domain>       Filter by domain

decision show Arguments:
  <number>                Decision number (e.g., 1 or 0001)
Options:
  --workspace <path>      Workspace directory (required)

decision status Options:
  --workspace <path>      Workspace directory (required)

decision export Arguments:
  [number]                Optional decision number (exports all if omitted)
Options:
  --workspace <path>      Workspace directory (required)
```

---

## Knowledge Base Commands

The CLI includes commands for managing Knowledge Base articles. Articles are stored as YAML files in the `knowledge/` folder and can be exported to Markdown.

### Create a New Article

```bash
# Create a new knowledge article
data-modelling-cli knowledge new --workspace ./my-workspace

# Create with title and type
data-modelling-cli knowledge new --title "API Authentication Guide" \
  --type guide --workspace ./my-workspace

# Create with full details
data-modelling-cli knowledge new \
  --title "Deployment Procedures" \
  --domain platform \
  --type runbook \
  --status draft \
  --workspace ./my-workspace
```

### List Articles

```bash
# List all articles
data-modelling-cli knowledge list --workspace ./my-workspace

# Filter by type
data-modelling-cli knowledge list --type guide --workspace ./my-workspace

# Filter by status
data-modelling-cli knowledge list --status published --workspace ./my-workspace

# Filter by domain
data-modelling-cli knowledge list --domain platform --workspace ./my-workspace

# Combine filters
data-modelling-cli knowledge list --type guide --status published \
  --workspace ./my-workspace
```

### Show Article Details

```bash
# Show by number
data-modelling-cli knowledge show 1 --workspace ./my-workspace

# Show by number with leading zeros
data-modelling-cli knowledge show 0001 --workspace ./my-workspace
```

### Search Articles

```bash
# Search in article content
data-modelling-cli knowledge search "authentication" --workspace ./my-workspace

# Search with domain filter
data-modelling-cli knowledge search "deployment" --domain platform \
  --workspace ./my-workspace
```

### Check Knowledge Base Status

```bash
# Show summary of all articles by status and type
data-modelling-cli knowledge status --workspace ./my-workspace
```

### Export Articles to Markdown

```bash
# Export all articles to knowledge-md/ folder
data-modelling-cli knowledge export --workspace ./my-workspace

# Export specific article
data-modelling-cli knowledge export 1 --workspace ./my-workspace
```

### Knowledge Command Reference

```
data-modelling-cli knowledge <subcommand> [options]

Subcommands:
  new       Create a new knowledge article
  list      List all articles
  show      Show a specific article
  search    Search article content
  status    Show knowledge base status summary
  export    Export articles to Markdown

knowledge new Options:
  --workspace <path>      Workspace directory (required)
  --title <title>         Article title
  --domain <domain>       Business domain
  --type <type>           Article type: guide, reference, concept, tutorial, troubleshooting, runbook
  --status <status>       Status: draft, review, published, archived, deprecated

knowledge list Options:
  --workspace <path>      Workspace directory (required)
  --type <type>           Filter by article type
  --status <status>       Filter by status
  --domain <domain>       Filter by domain

knowledge show Arguments:
  <number>                Article number (e.g., 1 or 0001)
Options:
  --workspace <path>      Workspace directory (required)

knowledge search Arguments:
  <query>                 Search query
Options:
  --workspace <path>      Workspace directory (required)
  --domain <domain>       Filter by domain

knowledge status Options:
  --workspace <path>      Workspace directory (required)

knowledge export Arguments:
  [number]                Optional article number (exports all if omitted)
Options:
  --workspace <path>      Workspace directory (required)
```

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

---

## Schema Mapping Commands

The CLI includes commands for mapping source schemas to target schemas, with support for fuzzy matching and transformation script generation.

### Map Schemas

```bash
# Basic schema mapping
odm map source-schema.json target-schema.json

# With fuzzy matching enabled
odm map source.json target.json --fuzzy --min-similarity 0.7

# Case-insensitive matching
odm map source.json target.json --case-insensitive

# Save mapping result to file
odm map source.json target.json --output mapping-result.json

# Generate SQL transformation script
odm map source.json target.json \
  --transform-format sql \
  --transform-output transform.sql

# Generate JQ filter
odm map source.json target.json \
  --transform-format jq \
  --transform-output transform.jq

# Generate Python transformation
odm map source.json target.json \
  --transform-format python \
  --transform-output transform.py

# Generate PySpark transformation
odm map source.json target.json \
  --transform-format pyspark \
  --transform-output transform_spark.py

# Verbose output with detailed mapping info
odm map source.json target.json --verbose
```

### Mapping Command Reference

```
odm map <source> <target> [options]

Arguments:
  <source>                     Source schema file (.json or .yaml)
  <target>                     Target schema file (.json or .yaml)

Options:
  -o, --output <file>          Output file for mapping result (JSON)
  --min-similarity <value>     Minimum similarity threshold (0.0-1.0, default: 0.7)
  --fuzzy                      Enable fuzzy matching using Levenshtein distance
  --case-insensitive           Enable case-insensitive field name matching
  --transform-format <format>  Transform output format: sql, jq, python, pyspark
  --transform-output <file>    Output file for generated transformation script
  -v, --verbose                Show detailed mapping information
```

### Mapping Output

The mapping result includes:
- **Direct mappings**: Fields that match directly between source and target
- **Transformations**: Fields requiring type conversions or format changes
- **Gaps**: Target fields with no corresponding source field
- **Extras**: Source fields not mapped to any target field
- **Compatibility score**: Overall compatibility percentage

---

## Pipeline Commands

The CLI includes a full data pipeline for ingesting JSON data, inferring schemas, and exporting results.

### Pipeline Run

Run the complete data pipeline with configurable stages:

```bash
# Basic pipeline run
odm pipeline run --database staging.duckdb \
  --source ./data \
  --output-dir ./output

# Specify file pattern
odm pipeline run --database staging.duckdb \
  --source ./data \
  --pattern "**/*.jsonl" \
  --output-dir ./output

# Run specific stages only
odm pipeline run --database staging.duckdb \
  --stages ingest,infer,export \
  --output-dir ./output

# With LLM-enhanced schema refinement (Ollama)
odm pipeline run --database staging.duckdb \
  --source ./data \
  --output-dir ./output \
  --llm-mode online \
  --ollama-url http://localhost:11434 \
  --model llama3.2

# With target schema mapping
odm pipeline run --database staging.duckdb \
  --source ./data \
  --output-dir ./output \
  --target-schema target-schema.json \
  --stages ingest,infer,map,export

# Dry run (validate without executing)
odm pipeline run --database staging.duckdb \
  --source ./data \
  --output-dir ./output \
  --dry-run

# Resume from checkpoint
odm pipeline run --database staging.duckdb \
  --output-dir ./output \
  --resume

# Verbose output
odm pipeline run --database staging.duckdb \
  --source ./data \
  --output-dir ./output \
  --verbose
```

### Pipeline Status

Check the status of a pipeline run:

```bash
# Show pipeline status
odm pipeline status --database staging.duckdb
```

### Pipeline Command Reference

```
odm pipeline run [options]

Options:
  -d, --database <path>        Staging database path (default: staging.duckdb)
  -s, --source <path>          Source directory for ingestion
  -p, --pattern <pattern>      File pattern (default: *.json)
  -k, --partition <key>        Partition key for data organization
  -o, --output-dir <path>      Output directory (default: ./output)
  --target-schema <file>       Target schema file for mapping stage
  --stages <stages>            Comma-separated stages: ingest,infer,refine,map,export
  --llm-mode <mode>            LLM mode: none, online, offline (default: none)
  --ollama-url <url>           Ollama API URL (default: http://localhost:11434)
  --model <name>               LLM model name (default: llama3.2)
  --model-path <path>          GGUF model path for offline mode
  --doc-path <path>            Documentation file for LLM context
  --temperature <value>        LLM temperature (default: 0.3)
  --dry-run                    Validate without executing
  --resume                     Resume from checkpoint
  -v, --verbose                Verbose output

odm pipeline status [options]

Options:
  -d, --database <path>        Staging database path (default: staging.duckdb)
```

### Pipeline Stages

The pipeline consists of the following stages:

1. **Ingest**: Load JSON/JSONL files into the staging database
2. **Infer**: Infer schema from staged data with type and format detection
3. **Refine**: (Optional) Enhance schema with LLM-based refinement
4. **Map**: (Optional) Map inferred schema to target schema
5. **Export**: Export results (schema, mappings, data)

### Checkpointing and Resume

The pipeline automatically saves checkpoints after each stage:
- Checkpoints are stored alongside the database
- Use `--resume` to continue from the last successful stage
- Configuration changes are detected and require confirmation
- Stage outputs (timing, success/failure) are tracked

### Example Workflow

```bash
# 1. Initialize staging database
odm staging init staging.duckdb

# 2. Run pipeline with all stages
odm pipeline run \
  --database staging.duckdb \
  --source ./raw-json-data \
  --pattern "**/*.json" \
  --partition my-dataset \
  --output-dir ./processed \
  --llm-mode online \
  --model llama3.2 \
  --target-schema ./schemas/target.json \
  --verbose

# 3. Check status if interrupted
odm pipeline status --database staging.duckdb

# 4. Resume if needed
odm pipeline run --database staging.duckdb --resume
```

---

## Staging Commands

The staging module provides commands for ingesting and managing JSON data in a staging database.

### Initialize Staging Database

```bash
# Initialize a new DuckDB staging database
odm staging init staging.duckdb

# Initialize with Iceberg catalog (REST/Lakekeeper)
odm staging init staging.duckdb \
  --catalog rest \
  --endpoint http://localhost:8181 \
  --warehouse ./local-warehouse

# Initialize with Unity Catalog
odm staging init staging.duckdb \
  --catalog unity \
  --endpoint https://workspace.cloud.databricks.com \
  --token $DATABRICKS_TOKEN

# Initialize with AWS S3 Tables
odm staging init staging.duckdb \
  --catalog s3-tables \
  --arn arn:aws:s3tables:us-east-1:123456789:bucket/my-table-bucket \
  --region us-east-1

# Initialize with AWS Glue
odm staging init staging.duckdb \
  --catalog glue \
  --region us-east-1 \
  --profile my-aws-profile
```

### Ingest Data

```bash
# Ingest JSON files from a directory
odm staging ingest \
  --database staging.duckdb \
  --source ./json-data

# Ingest with specific pattern
odm staging ingest \
  --database staging.duckdb \
  --source ./data \
  --pattern "**/*.jsonl"

# Ingest with partition key
odm staging ingest \
  --database staging.duckdb \
  --source ./data \
  --partition my-dataset-v1

# Ingest with deduplication
odm staging ingest \
  --database staging.duckdb \
  --source ./data \
  --dedup content  # Options: none, path, content, both

# Resume interrupted ingestion
odm staging ingest \
  --database staging.duckdb \
  --source ./data \
  --resume

# Custom batch size
odm staging ingest \
  --database staging.duckdb \
  --source ./data \
  --batch-size 5000
```

### Ingest from S3

Ingest JSON/JSONL files directly from AWS S3 buckets. Requires the `s3` feature.

```bash
# Build with S3 support
cargo build --release -p odm --features s3

# Ingest from S3 bucket
odm staging ingest \
  --database staging.duckdb \
  --s3-bucket my-data-bucket \
  --s3-prefix raw/json/ \
  --s3-region us-east-1

# With specific AWS profile
odm staging ingest \
  --database staging.duckdb \
  --s3-bucket my-data-bucket \
  --s3-prefix data/ \
  --s3-profile production

# With custom S3 endpoint (MinIO, LocalStack)
odm staging ingest \
  --database staging.duckdb \
  --s3-bucket local-bucket \
  --s3-endpoint http://localhost:9000
```

### Ingest from Databricks Unity Catalog

Ingest JSON/JSONL files from Databricks Unity Catalog Volumes. Requires the `databricks` feature.

```bash
# Build with Databricks support
cargo build --release -p odm --features databricks

# Ingest from Unity Catalog Volume
odm staging ingest \
  --database staging.duckdb \
  --databricks-host https://myworkspace.cloud.databricks.com \
  --databricks-token $DATABRICKS_TOKEN \
  --databricks-catalog main \
  --databricks-schema raw_data \
  --databricks-volume json_files \
  --databricks-path /2024/01/

# With partition key
odm staging ingest \
  --database staging.duckdb \
  --databricks-host https://myworkspace.cloud.databricks.com \
  --databricks-token $DATABRICKS_TOKEN \
  --databricks-catalog main \
  --databricks-schema raw_data \
  --databricks-volume json_files \
  --partition january-2024
```

### Query Staged Data

```bash
# Execute SQL query
odm staging query "SELECT * FROM staged_json LIMIT 10" \
  --database staging.duckdb

# Query with time travel (Iceberg)
odm staging query "SELECT * FROM staged" \
  --database staging.duckdb \
  --version 5

# Query as of timestamp (Iceberg)
odm staging query "SELECT * FROM staged" \
  --database staging.duckdb \
  --timestamp "2025-01-10T00:00:00Z"
```

### View Statistics and History

```bash
# Show ingestion statistics
odm staging stats --database staging.duckdb

# Show batch history
odm staging batches --database staging.duckdb

# Get sample records
odm staging sample --database staging.duckdb --limit 5

# Show table history (Iceberg)
odm staging history --database staging.duckdb
```

### Create Schema-Inferenced View

```bash
# Create a typed view from inferred schema
odm staging view create \
  --database staging.duckdb \
  --name staged_typed \
  --schema inferred_schema.json \
  --source-table raw_json
```

### Export to Production Catalogs

```bash
# Export to Unity Catalog
odm staging export \
  --database staging.duckdb \
  --target unity \
  --endpoint https://workspace.cloud.databricks.com \
  --catalog main \
  --schema staging \
  --table raw_json

# Export to S3 Tables
odm staging export \
  --database staging.duckdb \
  --target s3-tables \
  --arn arn:aws:s3tables:us-east-1:123456789:bucket/my-bucket \
  --table raw_json

# Export to Glue
odm staging export \
  --database staging.duckdb \
  --target glue \
  --database staging \
  --table raw_json
```

### Staging Command Reference

```
odm staging init <database> [options]

Arguments:
  <database>                   Path to the staging database file

Options:
  --catalog <type>             Catalog type: duckdb, rest, s3-tables, unity, glue
  --endpoint <url>             Catalog endpoint URL
  --warehouse <path>           Warehouse path for data storage
  --token <token>              Authentication token
  --region <region>            AWS region (for S3/Glue)
  --arn <arn>                  S3 Tables ARN
  --profile <profile>          AWS profile name

odm staging ingest [options]

Options:
  -d, --database <path>        Staging database path
  -s, --source <path>          Source directory or file
  -p, --pattern <pattern>      File pattern (default: *.json)
  -k, --partition <key>        Partition key
  --batch-size <size>          Insert batch size (default: 1000)
  --dedup <strategy>           Deduplication: none, path, content, both
  --resume                     Resume from last batch

odm staging query <sql> [options]

Options:
  -d, --database <path>        Staging database path
  --version <n>                Query specific version (Iceberg)
  --timestamp <ts>             Query as of timestamp (Iceberg)

odm staging stats [options]
odm staging batches [options]
odm staging sample [options]
odm staging history [options]

Options:
  -d, --database <path>        Staging database path
  --limit <n>                  Maximum results to show

odm staging view create [options]

Options:
  -d, --database <path>        Staging database path
  --name <name>                View name
  --schema <file>              Inferred schema JSON file
  --source-table <table>       Source table name

odm staging export [options]

Options:
  -d, --database <path>        Staging database path
  --target <type>              Target: unity, glue, s3-tables, local
  --endpoint <url>             Target catalog endpoint
  --catalog <name>             Target catalog name
  --schema <name>              Target schema name
  --table <name>               Target table name
  --arn <arn>                  S3 Tables ARN
```

---

## Inference Commands

The inference module provides commands for inferring schemas from staged JSON data.

### Infer Schema

```bash
# Basic schema inference
odm inference infer \
  --database staging.duckdb \
  --output inferred-schema.json

# Infer from specific partition
odm inference infer \
  --database staging.duckdb \
  --partition my-dataset \
  --output schema.json

# Limit sample size for faster inference
odm inference infer \
  --database staging.duckdb \
  --sample-size 10000 \
  --output schema.json

# Set minimum field frequency threshold
odm inference infer \
  --database staging.duckdb \
  --min-frequency 0.1 \
  --output schema.json

# Limit nesting depth
odm inference infer \
  --database staging.duckdb \
  --max-depth 5 \
  --output schema.json

# Disable format detection (faster)
odm inference infer \
  --database staging.duckdb \
  --no-formats \
  --output schema.json

# Output as YAML
odm inference infer \
  --database staging.duckdb \
  --format yaml \
  --output schema.yaml

# Output as JSON Schema
odm inference infer \
  --database staging.duckdb \
  --format json-schema \
  --output schema.json
```

### Analyze Schema Variations

```bash
# Group similar schemas
odm inference schemas \
  --database staging.duckdb \
  --threshold 0.8

# Output as JSON
odm inference schemas \
  --database staging.duckdb \
  --format json
```

### Inference Command Reference

```
odm inference infer [options]

Options:
  -d, --database <path>        Staging database path
  -o, --output <file>          Output schema file
  -p, --partition <key>        Filter by partition
  --sample-size <n>            Max records to sample (0 = all)
  --min-frequency <value>      Min field occurrence rate (0.0-1.0)
  --max-depth <n>              Max nesting depth (default: 10)
  --no-formats                 Disable format detection
  --format <format>            Output format: json, yaml, json-schema

odm inference schemas [options]

Options:
  -d, --database <path>        Staging database path
  --threshold <value>          Similarity threshold (0.0-1.0)
  --format <format>            Output format: table, json
```

### Detected Formats

The inference engine detects these standard formats in string fields:

| Format | Description | Example |
|--------|-------------|---------|
| `date-time` | ISO 8601 datetime | `2025-01-10T14:30:00Z` |
| `date` | ISO 8601 date | `2025-01-10` |
| `time` | ISO 8601 time | `14:30:00` |
| `email` | Email address | `user@example.com` |
| `uri` | URI/URL | `https://example.com/path` |
| `uuid` | UUID v4 | `550e8400-e29b-41d4-a716-446655440000` |
| `hostname` | Hostname | `api.example.com` |
| `ipv4` | IPv4 address | `192.168.1.1` |
| `ipv6` | IPv6 address | `2001:0db8:85a3::8a2e:0370:7334` |
| `semver` | Semantic version | `1.2.3` |
| `country_code` | ISO 3166-1 alpha-2 | `US`, `DE`, `JP` |
| `currency_code` | ISO 4217 | `USD`, `EUR`, `GBP` |
| `language_code` | ISO 639-1 | `en`, `de`, `ja` |
| `mime_type` | MIME type | `application/json` |
| `base64` | Base64 encoded | `SGVsbG8gV29ybGQ=` |
| `jwt` | JSON Web Token | `eyJhbGciOiJIUzI1NiIs...` |
| `slug` | URL slug | `my-article-title` |
| `phone` | E.164 phone | `+14155551234` |

---

## LLM Refinement Commands

The LLM module provides commands for enhancing inferred schemas using language models.

### Refine Schema with LLM

```bash
# Refine with Ollama (online mode)
odm inference infer \
  --database staging.duckdb \
  --output schema.json \
  --llm online \
  --ollama-url http://localhost:11434 \
  --model llama3.2

# Refine with custom documentation context
odm inference infer \
  --database staging.duckdb \
  --output schema.json \
  --llm online \
  --model llama3.2 \
  --doc-path ./docs/data-dictionary.md

# Adjust temperature for creativity
odm inference infer \
  --database staging.duckdb \
  --output schema.json \
  --llm online \
  --model llama3.2 \
  --temperature 0.5

# Verbose LLM output for debugging
odm inference infer \
  --database staging.duckdb \
  --output schema.json \
  --llm online \
  --verbose-llm

# Skip LLM refinement (inference only)
odm inference infer \
  --database staging.duckdb \
  --output schema.json \
  --no-refine
```

### LLM Options Reference

```
LLM Options (available with inference infer and pipeline run):

  --llm <mode>                 LLM mode: none, online, offline
  --ollama-url <url>           Ollama API URL (default: http://localhost:11434)
  --model <name>               Model name (default: llama3.2)
  --model-path <path>          GGUF model path for offline mode
  --doc-path <path>            Documentation file for context
  --temperature <value>        Generation temperature (default: 0.3)
  --no-refine                  Skip LLM refinement step
  --verbose-llm                Show LLM debug output
```

### Supported LLM Backends

| Backend | Mode | Description |
|---------|------|-------------|
| Ollama | `online` | Local or remote Ollama server |
| llama.cpp | `offline` | Embedded inference with GGUF models |

### LLM Refinement Features

The LLM refiner enhances schemas by:
- Adding meaningful field descriptions
- Improving type specificity (e.g., `string` → `string` with `format: email`)
- Suggesting constraints (min/max values, patterns)
- Identifying semantic relationships between fields
- Using documentation context for domain-specific terminology

---

## Troubleshooting

### Common Issues

**Database locked error:**
```
Error: database is locked
```
Solution: Ensure no other process is using the database. Close any open connections.

**LLM connection refused:**
```
Error: connection refused (http://localhost:11434)
```
Solution: Ensure Ollama is running: `ollama serve`

**Schema inference out of memory:**
```
Error: out of memory during inference
```
Solution: Use `--sample-size` to limit records: `--sample-size 10000`

**Iceberg catalog connection failed:**
```
Error: Failed to connect to catalog
```
Solution: Verify endpoint URL and authentication credentials.

**Pipeline checkpoint mismatch:**
```
Error: Configuration has changed since last checkpoint
```
Solution: Delete the checkpoint file or use `--force` to override.

### Debug Mode

Enable verbose output for troubleshooting:

```bash
# Verbose pipeline output
odm pipeline run --database staging.duckdb --verbose

# Verbose LLM output
odm inference infer --database staging.duckdb --verbose-llm

# Check staging database contents
odm staging query "SELECT COUNT(*) FROM staged_json" --database staging.duckdb
```

### Log Files

Pipeline and staging operations log to:
- Checkpoint files: `<database>.checkpoint.json`
- Batch metadata: Stored in database `processing_batches` table

### Getting Help

```bash
# General help
odm --help

# Command-specific help
odm staging --help
odm inference --help
odm pipeline --help
odm map --help
```
