//! Database schema definitions
//!
//! Provides SQL schema definitions that work with both DuckDB and PostgreSQL.
//! Complex nested data (JSONB) is used for fields that don't need to be indexed.

/// Schema version for migrations
pub const SCHEMA_VERSION: i32 = 1;

/// Database schema helper
pub struct DatabaseSchema;

impl DatabaseSchema {
    /// Get the initial schema creation SQL
    ///
    /// This SQL is compatible with both DuckDB and PostgreSQL.
    /// Note: DuckDB doesn't support CASCADE/SET NULL in foreign keys, so we use simple REFERENCES.
    pub fn create_tables_sql() -> &'static str {
        r#"
-- Workspace metadata
CREATE TABLE IF NOT EXISTS workspaces (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    owner_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_modified_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    yaml_hash TEXT,
    metadata JSON
);

-- Create unique index on workspace name (owner can be null)
CREATE UNIQUE INDEX IF NOT EXISTS idx_workspaces_name ON workspaces(name);

-- Domain definitions
CREATE TABLE IF NOT EXISTS domains (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    yaml_hash TEXT,
    metadata JSON,
    UNIQUE(workspace_id, name)
);

-- ODCS Tables (flattened from Table model)
CREATE TABLE IF NOT EXISTS tables (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    domain_id UUID REFERENCES domains(id),
    name TEXT NOT NULL,
    database_type TEXT,
    catalog_name TEXT,
    schema_name TEXT,
    owner TEXT,
    infrastructure_type TEXT,
    notes TEXT,
    medallion_layers JSON,
    scd_pattern TEXT,
    data_vault_classification TEXT,
    modeling_level TEXT,
    position_x DOUBLE PRECISION,
    position_y DOUBLE PRECISION,
    yaml_file_path TEXT,
    yaml_hash TEXT,
    sla JSON,
    contact_details JSON,
    quality JSON,
    tags JSON,
    custom_properties JSON,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create unique constraint on table identity (workspace + database context + name)
CREATE UNIQUE INDEX IF NOT EXISTS idx_tables_unique ON tables(
    workspace_id,
    COALESCE(database_type, ''),
    COALESCE(catalog_name, ''),
    COALESCE(schema_name, ''),
    name
);

-- Columns (with full ODCS v3.1.0 properties)
CREATE TABLE IF NOT EXISTS columns (
    id TEXT,
    table_id UUID NOT NULL REFERENCES tables(id),
    name TEXT NOT NULL,
    business_name TEXT,
    description TEXT,
    data_type TEXT NOT NULL,
    physical_type TEXT,
    physical_name TEXT,
    primary_key BOOLEAN DEFAULT FALSE,
    primary_key_position INTEGER,
    is_unique BOOLEAN DEFAULT FALSE,
    nullable BOOLEAN DEFAULT TRUE,
    partitioned BOOLEAN DEFAULT FALSE,
    partition_key_position INTEGER,
    clustered BOOLEAN DEFAULT FALSE,
    classification TEXT,
    critical_data_element BOOLEAN DEFAULT FALSE,
    encrypted_name TEXT,
    transform_source_objects JSON,
    transform_logic TEXT,
    transform_description TEXT,
    examples JSON,
    default_value JSON,
    relationships JSON,
    authoritative_definitions JSON,
    quality JSON,
    enum_values JSON,
    tags JSON,
    custom_properties JSON,
    logical_type_options JSON,
    column_order INTEGER DEFAULT 0,
    nested_data TEXT,
    PRIMARY KEY (table_id, name)
);

-- Table relationships
CREATE TABLE IF NOT EXISTS relationships (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    source_table_id UUID NOT NULL REFERENCES tables(id),
    target_table_id UUID NOT NULL REFERENCES tables(id),
    cardinality TEXT,
    source_optional BOOLEAN,
    target_optional BOOLEAN,
    relationship_type TEXT,
    notes TEXT,
    owner TEXT,
    infrastructure_type TEXT,
    etl_job_name TEXT,
    etl_job_frequency TEXT,
    foreign_key_details JSON,
    visual_metadata JSON,
    sla JSON,
    contact_details JSON,
    drawio_edge_id TEXT,
    color TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_table_id, target_table_id)
);

-- Systems (infrastructure nodes in domains)
CREATE TABLE IF NOT EXISTS systems (
    id UUID PRIMARY KEY,
    domain_id UUID NOT NULL REFERENCES domains(id),
    name TEXT NOT NULL,
    infrastructure_type TEXT NOT NULL,
    description TEXT,
    endpoints JSON,
    owner TEXT,
    version TEXT,
    position_x DOUBLE PRECISION,
    position_y DOUBLE PRECISION,
    metadata JSON,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(domain_id, name)
);

-- Tag index for fast filtering
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    tag_key TEXT NOT NULL,
    tag_value TEXT,
    tag_list JSON,
    UNIQUE(workspace_id, entity_type, entity_id, tag_key)
);

-- File hash tracking for change detection
CREATE TABLE IF NOT EXISTS file_hashes (
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    file_path TEXT NOT NULL,
    hash TEXT NOT NULL,
    last_synced_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (workspace_id, file_path)
);

-- Sync log for tracking sync operations
CREATE TABLE IF NOT EXISTS sync_log (
    id INTEGER PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    sync_started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    sync_completed_at TIMESTAMPTZ,
    tables_synced INTEGER DEFAULT 0,
    columns_synced INTEGER DEFAULT 0,
    relationships_synced INTEGER DEFAULT 0,
    domains_synced INTEGER DEFAULT 0,
    errors JSON,
    trigger TEXT
);

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"#
    }

    /// Get index creation SQL for performance optimization
    pub fn create_indexes_sql() -> &'static str {
        r#"
-- Fast lookup by name patterns
CREATE INDEX IF NOT EXISTS idx_tables_name ON tables(name);
CREATE INDEX IF NOT EXISTS idx_tables_workspace_name ON tables(workspace_id, name);
CREATE INDEX IF NOT EXISTS idx_columns_table_name ON columns(table_id, name);

-- Domain filtering
CREATE INDEX IF NOT EXISTS idx_tables_domain ON tables(domain_id);
CREATE INDEX IF NOT EXISTS idx_systems_domain ON systems(domain_id);

-- Tag-based queries
CREATE INDEX IF NOT EXISTS idx_tags_key_value ON tags(tag_key, tag_value);
CREATE INDEX IF NOT EXISTS idx_tags_entity ON tags(entity_type, entity_id);

-- Relationship graph queries
CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(source_table_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_table_id);
CREATE INDEX IF NOT EXISTS idx_relationships_workspace ON relationships(workspace_id);

-- Owner and infrastructure type filtering
CREATE INDEX IF NOT EXISTS idx_tables_owner ON tables(owner);
CREATE INDEX IF NOT EXISTS idx_tables_infrastructure ON tables(infrastructure_type);
CREATE INDEX IF NOT EXISTS idx_relationships_owner ON relationships(owner);

-- File hash lookups
CREATE INDEX IF NOT EXISTS idx_file_hashes_workspace ON file_hashes(workspace_id);

-- Sync log queries
CREATE INDEX IF NOT EXISTS idx_sync_log_workspace ON sync_log(workspace_id);
CREATE INDEX IF NOT EXISTS idx_sync_log_time ON sync_log(sync_started_at DESC);
"#
    }

    /// Get DuckDB-specific optimizations
    #[cfg(feature = "duckdb-backend")]
    pub fn duckdb_optimizations_sql() -> &'static str {
        r#"
-- DuckDB-specific settings for performance
PRAGMA memory_limit='512MB';
PRAGMA threads=4;
"#
    }

    /// Get PostgreSQL-specific optimizations
    #[cfg(feature = "postgres-backend")]
    pub fn postgres_optimizations_sql() -> &'static str {
        r#"
-- PostgreSQL-specific: Enable trigram extension for fuzzy search (if available)
-- CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Note: Most PostgreSQL optimizations are server-side configurations
"#
    }

    /// Insert a record into schema_version table
    pub fn record_schema_version_sql() -> &'static str {
        "INSERT INTO schema_version (version) VALUES ($1) ON CONFLICT (version) DO NOTHING"
    }

    /// Check current schema version
    pub fn check_schema_version_sql() -> &'static str {
        "SELECT MAX(version) as version FROM schema_version"
    }

    /// Drop all tables (for testing/reset)
    pub fn drop_all_tables_sql() -> &'static str {
        r#"
DROP TABLE IF EXISTS sync_log;
DROP TABLE IF EXISTS file_hashes;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS systems;
DROP TABLE IF EXISTS relationships;
DROP TABLE IF EXISTS columns;
DROP TABLE IF EXISTS tables;
DROP TABLE IF EXISTS domains;
DROP TABLE IF EXISTS workspaces;
DROP TABLE IF EXISTS schema_version;
"#
    }
}

/// SQL for inserting/updating workspaces
pub mod workspace_sql {
    pub const UPSERT: &str = r#"
INSERT INTO workspaces (id, name, owner_id, created_at, last_modified_at, yaml_hash, metadata)
VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    owner_id = EXCLUDED.owner_id,
    last_modified_at = EXCLUDED.last_modified_at,
    yaml_hash = EXCLUDED.yaml_hash,
    metadata = EXCLUDED.metadata
"#;

    pub const SELECT_BY_ID: &str = "SELECT * FROM workspaces WHERE id = $1";
    pub const SELECT_BY_NAME: &str = "SELECT * FROM workspaces WHERE name = $1";
    pub const DELETE: &str = "DELETE FROM workspaces WHERE id = $1";
}

/// SQL for inserting/updating domains
pub mod domain_sql {
    pub const UPSERT: &str = r#"
INSERT INTO domains (id, workspace_id, name, description, created_at, updated_at, yaml_hash, metadata)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    updated_at = EXCLUDED.updated_at,
    yaml_hash = EXCLUDED.yaml_hash,
    metadata = EXCLUDED.metadata
"#;

    pub const SELECT_BY_WORKSPACE: &str = "SELECT * FROM domains WHERE workspace_id = $1";
    pub const DELETE_BY_WORKSPACE: &str = "DELETE FROM domains WHERE workspace_id = $1";
}

/// SQL for inserting/updating tables
pub mod table_sql {
    pub const UPSERT: &str = r#"
INSERT INTO tables (
    id, workspace_id, domain_id, name, database_type, catalog_name, schema_name,
    owner, infrastructure_type, notes, medallion_layers, scd_pattern,
    data_vault_classification, modeling_level, position_x, position_y,
    yaml_file_path, yaml_hash, sla, contact_details, quality, tags,
    custom_properties, created_at, updated_at
)
VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
    $17, $18, $19, $20, $21, $22, $23, $24, $25
)
ON CONFLICT (id) DO UPDATE SET
    workspace_id = EXCLUDED.workspace_id,
    domain_id = EXCLUDED.domain_id,
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
"#;

    pub const SELECT_BY_WORKSPACE: &str = "SELECT * FROM tables WHERE workspace_id = $1";
    pub const SELECT_BY_ID: &str = "SELECT * FROM tables WHERE id = $1";
    pub const DELETE_BY_WORKSPACE: &str = "DELETE FROM tables WHERE workspace_id = $1";
    pub const COUNT_BY_WORKSPACE: &str =
        "SELECT COUNT(*) as count FROM tables WHERE workspace_id = $1";
}

/// SQL for inserting/updating columns
pub mod column_sql {
    pub const UPSERT: &str = r#"
INSERT INTO columns (
    id, table_id, name, business_name, description, data_type, physical_type,
    physical_name, primary_key, primary_key_position, is_unique, nullable,
    partitioned, partition_key_position, clustered, classification,
    critical_data_element, encrypted_name, transform_source_objects,
    transform_logic, transform_description, examples, default_value,
    relationships, authoritative_definitions, quality, enum_values, tags,
    custom_properties, logical_type_options, column_order, nested_data
)
VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
    $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32
)
ON CONFLICT (table_id, name) DO UPDATE SET
    id = EXCLUDED.id,
    business_name = EXCLUDED.business_name,
    description = EXCLUDED.description,
    data_type = EXCLUDED.data_type,
    physical_type = EXCLUDED.physical_type,
    physical_name = EXCLUDED.physical_name,
    primary_key = EXCLUDED.primary_key,
    primary_key_position = EXCLUDED.primary_key_position,
    is_unique = EXCLUDED.is_unique,
    nullable = EXCLUDED.nullable,
    partitioned = EXCLUDED.partitioned,
    partition_key_position = EXCLUDED.partition_key_position,
    clustered = EXCLUDED.clustered,
    classification = EXCLUDED.classification,
    critical_data_element = EXCLUDED.critical_data_element,
    encrypted_name = EXCLUDED.encrypted_name,
    transform_source_objects = EXCLUDED.transform_source_objects,
    transform_logic = EXCLUDED.transform_logic,
    transform_description = EXCLUDED.transform_description,
    examples = EXCLUDED.examples,
    default_value = EXCLUDED.default_value,
    relationships = EXCLUDED.relationships,
    authoritative_definitions = EXCLUDED.authoritative_definitions,
    quality = EXCLUDED.quality,
    enum_values = EXCLUDED.enum_values,
    tags = EXCLUDED.tags,
    custom_properties = EXCLUDED.custom_properties,
    logical_type_options = EXCLUDED.logical_type_options,
    column_order = EXCLUDED.column_order,
    nested_data = EXCLUDED.nested_data
"#;

    pub const SELECT_BY_TABLE: &str =
        "SELECT * FROM columns WHERE table_id = $1 ORDER BY column_order";
    pub const DELETE_BY_TABLE: &str = "DELETE FROM columns WHERE table_id = $1";
    pub const COUNT_BY_WORKSPACE: &str = r#"
SELECT COUNT(*) as count FROM columns c
JOIN tables t ON c.table_id = t.id
WHERE t.workspace_id = $1
"#;
}

/// SQL for inserting/updating relationships
pub mod relationship_sql {
    pub const UPSERT: &str = r#"
INSERT INTO relationships (
    id, workspace_id, source_table_id, target_table_id, cardinality,
    source_optional, target_optional, relationship_type, notes, owner,
    infrastructure_type, etl_job_name, etl_job_frequency, foreign_key_details,
    visual_metadata, sla, contact_details, drawio_edge_id, color, created_at, updated_at
)
VALUES (
    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
)
ON CONFLICT (id) DO UPDATE SET
    workspace_id = EXCLUDED.workspace_id,
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
"#;

    pub const SELECT_BY_WORKSPACE: &str = "SELECT * FROM relationships WHERE workspace_id = $1";
    pub const DELETE_BY_WORKSPACE: &str = "DELETE FROM relationships WHERE workspace_id = $1";
    pub const COUNT_BY_WORKSPACE: &str =
        "SELECT COUNT(*) as count FROM relationships WHERE workspace_id = $1";
}

/// SQL for file hash operations
pub mod file_hash_sql {
    pub const UPSERT: &str = r#"
INSERT INTO file_hashes (workspace_id, file_path, hash, last_synced_at)
VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
ON CONFLICT (workspace_id, file_path) DO UPDATE SET
    hash = EXCLUDED.hash,
    last_synced_at = CURRENT_TIMESTAMP
"#;

    pub const SELECT: &str =
        "SELECT hash FROM file_hashes WHERE workspace_id = $1 AND file_path = $2";
    pub const DELETE_BY_WORKSPACE: &str = "DELETE FROM file_hashes WHERE workspace_id = $1";
}

/// SQL for sync log operations
pub mod sync_log_sql {
    pub const INSERT: &str = r#"
INSERT INTO sync_log (workspace_id, sync_started_at, trigger)
VALUES ($1, CURRENT_TIMESTAMP, $2)
RETURNING id
"#;

    pub const UPDATE_COMPLETED: &str = r#"
UPDATE sync_log SET
    sync_completed_at = CURRENT_TIMESTAMP,
    tables_synced = $2,
    columns_synced = $3,
    relationships_synced = $4,
    domains_synced = $5,
    errors = $6
WHERE id = $1
"#;

    pub const SELECT_LATEST: &str = r#"
SELECT * FROM sync_log
WHERE workspace_id = $1
ORDER BY sync_started_at DESC
LIMIT 1
"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_sql_not_empty() {
        assert!(!DatabaseSchema::create_tables_sql().is_empty());
        assert!(!DatabaseSchema::create_indexes_sql().is_empty());
    }

    #[test]
    fn test_schema_version() {
        // Verify schema version is a positive integer
        assert_eq!(SCHEMA_VERSION, 1);
    }
}
