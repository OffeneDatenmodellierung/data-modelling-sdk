//! DuckDB database backend implementation
//!
//! Provides an embedded database backend using DuckDB for high-performance
//! queries on data model workspaces.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

use super::schema::{DatabaseSchema, SCHEMA_VERSION};
use super::{DatabaseBackend, DatabaseError, DatabaseResult, QueryResult, SyncStatus};
use crate::models::{Domain, Relationship, Table, Workspace};

/// DuckDB database backend
///
/// Provides an embedded SQL database for caching and querying workspace data.
/// Supports both file-based persistence and in-memory mode.
pub struct DuckDBBackend {
    /// Path to the database file (None for in-memory)
    db_path: Option<PathBuf>,
    /// DuckDB connection (wrapped in Mutex for thread safety)
    connection: Mutex<duckdb::Connection>,
}

impl DuckDBBackend {
    /// Create a new DuckDB backend with a file-based database
    ///
    /// # Arguments
    /// * `db_path` - Path to the DuckDB database file
    ///
    /// # Returns
    /// A new DuckDB backend instance
    pub fn new(db_path: impl AsRef<Path>) -> DatabaseResult<Self> {
        let path = db_path.as_ref().to_path_buf();
        let connection = duckdb::Connection::open(&path).map_err(|e| {
            DatabaseError::ConnectionFailed(format!("Failed to open DuckDB: {}", e))
        })?;

        Ok(Self {
            db_path: Some(path),
            connection: Mutex::new(connection),
        })
    }

    /// Create an in-memory DuckDB backend
    ///
    /// Useful for testing or temporary workspaces where persistence is not needed.
    pub fn in_memory() -> DatabaseResult<Self> {
        let connection = duckdb::Connection::open_in_memory().map_err(|e| {
            DatabaseError::ConnectionFailed(format!("Failed to create in-memory DuckDB: {}", e))
        })?;

        Ok(Self {
            db_path: None,
            connection: Mutex::new(connection),
        })
    }

    /// Get the database file path (None for in-memory)
    pub fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    /// Check if this is an in-memory database
    pub fn is_in_memory(&self) -> bool {
        self.db_path.is_none()
    }

    /// Execute a SQL statement that doesn't return rows
    fn execute(&self, sql: &str) -> DatabaseResult<usize> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        conn.execute(sql, [])
            .map_err(|e| DatabaseError::QueryFailed(format!("Execute failed: {}", e)))
    }

    /// Execute multiple SQL statements
    fn execute_batch(&self, sql: &str) -> DatabaseResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        conn.execute_batch(sql)
            .map_err(|e| DatabaseError::QueryFailed(format!("Batch execute failed: {}", e)))
    }

    /// Convert a DuckDB row to a JSON value
    fn row_to_json(row: &duckdb::Row, columns: &[String]) -> DatabaseResult<serde_json::Value> {
        let mut map = serde_json::Map::new();

        for (i, col_name) in columns.iter().enumerate() {
            let value: serde_json::Value = match row.get_ref(i) {
                Ok(value_ref) => Self::value_ref_to_json(value_ref),
                Err(_) => serde_json::Value::Null,
            };
            map.insert(col_name.clone(), value);
        }

        Ok(serde_json::Value::Object(map))
    }

    /// Convert a DuckDB ValueRef to a JSON value
    fn value_ref_to_json(value: duckdb::types::ValueRef) -> serde_json::Value {
        use duckdb::types::ValueRef;

        match value {
            ValueRef::Null => serde_json::Value::Null,
            ValueRef::Boolean(b) => serde_json::Value::Bool(b),
            ValueRef::TinyInt(i) => serde_json::Value::Number(i.into()),
            ValueRef::SmallInt(i) => serde_json::Value::Number(i.into()),
            ValueRef::Int(i) => serde_json::Value::Number(i.into()),
            ValueRef::BigInt(i) => serde_json::Value::Number(i.into()),
            ValueRef::HugeInt(i) => {
                // HugeInt is i128, which may not fit in JSON number
                serde_json::Value::String(i.to_string())
            }
            ValueRef::UTinyInt(i) => serde_json::Value::Number(i.into()),
            ValueRef::USmallInt(i) => serde_json::Value::Number(i.into()),
            ValueRef::UInt(i) => serde_json::Value::Number(i.into()),
            ValueRef::UBigInt(i) => serde_json::Value::Number(i.into()),
            ValueRef::Float(f) => serde_json::Number::from_f64(f as f64)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            ValueRef::Double(f) => serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).into_owned().into(),
            ValueRef::Blob(bytes) => {
                // Encode blob as base64
                use base64::Engine;
                serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(bytes))
            }
            ValueRef::Timestamp(_, _) => {
                // Timestamp handling - convert to string representation
                serde_json::Value::String(format!("{:?}", value))
            }
            ValueRef::Date32(_) => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::Time64(_, _) => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::Interval { .. } => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::List(_, _) => {
                // List/Array - serialize as string for now
                serde_json::Value::String(format!("{:?}", value))
            }
            ValueRef::Enum(_, _) => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::Struct(_, _) => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::Map(_, _) => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::Union(_, _) => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::Array(_, _) => serde_json::Value::String(format!("{:?}", value)),
            ValueRef::Decimal(d) => serde_json::Value::String(d.to_string()),
        }
    }

    /// Get column count from a row
    #[allow(dead_code)]
    fn get_column_count(stmt: &duckdb::Statement) -> usize {
        stmt.column_count()
    }

    /// Get column names from a statement
    #[allow(dead_code)]
    fn get_column_names(stmt: &duckdb::Statement) -> Vec<String> {
        (0..stmt.column_count())
            .map(|i| {
                stmt.column_name(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "?".to_string())
            })
            .collect()
    }
}

#[async_trait(?Send)]
impl DatabaseBackend for DuckDBBackend {
    async fn initialize(&self) -> DatabaseResult<()> {
        // Create tables
        self.execute_batch(DatabaseSchema::create_tables_sql())?;

        // Create indexes
        self.execute_batch(DatabaseSchema::create_indexes_sql())?;

        // Record schema version
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?) ON CONFLICT (version) DO NOTHING",
            [SCHEMA_VERSION],
        )
        .map_err(|e| {
            DatabaseError::MigrationFailed(format!("Failed to record schema version: {}", e))
        })?;

        Ok(())
    }

    async fn execute_query(&self, sql: &str) -> DatabaseResult<QueryResult> {
        let start = std::time::Instant::now();

        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DatabaseError::QueryFailed(format!("Prepare failed: {}", e)))?;

        // In DuckDB 1.4+, we need to execute the query first, then get columns
        let mut result_rows = stmt
            .query([])
            .map_err(|e| DatabaseError::QueryFailed(format!("Query failed: {}", e)))?;

        // Get column names from the result set
        let column_count = result_rows.as_ref().map(|r| r.column_count()).unwrap_or(0);
        let columns: Vec<String> = (0..column_count)
            .map(|i| {
                result_rows
                    .as_ref()
                    .and_then(|r| r.column_name(i).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("col{}", i))
            })
            .collect();

        let mut rows = Vec::new();
        while let Some(row) = result_rows
            .next()
            .map_err(|e| DatabaseError::QueryFailed(format!("Row fetch error: {}", e)))?
        {
            rows.push(Self::row_to_json(row, &columns).unwrap_or(serde_json::Value::Null));
        }

        Ok(QueryResult {
            columns,
            rows,
            rows_affected: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn execute_query_params(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> DatabaseResult<QueryResult> {
        let start = std::time::Instant::now();

        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DatabaseError::QueryFailed(format!("Prepare failed: {}", e)))?;

        // Convert JSON params to DuckDB params
        // For simplicity, we'll use string representation
        let string_params: Vec<String> = params
            .iter()
            .map(|p| match p {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            })
            .collect();

        let param_refs: Vec<&dyn duckdb::ToSql> = string_params
            .iter()
            .map(|s| s as &dyn duckdb::ToSql)
            .collect();

        // In DuckDB 1.4+, we need to execute the query first, then get columns
        let mut result_rows = stmt
            .query(param_refs.as_slice())
            .map_err(|e| DatabaseError::QueryFailed(format!("Query failed: {}", e)))?;

        // Get column names from the result set
        let column_count = result_rows.as_ref().map(|r| r.column_count()).unwrap_or(0);
        let columns: Vec<String> = (0..column_count)
            .map(|i| {
                result_rows
                    .as_ref()
                    .and_then(|r| r.column_name(i).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("col{}", i))
            })
            .collect();

        let mut rows = Vec::new();
        while let Some(row) = result_rows
            .next()
            .map_err(|e| DatabaseError::QueryFailed(format!("Row fetch error: {}", e)))?
        {
            rows.push(Self::row_to_json(row, &columns).unwrap_or(serde_json::Value::Null));
        }

        Ok(QueryResult {
            columns,
            rows,
            rows_affected: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn sync_tables(&self, workspace_id: Uuid, tables: &[Table]) -> DatabaseResult<usize> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        let mut count = 0;

        for table in tables {
            // Insert/update table
            let now = chrono::Utc::now();

            conn.execute(
                r#"
                INSERT INTO tables (
                    id, workspace_id, domain_id, name, database_type, catalog_name, schema_name,
                    owner, infrastructure_type, notes, medallion_layers, scd_pattern,
                    data_vault_classification, modeling_level, position_x, position_y,
                    yaml_file_path, yaml_hash, sla, contact_details, quality, tags,
                    custom_properties, created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    database_type = EXCLUDED.database_type,
                    catalog_name = EXCLUDED.catalog_name,
                    schema_name = EXCLUDED.schema_name,
                    owner = EXCLUDED.owner,
                    infrastructure_type = EXCLUDED.infrastructure_type,
                    notes = EXCLUDED.notes,
                    medallion_layers = EXCLUDED.medallion_layers,
                    scd_pattern = EXCLUDED.scd_pattern,
                    data_vault_classification = EXCLUDED.data_vault_classification,
                    modeling_level = EXCLUDED.modeling_level,
                    position_x = EXCLUDED.position_x,
                    position_y = EXCLUDED.position_y,
                    yaml_file_path = EXCLUDED.yaml_file_path,
                    yaml_hash = EXCLUDED.yaml_hash,
                    sla = EXCLUDED.sla,
                    contact_details = EXCLUDED.contact_details,
                    quality = EXCLUDED.quality,
                    tags = EXCLUDED.tags,
                    custom_properties = EXCLUDED.custom_properties,
                    updated_at = EXCLUDED.updated_at
                "#,
                duckdb::params![
                    table.id.to_string(),
                    workspace_id.to_string(),
                    Option::<String>::None, // domain_id - TODO: map from table
                    &table.name,
                    table.database_type.as_ref().map(|d| format!("{:?}", d)),
                    &table.catalog_name,
                    &table.schema_name,
                    &table.owner,
                    table
                        .infrastructure_type
                        .as_ref()
                        .map(|i| format!("{:?}", i)),
                    &table.notes,
                    if table.medallion_layers.is_empty() {
                        None
                    } else {
                        serde_json::to_string(&table.medallion_layers).ok()
                    },
                    table.scd_pattern.as_ref().map(|s| format!("{:?}", s)),
                    table
                        .data_vault_classification
                        .as_ref()
                        .map(|d| format!("{:?}", d)),
                    table.modeling_level.as_ref().map(|m| format!("{:?}", m)),
                    table.position.as_ref().map(|p| p.x),
                    table.position.as_ref().map(|p| p.y),
                    Option::<String>::None, // yaml_file_path
                    Option::<String>::None, // yaml_hash
                    if table.sla.is_some() {
                        serde_json::to_string(&table.sla).ok()
                    } else {
                        None
                    },
                    if table.contact_details.is_some() {
                        serde_json::to_string(&table.contact_details).ok()
                    } else {
                        None
                    },
                    if table.quality.is_empty() {
                        None
                    } else {
                        serde_json::to_string(&table.quality).ok()
                    },
                    if table.tags.is_empty() {
                        None
                    } else {
                        serde_json::to_string(&table.tags).ok()
                    },
                    Option::<String>::None, // custom_properties
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                ],
            )
            .map_err(|e| {
                DatabaseError::SyncFailed(format!("Failed to sync table {}: {}", table.name, e))
            })?;

            // Delete existing columns for this table and re-insert
            conn.execute(
                "DELETE FROM columns WHERE table_id = ?",
                [table.id.to_string()],
            )
            .map_err(|e| DatabaseError::SyncFailed(format!("Failed to delete columns: {}", e)))?;

            // Insert columns
            for (order, column) in table.columns.iter().enumerate() {
                conn.execute(
                    r#"
                    INSERT INTO columns (
                        id, table_id, name, business_name, description, data_type, physical_type,
                        physical_name, primary_key, primary_key_position, is_unique, nullable,
                        partitioned, partition_key_position, clustered, classification,
                        critical_data_element, encrypted_name, transform_source_objects,
                        transform_logic, transform_description, examples, default_value,
                        relationships, authoritative_definitions, quality, enum_values, tags,
                        custom_properties, logical_type_options, column_order, nested_data
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    duckdb::params![
                        column.id.as_deref(),
                        table.id.to_string(),
                        &column.name,
                        &column.business_name,
                        &column.description,
                        &column.data_type,
                        &column.physical_type,
                        &column.physical_name,
                        column.primary_key,
                        column.primary_key_position,
                        column.unique,
                        column.nullable,
                        column.partitioned,
                        column.partition_key_position,
                        column.clustered,
                        &column.classification,
                        column.critical_data_element,
                        &column.encrypted_name,
                        if column.transform_source_objects.is_empty() { None } else { serde_json::to_string(&column.transform_source_objects).ok() },
                        &column.transform_logic,
                        &column.transform_description,
                        if column.examples.is_empty() { None } else { serde_json::to_string(&column.examples).ok() },
                        column.default_value.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default()),
                        if column.relationships.is_empty() { None } else { serde_json::to_string(&column.relationships).ok() },
                        if column.authoritative_definitions.is_empty() { None } else { serde_json::to_string(&column.authoritative_definitions).ok() },
                        if column.quality.is_empty() { None } else { serde_json::to_string(&column.quality).ok() },
                        if column.enum_values.is_empty() { None } else { serde_json::to_string(&column.enum_values).ok() },
                        if column.tags.is_empty() { None } else { serde_json::to_string(&column.tags).ok() },
                        if column.custom_properties.is_empty() { None } else { serde_json::to_string(&column.custom_properties).ok() },
                        column.logical_type_options.as_ref().map(|l| serde_json::to_string(l).unwrap_or_default()),
                        order as i32,
                        &column.nested_data,
                    ],
                ).map_err(|e| DatabaseError::SyncFailed(format!("Failed to sync column {}.{}: {}", table.name, column.name, e)))?;
            }

            count += 1;
        }

        Ok(count)
    }

    async fn sync_domains(&self, workspace_id: Uuid, domains: &[Domain]) -> DatabaseResult<usize> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        let mut count = 0;
        let now = chrono::Utc::now();

        for domain in domains {
            conn.execute(
                r#"
                INSERT INTO domains (id, workspace_id, name, description, created_at, updated_at, yaml_hash, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    description = EXCLUDED.description,
                    updated_at = EXCLUDED.updated_at,
                    yaml_hash = EXCLUDED.yaml_hash,
                    metadata = EXCLUDED.metadata
                "#,
                duckdb::params![
                    domain.id.to_string(),
                    workspace_id.to_string(),
                    &domain.name,
                    &domain.description,
                    domain.created_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| now.to_rfc3339()),
                    now.to_rfc3339(),
                    Option::<String>::None,
                    Option::<String>::None,
                ],
            ).map_err(|e| DatabaseError::SyncFailed(format!("Failed to sync domain {}: {}", domain.name, e)))?;

            count += 1;
        }

        Ok(count)
    }

    async fn sync_relationships(
        &self,
        workspace_id: Uuid,
        relationships: &[Relationship],
    ) -> DatabaseResult<usize> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        let mut count = 0;
        let now = chrono::Utc::now();

        for rel in relationships {
            conn.execute(
                r#"
                INSERT INTO relationships (
                    id, workspace_id, source_table_id, target_table_id, cardinality,
                    source_optional, target_optional, relationship_type, notes, owner,
                    infrastructure_type, etl_job_name, etl_job_frequency, foreign_key_details,
                    visual_metadata, sla, contact_details, drawio_edge_id, color, created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (id) DO UPDATE SET
                    source_table_id = EXCLUDED.source_table_id,
                    target_table_id = EXCLUDED.target_table_id,
                    cardinality = EXCLUDED.cardinality,
                    source_optional = EXCLUDED.source_optional,
                    target_optional = EXCLUDED.target_optional,
                    relationship_type = EXCLUDED.relationship_type,
                    notes = EXCLUDED.notes,
                    owner = EXCLUDED.owner,
                    infrastructure_type = EXCLUDED.infrastructure_type,
                    etl_job_name = EXCLUDED.etl_job_name,
                    etl_job_frequency = EXCLUDED.etl_job_frequency,
                    foreign_key_details = EXCLUDED.foreign_key_details,
                    visual_metadata = EXCLUDED.visual_metadata,
                    sla = EXCLUDED.sla,
                    contact_details = EXCLUDED.contact_details,
                    drawio_edge_id = EXCLUDED.drawio_edge_id,
                    color = EXCLUDED.color,
                    updated_at = EXCLUDED.updated_at
                "#,
                duckdb::params![
                    rel.id.to_string(),
                    workspace_id.to_string(),
                    rel.source_table_id.to_string(),
                    rel.target_table_id.to_string(),
                    rel.cardinality.as_ref().map(|c| format!("{:?}", c)),
                    rel.source_optional,
                    rel.target_optional,
                    rel.relationship_type.as_ref().map(|r| format!("{:?}", r)),
                    &rel.notes,
                    &rel.owner,
                    rel.infrastructure_type.as_ref().map(|i| format!("{:?}", i)),
                    rel.etl_job_metadata.as_ref().map(|e| e.job_name.clone()),
                    rel.etl_job_metadata
                        .as_ref()
                        .and_then(|e| e.frequency.clone()),
                    rel.foreign_key_details
                        .as_ref()
                        .and_then(|f| serde_json::to_string(f).ok()),
                    rel.visual_metadata
                        .as_ref()
                        .and_then(|v| serde_json::to_string(v).ok()),
                    rel.sla.as_ref().and_then(|s| serde_json::to_string(s).ok()),
                    rel.contact_details
                        .as_ref()
                        .and_then(|c| serde_json::to_string(c).ok()),
                    &rel.drawio_edge_id,
                    &rel.color,
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                ],
            )
            .map_err(|e| {
                DatabaseError::SyncFailed(format!("Failed to sync relationship {}: {}", rel.id, e))
            })?;

            count += 1;
        }

        Ok(count)
    }

    async fn export_tables(&self, workspace_id: Uuid) -> DatabaseResult<Vec<Table>> {
        // Query tables
        let result = self
            .execute_query(&format!(
                "SELECT * FROM tables WHERE workspace_id = '{}'",
                workspace_id
            ))
            .await?;

        let mut tables = Vec::new();

        for row in &result.rows {
            // Parse table from row
            let id: Uuid = row
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let name = row
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Create a basic table - full implementation would parse all fields
            let mut table = Table::new(name, vec![]);
            table.id = id;

            if let Some(owner) = row.get("owner").and_then(|v| v.as_str()) {
                table.owner = Some(owner.to_string());
            }

            if let Some(notes) = row.get("notes").and_then(|v| v.as_str()) {
                table.notes = Some(notes.to_string());
            }

            // Query columns for this table
            let col_result = self
                .execute_query(&format!(
                    "SELECT * FROM columns WHERE table_id = '{}' ORDER BY column_order",
                    id
                ))
                .await?;

            for col_row in &col_result.rows {
                let col_name = col_row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let data_type = col_row
                    .get("data_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("string")
                    .to_string();

                let mut column = crate::models::Column::new(col_name, data_type);

                if let Some(desc) = col_row.get("description").and_then(|v| v.as_str()) {
                    column.description = desc.to_string();
                }

                column.primary_key = col_row
                    .get("primary_key")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                column.nullable = col_row
                    .get("nullable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                table.columns.push(column);
            }

            tables.push(table);
        }

        Ok(tables)
    }

    async fn export_domains(&self, workspace_id: Uuid) -> DatabaseResult<Vec<Domain>> {
        let result = self
            .execute_query(&format!(
                "SELECT * FROM domains WHERE workspace_id = '{}'",
                workspace_id
            ))
            .await?;

        let mut domains = Vec::new();

        for row in &result.rows {
            let id: Uuid = row
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let name = row
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut domain = Domain::new(name);
            domain.id = id;

            if let Some(desc) = row.get("description").and_then(|v| v.as_str()) {
                domain.description = Some(desc.to_string());
            }

            domains.push(domain);
        }

        Ok(domains)
    }

    async fn export_relationships(&self, workspace_id: Uuid) -> DatabaseResult<Vec<Relationship>> {
        let result = self
            .execute_query(&format!(
                "SELECT * FROM relationships WHERE workspace_id = '{}'",
                workspace_id
            ))
            .await?;

        let mut relationships = Vec::new();

        for row in &result.rows {
            let id: Uuid = row
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let source_table_id: Uuid = row
                .get("source_table_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let target_table_id: Uuid = row
                .get("target_table_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let mut rel = Relationship::new(source_table_id, target_table_id);
            rel.id = id;

            if let Some(notes) = row.get("notes").and_then(|v| v.as_str()) {
                rel.notes = Some(notes.to_string());
            }

            relationships.push(rel);
        }

        Ok(relationships)
    }

    async fn get_sync_status(&self, workspace_id: Uuid) -> DatabaseResult<SyncStatus> {
        let workspace_id_str = workspace_id.to_string();

        // Count tables
        let table_result = self
            .execute_query(&format!(
                "SELECT COUNT(*) as count FROM tables WHERE workspace_id = '{}'",
                workspace_id_str
            ))
            .await?;
        let table_count = table_result
            .rows
            .first()
            .and_then(|r| r.get("count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Count columns
        let column_result = self.execute_query(&format!(
            "SELECT COUNT(*) as count FROM columns c JOIN tables t ON c.table_id = t.id WHERE t.workspace_id = '{}'",
            workspace_id_str
        )).await?;
        let column_count = column_result
            .rows
            .first()
            .and_then(|r| r.get("count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Count relationships
        let rel_result = self
            .execute_query(&format!(
                "SELECT COUNT(*) as count FROM relationships WHERE workspace_id = '{}'",
                workspace_id_str
            ))
            .await?;
        let relationship_count = rel_result
            .rows
            .first()
            .and_then(|r| r.get("count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Count domains
        let domain_result = self
            .execute_query(&format!(
                "SELECT COUNT(*) as count FROM domains WHERE workspace_id = '{}'",
                workspace_id_str
            ))
            .await?;
        let domain_count = domain_result
            .rows
            .first()
            .and_then(|r| r.get("count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Get last sync time
        let sync_result = self.execute_query(&format!(
            "SELECT sync_completed_at FROM sync_log WHERE workspace_id = '{}' ORDER BY sync_started_at DESC LIMIT 1",
            workspace_id_str
        )).await?;
        let last_sync_at = sync_result
            .rows
            .first()
            .and_then(|r| r.get("sync_completed_at"))
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc));

        Ok(SyncStatus {
            workspace_id,
            last_sync_at,
            table_count,
            column_count,
            relationship_count,
            domain_count,
            decision_count: 0,  // TODO: query decisions table when implemented
            knowledge_count: 0, // TODO: query knowledge_articles table when implemented
            is_stale: false,    // Would need file hash comparison to determine
            pending_sync_count: 0,
        })
    }

    async fn upsert_workspace(&self, workspace: &Workspace) -> DatabaseResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        let now = chrono::Utc::now();

        conn.execute(
            r#"
            INSERT INTO workspaces (id, name, owner_id, created_at, last_modified_at, yaml_hash, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                owner_id = EXCLUDED.owner_id,
                last_modified_at = EXCLUDED.last_modified_at,
                yaml_hash = EXCLUDED.yaml_hash,
                metadata = EXCLUDED.metadata
            "#,
            duckdb::params![
                workspace.id.to_string(),
                &workspace.name,
                workspace.owner_id.to_string(),
                workspace.created_at.to_rfc3339(),
                now.to_rfc3339(),
                Option::<String>::None,
                Option::<String>::None,
            ],
        ).map_err(|e| DatabaseError::SyncFailed(format!("Failed to upsert workspace: {}", e)))?;

        Ok(())
    }

    async fn get_workspace(&self, workspace_id: Uuid) -> DatabaseResult<Option<Workspace>> {
        let result = self
            .execute_query(&format!(
                "SELECT * FROM workspaces WHERE id = '{}'",
                workspace_id
            ))
            .await?;

        if let Some(row) = result.rows.first() {
            let id: Uuid = row
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or(workspace_id);

            let name = row
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let owner_id: Uuid = row
                .get("owner_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let workspace = Workspace::with_id(id, name, owner_id);

            Ok(Some(workspace))
        } else {
            Ok(None)
        }
    }

    async fn get_workspace_by_name(&self, name: &str) -> DatabaseResult<Option<Workspace>> {
        let result = self
            .execute_query(&format!(
                "SELECT * FROM workspaces WHERE name = '{}'",
                name.replace('\'', "''") // Escape single quotes
            ))
            .await?;

        if let Some(row) = result.rows.first() {
            let id: Uuid = row
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let ws_name = row
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let owner_id: Uuid = row
                .get("owner_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let workspace = Workspace::with_id(id, ws_name, owner_id);

            Ok(Some(workspace))
        } else {
            Ok(None)
        }
    }

    async fn delete_workspace(&self, workspace_id: Uuid) -> DatabaseResult<()> {
        // Cascade delete will handle related tables
        self.execute(&format!(
            "DELETE FROM workspaces WHERE id = '{}'",
            workspace_id
        ))?;
        Ok(())
    }

    async fn record_file_hash(
        &self,
        workspace_id: Uuid,
        file_path: &str,
        hash: &str,
    ) -> DatabaseResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| DatabaseError::ConnectionFailed(format!("Lock error: {}", e)))?;

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO file_hashes (workspace_id, file_path, hash, last_synced_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (workspace_id, file_path) DO UPDATE SET
                hash = EXCLUDED.hash,
                last_synced_at = EXCLUDED.last_synced_at
            "#,
            duckdb::params![workspace_id.to_string(), file_path, hash, now],
        )
        .map_err(|e| DatabaseError::SyncFailed(format!("Failed to record file hash: {}", e)))?;

        Ok(())
    }

    async fn get_file_hash(
        &self,
        workspace_id: Uuid,
        file_path: &str,
    ) -> DatabaseResult<Option<String>> {
        let result = self
            .execute_query(&format!(
                "SELECT hash FROM file_hashes WHERE workspace_id = '{}' AND file_path = '{}'",
                workspace_id,
                file_path.replace('\'', "''")
            ))
            .await?;

        Ok(result
            .rows
            .first()
            .and_then(|r| r.get("hash"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    async fn health_check(&self) -> DatabaseResult<bool> {
        let result = self.execute_query("SELECT 1 as healthy").await?;
        Ok(!result.rows.is_empty())
    }

    fn backend_type(&self) -> &'static str {
        "duckdb"
    }

    async fn close(&self) -> DatabaseResult<()> {
        // DuckDB connection is closed when dropped
        // For explicit close, we could drop the connection, but that would
        // require interior mutability. For now, this is a no-op.
        Ok(())
    }

    async fn sync_decisions(
        &self,
        _workspace_id: Uuid,
        _decisions: &[crate::models::decision::Decision],
    ) -> DatabaseResult<usize> {
        // TODO: Implement decision sync when decisions table is created
        Ok(0)
    }

    async fn sync_knowledge(
        &self,
        _workspace_id: Uuid,
        _articles: &[crate::models::knowledge::KnowledgeArticle],
    ) -> DatabaseResult<usize> {
        // TODO: Implement knowledge sync when knowledge_articles table is created
        Ok(0)
    }

    async fn export_decisions(
        &self,
        _workspace_id: Uuid,
    ) -> DatabaseResult<Vec<crate::models::decision::Decision>> {
        // TODO: Implement decision export when decisions table is created
        Ok(Vec::new())
    }

    async fn export_knowledge(
        &self,
        _workspace_id: Uuid,
    ) -> DatabaseResult<Vec<crate::models::knowledge::KnowledgeArticle>> {
        // TODO: Implement knowledge export when knowledge_articles table is created
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database() {
        let backend = DuckDBBackend::in_memory().unwrap();
        assert!(backend.is_in_memory());
        assert!(backend.db_path().is_none());
    }

    #[tokio::test]
    async fn test_initialize() {
        let backend = DuckDBBackend::in_memory().unwrap();
        backend.initialize().await.unwrap();

        // Verify tables exist
        // DuckDB doesn't have sqlite_master, but we can query information_schema
        let result = backend
            .execute_query(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'main'",
            )
            .await
            .unwrap();

        assert!(!result.rows.is_empty());
    }

    #[tokio::test]
    async fn test_health_check() {
        let backend = DuckDBBackend::in_memory().unwrap();
        assert!(backend.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_backend_type() {
        let backend = DuckDBBackend::in_memory().unwrap();
        assert_eq!(backend.backend_type(), "duckdb");
    }

    #[tokio::test]
    async fn test_workspace_crud() {
        let backend = DuckDBBackend::in_memory().unwrap();
        backend.initialize().await.unwrap();

        let workspace = Workspace::new("test-workspace".to_string(), Uuid::new_v4());

        // Create
        backend.upsert_workspace(&workspace).await.unwrap();

        // Read
        let loaded = backend.get_workspace(workspace.id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "test-workspace");

        // Read by name
        let loaded_by_name = backend
            .get_workspace_by_name("test-workspace")
            .await
            .unwrap();
        assert!(loaded_by_name.is_some());

        // Delete
        backend.delete_workspace(workspace.id).await.unwrap();
        let deleted = backend.get_workspace(workspace.id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_sync_tables() {
        let backend = DuckDBBackend::in_memory().unwrap();
        backend.initialize().await.unwrap();

        let workspace = Workspace::new("test-workspace".to_string(), Uuid::new_v4());
        backend.upsert_workspace(&workspace).await.unwrap();

        let table = Table::new(
            "users".to_string(),
            vec![
                crate::models::Column::new("id".to_string(), "uuid".to_string()),
                crate::models::Column::new("name".to_string(), "varchar".to_string()),
            ],
        );

        let count = backend.sync_tables(workspace.id, &[table]).await.unwrap();
        assert_eq!(count, 1);

        // Verify sync status
        let status = backend.get_sync_status(workspace.id).await.unwrap();
        assert_eq!(status.table_count, 1);
        assert_eq!(status.column_count, 2);
    }

    #[tokio::test]
    async fn test_file_hash() {
        let backend = DuckDBBackend::in_memory().unwrap();
        backend.initialize().await.unwrap();

        let workspace = Workspace::new("test-workspace".to_string(), Uuid::new_v4());
        backend.upsert_workspace(&workspace).await.unwrap();

        // Record hash
        backend
            .record_file_hash(workspace.id, "test.yaml", "abc123")
            .await
            .unwrap();

        // Get hash
        let hash = backend
            .get_file_hash(workspace.id, "test.yaml")
            .await
            .unwrap();
        assert_eq!(hash, Some("abc123".to_string()));

        // Non-existent file
        let no_hash = backend
            .get_file_hash(workspace.id, "missing.yaml")
            .await
            .unwrap();
        assert!(no_hash.is_none());
    }
}
