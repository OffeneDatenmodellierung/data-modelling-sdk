//! Database configuration file support
//!
//! Handles parsing of `.data-model.toml` configuration files and
//! environment variable overrides.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{DatabaseError, DatabaseResult};

/// Default database filename for DuckDB
pub const DEFAULT_DUCKDB_FILENAME: &str = ".data-model.duckdb";

/// Default configuration filename
pub const CONFIG_FILENAME: &str = ".data-model.toml";

/// Environment variable for database backend
pub const ENV_DB_BACKEND: &str = "DATA_MODEL_DB_BACKEND";

/// Environment variable for DuckDB path
pub const ENV_DUCKDB_PATH: &str = "DATA_MODEL_DUCKDB_PATH";

/// Environment variable for PostgreSQL connection string
pub const ENV_POSTGRES_URL: &str = "DATA_MODEL_POSTGRES_URL";

/// Environment variable for PostgreSQL pool size
pub const ENV_POSTGRES_POOL_SIZE: &str = "DATA_MODEL_POSTGRES_POOL_SIZE";

/// Database backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseBackendType {
    /// DuckDB embedded database (default)
    #[default]
    DuckDB,
    /// PostgreSQL database
    Postgres,
}

impl std::str::FromStr for DatabaseBackendType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "duckdb" => Ok(DatabaseBackendType::DuckDB),
            "postgres" | "postgresql" => Ok(DatabaseBackendType::Postgres),
            _ => Err(format!(
                "Unknown database backend: {}. Use 'duckdb' or 'postgres'.",
                s
            )),
        }
    }
}

impl std::fmt::Display for DatabaseBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseBackendType::DuckDB => write!(f, "duckdb"),
            DatabaseBackendType::Postgres => write!(f, "postgres"),
        }
    }
}

/// Database configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSection {
    /// Database backend type
    #[serde(default)]
    pub backend: DatabaseBackendType,

    /// Path to DuckDB database file (relative to workspace)
    #[serde(default = "default_duckdb_path")]
    pub path: String,
}

fn default_duckdb_path() -> String {
    DEFAULT_DUCKDB_FILENAME.to_string()
}

impl Default for DatabaseSection {
    fn default() -> Self {
        Self {
            backend: DatabaseBackendType::default(),
            path: default_duckdb_path(),
        }
    }
}

/// PostgreSQL configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostgresSection {
    /// Connection string (e.g., "postgresql://user:pass@localhost/db")
    #[serde(default)]
    pub connection_string: Option<String>,

    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,
}

fn default_pool_size() -> usize {
    5
}

/// Sync configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSection {
    /// Enable automatic sync on file changes
    #[serde(default)]
    pub auto_sync: bool,

    /// Enable file watching
    #[serde(default)]
    pub watch: bool,
}

impl Default for SyncSection {
    fn default() -> Self {
        Self {
            auto_sync: true,
            watch: false,
        }
    }
}

/// Git integration configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSection {
    /// Enable Git hooks
    #[serde(default = "default_hooks_enabled")]
    pub hooks_enabled: bool,
}

fn default_hooks_enabled() -> bool {
    true
}

impl Default for GitSection {
    fn default() -> Self {
        Self {
            hooks_enabled: default_hooks_enabled(),
        }
    }
}

/// Main configuration structure
///
/// Represents the `.data-model.toml` configuration file format.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseSection,

    /// PostgreSQL-specific configuration
    #[serde(default)]
    pub postgres: PostgresSection,

    /// Sync configuration
    #[serde(default)]
    pub sync: SyncSection,

    /// Git integration configuration
    #[serde(default)]
    pub git: GitSection,
}

impl DatabaseConfig {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a DuckDB configuration
    pub fn duckdb(path: impl Into<String>) -> Self {
        Self {
            database: DatabaseSection {
                backend: DatabaseBackendType::DuckDB,
                path: path.into(),
            },
            ..Default::default()
        }
    }

    /// Create a PostgreSQL configuration
    pub fn postgres(connection_string: impl Into<String>) -> Self {
        Self {
            database: DatabaseSection {
                backend: DatabaseBackendType::Postgres,
                path: String::new(),
            },
            postgres: PostgresSection {
                connection_string: Some(connection_string.into()),
                pool_size: default_pool_size(),
            },
            ..Default::default()
        }
    }

    /// Load configuration from a workspace directory
    ///
    /// Looks for `.data-model.toml` in the workspace directory.
    /// Falls back to defaults if not found.
    pub fn load(workspace_path: &Path) -> DatabaseResult<Self> {
        let config_path = workspace_path.join(CONFIG_FILENAME);

        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| DatabaseError::IoError(format!("Failed to read config: {}", e)))?;

            Self::parse(&content)?
        } else {
            Self::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Parse configuration from TOML string
    pub fn parse(content: &str) -> DatabaseResult<Self> {
        toml::from_str(content)
            .map_err(|e| DatabaseError::ConfigError(format!("Failed to parse config: {}", e)))
    }

    /// Save configuration to a workspace directory
    pub fn save(&self, workspace_path: &Path) -> DatabaseResult<()> {
        let config_path = workspace_path.join(CONFIG_FILENAME);
        let content = self.to_toml()?;

        std::fs::write(&config_path, content)
            .map_err(|e| DatabaseError::IoError(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    /// Convert configuration to TOML string
    pub fn to_toml(&self) -> DatabaseResult<String> {
        toml::to_string_pretty(self).map_err(|e| {
            DatabaseError::SerializationError(format!("Failed to serialize config: {}", e))
        })
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        // Backend type
        if let Ok(backend) = std::env::var(ENV_DB_BACKEND)
            && let Ok(backend_type) = backend.parse()
        {
            self.database.backend = backend_type;
        }

        // DuckDB path
        if let Ok(path) = std::env::var(ENV_DUCKDB_PATH) {
            self.database.path = path;
        }

        // PostgreSQL connection string
        if let Ok(url) = std::env::var(ENV_POSTGRES_URL) {
            self.postgres.connection_string = Some(url);
        }

        // PostgreSQL pool size
        if let Ok(size) = std::env::var(ENV_POSTGRES_POOL_SIZE)
            && let Ok(size) = size.parse()
        {
            self.postgres.pool_size = size;
        }
    }

    /// Get the DuckDB database path for a workspace
    pub fn get_duckdb_path(&self, workspace_path: &Path) -> PathBuf {
        if self.database.path.is_empty() {
            workspace_path.join(DEFAULT_DUCKDB_FILENAME)
        } else if Path::new(&self.database.path).is_absolute() {
            PathBuf::from(&self.database.path)
        } else {
            workspace_path.join(&self.database.path)
        }
    }

    /// Get the PostgreSQL connection string
    pub fn get_postgres_connection_string(&self) -> Option<&str> {
        self.postgres.connection_string.as_deref()
    }

    /// Check if configuration exists in a workspace
    pub fn exists(workspace_path: &Path) -> bool {
        workspace_path.join(CONFIG_FILENAME).exists()
    }

    /// Check if database is initialized (config file exists)
    pub fn is_initialized(workspace_path: &Path) -> bool {
        let config_path = workspace_path.join(CONFIG_FILENAME);
        if !config_path.exists() {
            return false;
        }

        // Also check if the database file/connection is valid
        if let Ok(config) = Self::load(workspace_path) {
            match config.database.backend {
                DatabaseBackendType::DuckDB => config.get_duckdb_path(workspace_path).exists(),
                DatabaseBackendType::Postgres => config.postgres.connection_string.is_some(),
            }
        } else {
            false
        }
    }
}

/// Generate a sample configuration file content
pub fn sample_config() -> &'static str {
    r#"# Data Model SDK Configuration
# This file configures the database backend for the data modelling SDK.

[database]
# Database backend: "duckdb" (default) or "postgres"
backend = "duckdb"

# Path to DuckDB database file (relative to workspace, or absolute)
path = ".data-model.duckdb"

# PostgreSQL configuration (used when backend = "postgres")
[database.postgres]
# connection_string = "postgresql://user:password@localhost:5432/datamodel"
pool_size = 5

[sync]
# Enable automatic sync when files change
auto_sync = true

# Enable file watching (requires --watch flag)
watch = false

[git]
# Enable Git hooks for automatic database rebuild
hooks_enabled = true
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = DatabaseConfig::new();
        assert_eq!(config.database.backend, DatabaseBackendType::DuckDB);
        assert_eq!(config.database.path, DEFAULT_DUCKDB_FILENAME);
        assert!(config.sync.auto_sync);
        assert!(config.git.hooks_enabled);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[database]
backend = "duckdb"
path = "custom.duckdb"

[sync]
auto_sync = false

[git]
hooks_enabled = false
"#;
        let config = DatabaseConfig::parse(toml).unwrap();
        assert_eq!(config.database.backend, DatabaseBackendType::DuckDB);
        assert_eq!(config.database.path, "custom.duckdb");
        assert!(!config.sync.auto_sync);
        assert!(!config.git.hooks_enabled);
    }

    #[test]
    fn test_parse_postgres_config() {
        let toml = r#"
[database]
backend = "postgres"

[postgres]
connection_string = "postgresql://localhost/test"
pool_size = 10
"#;
        let config = DatabaseConfig::parse(toml).unwrap();
        assert_eq!(config.database.backend, DatabaseBackendType::Postgres);
        assert_eq!(
            config.postgres.connection_string,
            Some("postgresql://localhost/test".to_string())
        );
        assert_eq!(config.postgres.pool_size, 10);
    }

    #[test]
    fn test_to_toml() {
        let config = DatabaseConfig::duckdb("test.duckdb");
        let toml = config.to_toml().unwrap();
        assert!(toml.contains("duckdb"));
        assert!(toml.contains("test.duckdb"));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let config = DatabaseConfig::duckdb("my-db.duckdb");

        config.save(dir.path()).unwrap();
        assert!(dir.path().join(CONFIG_FILENAME).exists());

        let loaded = DatabaseConfig::load(dir.path()).unwrap();
        assert_eq!(loaded.database.path, "my-db.duckdb");
    }

    #[test]
    fn test_get_duckdb_path() {
        let config = DatabaseConfig::duckdb("relative.duckdb");
        let workspace = Path::new("/workspace");
        assert_eq!(
            config.get_duckdb_path(workspace),
            PathBuf::from("/workspace/relative.duckdb")
        );
    }

    #[test]
    fn test_backend_type_from_str() {
        assert_eq!(
            "duckdb".parse::<DatabaseBackendType>().unwrap(),
            DatabaseBackendType::DuckDB
        );
        assert_eq!(
            "postgres".parse::<DatabaseBackendType>().unwrap(),
            DatabaseBackendType::Postgres
        );
        assert_eq!(
            "postgresql".parse::<DatabaseBackendType>().unwrap(),
            DatabaseBackendType::Postgres
        );
        assert!("invalid".parse::<DatabaseBackendType>().is_err());
    }

    #[test]
    fn test_sample_config_is_valid() {
        let sample = sample_config();
        // Should parse without error
        let result = DatabaseConfig::parse(sample);
        assert!(result.is_ok(), "Sample config should be valid TOML");
    }
}
