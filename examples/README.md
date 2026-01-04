# SQL Parser Test Tool

A simple CLI tool to manually test SQL parsing with different dialects.

## Quick Start

### Method 1: Using cargo run directly

```bash
# Parse SQL string
cargo run --example test_sql -- --dialect databricks --sql "CREATE TABLE test (id STRING);"

# Parse from file
cargo run --example test_sql -- --dialect databricks --file test.sql --pretty

# Parse from stdin
echo "CREATE TABLE test (id STRING);" | cargo run --example test_sql -- --dialect postgres
```

### Method 2: Using the convenience script

```bash
# Parse from file
./test_sql.sh databricks test.sql

# Parse from stdin
echo "CREATE TABLE test (id STRING);" | ./test_sql.sh databricks -
```

## Options

- `-d, --dialect <DIALECT>` - SQL dialect (postgres, mysql, sqlite, generic, databricks) [default: generic]
- `-s, --sql <SQL>` - SQL string to parse
- `-f, --file <FILE>` - Read SQL from file
- `-p, --pretty` - Pretty print column details (shows comments, primary keys, etc.)
- `-h, --help` - Show help message

## Examples

### Test Databricks SQL with IDENTIFIER()

```bash
cargo run --example test_sql -- --dialect databricks --sql \
  "CREATE TABLE IDENTIFIER(:catalog || '.schema.table') (id STRING, name STRING);" \
  --pretty
```

### Test multiline COMMENT clauses

```bash
cargo run --example test_sql -- --dialect databricks --file test.sql --pretty
```

### Test different dialects

```bash
# PostgreSQL
cargo run --example test_sql -- --dialect postgres --sql \
  "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));" --pretty

# MySQL
cargo run --example test_sql -- --dialect mysql --sql \
  "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100));" --pretty

# SQLite
cargo run --example test_sql -- --dialect sqlite --sql \
  "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);" --pretty
```

## Output Format

The tool shows:
- Dialect used
- SQL input
- Parse errors (if any)
- Tables requiring name resolution (if any)
- Parsed tables with column details
- Success/failure status

With `--pretty` flag, you get detailed column information including:
- Column names and data types
- COMMENT clauses
- Primary key indicators
- Nullable status

## Troubleshooting

If parsing fails, check:
1. The dialect matches your SQL syntax
2. The SQL is valid for that dialect
3. For Databricks: ensure IDENTIFIER() expressions are properly formatted
4. For multiline SQL: ensure quoted strings are properly closed
