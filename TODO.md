# Implementation TODO: Data Pipeline with LLM-Enhanced Schema Inference

**Related:** [PLAN.md](./PLAN.md) | [Issue #48](https://github.com/OffeneDatenmodellierung/data-modelling-sdk/issues/48)

This document provides a detailed task breakdown for implementing the data pipeline features.

---

## Legend

- [ ] Not started
- [x] Completed
- [~] In progress
- [!] Blocked

---

## Phase 0: Workspace Restructure

### 0.1 Workspace Setup
- [ ] Create root `Cargo.toml` as workspace
  ```toml
  [workspace]
  resolver = "2"
  members = ["crates/core", "crates/cli", "crates/wasm"]
  ```
- [ ] Create `crates/` directory structure
- [ ] Create `crates/core/Cargo.toml` with all current dependencies
- [ ] Create `crates/cli/Cargo.toml` depending on core
- [ ] Create `crates/wasm/Cargo.toml` depending on core

### 0.2 Core Crate Migration
- [ ] Move `src/models/` to `crates/core/src/models/`
- [ ] Move `src/import/` to `crates/core/src/import/`
- [ ] Move `src/export/` to `crates/core/src/export/`
- [ ] Move `src/validation/` to `crates/core/src/validation/`
- [ ] Move `src/storage/` to `crates/core/src/storage/`
- [ ] Move `src/database/` to `crates/core/src/database/`
- [ ] Move `src/model/` to `crates/core/src/model/`
- [ ] Move `src/convert/` to `crates/core/src/convert/`
- [ ] Move `src/auth/` to `crates/core/src/auth/`
- [ ] Move `src/workspace/` to `crates/core/src/workspace/`
- [ ] Move `src/git/` to `crates/core/src/git/`
- [ ] Create `crates/core/src/lib.rs` with all re-exports
- [ ] Update all internal imports to use `crate::` paths

### 0.3 CLI Crate Extraction (odm)
- [ ] Move `src/cli/` to `crates/odm/src/`
- [ ] Rename binary from `data-modelling-cli` to `odm`
- [ ] Update `crates/odm/src/main.rs` to import from `data_modelling_core`
- [ ] Move CLI-specific error types to odm crate
- [ ] Move CLI-specific output formatting to odm crate
- [ ] Update command handlers to use core imports
- [ ] Add `[[bin]] name = "odm"` target in `crates/odm/Cargo.toml`

### 0.4 WASM Crate Extraction
- [ ] Create `crates/wasm/src/lib.rs`
- [ ] Move all `#[wasm_bindgen]` functions from `src/lib.rs`
- [ ] Move `WasmError` type to WASM crate
- [ ] Move browser storage backend to WASM crate
- [ ] Update WASM-specific imports
- [ ] Configure `crate-type = ["cdylib"]` for WASM crate
- [ ] Update `wasm-pack` build to use WASM crate

### 0.5 Backward Compatibility Layer
- [ ] Create root `src/lib.rs` that re-exports from core
  ```rust
  pub use data_modelling_core::*;
  ```
- [ ] Ensure all existing public types are re-exported
- [ ] Add deprecation notices where appropriate
- [ ] Document migration path in README

### 0.6 CI/CD Updates
- [ ] Update `.github/workflows/ci.yml` for workspace
- [ ] Update `.github/workflows/release-cli.yml` for new paths and `odm` binary name
- [ ] Update `.github/workflows/release-wasm.yml` for new paths
- [ ] Update `wasm-pack` build commands
- [ ] Test all platforms (Linux, macOS, Windows)
- [ ] Update release artifacts naming (odm-linux-x86_64, odm-macos-arm64, etc.)

### 0.7 Testing & Validation
- [ ] Run all existing unit tests
- [ ] Run all existing integration tests
- [ ] Verify CLI binary works identically
- [ ] Verify WASM package exports all 83 functions
- [ ] Verify NPM package works in browser
- [ ] Update test imports if needed
- [ ] Add workspace-level test runner

---

## Phase 1: Staging Database ✅ COMPLETE

### 1.1 Module Structure
- [x] Create `src/staging/mod.rs`
- [x] Create `src/staging/config.rs` - IngestConfig, DedupStrategy
- [x] Create `src/staging/db.rs` - StagingDb struct
- [x] Create `src/staging/ingest.rs` - Ingestion logic
- [x] Create `src/staging/batch.rs` - Batch tracking
- [x] Create `src/staging/error.rs` - Error types
- [x] Add `staging` feature to Cargo.toml

### 1.2 Database Schema
- [x] Define `staged_json` table DDL
- [x] Define `processing_batches` table DDL
- [x] Define `inferred_schemas` table DDL
- [x] Create indexes for performance
- [x] Implement schema initialization for DuckDB
- [ ] Implement schema initialization for PostgreSQL (deferred)
- [x] Add schema version tracking

### 1.3 StagingDb Implementation
- [x] Implement `StagingDb::open(path)` - Open/create database
- [x] Implement `StagingDb::memory()` - In-memory for testing
- [x] Implement `StagingDb::init()` - Initialize schema
- [x] Implement `StagingDb::record_count(partition)` - Count records
- [x] Implement `StagingDb::get_sample(limit, partition)` - Get samples
- [x] Implement `StagingDb::query(sql)` - Execute arbitrary SQL
- [x] Implement `StagingDb::list_batches(limit)` - List batch history
- [x] Implement `StagingDb::partition_stats()` - Get partition statistics
- [ ] Add PostgreSQL backend support via trait (deferred)

### 1.4 Local Ingestion
- [x] Implement file discovery with glob patterns
- [x] Implement JSON file parsing (single object)
- [x] Implement JSONL file parsing (newline-delimited)
- [x] Implement batched database inserts
- [x] Calculate content hashes for deduplication
- [x] Handle malformed JSON gracefully
- [ ] Implement parallel file processing with rayon (deferred)
- [ ] Implement progress reporting with indicatif (deferred)

### 1.5 Deduplication
- [x] Implement `DedupStrategy::None` - No deduplication
- [x] Implement `DedupStrategy::ByPath` - Skip same file paths
- [x] Implement `DedupStrategy::ByContent` - Skip same content hash
- [x] Implement `DedupStrategy::Both` - Path and content
- [x] Add dedup statistics to IngestStats

### 1.6 Batch Tracking & Resume
- [x] Create batch record on ingestion start
- [x] Update batch progress during ingestion
- [x] Mark batch complete/failed on finish
- [x] Implement resume from last successful file
- [x] Add batch history query

### 1.7 S3 Ingestion (Feature-Gated)
- [ ] Add `s3` feature with `aws-sdk-s3` dependency (deferred to later phase)

### 1.8 Unity Catalog Volumes Ingestion (Feature-Gated)
- [ ] Add `databricks` feature (deferred to later phase)

### 1.9 CLI Commands
- [x] Add `staging init` command - Initialize staging database
- [x] Add `staging ingest` command - Ingest from source
  - [x] `--database` - Database path
  - [x] `--source` - Source path
  - [x] `--pattern` - File glob pattern
  - [x] `--partition` - Partition key
  - [x] `--batch-size` - Insert batch size
  - [x] `--dedup` - Deduplication strategy
  - [x] `--resume` - Resume previous batch
- [x] Add `staging stats` command - Show ingestion statistics
- [x] Add `staging query` command - Query staged data
- [x] Add `staging batches` command - Show batch history
- [x] Add `staging sample` command - Get sample records

### 1.10 Testing
- [x] Unit tests for IngestConfig
- [x] Unit tests for deduplication logic
- [x] Unit tests for StagingDb operations
- [x] Integration tests for ingestion

---

## Phase 2: Schema Inference Engine ✅ COMPLETE

### 2.1 Module Structure
- [x] Create `src/inference/mod.rs`
- [x] Create `src/inference/config.rs` - InferenceConfig
- [x] Create `src/inference/inferrer.rs` - SchemaInferrer
- [x] Create `src/inference/types.rs` - Type inference logic
- [x] Create `src/inference/formats.rs` - Format detection
- [x] Create `src/inference/merge.rs` - Schema merging
- [x] Create `src/inference/error.rs` - Error types
- [x] Add `inference` feature to Cargo.toml

### 2.2 InferenceConfig
- [x] `sample_size: usize` - Max records to sample (0 = all)
- [x] `min_field_frequency: f64` - Min occurrence rate for inclusion
- [x] `detect_formats: bool` - Enable format detection
- [x] `max_depth: usize` - Max nested object depth
- [x] `collect_examples: bool` - Collect example values
- [x] `max_examples: usize` - Max examples per field
- [x] Implement `Default` trait
- [x] Implement builder pattern

### 2.3 SchemaInferrer Implementation
- [x] Implement `SchemaInferrer::new()` - Default config
- [x] Implement `SchemaInferrer::with_config(config)` - Custom config
- [x] Implement `add_json(&mut self, json: &str)` - Add single record
- [x] Implement `add_value(&mut self, value: &Value)` - Add parsed JSON
- [x] Implement `add_json_batch(&mut self, records: &[String])` - Add batch
- [x] Implement `finalize(self) -> InferredSchema` - Generate schema
- [x] Implement `stats(&self) -> InferenceStats` - Get statistics
- [x] Track field occurrences per path
- [x] Track type occurrences per path
- [x] Track numeric stats (min/max/avg)

### 2.4 Type Inference
- [x] Infer `null` from JSON null
- [x] Infer `boolean` from true/false
- [x] Infer `integer` from whole numbers
- [x] Infer `number` from decimals
- [x] Infer `string` from strings
- [x] Infer `array` from arrays (recursive)
- [x] Infer `object` from objects (recursive)
- [x] Handle mixed types with `Mixed` variant
- [x] Promote integer to number when mixed
- [x] Track nullability separately from type

### 2.5 Format Detection
- [x] Detect `date-time` (ISO 8601)
- [x] Detect `date` (YYYY-MM-DD)
- [x] Detect `time` (HH:MM:SS)
- [x] Detect `email` (RFC 5322)
- [x] Detect `uri` (RFC 3986)
- [x] Detect `uuid` (RFC 4122)
- [x] Detect `hostname` (RFC 1123)
- [x] Detect `ipv4` (dotted decimal)
- [x] Detect `ipv6` (RFC 5952)
- [x] Detect `semver` (semantic versioning)
- [x] Detect `country_code` (ISO 3166-1 alpha-2)
- [x] Detect `currency_code` (ISO 4217)
- [x] Detect `language_code` (ISO 639-1)
- [x] Detect `mime_type`
- [x] Detect `base64`
- [x] Detect `jwt`
- [x] Detect `slug`
- [x] Detect `phone` (E.164)
- [x] Use regex-based pattern matching

### 2.6 Schema Merging/Evolution
- [x] Merge schemas with same structure
- [x] Evolve schemas when new fields appear
- [x] Calculate schema similarity score (Jaccard)
- [x] Configurable merge threshold
- [x] Group records by schema similarity
- [x] `merge_schemas()` function
- [x] `group_similar_schemas()` function

### 2.7 Example Collection
- [x] Collect unique example values per field
- [x] Limit examples to `max_examples`
- [x] Exclude null values

### 2.8 JSON Schema Export
- [x] Implement `to_json_schema()` on InferredSchema
- [x] Convert InferredType to JSON Schema format
- [x] Handle required fields
- [x] Handle format annotations

### 2.9 CLI Commands
- [x] Add `inference infer` command - Infer schema from staged data
  - [x] `--database` - Database path
  - [x] `--output` - Output schema file
  - [x] `--sample-size` - Sample size
  - [x] `--partition` - Filter by partition
  - [x] `--min-frequency` - Field frequency threshold
  - [x] `--max-depth` - Max nesting depth
  - [x] `--no-formats` - Disable format detection
  - [x] `--format` - Output format (json, yaml, json-schema)
- [x] Add `inference schemas` command - Analyze and group schemas
  - [x] `--database` - Database path
  - [x] `--threshold` - Similarity threshold
  - [x] `--format` - Output format (table, json)

### 2.10 Testing
- [x] Unit tests for type inference rules (6 tests)
- [x] Unit tests for format detection patterns (12 tests)
- [x] Unit tests for schema merging (6 tests)
- [x] Unit tests for InferenceConfig (3 tests)
- [x] Unit tests for SchemaInferrer (9 tests)
- [x] All 36 inference tests passing

---

## Phase 3: Apache Iceberg Integration ✅

**Status:** Core Implementation Complete (2026-01-11)

### 3.1 Dependencies & Setup
- [x] Add `iceberg = "0.7"` to Cargo.toml
- [x] Add `iceberg-catalog-rest = "0.7"` to Cargo.toml
- [ ] Add `iceberg-datafusion = "0.7"` to Cargo.toml (deferred - not needed for core)
- [ ] Add `datafusion = "44"` to Cargo.toml (deferred - not needed for core)
- [x] Add `iceberg` feature flag

### 3.2 Catalog Abstraction
- [x] Create `src/staging/catalog.rs`
- [x] Define `CatalogConfig` enum (Rest, S3Tables, Unity, Glue)
- [x] Implement `CatalogOperations` trait
- [x] Implement REST catalog client (for Lakekeeper/Nessie/Polaris)
- [x] Define S3 Tables catalog config (client implementation deferred)
- [x] Define Unity Catalog config (client implementation deferred)
- [x] Define Glue catalog config (client implementation deferred)

### 3.3 Iceberg Table Operations
- [x] Create `src/staging/iceberg_table.rs`
- [x] Implement `IcebergTable` struct
- [x] Implement table creation with schema
- [x] Define `RawJsonRecord` for JSON staging
- [x] Implement `append_records()` method with Arrow/Parquet writing
- [x] Implement `write_parquet_file()` helper function
- [x] Implement time travel reads (by version) - `get_snapshot()`
- [x] Implement time travel reads (by timestamp) - `list_snapshots()`
- [x] Store batch metadata in table properties - `BatchMetadata`
- [x] Implement table history listing - `list_snapshots()`

### 3.4 Ingestion Migration
- [x] Add `arrow = "55"` and `parquet = "55"` dependencies
- [x] Update `src/staging/ingest.rs` for Iceberg writes (`ingest_to_iceberg()`)
- [x] Add `to_raw_json_records()` conversion function
- [x] Migrate batch tracking to table properties
- [x] Keep both DuckDB and Iceberg backends available (user choice)
- [x] Keep DuckDB for complex batch queries (optional)
- [x] Implement resume logic for Iceberg ingestion
  - [x] Enhanced `BatchMetadata` with `last_file_path`, `status`, `files_processed`, etc.
  - [x] Added `BatchStatus` enum (Running, Completed, Failed)
  - [x] Added `IcebergIngestConfig` with resume support
  - [x] Added `ingest_to_iceberg_with_config()` for full control
  - [x] Resume skips files before last processed file
  - [x] Periodic batch metadata updates for crash recovery

### 3.5 Schema-Inferenced Views
- [x] Generate CREATE VIEW SQL from inferred schema (in CLI)
- [x] Apply JSON extract functions for each field
- [x] Handle nested objects with flattening
- [x] Handle arrays as JSON strings
- [x] Validate view creation

### 3.6 Production Export
- [x] Create `src/staging/export.rs`
- [x] Define `ExportTarget` enum (Unity, Glue, S3Tables, Local)
- [x] Define `ExportConfig` struct
- [x] Define `ExportResult` struct
- [x] Implement `export_to_catalog()` function
- [x] Implement `export_to_local()` for local Parquet file copying
- [x] Define Unity Catalog export config
- [x] Define S3 Tables export config
- [x] Define Glue export config
- [ ] Implement actual cloud catalog registration (future)

### 3.7 CLI Commands
- [x] Update `staging init` for catalog config
  - [x] `--catalog` - Catalog type (rest, s3-tables, unity, glue)
  - [x] `--endpoint` - Catalog endpoint URL
  - [x] `--warehouse` - Warehouse path
  - [x] `--token` - Authentication token
  - [x] `--region` - AWS region
  - [x] `--arn` - S3 Tables ARN
  - [x] `--profile` - AWS profile
- [x] Add `staging history` command
- [x] Update `staging query` for time travel
  - [x] `--version` - Query specific version
  - [x] `--timestamp` - Query as of timestamp
- [x] Add `staging view create` command
  - [x] `--name` - View name
  - [x] `--schema` - Inferred schema file
  - [x] `--source-table` - Source table name
- [x] Add `staging export` command
  - [x] `--target` - Target catalog (unity, glue, s3-tables)
  - [x] `--endpoint` - Target endpoint
  - [x] `--catalog` - Target catalog name
  - [x] `--schema` - Target schema name
  - [x] `--table` - Target table name

### 3.8 Testing
- [x] Unit tests for catalog abstraction (12 tests)
- [x] Unit tests for Iceberg table operations (10 tests)
- [x] Unit tests for view generation (in CLI)
- [ ] Integration test: local Lakekeeper workflow (requires running catalog)
- [ ] Integration test: time travel queries (requires running catalog)
- [ ] Integration test: schema-inferenced view (requires running catalog)
- [ ] Manual test: export to Unity Catalog
- [ ] Manual test: export to S3 Tables

---

## Phase 4: LLM-Enhanced Refinement

### 4.1 Module Structure
- [ ] Create `crates/core/src/llm/mod.rs`
- [ ] Create `crates/core/src/llm/config.rs` - LlmMode, RefinementConfig
- [ ] Create `crates/core/src/llm/client.rs` - LlmClient trait
- [ ] Create `crates/core/src/llm/ollama.rs` - Ollama implementation
- [ ] Create `crates/core/src/llm/llamacpp.rs` - llama.cpp implementation
- [ ] Create `crates/core/src/llm/prompt.rs` - Prompt templates
- [ ] Create `crates/core/src/llm/validation.rs` - Output validation
- [ ] Create `crates/core/src/llm/docs.rs` - Documentation loading
- [ ] Create `crates/core/src/llm/error.rs` - Error types
- [ ] Add `llm-online` feature with `reqwest`
- [ ] Add `llm-offline` feature with `llama-cpp-2`

### 4.2 LlmMode Enum
- [ ] `None` - No LLM refinement
- [ ] `Online { url, model }` - Ollama API
- [ ] `Offline { model_path }` - Embedded llama.cpp
- [ ] Implement convenience constructors
- [ ] Implement `Default` (None)

### 4.3 RefinementConfig
- [ ] `llm_mode: LlmMode`
- [ ] `documentation_path: Option<PathBuf>`
- [ ] `documentation_text: Option<String>`
- [ ] `max_context_tokens: usize`
- [ ] `timeout_seconds: u64`
- [ ] `max_retries: usize`
- [ ] Implement `Default`

### 4.4 LlmClient Trait
- [ ] Define `async fn complete(&self, prompt: &str) -> Result<String>`
- [ ] Define `fn model_name(&self) -> &str`
- [ ] Define `fn max_tokens(&self) -> usize`

### 4.5 Ollama Client (Online)
- [ ] Implement HTTP client for Ollama API
- [ ] Handle `/api/generate` endpoint
- [ ] Support model selection
- [ ] Implement timeout handling
- [ ] Implement retry logic
- [ ] Parse streaming responses
- [ ] Handle rate limiting

### 4.6 llama.cpp Client (Offline)
- [ ] Integrate `llama-cpp-2` crate
- [ ] Load GGUF model files
- [ ] Implement inference
- [ ] Configure context size
- [ ] Handle model loading errors
- [ ] Support multiple quantization levels

### 4.7 Documentation Loading
- [ ] Load `.txt` files (plain text)
- [ ] Load `.md` files (Markdown)
- [ ] Load `.docx` files (Word) - use simple extraction
- [ ] Truncate to max context tokens
- [ ] Extract relevant sections if too long

### 4.8 Prompt Engineering
- [ ] Design refinement prompt template
- [ ] Include schema JSON
- [ ] Include sample records
- [ ] Include documentation context
- [ ] Emphasize field name preservation
- [ ] Request structured JSON output
- [ ] Add examples of good refinements

### 4.9 Output Validation
- [ ] Parse LLM output as JSON
- [ ] Verify all original fields present
- [ ] Verify no fields renamed
- [ ] Verify types not changed (only narrowed)
- [ ] Verify only additive changes
- [ ] Reject and retry on validation failure
- [ ] Log validation failures for debugging

### 4.10 Refinement Pipeline
- [ ] Implement `refine_schema(schema, samples, config)`
- [ ] Chunk large schemas for token limits
- [ ] Merge refined chunks
- [ ] Store refinement metadata
- [ ] Track LLM model used
- [ ] Measure refinement quality

### 4.11 CLI Integration
- [ ] Add `--llm` flag (none/online/offline)
- [ ] Add `--ollama-url` for online mode
- [ ] Add `--model` for model selection
- [ ] Add `--model-path` for offline mode
- [ ] Add `--doc-path` for documentation
- [ ] Add `--no-refine` to skip LLM step

### 4.12 Testing
- [ ] Unit tests with mock LLM responses
- [ ] Unit tests for prompt generation
- [ ] Unit tests for output validation
- [ ] Integration test: Ollama refinement (manual)
- [ ] Integration test: offline refinement (manual)
- [ ] Test validation rejects bad output
- [ ] Test retry logic

---

## Phase 5: Target Schema Mapping

### 5.1 Module Structure
- [ ] Create `crates/core/src/mapping/mod.rs`
- [ ] Create `crates/core/src/mapping/config.rs` - MappingConfig
- [ ] Create `crates/core/src/mapping/matcher.rs` - Field matching
- [ ] Create `crates/core/src/mapping/transform.rs` - Transformation types
- [ ] Create `crates/core/src/mapping/generator.rs` - Script generation
- [ ] Create `crates/core/src/mapping/result.rs` - SchemaMapping result
- [ ] Create `crates/core/src/mapping/error.rs` - Error types
- [ ] Add `mapping` feature depending on `inference`

### 5.2 SchemaMapping Result
- [ ] `direct_mappings: Vec<FieldMapping>`
- [ ] `transformations: Vec<TransformMapping>`
- [ ] `gaps: Vec<FieldGap>`
- [ ] `extras: Vec<String>`
- [ ] `compatibility_score: f64`
- [ ] Implement serialization

### 5.3 FieldMapping
- [ ] `source_path: String`
- [ ] `target_path: String`
- [ ] `confidence: f64`
- [ ] `type_compatible: bool`

### 5.4 TransformMapping
- [ ] `source_path: String`
- [ ] `target_path: String`
- [ ] `transform_type: TransformType`
- [ ] `description: String`

### 5.5 TransformType Enum
- [ ] `TypeCast { from, to }`
- [ ] `Rename`
- [ ] `Merge { source_paths }`
- [ ] `Split { target_paths }`
- [ ] `FormatChange { from_format, to_format }`
- [ ] `Custom { expression }`

### 5.6 Field Matching
- [ ] Exact name match
- [ ] Case-insensitive match
- [ ] Fuzzy match (Levenshtein distance)
- [ ] Semantic match (LLM-enhanced, optional)
- [ ] Type compatibility check
- [ ] Configurable confidence thresholds

### 5.7 LLM-Enhanced Matching (Optional)
- [ ] Generate matching prompt
- [ ] Include source and target schemas
- [ ] Request field-to-field suggestions
- [ ] Parse and validate LLM suggestions
- [ ] Combine with algorithmic matching

### 5.8 Gap Analysis
- [ ] Identify required target fields missing from source
- [ ] Identify optional target fields missing from source
- [ ] Suggest defaults where possible
- [ ] Find similar source fields for gaps

### 5.9 Transformation Script Generation
- [ ] Generate DuckDB/SQL transformations
- [ ] Generate JQ filter expressions
- [ ] Generate Python transformation script
- [ ] Generate PySpark transformation
- [ ] Include comments explaining each mapping

### 5.10 CLI Commands
- [ ] Add `map` command
  - [ ] `--source` - Source schema file
  - [ ] `--target` - Target schema file
  - [ ] `--output` - Mapping result file
  - [ ] `--llm` - Enable LLM matching
  - [ ] `--min-similarity` - Fuzzy match threshold
  - [ ] `--transform-format` - sql/jq/python/spark
  - [ ] `--transform-output` - Transformation script file

### 5.11 Testing
- [ ] Unit tests for exact matching
- [ ] Unit tests for fuzzy matching
- [ ] Unit tests for type compatibility
- [ ] Unit tests for transformation detection
- [ ] Integration test: map similar schemas
- [ ] Integration test: map with gaps
- [ ] Integration test: generate SQL transform
- [ ] Verify generated SQL executes correctly

---

## Phase 6: Full Pipeline Integration

### 6.1 Pipeline Command
- [ ] Create `crates/cli/src/commands/pipeline.rs`
- [ ] Implement pipeline orchestration
- [ ] Define pipeline stages enum
- [ ] Implement stage execution order
- [ ] Handle stage dependencies

### 6.2 Pipeline Configuration
- [ ] Define `PipelineConfig` struct
- [ ] Support YAML configuration files
- [ ] CLI args override config file
- [ ] Validate configuration

### 6.3 Pipeline Stages
- [ ] Stage 1: Ingest (staging database)
- [ ] Stage 2: Infer (schema inference)
- [ ] Stage 3: Refine (LLM enhancement, optional)
- [ ] Stage 4: Map (target schema mapping, optional)
- [ ] Stage 5: Export (Parquet output)
- [ ] Stage 6: Generate (ODCS contracts)

### 6.4 Checkpointing
- [ ] Store pipeline state in database
- [ ] Track completed stages
- [ ] Resume from last completed stage
- [ ] Clean up partial outputs on resume

### 6.5 Dry Run Mode
- [ ] `--dry-run` flag
- [ ] Validate all inputs
- [ ] Show planned operations
- [ ] Don't write any outputs
- [ ] Estimate resource requirements

### 6.6 Progress Reporting
- [ ] Overall pipeline progress bar
- [ ] Per-stage progress bars
- [ ] Estimated time remaining
- [ ] Stage completion summaries
- [ ] Error aggregation

### 6.7 CLI Command
- [ ] Add `pipeline` command
  - [ ] `--database` - Staging database
  - [ ] `--source` - Input source (local path, s3://, or /Volumes/)
  - [ ] `--output-dir` - Output directory
  - [ ] `--target-schema` - Target schema for mapping
  - [ ] `--llm` - LLM mode
  - [ ] `--doc-path` - Documentation for LLM
  - [ ] `--config` - Configuration file
  - [ ] `--dry-run` - Validate without execution
  - [ ] `--resume` - Resume from checkpoint
  - [ ] `--stages` - Run specific stages only
  - [ ] `--databricks-host` - Databricks workspace URL
  - [ ] `--databricks-token` - Databricks API token

### 6.8 Documentation
- [ ] Update README with pipeline usage
- [ ] Add pipeline tutorial
- [ ] Add configuration file examples
- [ ] Document all CLI options
- [ ] Add troubleshooting guide

### 6.9 Testing
- [ ] Integration test: full pipeline local files
- [ ] Integration test: full pipeline S3
- [ ] Integration test: full pipeline Unity Catalog Volumes
- [ ] Integration test: pipeline resume
- [ ] Integration test: dry run
- [ ] Performance test: end-to-end 1M records

---

## Cross-Cutting Concerns

### Documentation
- [ ] Update README.md with new features
- [ ] Update LLM.txt with new modules
- [ ] Add pipeline tutorial in docs/
- [ ] Add API documentation for new modules
- [ ] Update CHANGELOG.md for each phase

### Error Handling
- [ ] Define error types for each module
- [ ] Implement `std::error::Error` for all errors
- [ ] Add context to errors (file paths, record IDs)
- [ ] User-friendly error messages for CLI
- [ ] Structured errors for programmatic use

### Logging
- [ ] Add tracing spans for major operations
- [ ] Log progress at INFO level
- [ ] Log details at DEBUG level
- [ ] Add `--verbose` flag to CLI
- [ ] Structured logging for production use

### Performance
- [ ] Profile memory usage
- [ ] Optimize hot paths
- [ ] Use streaming where possible
- [ ] Parallelize CPU-bound operations
- [ ] Add benchmarks for critical paths

### Security
- [ ] Validate all user inputs
- [ ] Sanitize file paths
- [ ] Secure credential handling for S3
- [ ] No secrets in logs
- [ ] SQL injection prevention in queries

---

## Milestones

### Milestone 1: Workspace Restructure Complete
- [ ] Phase 0 complete
- [ ] All existing tests pass
- [ ] CLI works identically
- [ ] WASM package unchanged
- [ ] CI/CD updated

### Milestone 2: Data Ingestion MVP ✅ COMPLETE
- [x] Phase 1 complete (staging + ingestion)
- [x] Can ingest large datasets
- [x] Resume works
- [x] DuckDB supported (PostgreSQL deferred)

### Milestone 3: Schema Inference MVP ✅ COMPLETE
- [x] Phase 2 complete
- [x] Accurate type inference
- [x] Format detection works (18 formats)
- [x] Schema merging and grouping
- [x] JSON Schema export

### Milestone 4: Iceberg Integration MVP ✅
- [x] Phase 3 complete (2026-01-11)
- [x] Iceberg tables with time travel (IcebergTable, SnapshotInfo)
- [x] Schema-inferenced views (staging view create command)
- [x] Export CLI commands to Unity Catalog and S3 Tables (implementation scaffolded)

### Milestone 5: LLM Integration
- [ ] Phase 4 complete
- [ ] Ollama integration works
- [ ] Offline LLM works
- [ ] Schema quality improved

### Milestone 6: Full Pipeline
- [ ] Phase 5 complete (mapping)
- [ ] Phase 6 complete (integration)
- [ ] Single command runs full pipeline
- [ ] Documentation complete

### Milestone 7: Production Ready (v2.0.0)
- [ ] All phases complete
- [ ] 80%+ test coverage on new code
- [ ] Performance targets met
- [ ] Documentation complete
- [ ] Migration guide published

---

## Notes

- Each phase should be a separate PR for easier review
- Run full test suite before each merge
- Update CHANGELOG.md with each phase
- Tag releases at each milestone
- Gather user feedback after MVP milestones
