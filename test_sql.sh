#!/bin/bash
# Convenience script to test SQL parsing
# Usage: ./test_sql.sh [dialect] [sql_file or - for stdin]

DIALECT="${1:-databricks}"
SQL_INPUT="${2:-}"

if [ -z "$SQL_INPUT" ] || [ "$SQL_INPUT" = "-" ]; then
    # Read from stdin
    cargo run --example test_sql -- --dialect "$DIALECT" --pretty
else
    # Read from file
    cargo run --example test_sql -- --dialect "$DIALECT" --file "$SQL_INPUT" --pretty
fi
