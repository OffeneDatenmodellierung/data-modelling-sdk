//! SQL query CLI command
//!
//! Provides a command to execute SQL queries against the workspace database.

use std::path::PathBuf;

use crate::error::CliError;

#[cfg(feature = "duckdb-backend")]
use data_modelling_core::database::{
    DatabaseBackend, OutputFormat,
    config::{DatabaseBackendType, DatabaseConfig},
    duckdb::DuckDBBackend,
    format_query_result,
};

/// Query command arguments
#[derive(Debug, Clone)]
pub struct QueryArgs {
    /// SQL query to execute
    pub sql: String,
    /// Workspace path
    pub workspace: PathBuf,
    /// Output format
    pub format: String,
}

/// Execute a SQL query against the workspace database
#[cfg(feature = "duckdb-backend")]
pub fn handle_query(args: &QueryArgs) -> Result<(), CliError> {
    let workspace_path = &args.workspace;

    // Load config
    let config = DatabaseConfig::load(workspace_path)
        .map_err(|e| CliError::IoError(format!("Failed to load config: {}", e)))?;

    if !DatabaseConfig::is_initialized(workspace_path) {
        return Err(CliError::InvalidArgument(
            "Database not initialized. Run 'db init' first.".to_string(),
        ));
    }

    // Parse output format
    let output_format: OutputFormat = args
        .format
        .parse()
        .map_err(|e: String| CliError::InvalidArgument(e))?;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::IoError(format!("Failed to create runtime: {}", e)))?;

    rt.block_on(async {
        match config.database.backend {
            DatabaseBackendType::DuckDB => {
                let db_path = config.get_duckdb_path(workspace_path);
                let backend = DuckDBBackend::new(&db_path)
                    .map_err(|e| CliError::IoError(format!("Failed to open database: {}", e)))?;

                let result = backend
                    .execute_query(&args.sql)
                    .await
                    .map_err(|e| CliError::IoError(format!("Query failed: {}", e)))?;

                // Format and print result
                let output = format_query_result(&result, output_format);
                println!("{}", output);

                // Print execution time for non-JSON formats
                if output_format != OutputFormat::Json {
                    eprintln!("\nExecution time: {}ms", result.execution_time_ms);
                }
            }
            DatabaseBackendType::Postgres => {
                #[cfg(feature = "postgres-backend")]
                {
                    use data_modelling_core::database::postgres::PostgresBackend;

                    let conn_str = config.get_postgres_connection_string().ok_or_else(|| {
                        CliError::InvalidArgument(
                            "PostgreSQL connection string not configured".to_string(),
                        )
                    })?;

                    let backend = PostgresBackend::new(conn_str)
                        .await
                        .map_err(|e| CliError::IoError(format!("Failed to connect: {}", e)))?;

                    let result = backend
                        .execute_query(&args.sql)
                        .await
                        .map_err(|e| CliError::IoError(format!("Query failed: {}", e)))?;

                    let output = format_query_result(&result, output_format);
                    println!("{}", output);

                    if output_format != OutputFormat::Json {
                        eprintln!("\nExecution time: {}ms", result.execution_time_ms);
                    }
                }
                #[cfg(not(feature = "postgres-backend"))]
                {
                    return Err(CliError::InvalidArgument(
                        "PostgreSQL backend not enabled".to_string(),
                    ));
                }
            }
        }

        Ok(())
    })
}

#[cfg(not(feature = "duckdb-backend"))]
pub fn handle_query(_args: &QueryArgs) -> Result<(), CliError> {
    Err(CliError::InvalidArgument(
        "Database support not enabled. Build with --features duckdb-backend".to_string(),
    ))
}

/// Common SQL queries as helper functions
#[cfg(feature = "duckdb-backend")]
pub mod queries {
    /// List all tables in the workspace
    pub const LIST_TABLES: &str = r#"
SELECT
    t.name,
    t.database_type,
    t.schema_name,
    COUNT(c.name) as column_count,
    t.owner
FROM tables t
LEFT JOIN columns c ON t.id = c.table_id
GROUP BY t.id, t.name, t.database_type, t.schema_name, t.owner
ORDER BY t.name
"#;

    /// List all relationships
    pub const LIST_RELATIONSHIPS: &str = r#"
SELECT
    r.id,
    s.name as source_table,
    t.name as target_table,
    r.cardinality,
    r.relationship_type
FROM relationships r
JOIN tables s ON r.source_table_id = s.id
JOIN tables t ON r.target_table_id = t.id
ORDER BY s.name, t.name
"#;

    /// Find tables by name pattern
    pub fn find_tables_by_name(pattern: &str) -> String {
        format!(
            "SELECT name, database_type, schema_name, owner FROM tables WHERE name LIKE '%{}%' ORDER BY name",
            pattern.replace('\'', "''")
        )
    }

    /// Get column details for a table
    pub fn get_table_columns(table_name: &str) -> String {
        format!(
            r#"
SELECT
    c.name,
    c.data_type,
    c.primary_key,
    c.nullable,
    c.description
FROM columns c
JOIN tables t ON c.table_id = t.id
WHERE t.name = '{}'
ORDER BY c.column_order
"#,
            table_name.replace('\'', "''")
        )
    }

    /// Count entities in the database
    pub const COUNT_ENTITIES: &str = r#"
SELECT
    (SELECT COUNT(*) FROM workspaces) as workspaces,
    (SELECT COUNT(*) FROM domains) as domains,
    (SELECT COUNT(*) FROM tables) as tables,
    (SELECT COUNT(*) FROM columns) as columns,
    (SELECT COUNT(*) FROM relationships) as relationships
"#;

    /// Find tables by owner
    pub fn find_tables_by_owner(owner: &str) -> String {
        format!(
            "SELECT name, database_type, schema_name FROM tables WHERE owner = '{}' ORDER BY name",
            owner.replace('\'', "''")
        )
    }

    /// Find tables by infrastructure type
    pub fn find_tables_by_infrastructure(infra_type: &str) -> String {
        format!(
            "SELECT name, owner, schema_name FROM tables WHERE infrastructure_type = '{}' ORDER BY name",
            infra_type.replace('\'', "''")
        )
    }
}
