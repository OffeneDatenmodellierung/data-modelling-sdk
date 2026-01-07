//! Database backend abstraction for high-performance queries
//!
//! This module provides a database abstraction layer that supports:
//! - DuckDB: Embedded database for native CLI and in-memory for WASM
//! - PostgreSQL: For server deployments (CLI only for now)
//!
//! The database layer provides 10-100x performance improvements over
//! file-based operations for large workspaces by caching YAML data
//! in an indexed database format.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export implementations based on features
#[cfg(feature = "duckdb-backend")]
pub mod duckdb;

#[cfg(feature = "postgres-backend")]
pub mod postgres;

pub mod config;
pub mod schema;
pub mod sync;

#[cfg(feature = "duckdb-backend")]
pub use self::duckdb::DuckDBBackend;

#[cfg(feature = "postgres-backend")]
pub use self::postgres::PostgresBackend;

pub use config::DatabaseConfig;
pub use schema::DatabaseSchema;
pub use sync::{SyncEngine, SyncResult};

/// Error type for database operations
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Failed to connect to database
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Query execution failed
    #[error("Query failed: {0}")]
    QueryFailed(String),

    /// Sync operation failed
    #[error("Sync failed: {0}")]
    SyncFailed(String),

    /// Schema migration failed
    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Database not initialized
    #[error("Database not initialized. Run 'db init' first.")]
    NotInitialized,

    /// Workspace not found
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

/// Result type for database operations
pub type DatabaseResult<T> = Result<T, DatabaseError>;

/// Sync status for a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Workspace ID
    pub workspace_id: Uuid,
    /// Last sync timestamp
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of tables in the database
    pub table_count: usize,
    /// Number of columns in the database
    pub column_count: usize,
    /// Number of relationships in the database
    pub relationship_count: usize,
    /// Number of domains in the database
    pub domain_count: usize,
    /// Whether the database cache is stale (YAML files changed)
    pub is_stale: bool,
    /// Number of files that need to be synced
    pub pending_sync_count: usize,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            workspace_id: Uuid::nil(),
            last_sync_at: None,
            table_count: 0,
            column_count: 0,
            relationship_count: 0,
            domain_count: 0,
            is_stale: true,
            pending_sync_count: 0,
        }
    }
}

/// Query result row as a JSON value
pub type QueryRow = serde_json::Value;

/// Query result set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Column names
    pub columns: Vec<String>,
    /// Rows of data
    pub rows: Vec<QueryRow>,
    /// Number of rows affected (for INSERT/UPDATE/DELETE)
    pub rows_affected: Option<u64>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl QueryResult {
    /// Create a new query result
    pub fn new(columns: Vec<String>, rows: Vec<QueryRow>) -> Self {
        Self {
            columns,
            rows,
            rows_affected: None,
            execution_time_ms: 0,
        }
    }

    /// Create an empty result
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: None,
            execution_time_ms: 0,
        }
    }

    /// Get the number of rows
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Check if the result is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Database backend trait for query and sync operations
///
/// This trait defines the interface for database backends (DuckDB, PostgreSQL).
/// All operations are async to support both native and WASM environments.
#[async_trait(?Send)]
pub trait DatabaseBackend: Send + Sync {
    /// Initialize database schema (run migrations)
    ///
    /// Creates all required tables and indexes if they don't exist.
    async fn initialize(&self) -> DatabaseResult<()>;

    /// Execute a SQL query and return results
    ///
    /// # Arguments
    /// * `sql` - SQL query to execute
    ///
    /// # Returns
    /// Query result with columns and rows
    async fn execute_query(&self, sql: &str) -> DatabaseResult<QueryResult>;

    /// Execute a parameterized SQL query
    ///
    /// # Arguments
    /// * `sql` - SQL query with parameter placeholders ($1, $2, etc.)
    /// * `params` - Parameter values as JSON
    ///
    /// # Returns
    /// Query result with columns and rows
    async fn execute_query_params(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> DatabaseResult<QueryResult>;

    /// Sync tables from YAML data to database
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    /// * `tables` - Tables to sync
    ///
    /// # Returns
    /// Number of tables synced
    async fn sync_tables(
        &self,
        workspace_id: Uuid,
        tables: &[crate::models::Table],
    ) -> DatabaseResult<usize>;

    /// Sync domains from YAML data to database
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    /// * `domains` - Domains to sync
    ///
    /// # Returns
    /// Number of domains synced
    async fn sync_domains(
        &self,
        workspace_id: Uuid,
        domains: &[crate::models::Domain],
    ) -> DatabaseResult<usize>;

    /// Sync relationships from YAML data to database
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    /// * `relationships` - Relationships to sync
    ///
    /// # Returns
    /// Number of relationships synced
    async fn sync_relationships(
        &self,
        workspace_id: Uuid,
        relationships: &[crate::models::Relationship],
    ) -> DatabaseResult<usize>;

    /// Export tables from database back to Table models
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    ///
    /// # Returns
    /// Vector of Table models
    async fn export_tables(&self, workspace_id: Uuid) -> DatabaseResult<Vec<crate::models::Table>>;

    /// Export domains from database back to Domain models
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    ///
    /// # Returns
    /// Vector of Domain models
    async fn export_domains(
        &self,
        workspace_id: Uuid,
    ) -> DatabaseResult<Vec<crate::models::Domain>>;

    /// Export relationships from database back to Relationship models
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    ///
    /// # Returns
    /// Vector of Relationship models
    async fn export_relationships(
        &self,
        workspace_id: Uuid,
    ) -> DatabaseResult<Vec<crate::models::Relationship>>;

    /// Get sync status for a workspace
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    ///
    /// # Returns
    /// Sync status including counts and staleness
    async fn get_sync_status(&self, workspace_id: Uuid) -> DatabaseResult<SyncStatus>;

    /// Create or update a workspace record
    ///
    /// # Arguments
    /// * `workspace` - Workspace to upsert
    ///
    /// # Returns
    /// Unit on success
    async fn upsert_workspace(&self, workspace: &crate::models::Workspace) -> DatabaseResult<()>;

    /// Get workspace by ID
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    ///
    /// # Returns
    /// Optional Workspace if found
    async fn get_workspace(
        &self,
        workspace_id: Uuid,
    ) -> DatabaseResult<Option<crate::models::Workspace>>;

    /// Get workspace by name
    ///
    /// # Arguments
    /// * `name` - Workspace name
    ///
    /// # Returns
    /// Optional Workspace if found
    async fn get_workspace_by_name(
        &self,
        name: &str,
    ) -> DatabaseResult<Option<crate::models::Workspace>>;

    /// Delete a workspace and all its data
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    ///
    /// # Returns
    /// Unit on success
    async fn delete_workspace(&self, workspace_id: Uuid) -> DatabaseResult<()>;

    /// Record a file hash for change detection
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    /// * `file_path` - Relative file path
    /// * `hash` - SHA256 hash of file content
    ///
    /// # Returns
    /// Unit on success
    async fn record_file_hash(
        &self,
        workspace_id: Uuid,
        file_path: &str,
        hash: &str,
    ) -> DatabaseResult<()>;

    /// Get stored file hash
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace UUID
    /// * `file_path` - Relative file path
    ///
    /// # Returns
    /// Optional hash if recorded
    async fn get_file_hash(
        &self,
        workspace_id: Uuid,
        file_path: &str,
    ) -> DatabaseResult<Option<String>>;

    /// Check if database is healthy and accessible
    ///
    /// # Returns
    /// True if healthy
    async fn health_check(&self) -> DatabaseResult<bool>;

    /// Get the database backend type name
    ///
    /// # Returns
    /// Backend type string ("duckdb" or "postgres")
    fn backend_type(&self) -> &'static str;

    /// Close the database connection
    ///
    /// # Returns
    /// Unit on success
    async fn close(&self) -> DatabaseResult<()>;
}

/// Output format for query results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// ASCII table format (default)
    #[default]
    Table,
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "csv" => Ok(OutputFormat::Csv),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Csv => write!(f, "csv"),
        }
    }
}

/// Format query results for display
pub fn format_query_result(result: &QueryResult, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(&result.rows).unwrap_or_else(|_| "[]".to_string())
        }
        OutputFormat::Csv => format_as_csv(result),
        OutputFormat::Table => format_as_table(result),
    }
}

fn format_as_csv(result: &QueryResult) -> String {
    let mut output = String::new();

    // Header row
    output.push_str(&result.columns.join(","));
    output.push('\n');

    // Data rows
    for row in &result.rows {
        let values: Vec<String> = result
            .columns
            .iter()
            .map(|col| {
                let value = row.get(col).unwrap_or(&serde_json::Value::Null);
                match value {
                    serde_json::Value::String(s) => {
                        // Escape quotes and wrap in quotes if contains comma
                        if s.contains(',') || s.contains('"') || s.contains('\n') {
                            format!("\"{}\"", s.replace('"', "\"\""))
                        } else {
                            s.clone()
                        }
                    }
                    serde_json::Value::Null => String::new(),
                    other => other.to_string(),
                }
            })
            .collect();
        output.push_str(&values.join(","));
        output.push('\n');
    }

    output
}

fn format_as_table(result: &QueryResult) -> String {
    if result.is_empty() {
        return "(0 rows)".to_string();
    }

    // Calculate column widths
    let mut widths: Vec<usize> = result.columns.iter().map(|c| c.len()).collect();

    for row in &result.rows {
        for (i, col) in result.columns.iter().enumerate() {
            let value = row.get(col).unwrap_or(&serde_json::Value::Null);
            let len = match value {
                serde_json::Value::String(s) => s.len(),
                serde_json::Value::Null => 4, // "null"
                other => other.to_string().len(),
            };
            widths[i] = widths[i].max(len);
        }
    }

    let mut output = String::new();

    // Header
    let header: Vec<String> = result
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:width$}", c, width = widths[i]))
        .collect();
    output.push_str(&header.join(" | "));
    output.push('\n');

    // Separator
    let separator: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    output.push_str(&separator.join("-+-"));
    output.push('\n');

    // Data rows
    for row in &result.rows {
        let values: Vec<String> = result
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let value = row.get(col).unwrap_or(&serde_json::Value::Null);
                let s = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "null".to_string(),
                    other => other.to_string(),
                };
                format!("{:width$}", s, width = widths[i])
            })
            .collect();
        output.push_str(&values.join(" | "));
        output.push('\n');
    }

    output.push_str(&format!("({} rows)", result.row_count()));

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(
            OutputFormat::from_str("table").unwrap(),
            OutputFormat::Table
        );
        assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("csv").unwrap(), OutputFormat::Csv);
        assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);
        assert!(OutputFormat::from_str("unknown").is_err());
    }

    #[test]
    fn test_query_result_empty() {
        let result = QueryResult::empty();
        assert!(result.is_empty());
        assert_eq!(result.row_count(), 0);
    }

    #[test]
    fn test_format_as_table() {
        let result = QueryResult::new(
            vec!["name".to_string(), "count".to_string()],
            vec![
                serde_json::json!({"name": "users", "count": 10}),
                serde_json::json!({"name": "orders", "count": 100}),
            ],
        );

        let output = format_as_table(&result);
        assert!(output.contains("name"));
        assert!(output.contains("count"));
        assert!(output.contains("users"));
        assert!(output.contains("(2 rows)"));
    }

    #[test]
    fn test_format_as_csv() {
        let result = QueryResult::new(
            vec!["name".to_string(), "description".to_string()],
            vec![
                serde_json::json!({"name": "test", "description": "simple"}),
                serde_json::json!({"name": "complex", "description": "has, comma"}),
            ],
        );

        let output = format_as_csv(&result);
        assert!(output.contains("name,description"));
        assert!(output.contains("test,simple"));
        assert!(output.contains("\"has, comma\"")); // Quoted due to comma
    }

    #[test]
    fn test_sync_status_default() {
        let status = SyncStatus::default();
        assert!(status.is_stale);
        assert_eq!(status.table_count, 0);
        assert!(status.last_sync_at.is_none());
    }
}
