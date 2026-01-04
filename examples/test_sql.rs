//! Simple CLI tool to test SQL parsing
//!
//! Usage:
//!   cargo run --example test_sql -- --dialect databricks --sql "CREATE TABLE test (id STRING);"
//!   cargo run --example test_sql -- --dialect postgres --file test.sql
//!
//! Or read from stdin:
//!   echo "CREATE TABLE test (id STRING);" | cargo run --example test_sql -- --dialect databricks

use data_modelling_sdk::import::sql::SQLImporter;
use std::io::{self, Read};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut dialect = "generic".to_string();
    let mut sql: Option<String> = None;
    let mut file: Option<PathBuf> = None;
    let mut pretty = false;

    // Simple argument parsing
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dialect" | "-d" => {
                if i + 1 < args.len() {
                    dialect = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --dialect requires a value");
                    print_usage();
                    std::process::exit(1);
                }
            }
            "--sql" | "-s" => {
                if i + 1 < args.len() {
                    sql = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --sql requires a value");
                    print_usage();
                    std::process::exit(1);
                }
            }
            "--file" | "-f" => {
                if i + 1 < args.len() {
                    file = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Error: --file requires a value");
                    print_usage();
                    std::process::exit(1);
                }
            }
            "--pretty" | "-p" => {
                pretty = true;
                i += 1;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
    }

    // Get SQL from file, argument, or stdin
    let sql_content = if let Some(file_path) = file {
        std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file {}: {}", file_path.display(), e))?
    } else if let Some(sql_str) = sql {
        sql_str
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    if sql_content.trim().is_empty() {
        eprintln!("Error: No SQL provided");
        print_usage();
        std::process::exit(1);
    }

    // Parse SQL
    println!("Dialect: {}", dialect);
    println!("SQL:\n{}\n", sql_content);
    println!("{}", "=".repeat(80));

    let importer = SQLImporter::new(&dialect);
    match importer.parse(&sql_content) {
        Ok(result) => {
            if !result.errors.is_empty() {
                println!("\n⚠️  Parse Errors:");
                for error in &result.errors {
                    println!("  - {:?}", error);
                }
            }

            if !result.tables_requiring_name.is_empty() {
                println!("\n⚠️  Tables Requiring Name Resolution:");
                for table in &result.tables_requiring_name {
                    println!("  - Table index: {}", table.table_index);
                    if let Some(name) = &table.suggested_name {
                        println!("    Suggested name: {}", name);
                    }
                }
            }

            println!("\n✅ Parsed {} table(s):", result.tables.len());
            for (idx, table) in result.tables.iter().enumerate() {
                println!("\nTable {}:", idx + 1);
                println!("  Name: {:?}", table.name);
                println!("  Columns: {}", table.columns.len());

                if pretty {
                    println!("  Column Details:");
                    for col in &table.columns {
                        println!("    - {} ({})", col.name, col.data_type);
                        if let Some(desc) = &col.description {
                            println!("      Comment: {}", desc);
                        }
                        if col.primary_key {
                            println!("      Primary Key: true");
                        }
                        if !col.nullable {
                            println!("      Nullable: false");
                        }
                    }
                } else {
                    // Compact output
                    let col_names: Vec<String> = table
                        .columns
                        .iter()
                        .map(|c| format!("{}:{}", c.name, c.data_type))
                        .collect();
                    println!("  Columns: {}", col_names.join(", "));
                }
            }

            if result.errors.is_empty() && result.tables_requiring_name.is_empty() {
                println!("\n✅ All checks passed!");
            }
        }
        Err(e) => {
            eprintln!("\n❌ Parse failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_usage() {
    println!(
        r#"
SQL Parser Test Tool

Usage:
  cargo run --example test_sql -- [OPTIONS]

Options:
  -d, --dialect <DIALECT>    SQL dialect (postgres, mysql, sqlite, generic, databricks) [default: generic]
  -s, --sql <SQL>            SQL string to parse
  -f, --file <FILE>          Read SQL from file
  -p, --pretty               Pretty print column details
  -h, --help                 Show this help message

Examples:
  # Parse SQL string
  cargo run --example test_sql -- --dialect databricks --sql "CREATE TABLE test (id STRING COMMENT 'test');"

  # Parse from file
  cargo run --example test_sql -- --dialect databricks --file test.sql

  # Parse from stdin
  echo "CREATE TABLE test (id STRING);" | cargo run --example test_sql -- --dialect postgres

  # Pretty output
  cargo run --example test_sql -- --dialect databricks --sql "CREATE TABLE test (id STRING, name STRING);" --pretty

Supported Dialects:
  - postgres / postgresql
  - mysql
  - sqlite
  - generic (default)
  - databricks
"#
    );
}
