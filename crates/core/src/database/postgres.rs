//! PostgreSQL database backend implementation
//!
//! Provides a PostgreSQL backend for server deployments and team scenarios.
//! Uses connection pooling via deadpool-postgres.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::schema::DatabaseSchema;
use super::{DatabaseBackend, DatabaseError, DatabaseResult, QueryResult, SyncStatus};
use crate::models::{Domain, Relationship, Table, Workspace};

/// PostgreSQL database backend
///
/// Provides a PostgreSQL database backend for production deployments.
/// Uses connection pooling for efficient resource usage.
pub struct PostgresBackend {
    /// Connection string
    connection_string: String,
    /// PostgreSQL client (wrapped for async access)
    client: Arc<Mutex<tokio_postgres::Client>>,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend
    ///
    /// # Arguments
    /// * `connection_string` - PostgreSQL connection string
    ///
    /// # Returns
    /// A new PostgreSQL backend instance
    pub async fn new(connection_string: &str) -> DatabaseResult<Self> {
        let (client, connection) =
            tokio_postgres::connect(connection_string, tokio_postgres::NoTls)
                .await
                .map_err(|e| {
                    DatabaseError::ConnectionFailed(format!(
                        "Failed to connect to PostgreSQL: {}",
                        e
                    ))
                })?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        Ok(Self {
            connection_string: connection_string.to_string(),
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// Get the connection string (masked for security)
    pub fn connection_string_masked(&self) -> String {
        // Mask password in connection string
        if let Some(at_pos) = self.connection_string.find('@')
            && let Some(colon_pos) = self.connection_string[..at_pos].rfind(':')
        {
            let prefix = &self.connection_string[..colon_pos + 1];
            let suffix = &self.connection_string[at_pos..];
            return format!("{}****{}", prefix, suffix);
        }
        self.connection_string.clone()
    }

    /// Convert a PostgreSQL row to a JSON value
    fn row_to_json(row: &tokio_postgres::Row, columns: &[String]) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        for (i, col_name) in columns.iter().enumerate() {
            let value = Self::get_column_value(row, i);
            map.insert(col_name.clone(), value);
        }

        serde_json::Value::Object(map)
    }

    /// Get a column value as JSON
    fn get_column_value(row: &tokio_postgres::Row, idx: usize) -> serde_json::Value {
        // Try different types
        if let Ok(v) = row.try_get::<_, Option<String>>(idx) {
            return v
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<_, Option<i64>>(idx) {
            return v
                .map(|n| serde_json::Value::Number(n.into()))
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<_, Option<i32>>(idx) {
            return v
                .map(|n| serde_json::Value::Number(n.into()))
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<_, Option<bool>>(idx) {
            return v
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null);
        }
        if let Ok(v) = row.try_get::<_, Option<f64>>(idx) {
            return v
                .and_then(serde_json::Number::from_f64)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null);
        }

        serde_json::Value::Null
    }
}

#[async_trait(?Send)]
impl DatabaseBackend for PostgresBackend {
    async fn initialize(&self) -> DatabaseResult<()> {
        let client = self.client.lock().await;

        // Create tables
        client
            .batch_execute(DatabaseSchema::create_tables_sql())
            .await
            .map_err(|e| {
                DatabaseError::MigrationFailed(format!("Failed to create tables: {}", e))
            })?;

        // Create indexes
        client
            .batch_execute(DatabaseSchema::create_indexes_sql())
            .await
            .map_err(|e| {
                DatabaseError::MigrationFailed(format!("Failed to create indexes: {}", e))
            })?;

        // Record schema version
        client
            .execute(
                "INSERT INTO schema_version (version) VALUES ($1) ON CONFLICT (version) DO NOTHING",
                &[&super::schema::SCHEMA_VERSION],
            )
            .await
            .map_err(|e| {
                DatabaseError::MigrationFailed(format!("Failed to record schema version: {}", e))
            })?;

        Ok(())
    }

    async fn execute_query(&self, sql: &str) -> DatabaseResult<QueryResult> {
        let start = std::time::Instant::now();
        let client = self.client.lock().await;

        let rows = client
            .query(sql, &[])
            .await
            .map_err(|e| DatabaseError::QueryFailed(format!("Query failed: {}", e)))?;

        let columns: Vec<String> = if !rows.is_empty() {
            rows[0]
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect()
        } else {
            Vec::new()
        };

        let json_rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| Self::row_to_json(row, &columns))
            .collect();

        Ok(QueryResult {
            columns,
            rows: json_rows,
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
        let client = self.client.lock().await;

        // Convert JSON params to strings for simplicity
        let string_params: Vec<String> = params
            .iter()
            .map(|p| match p {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            })
            .collect();

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = string_params
            .iter()
            .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = client
            .query(sql, &param_refs)
            .await
            .map_err(|e| DatabaseError::QueryFailed(format!("Query failed: {}", e)))?;

        let columns: Vec<String> = if !rows.is_empty() {
            rows[0]
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect()
        } else {
            Vec::new()
        };

        let json_rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| Self::row_to_json(row, &columns))
            .collect();

        Ok(QueryResult {
            columns,
            rows: json_rows,
            rows_affected: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn sync_tables(&self, workspace_id: Uuid, tables: &[Table]) -> DatabaseResult<usize> {
        let client = self.client.lock().await;
        let mut count = 0;

        for table in tables {
            let now = chrono::Utc::now();

            // Upsert table
            client
                .execute(
                    r#"
                INSERT INTO tables (
                    id, workspace_id, domain_id, name, database_type, catalog_name, schema_name,
                    owner, infrastructure_type, notes, medallion_layers, scd_pattern,
                    data_vault_classification, modeling_level, position_x, position_y,
                    yaml_file_path, yaml_hash, sla, contact_details, quality, tags,
                    custom_properties, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                        $17, $18, $19, $20, $21, $22, $23, $24, $25)
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
                    &[
                        &table.id.to_string(),
                        &workspace_id.to_string(),
                        &Option::<String>::None, // domain_id
                        &table.name,
                        &table.database_type.as_ref().map(|d| format!("{:?}", d)),
                        &table.catalog_name,
                        &table.schema_name,
                        &table.owner,
                        &table
                            .infrastructure_type
                            .as_ref()
                            .map(|i| format!("{:?}", i)),
                        &table.notes,
                        &if table.medallion_layers.is_empty() {
                            None
                        } else {
                            serde_json::to_string(&table.medallion_layers).ok()
                        },
                        &table.scd_pattern.as_ref().map(|s| format!("{:?}", s)),
                        &table
                            .data_vault_classification
                            .as_ref()
                            .map(|d| format!("{:?}", d)),
                        &table.modeling_level.as_ref().map(|m| format!("{:?}", m)),
                        &table.position.as_ref().map(|p| p.x),
                        &table.position.as_ref().map(|p| p.y),
                        &Option::<String>::None, // yaml_file_path
                        &Option::<String>::None, // yaml_hash
                        &table
                            .sla
                            .as_ref()
                            .and_then(|s| serde_json::to_string(s).ok()),
                        &table
                            .contact_details
                            .as_ref()
                            .and_then(|c| serde_json::to_string(c).ok()),
                        &if table.quality.is_empty() {
                            None
                        } else {
                            serde_json::to_string(&table.quality).ok()
                        },
                        &if table.tags.is_empty() {
                            None
                        } else {
                            serde_json::to_string(&table.tags).ok()
                        },
                        &Option::<String>::None, // custom_properties
                        &now.to_rfc3339(),
                        &now.to_rfc3339(),
                    ],
                )
                .await
                .map_err(|e| {
                    DatabaseError::SyncFailed(format!("Failed to sync table {}: {}", table.name, e))
                })?;

            // Delete existing columns
            client
                .execute(
                    "DELETE FROM columns WHERE table_id = $1",
                    &[&table.id.to_string()],
                )
                .await
                .map_err(|e| {
                    DatabaseError::SyncFailed(format!("Failed to delete columns: {}", e))
                })?;

            // Insert columns
            for (order, column) in table.columns.iter().enumerate() {
                client.execute(
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
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                            $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32)
                    "#,
                    &[
                        &column.id.as_deref(),
                        &table.id.to_string(),
                        &column.name,
                        &column.business_name,
                        &column.description,
                        &column.data_type,
                        &column.physical_type,
                        &column.physical_name,
                        &column.primary_key,
                        &column.primary_key_position,
                        &column.unique,
                        &column.nullable,
                        &column.partitioned,
                        &column.partition_key_position,
                        &column.clustered,
                        &column.classification,
                        &column.critical_data_element,
                        &column.encrypted_name,
                        &if column.transform_source_objects.is_empty() { None } else { serde_json::to_string(&column.transform_source_objects).ok() },
                        &column.transform_logic,
                        &column.transform_description,
                        &if column.examples.is_empty() { None } else { serde_json::to_string(&column.examples).ok() },
                        &column.default_value.as_ref().and_then(|d| serde_json::to_string(d).ok()),
                        &if column.relationships.is_empty() { None } else { serde_json::to_string(&column.relationships).ok() },
                        &if column.authoritative_definitions.is_empty() { None } else { serde_json::to_string(&column.authoritative_definitions).ok() },
                        &if column.quality.is_empty() { None } else { serde_json::to_string(&column.quality).ok() },
                        &if column.enum_values.is_empty() { None } else { serde_json::to_string(&column.enum_values).ok() },
                        &if column.tags.is_empty() { None } else { serde_json::to_string(&column.tags).ok() },
                        &if column.custom_properties.is_empty() { None } else { serde_json::to_string(&column.custom_properties).ok() },
                        &column.logical_type_options.as_ref().and_then(|l| serde_json::to_string(l).ok()),
                        &(order as i32),
                        &column.nested_data,
                    ],
                ).await
                    .map_err(|e| DatabaseError::SyncFailed(format!("Failed to sync column {}.{}: {}", table.name, column.name, e)))?;
            }

            count += 1;
        }

        Ok(count)
    }

    async fn sync_domains(&self, workspace_id: Uuid, domains: &[Domain]) -> DatabaseResult<usize> {
        let client = self.client.lock().await;
        let mut count = 0;
        let now = chrono::Utc::now();

        for domain in domains {
            client.execute(
                r#"
                INSERT INTO domains (id, workspace_id, name, description, created_at, updated_at, yaml_hash, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    description = EXCLUDED.description,
                    updated_at = EXCLUDED.updated_at,
                    yaml_hash = EXCLUDED.yaml_hash,
                    metadata = EXCLUDED.metadata
                "#,
                &[
                    &domain.id.to_string(),
                    &workspace_id.to_string(),
                    &domain.name,
                    &domain.description,
                    &domain.created_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| now.to_rfc3339()),
                    &now.to_rfc3339(),
                    &Option::<String>::None,
                    &Option::<String>::None,
                ],
            ).await
                .map_err(|e| DatabaseError::SyncFailed(format!("Failed to sync domain {}: {}", domain.name, e)))?;

            count += 1;
        }

        Ok(count)
    }

    async fn sync_relationships(
        &self,
        workspace_id: Uuid,
        relationships: &[Relationship],
    ) -> DatabaseResult<usize> {
        let client = self.client.lock().await;
        let mut count = 0;
        let now = chrono::Utc::now();

        for rel in relationships {
            client.execute(
                r#"
                INSERT INTO relationships (
                    id, workspace_id, source_table_id, target_table_id, cardinality,
                    source_optional, target_optional, relationship_type, notes, owner,
                    infrastructure_type, etl_job_name, etl_job_frequency, foreign_key_details,
                    visual_metadata, sla, contact_details, drawio_edge_id, color, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
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
                &[
                    &rel.id.to_string(),
                    &workspace_id.to_string(),
                    &rel.source_table_id.to_string(),
                    &rel.target_table_id.to_string(),
                    &rel.cardinality.as_ref().map(|c| format!("{:?}", c)),
                    &rel.source_optional,
                    &rel.target_optional,
                    &rel.relationship_type.as_ref().map(|r| format!("{:?}", r)),
                    &rel.notes,
                    &rel.owner,
                    &rel.infrastructure_type.as_ref().map(|i| format!("{:?}", i)),
                    &rel.etl_job_metadata.as_ref().map(|e| e.job_name.clone()),
                    &rel.etl_job_metadata.as_ref().and_then(|e| e.frequency.clone()),
                    &rel.foreign_key_details.as_ref().and_then(|f| serde_json::to_string(f).ok()),
                    &rel.visual_metadata.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                    &rel.sla.as_ref().and_then(|s| serde_json::to_string(s).ok()),
                    &rel.contact_details.as_ref().and_then(|c| serde_json::to_string(c).ok()),
                    &rel.drawio_edge_id,
                    &rel.color,
                    &now.to_rfc3339(),
                    &now.to_rfc3339(),
                ],
            ).await
                .map_err(|e| DatabaseError::SyncFailed(format!("Failed to sync relationship {}: {}", rel.id, e)))?;

            count += 1;
        }

        Ok(count)
    }

    async fn export_tables(&self, workspace_id: Uuid) -> DatabaseResult<Vec<Table>> {
        let result = self
            .execute_query(&format!(
                "SELECT * FROM tables WHERE workspace_id = '{}'",
                workspace_id
            ))
            .await?;

        let mut tables = Vec::new();

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

            let mut table = Table::new(name, vec![]);
            table.id = id;

            // Query columns
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

                let column = crate::models::Column::new(col_name, data_type);
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
            .and_then(|v| v.as_i64())
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
            .and_then(|v| v.as_i64())
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
            .and_then(|v| v.as_i64())
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
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;

        Ok(SyncStatus {
            workspace_id,
            last_sync_at: None,
            table_count,
            column_count,
            relationship_count,
            domain_count,
            decision_count: 0,  // TODO: query decisions table when implemented
            knowledge_count: 0, // TODO: query knowledge_articles table when implemented
            is_stale: false,
            pending_sync_count: 0,
        })
    }

    async fn upsert_workspace(&self, workspace: &Workspace) -> DatabaseResult<()> {
        let client = self.client.lock().await;
        let now = chrono::Utc::now();

        client.execute(
            r#"
            INSERT INTO workspaces (id, name, owner_id, created_at, last_modified_at, yaml_hash, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                owner_id = EXCLUDED.owner_id,
                last_modified_at = EXCLUDED.last_modified_at,
                yaml_hash = EXCLUDED.yaml_hash,
                metadata = EXCLUDED.metadata
            "#,
            &[
                &workspace.id.to_string(),
                &workspace.name,
                &Some(workspace.owner_id.to_string()),
                &workspace.created_at.to_rfc3339(),
                &now.to_rfc3339(),
                &Option::<String>::None,
                &Option::<String>::None,
            ],
        ).await
            .map_err(|e| DatabaseError::SyncFailed(format!("Failed to upsert workspace: {}", e)))?;

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
            let name = row
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let owner_id = row
                .get("owner_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let mut workspace = Workspace::new(name, owner_id);
            workspace.id = workspace_id;

            Ok(Some(workspace))
        } else {
            Ok(None)
        }
    }

    async fn get_workspace_by_name(&self, name: &str) -> DatabaseResult<Option<Workspace>> {
        let result = self
            .execute_query(&format!(
                "SELECT * FROM workspaces WHERE name = '{}'",
                name.replace('\'', "''")
            ))
            .await?;

        if let Some(row) = result.rows.first() {
            let id: Uuid = row
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let owner_id = row
                .get("owner_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            let ws_name = row
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut workspace = Workspace::new(ws_name, owner_id);
            workspace.id = id;

            Ok(Some(workspace))
        } else {
            Ok(None)
        }
    }

    async fn delete_workspace(&self, workspace_id: Uuid) -> DatabaseResult<()> {
        let client = self.client.lock().await;
        client
            .execute(
                "DELETE FROM workspaces WHERE id = $1",
                &[&workspace_id.to_string()],
            )
            .await
            .map_err(|e| {
                DatabaseError::QueryFailed(format!("Failed to delete workspace: {}", e))
            })?;
        Ok(())
    }

    async fn record_file_hash(
        &self,
        workspace_id: Uuid,
        file_path: &str,
        hash: &str,
    ) -> DatabaseResult<()> {
        let client = self.client.lock().await;

        client
            .execute(
                r#"
            INSERT INTO file_hashes (workspace_id, file_path, hash, last_synced_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ON CONFLICT (workspace_id, file_path) DO UPDATE SET
                hash = EXCLUDED.hash,
                last_synced_at = CURRENT_TIMESTAMP
            "#,
                &[&workspace_id.to_string(), &file_path, &hash],
            )
            .await
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
        "postgres"
    }

    async fn close(&self) -> DatabaseResult<()> {
        // PostgreSQL connection is closed when client is dropped
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
