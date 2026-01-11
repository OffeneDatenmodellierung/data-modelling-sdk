# Data Pipeline Tutorial

This tutorial walks you through using the data pipeline to ingest JSON data, infer schemas, and map to target formats.

## Overview

The data pipeline consists of five stages:

1. **Ingest** - Load JSON/JSONL files into a staging database
2. **Infer** - Automatically infer schema from staged data
3. **Refine** - (Optional) Enhance schema using LLM
4. **Map** - (Optional) Map inferred schema to a target schema
5. **Export** - Export results (schemas, mappings, transformed data)

## Prerequisites

Build the CLI with pipeline support:

```bash
cargo build --release -p odm --features pipeline
```

For LLM refinement, also include:

```bash
cargo build --release -p odm --features pipeline,llm-online
```

## Quick Start

### 1. Prepare Your Data

Create a directory with JSON files:

```bash
mkdir -p data/raw
echo '{"id": 1, "name": "Alice", "email": "alice@example.com", "created_at": "2025-01-10T10:30:00Z"}' > data/raw/user1.json
echo '{"id": 2, "name": "Bob", "email": "bob@example.com", "created_at": "2025-01-10T11:00:00Z"}' > data/raw/user2.json
```

Or use JSONL format (one JSON object per line):

```bash
cat > data/raw/users.jsonl << 'EOF'
{"id": 1, "name": "Alice", "email": "alice@example.com", "created_at": "2025-01-10T10:30:00Z"}
{"id": 2, "name": "Bob", "email": "bob@example.com", "created_at": "2025-01-10T11:00:00Z"}
{"id": 3, "name": "Charlie", "email": "charlie@example.com", "created_at": "2025-01-10T11:30:00Z"}
EOF
```

### 2. Initialize Staging Database

```bash
odm staging init staging.duckdb
```

### 3. Run the Pipeline

Basic pipeline run:

```bash
odm pipeline run \
  --database staging.duckdb \
  --source ./data/raw \
  --output-dir ./output \
  --verbose
```

This will:
- Ingest all JSON files from `./data/raw`
- Infer schema from the staged data
- Export the inferred schema to `./output/`

### 4. Check Results

```bash
# View inferred schema
cat output/inferred_schema.json

# Query staged data
odm staging query "SELECT * FROM staged_json LIMIT 5" --database staging.duckdb
```

## Step-by-Step Guide

### Stage 1: Data Ingestion

The ingest stage loads JSON files into a DuckDB staging database.

```bash
# Ingest with specific file pattern
odm staging ingest \
  --database staging.duckdb \
  --source ./data/raw \
  --pattern "**/*.jsonl"

# Ingest with partition key (for organizing data)
odm staging ingest \
  --database staging.duckdb \
  --source ./data/raw \
  --partition users-v1

# Ingest with deduplication
odm staging ingest \
  --database staging.duckdb \
  --source ./data/raw \
  --dedup content  # Skip files with duplicate content hashes
```

#### Deduplication Strategies

| Strategy | Description |
|----------|-------------|
| `none` | No deduplication (default) |
| `path` | Skip files with same path |
| `content` | Skip files with same content hash |
| `both` | Skip if path OR content matches |

### Stage 2: Schema Inference

The infer stage analyzes staged data to determine field types, formats, and nullability.

```bash
odm inference infer \
  --database staging.duckdb \
  --output schema.json \
  --format json-schema
```

#### Inference Options

```bash
# Limit sample size for faster inference
odm inference infer \
  --database staging.duckdb \
  --sample-size 10000 \
  --output schema.json

# Set minimum field frequency (fields appearing in <10% of records are excluded)
odm inference infer \
  --database staging.duckdb \
  --min-frequency 0.1 \
  --output schema.json

# Disable format detection (faster, but no email/date/uuid detection)
odm inference infer \
  --database staging.duckdb \
  --no-formats \
  --output schema.json
```

#### Example Output

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "id": {
      "type": "integer"
    },
    "name": {
      "type": "string"
    },
    "email": {
      "type": "string",
      "format": "email"
    },
    "created_at": {
      "type": "string",
      "format": "date-time"
    }
  },
  "required": ["id", "name", "email", "created_at"]
}
```

### Stage 3: LLM Refinement (Optional)

Enhance the inferred schema using a language model to add descriptions and improve type specificity.

#### Using Ollama

Start Ollama:

```bash
ollama serve
ollama pull llama3.2
```

Run with LLM refinement:

```bash
odm pipeline run \
  --database staging.duckdb \
  --source ./data/raw \
  --output-dir ./output \
  --llm-mode online \
  --ollama-url http://localhost:11434 \
  --model llama3.2
```

#### Providing Context

Add domain documentation for better refinement:

```bash
odm pipeline run \
  --database staging.duckdb \
  --source ./data/raw \
  --output-dir ./output \
  --llm-mode online \
  --model llama3.2 \
  --doc-path ./docs/data-dictionary.md
```

#### LLM Enhancements

The LLM refiner can:
- Add meaningful field descriptions
- Improve type specificity (e.g., `string` → `string` with `format: email`)
- Suggest constraints (min/max values, patterns)
- Identify semantic relationships between fields

### Stage 4: Schema Mapping (Optional)

Map the inferred schema to a target schema for data transformation.

#### Create Target Schema

```json
{
  "type": "object",
  "properties": {
    "user_id": {"type": "integer", "description": "Unique user identifier"},
    "full_name": {"type": "string", "description": "User's full name"},
    "email_address": {"type": "string", "format": "email"},
    "registration_date": {"type": "string", "format": "date-time"}
  },
  "required": ["user_id", "full_name", "email_address"]
}
```

Save as `target-schema.json`.

#### Run with Mapping

```bash
odm pipeline run \
  --database staging.duckdb \
  --source ./data/raw \
  --output-dir ./output \
  --target-schema target-schema.json \
  --stages ingest,infer,map,export
```

#### Standalone Mapping

```bash
# Map schemas with fuzzy matching
odm map inferred-schema.json target-schema.json \
  --fuzzy \
  --min-similarity 0.7 \
  --output mapping-result.json

# Generate SQL transformation
odm map inferred-schema.json target-schema.json \
  --transform-format sql \
  --transform-output transform.sql
```

#### Generated SQL Example

```sql
-- Transformation from source to target schema
-- Generated by ODM Schema Mapper

INSERT INTO target_table (user_id, full_name, email_address, registration_date)
SELECT
  id AS user_id,                    -- Direct mapping (confidence: 0.85)
  name AS full_name,                -- Fuzzy match (confidence: 0.72)
  email AS email_address,           -- Fuzzy match (confidence: 0.78)
  created_at AS registration_date   -- Fuzzy match (confidence: 0.65)
FROM source_table;
```

### Stage 5: Export

The export stage writes results to the output directory:

```
output/
├── inferred_schema.json       # Inferred schema
├── refined_schema.json        # LLM-refined schema (if enabled)
├── mapping_result.json        # Schema mapping (if target provided)
├── transform.sql              # Generated transformation script
└── pipeline_report.json       # Execution summary
```

## Checkpointing and Resume

The pipeline automatically saves checkpoints after each stage.

### Resume After Failure

If the pipeline fails mid-execution:

```bash
# Check status
odm pipeline status --database staging.duckdb

# Resume from last checkpoint
odm pipeline run --database staging.duckdb --resume
```

### Checkpoint Contents

Checkpoints track:
- Completed stages
- Current stage
- Stage timing and results
- Configuration hash (detects changes)

## Advanced Usage

### Running Specific Stages

```bash
# Only run ingest and infer
odm pipeline run \
  --database staging.duckdb \
  --source ./data/raw \
  --stages ingest,infer \
  --output-dir ./output

# Skip ingest (data already staged)
odm pipeline run \
  --database staging.duckdb \
  --stages infer,refine,export \
  --output-dir ./output
```

### Dry Run

Validate configuration without executing:

```bash
odm pipeline run \
  --database staging.duckdb \
  --source ./data/raw \
  --output-dir ./output \
  --dry-run
```

### Apache Iceberg Backend

For production workloads, use Iceberg for time travel and catalog integration:

```bash
# Initialize with REST catalog (Lakekeeper)
odm staging init staging.duckdb \
  --catalog rest \
  --endpoint http://localhost:8181 \
  --warehouse ./warehouse

# Query with time travel
odm staging query "SELECT * FROM staged" --version 5
odm staging query "SELECT * FROM staged" --timestamp "2025-01-10T00:00:00Z"

# Export to Unity Catalog
odm staging export \
  --database staging.duckdb \
  --target unity \
  --endpoint https://workspace.cloud.databricks.com \
  --catalog main \
  --schema staging \
  --table users
```

## Troubleshooting

### Out of Memory

For large datasets, limit the sample size:

```bash
odm inference infer \
  --database staging.duckdb \
  --sample-size 10000 \
  --output schema.json
```

### LLM Timeout

Increase timeout or use a smaller model:

```bash
odm pipeline run \
  --database staging.duckdb \
  --llm-mode online \
  --model llama3.2:1b \  # Smaller model
  --verbose
```

### Schema Mismatch

If mapping produces poor results:

1. Check inferred schema accuracy
2. Increase fuzzy matching threshold
3. Add documentation context for LLM refinement
4. Manually adjust the mapping result

### Database Locked

Ensure no other process is using the database:

```bash
# Kill stuck connections
lsof staging.duckdb

# Or use a fresh database
rm staging.duckdb
odm staging init staging.duckdb
```

## Best Practices

1. **Use partitions** to organize data by source or version
2. **Enable deduplication** for incremental ingestion
3. **Start with small samples** to validate schema inference
4. **Provide documentation context** for better LLM refinement
5. **Review mapping results** before generating transformations
6. **Use checkpointing** for long-running pipelines
7. **Monitor with `--verbose`** during development

## Next Steps

- [CLI Reference](./CLI.md) - Complete command documentation
- [Schema Overview](./SCHEMA_OVERVIEW.md) - Supported schema formats
- [Architecture Guide](./ARCHITECTURE.md) - System design details
