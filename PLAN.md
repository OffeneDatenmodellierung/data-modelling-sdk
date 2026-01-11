# Implementation Plan: Data Pipeline with LLM-Enhanced Schema Inference

**Issue:** [#48](https://github.com/OffeneDatenmodellierung/data-modelling-sdk/issues/48)
**Target Version:** 2.0.0
**Author:** Mark Olliver
**Date:** 2026-01-11

---

## Executive Summary

This plan restructures the `data-modelling-sdk` into a **monorepo with three sub-crates** while adding comprehensive data pipeline capabilities including staging, schema inference, LLM enhancement, and Parquet export.

### Goals

1. **Reorganize** into `core`, `cli`, and `wasm` crates for clear separation of concerns
2. **Add staging database** with DuckDB and PostgreSQL support
3. **Implement schema inference** from raw JSON data
4. **Add LLM-enhanced refinement** (online via Ollama, offline via llama.cpp)
5. **Add target schema mapping** with transformation generation
6. **Add Parquet export** with memory-efficient batched processing
7. **Maintain 100% backward compatibility** for existing public APIs

---

## Architecture

### Current Structure (Monolithic)

```
data-modelling-sdk/
├── src/
│   ├── lib.rs              # Everything: models, import, export, CLI, WASM
│   ├── cli/                # CLI binary (feature-gated)
│   ├── models/             # Core models
│   ├── import/             # Format importers
│   ├── export/             # Format exporters
│   ├── database/           # Database backends
│   ├── storage/            # Storage backends
│   ├── validation/         # Validation logic
│   └── ...
└── Cargo.toml              # Single crate with many features
```

### Target Structure (Workspace Monorepo)

```
data-modelling-sdk/
├── Cargo.toml              # Workspace definition
├── crates/
│   ├── core/               # Core SDK library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models/     # Data models
│   │       ├── import/     # Format importers
│   │       ├── export/     # Format exporters
│   │       ├── validation/ # Validation logic
│   │       ├── storage/    # Storage backends
│   │       ├── database/   # Database backends (DuckDB, PostgreSQL)
│   │       ├── inference/  # NEW: Schema inference engine
│   │       ├── staging/    # NEW: Staging database
│   │       ├── mapping/    # NEW: Schema mapping
│   │       ├── llm/        # NEW: LLM integration
│   │       └── parquet/    # NEW: Parquet export
│   │
│   ├── odm/                # CLI wrapper (binary name: odm)
│   │   ├── Cargo.toml      # Depends on core
│   │   └── src/
│   │       ├── main.rs
│   │       └── commands/
│   │
│   └── wasm/               # WASM/Node wrapper
│       ├── Cargo.toml      # Depends on core (wasm features)
│       └── src/
│           └── lib.rs      # WASM bindings only
│
├── pkg/                    # WASM build output (unchanged)
├── schemas/                # JSON schemas (unchanged)
├── docs/                   # Documentation (unchanged)
└── tests/                  # Integration tests
```

### Crate Responsibilities

| Crate | Responsibility | Targets |
|-------|---------------|---------|
| `data-modelling-core` | All business logic, models, import/export, database, inference, LLM | Library (rlib) |
| `odm` | CLI binary (`odm`), user interaction, output formatting | Binary |
| `data-modelling-wasm` | WASM bindings, browser-specific storage | cdylib (WASM) |

### Dependency Graph

```
                    ┌─────────────────┐
                    │       odm       │
                    │   (CLI binary)  │
                    └────────┬────────┘
                             │ depends on
                             ▼
┌─────────────────┐  ┌─────────────────┐
│ data-modelling  │  │ data-modelling  │
│     -wasm       │──│     -core       │
└─────────────────┘  └─────────────────┘
       │                     │
       │ depends on          │ depends on
       ▼                     ▼
   wasm-bindgen         serde, duckdb,
   web-sys              tokio-postgres,
   js-sys               arrow, etc.
```

---

## Feature Flags (Core Crate)

```toml
[features]
default = []

# Storage backends
storage-api = ["reqwest", "urlencoding"]
storage-fs = ["tokio/fs"]

# Database backends
database = ["toml", "sha2"]
duckdb-backend = ["database", "duckdb"]
postgres-backend = ["database", "tokio-postgres", "deadpool-postgres"]

# Format support
bpmn = ["quick-xml"]
dmn = ["quick-xml"]
openapi = []
schema-validation = ["jsonschema"]

# New pipeline features
inference = []                              # Schema inference engine
staging = ["duckdb-backend"]                # Staging database
staging-postgres = ["postgres-backend"]     # PostgreSQL staging
s3 = ["staging", "aws-sdk-s3", "aws-config"] # S3 ingestion
databricks = ["staging", "reqwest"]          # Unity Catalog Volumes
llm-online = ["reqwest"]                    # Ollama API
llm-offline = ["llama-cpp-2"]               # Embedded LLM
parquet-export = ["arrow", "parquet"]       # Parquet output
mapping = ["inference"]                     # Schema mapping

# Convenience bundles
pipeline = ["staging", "inference", "parquet-export"]
pipeline-full = ["pipeline", "s3", "llm-online", "mapping"]
```

---

## Implementation Phases

### Phase 0: Workspace Restructure (Foundation)
**Duration:** 1-2 weeks
**Risk:** Medium (breaking change potential)

Reorganize into workspace without changing any functionality.

1. Create workspace `Cargo.toml`:
   ```toml
   [workspace]
   resolver = "2"
   members = ["crates/core", "crates/odm", "crates/wasm"]
   ```
2. Move `src/` to `crates/core/src/`
3. Extract CLI to `crates/odm/` (binary name: `odm`)
4. Extract WASM bindings to `crates/wasm/`
5. Update all imports and re-exports
6. Ensure all existing tests pass
7. Update CI/CD workflows

**Acceptance Criteria:**
- All 65+ existing tests pass
- WASM builds and exports same functions
- CLI binary works identically
- NPM package unchanged

### Phase 1: Staging Database (Data Ingestion)
**Duration:** 2-3 weeks
**Dependencies:** Phase 0

Implement staging database with local and S3 ingestion.

1. Create `staging/` module with `StagingDb` struct
2. Implement database schema (staged_json, processing_batches, inferred_schemas)
3. Implement local file ingestion with parallel workers
4. Implement deduplication strategies (path, content hash, both)
5. Implement batch tracking and resume support
6. Add S3 ingestion with credential refresh (feature-gated)
7. Add CLI commands: `init`, `ingest`, `stats`, `query`, `batches`
8. Support both DuckDB and PostgreSQL backends

**Database Schema:**
```sql
CREATE TABLE staged_json (
    id BIGINT PRIMARY KEY,
    file_path VARCHAR NOT NULL,
    record_index INTEGER NOT NULL,
    partition_key VARCHAR,
    raw_json JSON NOT NULL,
    content_hash VARCHAR,
    file_size_bytes BIGINT,
    ingested_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(file_path, record_index)
);

CREATE TABLE processing_batches (
    id VARCHAR PRIMARY KEY,
    source_path VARCHAR NOT NULL,
    partition_key VARCHAR,
    status VARCHAR NOT NULL,
    files_total INTEGER,
    files_processed INTEGER,
    records_ingested BIGINT,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    error_message VARCHAR
);

CREATE TABLE inferred_schemas (
    id VARCHAR PRIMARY KEY,
    schema_name VARCHAR NOT NULL,
    partition_key VARCHAR,
    schema_json JSON NOT NULL,
    sample_count INTEGER,
    version INTEGER DEFAULT 1,
    parent_id VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_staged_partition ON staged_json(partition_key);
CREATE INDEX idx_staged_file ON staged_json(file_path);
CREATE INDEX idx_staged_hash ON staged_json(content_hash);
```

**Acceptance Criteria:**
- Can ingest 1M+ records without OOM
- Resume works after interruption
- Deduplication prevents duplicate ingestion
- Works with both DuckDB and PostgreSQL

### Phase 2: Schema Inference Engine
**Duration:** 2 weeks
**Dependencies:** Phase 1

Implement automatic JSON schema inference from staged data.

1. Create `inference/` module with `SchemaInferrer` struct
2. Implement incremental field discovery
3. Implement type inference with conflict resolution
4. Implement format detection (date-time, email, uuid, etc.)
5. Implement frequency-based field filtering
6. Implement example value collection
7. Store inferred schemas in database with versioning
8. Add CLI command: `schema --database --sample --partition`

**Type Inference Rules:**
| JSON Value | Inferred Type | Notes |
|------------|---------------|-------|
| `null` | `null` | Tracks nullability |
| `true`/`false` | `boolean` | |
| Integer (fits i64) | `integer` | |
| Floating point | `number` | Integers promoted if mixed |
| String | `string` | Format detection applied |
| Array | `array` | Items type inferred recursively |
| Object | `object` | Properties inferred recursively |
| Mixed types | `oneOf` | Multiple types for same field |

**Acceptance Criteria:**
- Infers correct types for 95%+ of fields
- Detects common formats (ISO dates, emails, UUIDs)
- Handles nested objects and arrays
- Merges/evolves schemas across records

### Phase 3: Apache Iceberg Integration
**Duration:** 2 weeks
**Dependencies:** Phase 1, Phase 2

Replace DuckDB-only staging with Apache Iceberg for Parquet storage, time travel, and cloud catalog support.

1. Add iceberg-rust dependencies (`iceberg`, `iceberg-catalog-rest`, `iceberg-datafusion`, `datafusion`)
2. Create catalog abstraction supporting REST (Lakekeeper), S3 Tables, Unity Catalog, Glue
3. Migrate `staged_json` table to Iceberg table with Parquet storage
4. Use Iceberg table properties for batch tracking (replaces DuckDB batch table)
5. Add time travel query support (by version and timestamp)
6. Create schema-inferenced views over raw_json (validates without data movement)
7. Implement export to production catalogs (Unity Catalog, Glue, S3 Tables)
8. Maintain DuckDB query layer for manual inspection via Iceberg extension

**Architecture:**
```
JSON files → Iceberg Table (raw_json)
                  ↓
            Schema Inference (Phase 2)
                  ↓
            Schema-Inferenced View (validates without data movement)
                  ↓
            Export to Production Catalog
            ├── Unity Catalog (Databricks)
            └── Glue / S3 Tables (AWS)
```

**Key Components:**
| Component | Technology | Purpose |
|-----------|------------|---------|
| Raw Data | Iceberg `raw_json` table | JSON ingestion with time travel |
| Batch Tracking | Iceberg table properties | Resume support (prod-optimized) |
| Query Layer | DuckDB Iceberg ext + DataFusion | Manual + programmatic access |
| Local Catalog | Lakekeeper | REST catalog for development |
| Prod Catalogs | Unity Catalog / Glue / S3 Tables | Production deployment |

**New CLI Commands:**
- `staging init --catalog <type>` - Initialize with catalog type (rest, s3-tables, unity, glue)
- `staging history` - Show table version history
- `staging query --version N` - Time travel queries
- `staging view create` - Create typed view from inferred schema
- `staging export --target <unity|glue|s3-tables>` - Export to production catalog

**Acceptance Criteria:**
- Local development works with Lakekeeper REST catalog
- Time travel queries work (by version and timestamp)
- Can export to Unity Catalog and S3 Tables
- DuckDB can query Iceberg tables for inspection
- Schema-inferenced views validate data without movement

### Phase 4: LLM-Enhanced Refinement
**Duration:** 2-3 weeks
**Dependencies:** Phase 2

Add LLM integration for semantic schema enhancement.

1. Create `llm/` module with `LlmClient` trait
2. Implement Ollama client (online mode)
3. Implement llama.cpp integration (offline mode, feature-gated)
4. Implement documentation loading (.docx, .txt, .md)
5. Design refinement prompts with strict field preservation
6. Implement post-processing validation (no field renames allowed)
7. Store refined schemas with LLM metadata
8. Add CLI flags: `--llm online|offline`, `--ollama-url`, `--model-path`, `--doc-path`

**LLM Prompt Structure:**
```
You are a data engineer. Given the following JSON schema that was
automatically inferred, improve it by:
1. Adding meaningful "description" fields
2. Identifying semantic "format" values (date-time, email, uri, uuid)
3. Adding "examples" arrays with sample values
4. Suggesting constraints (min/max, patterns)

CRITICAL: Do NOT change any field names. They must match exactly.

[Optional: Reference documentation provided by user]

Schema:
{schema_json}

Sample records:
{samples}

Return ONLY the improved JSON schema.
```

**Validation Rules:**
- All original field names must be present
- No new required fields added
- Types can only be narrowed, not changed
- Descriptions are additive only

**Acceptance Criteria:**
- Adds meaningful descriptions to 80%+ of fields
- Detects semantic formats correctly
- Never corrupts field names
- Works offline with local GGUF model

### Phase 5: Target Schema Mapping
**Duration:** 2-3 weeks
**Dependencies:** Phase 4

Implement schema comparison and transformation generation.

1. Create `mapping/` module with `SchemaMapper` struct
2. Implement field matching (exact, case-insensitive, fuzzy)
3. Implement LLM-enhanced semantic matching (optional)
4. Implement transformation detection (type cast, rename, merge, split)
5. Implement gap analysis (missing fields in source)
6. Implement extra field detection (fields not in target)
7. Generate transformation scripts (SQL, JQ, Python, Spark)
8. Add CLI command: `map --source --target --output --transform-format`

**Mapping Result Structure:**
```rust
pub struct SchemaMapping {
    pub direct_mappings: Vec<FieldMapping>,
    pub transformations: Vec<TransformMapping>,
    pub gaps: Vec<FieldGap>,
    pub extras: Vec<String>,
    pub compatibility_score: f64,
}
```

**Acceptance Criteria:**
- Identifies 90%+ of obvious mappings
- Generates working SQL transformations
- Provides actionable gap analysis
- LLM improves semantic matching when enabled

### Phase 6: Full Pipeline Integration
**Duration:** 1-2 weeks
**Dependencies:** Phases 1-5

Integrate all features into cohesive pipeline command.

1. Implement `pipeline` CLI command with all stages
2. Add checkpoint/resume for full pipeline
3. Add dry-run mode for validation
4. Add comprehensive progress reporting
5. Add pipeline configuration files (YAML)
6. Update documentation and examples

**CLI Command:**
```bash
# From local files
odm pipeline \
  --database pipeline.duckdb \
  --source ./data/ \
  --output-dir ./output/

# From S3
odm pipeline \
  --database pipeline.duckdb \
  --source s3://my-bucket/legacy-data/ \
  --output-dir ./output/

# From Unity Catalog Volume
odm pipeline \
  --database pipeline.duckdb \
  --source /Volumes/catalog/schema/volume/path/ \
  --databricks-host https://my-workspace.cloud.databricks.com \
  --output-dir ./output/

# Full pipeline with all options
odm pipeline \
  --database pipeline.duckdb \
  --source s3://my-bucket/legacy-data/ \
  --target-schema canonical_schema.json \
  --output-dir ./output/ \
  --llm online \
  --doc-path data_dictionary.docx
```

**Source Types:**
| Prefix | Source Type | Authentication |
|--------|-------------|----------------|
| `./` or `/` | Local filesystem | None |
| `s3://` | AWS S3 bucket | AWS credentials (env, profile, SSO) |
| `/Volumes/` | Unity Catalog Volume | Databricks token + workspace URL |

**Unity Catalog Volume Path Format:**
```
/Volumes/<catalog>/<schema>/<volume>/<path>/<file-name>
```

**Acceptance Criteria:**
- Single command runs full pipeline
- Resume works after any stage failure
- Configuration file alternative to CLI args
- Clear progress and error reporting

---

## Testing Strategy

### Unit Tests
- All new modules have 80%+ coverage
- Mock LLM responses for deterministic testing
- Test edge cases (empty data, malformed JSON, type conflicts)

### Integration Tests
- End-to-end pipeline tests with sample data
- Database backend compatibility tests (DuckDB + PostgreSQL)
- S3 tests using LocalStack in CI
- WASM tests in browser environment

### Backward Compatibility Tests
- All existing public APIs unchanged
- Existing tests pass without modification
- CLI commands work identically
- WASM exports same 83 functions

### Performance Tests
- Ingest 1M records < 5 minutes
- Export 1M records to Parquet < 10 minutes
- Schema inference on 10K samples < 30 seconds
- Memory usage < 500MB for standard operations

---

## Migration Guide

### For Library Users

```rust
// Before (1.x)
use data_modelling_sdk::{Table, Column, import_from_sql};

// After (2.x) - unchanged!
use data_modelling_sdk::{Table, Column, import_from_sql};

// OR use core directly
use data_modelling_core::{Table, Column, import_from_sql};
```

The root `data-modelling-sdk` crate will re-export everything from `data-modelling-core` for backward compatibility.

### For CLI Users

The binary is renamed from `data-modelling-cli` to `odm` (OpenDataModelling).

```bash
# Before
data-modelling-cli import sql schema.sql

# After
odm import sql schema.sql
```

### For WASM/NPM Users

No changes required. The package name remains `@offenedatenmodellierung/data-modelling-sdk`.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing APIs | Medium | High | Comprehensive test suite, re-exports |
| LLM hallucinations corrupt schemas | Medium | High | Strict post-validation, additive-only changes |
| Memory issues with large datasets | Medium | Medium | Chunked processing, streaming |
| S3 credential issues in long operations | Low | Medium | Periodic refresh, session validation |
| WASM size increase | Low | Low | Feature gating, tree shaking |
| PostgreSQL/DuckDB incompatibilities | Low | Medium | Abstract trait, backend-specific tests |

---

## Success Metrics

1. **Backward Compatibility:** 100% of existing tests pass
2. **Performance:** Handle 1M+ records without OOM
3. **Schema Quality:** LLM descriptions on 80%+ of fields
4. **Mapping Accuracy:** 90%+ correct field mappings
5. **Test Coverage:** 80%+ on new code
6. **Documentation:** All new features documented with examples

---

## Timeline Summary

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 0: Workspace Restructure | 1-2 weeks | 1-2 weeks |
| Phase 1: Staging Database | 2-3 weeks | 3-5 weeks |
| Phase 2: Schema Inference | 2 weeks | 5-7 weeks |
| Phase 3: Parquet Export | 2 weeks | 7-9 weeks |
| Phase 4: LLM Refinement | 2-3 weeks | 9-12 weeks |
| Phase 5: Schema Mapping | 2-3 weeks | 11-15 weeks |
| Phase 6: Pipeline Integration | 1-2 weeks | 12-17 weeks |

**Total Estimated Duration:** 12-17 weeks (3-4 months)

---

## Open Questions

1. **Schema Grouping Threshold:** How different must schemas be to create separate groups? Single missing field? Type difference?
   - **Proposal:** Use configurable similarity threshold (default 95%)

2. **LLM Model Selection:** Which models work best for schema refinement?
   - **Proposal:** Test CodeLlama 7B/13B, Mistral 7B, document findings

3. **Parquet Partitioning Strategy:** By date? By schema hash? User-defined?
   - **Proposal:** Support all three, user chooses via CLI flag

4. **Error Handling in Pipeline:** Stop on first error or collect all?
   - **Proposal:** Configurable, default to collect with summary

---

## Appendix: New Dependencies

| Dependency | Purpose | Feature Flag | Size Impact |
|------------|---------|--------------|-------------|
| `aws-sdk-s3` | S3 ingestion | `s3` | ~5MB |
| `aws-config` | AWS credential handling | `s3` | ~2MB |
| `arrow` | Arrow data format | `parquet-export` | ~10MB |
| `parquet` | Parquet file writing | `parquet-export` | ~5MB |
| `llama-cpp-2` | Embedded LLM | `llm-offline` | ~50MB+ |
| `indicatif` | Progress bars | Core | ~100KB |
| `sha2` | Content hashing | `staging` | Already included |
| `rayon` | Parallel processing | `staging` | ~500KB |

**Note:** All heavy dependencies are feature-gated to minimize default binary size.
