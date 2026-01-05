# Research: CLI Wrapper Implementation

**Date**: 2026-01-05
**Feature**: CLI Wrapper for Data Modelling SDK
**Purpose**: Research technical decisions and best practices for implementing a comprehensive CLI wrapper

## CLI Argument Parsing Library

### Decision: Use `clap` 4.x with derive API

**Rationale**:
- `clap` is the de facto standard for Rust CLI applications
- Version 4.x provides excellent derive macros for type-safe argument parsing
- Excellent error messages and help text generation
- Supports subcommands, which is perfect for `import` and `export` commands
- Well-maintained and widely used in the Rust ecosystem
- Good performance and minimal binary size impact

**Alternatives considered**:
- Manual argument parsing (current approach in `test_sql.rs`): Too verbose and error-prone for complex CLI
- `structopt`: Merged into `clap` 3.x, no longer maintained separately
- `argh`: Smaller but less feature-rich, doesn't support subcommands as elegantly

**Implementation approach**:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "data-modelling-cli")]
#[command(about = "CLI wrapper for Data Modelling SDK")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Import(ImportArgs),
    Export(ExportArgs),
}
```

## External Reference Resolution

### Decision: Use `reqwest` for HTTP/HTTPS URL fetching

**Rationale**:
- `reqwest` is already in SDK dependencies (feature-gated)
- Supports async/await for non-blocking network operations
- Handles HTTPS, redirects, and timeouts gracefully
- Can be reused from existing SDK dependencies

**Implementation approach**:
- Create `src/cli/reference.rs` module for reference resolution
- Support both local file paths (relative to source file directory) and HTTP/HTTPS URLs
- Implement timeout (10 seconds default) and proper error handling
- Cache resolved references to avoid duplicate fetches during single import

**Local file resolution**:
- Resolve relative paths from the source file's directory
- Support both relative (`./definitions.json`) and absolute paths
- Handle path traversal safely (prevent `../` attacks)

## JAR File Extraction

### Decision: Use `zip` crate for JAR file extraction

**Rationale**:
- JAR files are ZIP archives, so standard ZIP library works
- `zip` crate is well-maintained and widely used
- Supports reading ZIP files without full extraction to disk
- Can filter for `.proto` files during iteration

**Alternatives considered**:
- `jar` crate: Less common, JAR-specific but not necessary since JARs are ZIPs
- Manual ZIP parsing: Too complex and error-prone

**Implementation approach**:
- Open JAR file as ZIP archive
- Iterate entries, filter for `.proto` files
- Extract `.proto` file contents to memory
- Merge multiple proto files if needed (as documented in user requirements)

## Protobuf Descriptor Generation

### Decision: Use external `protoc` binary via `std::process::Command`

**Rationale**:
- `protoc` is the official Protocol Buffer compiler
- Required for generating binary descriptor files (`.pb`)
- CLI will check for `protoc` availability and provide helpful error messages if missing
- Use `--include_imports` flag to include all imported proto files

**Error handling**:
- Check `protoc` availability at command start
- Provide clear error message with installation instructions if missing
- Handle `protoc` compilation errors gracefully with clear messages

**Implementation approach**:
```rust
use std::process::Command;

fn check_protoc() -> Result<(), CliError> {
    Command::new("protoc")
        .arg("--version")
        .output()
        .map_err(|_| CliError::ProtocNotFound)?;
    Ok(())
}
```

## UUID Validation

### Decision: Use `uuid` crate's `Uuid::parse_str()` for validation

**Rationale**:
- `uuid` crate is already in SDK dependencies
- Provides robust UUID parsing and validation
- Supports both hyphenated and non-hyphenated formats
- Clear error messages for invalid UUIDs

**Implementation approach**:
- Parse UUID from `--uuid` flag argument
- Validate format before proceeding with import
- Report clear error if UUID format is invalid

## Output Formatting

### Decision: Support both compact and pretty output modes

**Rationale**:
- Compact mode for quick overview and scripting
- Pretty mode for detailed inspection and human readability
- Follows existing pattern from `test_sql.rs`

**Implementation approach**:
- Default to compact mode
- `--pretty` flag enables detailed output
- Format tables, columns, mappings, and errors consistently
- Use colored output (via `colored` or `termcolor` crate) for better readability (optional enhancement)

## Error Handling Strategy

### Decision: Create CLI-specific error types using `thiserror`

**Rationale**:
- Follows SDK constitution (structured error types)
- Enables clear, actionable error messages
- Allows error chaining for context

**Error categories**:
- File I/O errors (file not found, permission denied)
- Validation errors (invalid schema, invalid UUID)
- Network errors (timeout, connection failed)
- External tool errors (`protoc` not found, compilation failed)
- Import/export errors (from SDK modules)

**Implementation approach**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Invalid UUID format: {0}")]
    InvalidUuid(String),
    #[error("protoc not found. Install from https://protobuf.dev/downloads")]
    ProtocNotFound,
    // ... more variants
}
```

## Schema Validation

### Decision: Use existing SDK validation modules

**Rationale**:
- SDK already has validation modules (`src/validation/`)
- JSON Schema validation via `jsonschema` crate (feature-gated)
- XML validation for BPMN/DMN via XSD schemas
- Reuse existing validation logic to avoid duplication

**Implementation approach**:
- Call SDK validation functions before import
- Report validation errors clearly with file location and field paths
- Continue with import only if validation passes (or with warnings for non-critical issues)

## Multiple Table UUID Override

### Decision: Only allow UUID override for single-table imports

**Rationale**:
- Clarified in spec: UUID override only supported when importing a single table
- Prevents ambiguity about which table gets the UUID
- Simpler implementation and clearer user experience

**Implementation approach**:
- Check number of tables in import result
- If `--uuid` flag provided and multiple tables found, return error
- Error message: "UUID override is only supported when importing a single table. Found {count} tables."

## File Overwrite Handling

### Decision: Use `--force` flag for overwrite, prompt by default

**Rationale**:
- Prevents accidental data loss
- `--force` flag enables scripting and automation
- Follows common CLI patterns (git, cargo, etc.)

**Implementation approach**:
- Check if output file exists
- If exists and `--force` not set, prompt user for confirmation
- If `--force` set, overwrite without prompt
- In non-interactive mode (no TTY), require `--force` for overwrite

## Summary

All technical decisions have been made with clear rationale. The implementation will:
1. Use `clap` 4.x for argument parsing
2. Use `reqwest` for HTTP reference resolution
3. Use `zip` crate for JAR extraction
4. Use external `protoc` binary for descriptor generation
5. Use `uuid` crate for UUID validation
6. Support compact and pretty output modes
7. Create CLI-specific error types with `thiserror`
8. Reuse existing SDK validation modules
9. Enforce single-table UUID override restriction
10. Handle file overwrites with `--force` flag

No blocking technical unknowns remain. Ready to proceed to Phase 1 design.
