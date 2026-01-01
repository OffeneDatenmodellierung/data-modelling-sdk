# Security Policy

## Security Measures

The Data Modelling SDK implements several security measures to protect against common vulnerabilities.

### 1. Path Traversal Protection

The `FileSystemStorageBackend` prevents directory traversal attacks:

- Paths containing `..` are rejected with `StorageError::PermissionDenied`
- All resolved paths are verified to remain within the configured base directory
- Symlinks are resolved and validated to prevent escape via symbolic links

```rust
// This will fail with PermissionDenied
backend.read_file("../etc/passwd").await; // Error!
backend.read_file("/foo/../../etc/passwd").await; // Error!
```

### 2. Input Validation

All imported data is validated before processing:

- **Table names**: Must start with a letter or underscore, contain only alphanumeric characters, hyphens, and underscores
- **Column names**: Same as table names, plus dots for nested columns
- **Data types**: Checked for SQL injection patterns
- **Lengths**: All inputs have maximum length limits (255 chars for identifiers)

```rust
use data_modelling_sdk::validation::input::validate_table_name;

// Valid
assert!(validate_table_name("users").is_ok());

// Invalid - contains SQL injection attempt
assert!(validate_table_name("users; DROP TABLE users").is_err());
```

### 3. SQL Identifier Escaping

When exporting to SQL, all identifiers are properly quoted and escaped:

- PostgreSQL: `"identifier"` with internal `"` doubled
- MySQL: `` `identifier` `` with internal `` ` `` doubled
- SQL Server: `[identifier]` with internal `]` doubled

```rust
use data_modelling_sdk::validation::input::sanitize_sql_identifier;

// Returns: "user""table"
let safe = sanitize_sql_identifier("user\"table", "postgres");
```

### 4. Domain Validation (API Backend)

Domain parameters in API requests are validated:

- Maximum 100 characters
- Only alphanumeric, hyphens, and underscores allowed
- Cannot start with a period
- URL-encoded before use in API paths

### 5. Reserved Word Detection

SQL reserved words are detected and flagged:

- `SELECT`, `TABLE`, `INSERT`, `UPDATE`, `DELETE`, etc.
- Validation warnings are logged but don't block import
- Use `validate_table_name()` or `validate_column_name()` to check

### 6. Safe Error Handling

- No `unwrap()` calls on user-provided data in non-test code
- All errors are properly propagated with context
- Sensitive information is not leaked in error messages

## Reporting Vulnerabilities

If you discover a security vulnerability, please:

1. **Do not** open a public GitHub issue
2. Email the maintainers directly at [security contact TBD]
3. Include a detailed description and steps to reproduce
4. Allow reasonable time for a fix before public disclosure

## Secure Usage Guidelines

### For SDK Users

1. **Always validate user input** before passing to SDK functions
2. **Use the provided validation functions** (`validate_table_name`, etc.)
3. **Handle errors gracefully** - don't expose internal errors to end users
4. **Keep the SDK updated** to receive security patches

### For Import Operations

```rust
use data_modelling_sdk::import::sql::SQLImporter;
use data_modelling_sdk::validation::input::validate_table_name;

let importer = SQLImporter::new("postgres");
let result = importer.parse(user_provided_sql)?;

// Additional validation on imported data
for table in &result.tables {
    if let Some(name) = &table.name {
        if let Err(e) = validate_table_name(name) {
            log::warn!("Potentially unsafe table name: {}", e);
        }
    }
}
```

### For Export Operations

```rust
use data_modelling_sdk::export::sql::SQLExporter;

// The exporter automatically quotes and escapes identifiers
let exporter = SQLExporter;
let result = exporter.export(&tables, Some("postgres"))?;
// result.content contains safe SQL with properly escaped identifiers
```

### For File System Operations

```rust
use data_modelling_sdk::storage::filesystem::FileSystemStorageBackend;

// Always use a restricted base path
let backend = FileSystemStorageBackend::new("/app/data");

// User paths are automatically validated
backend.read_file(user_path).await?; // Safe - validates against base path
```

## Changelog

### v0.3.0
- Added path traversal protection to `FileSystemStorageBackend`
- Added domain validation to `ApiStorageBackend`
- Added input validation module with sanitizers
- Replaced unsafe `unwrap()` calls with proper error handling
- Added SQL identifier quoting with escape handling
- Added Protobuf reserved word handling
