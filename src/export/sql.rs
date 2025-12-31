//! SQL exporter for generating CREATE TABLE statements from data models.
//!
//! # Security
//!
//! All identifiers (table names, column names, schema names) are properly quoted
//! and escaped to prevent SQL injection. Internal quote characters are escaped
//! by doubling them according to SQL standards.

use crate::export::{ExportError, ExportResult};
use crate::models::{DataModel, Table};

/// Exporter for SQL CREATE TABLE format.
pub struct SQLExporter;

impl SQLExporter {
    /// Export a table to SQL CREATE TABLE statement.
    ///
    /// # Arguments
    ///
    /// * `table` - The table to export
    /// * `dialect` - Optional SQL dialect ("postgres", "mysql", "sqlserver", etc.)
    ///
    /// # Returns
    ///
    /// A SQL CREATE TABLE statement as a string, with proper identifier quoting
    /// and escaping based on the dialect.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::sql::SQLExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let table = Table::new(
    ///     "users".to_string(),
    ///     vec![Column::new("id".to_string(), "INT".to_string())],
    /// );
    ///
    /// let sql = SQLExporter::export_table(&table, Some("postgres"));
    /// // Returns: CREATE TABLE "users" (\n  "id" INT\n);
    /// ```
    pub fn export_table(table: &Table, dialect: Option<&str>) -> String {
        let dialect = dialect.unwrap_or("standard");
        let mut sql = String::new();

        // CREATE TABLE statement
        sql.push_str(&format!(
            "CREATE TABLE {}",
            Self::quote_identifier(&table.name, dialect)
        ));

        // Build fully-qualified table name based on catalog and schema
        sql = match (&table.catalog_name, &table.schema_name) {
            (Some(catalog), Some(schema)) => {
                format!(
                    "CREATE TABLE {}.{}.{}",
                    Self::quote_identifier(catalog, dialect),
                    Self::quote_identifier(schema, dialect),
                    Self::quote_identifier(&table.name, dialect)
                )
            }
            (Some(catalog), None) => {
                format!(
                    "CREATE TABLE {}.{}",
                    Self::quote_identifier(catalog, dialect),
                    Self::quote_identifier(&table.name, dialect)
                )
            }
            (None, Some(schema)) => {
                format!(
                    "CREATE TABLE {}.{}",
                    Self::quote_identifier(schema, dialect),
                    Self::quote_identifier(&table.name, dialect)
                )
            }
            (None, None) => sql, // Keep default "CREATE TABLE tablename"
        };

        sql.push_str(" (\n");

        // Column definitions
        let mut column_defs = Vec::new();
        for column in &table.columns {
            let mut col_def = format!("  {}", Self::quote_identifier(&column.name, dialect));
            col_def.push(' ');
            col_def.push_str(&column.data_type);

            if !column.nullable {
                col_def.push_str(" NOT NULL");
            }

            if column.primary_key {
                col_def.push_str(" PRIMARY KEY");
            }

            if !column.description.is_empty() {
                // Add comment (dialect-specific)
                match dialect {
                    "postgres" | "postgresql" => {
                        col_def.push_str(&format!(" -- {}", column.description));
                    }
                    "mysql" => {
                        col_def.push_str(&format!(
                            " COMMENT '{}'",
                            column.description.replace('\'', "''")
                        ));
                    }
                    _ => {
                        col_def.push_str(&format!(" -- {}", column.description));
                    }
                }
            }

            column_defs.push(col_def);
        }

        sql.push_str(&column_defs.join(",\n"));
        sql.push_str("\n);\n");

        // Add table comment if available (from odcl_metadata)
        if let Some(desc) = table
            .odcl_metadata
            .get("description")
            .and_then(|v| v.as_str())
        {
            match dialect {
                "postgres" | "postgresql" => {
                    sql.push_str(&format!(
                        "COMMENT ON TABLE {} IS '{}';\n",
                        Self::quote_identifier(&table.name, dialect),
                        desc.replace('\'', "''")
                    ));
                }
                "mysql" => {
                    sql.push_str(&format!(
                        "ALTER TABLE {} COMMENT = '{}';\n",
                        Self::quote_identifier(&table.name, dialect),
                        desc.replace("'", "''")
                    ));
                }
                _ => {
                    // Default: SQL comment
                    sql.push_str(&format!("-- Table: {}\n", table.name));
                    sql.push_str(&format!("-- Description: {}\n", desc));
                }
            }
        }

        sql
    }

    /// Export tables to SQL CREATE TABLE statements (SDK interface).
    ///
    /// # Arguments
    ///
    /// * `tables` - Slice of tables to export
    /// * `dialect` - Optional SQL dialect
    ///
    /// # Returns
    ///
    /// An `ExportResult` containing the SQL statements for all tables.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::sql::SQLExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let tables = vec![
    ///     Table::new("users".to_string(), vec![Column::new("id".to_string(), "INT".to_string())]),
    ///     Table::new("orders".to_string(), vec![Column::new("id".to_string(), "INT".to_string())]),
    /// ];
    ///
    /// let exporter = SQLExporter;
    /// let result = exporter.export(&tables, Some("postgres")).unwrap();
    /// assert_eq!(result.format, "sql");
    /// ```
    pub fn export(
        &self,
        tables: &[Table],
        dialect: Option<&str>,
    ) -> Result<ExportResult, ExportError> {
        let mut sql = String::new();
        for table in tables {
            sql.push_str(&Self::export_table(table, dialect));
            sql.push('\n');
        }
        Ok(ExportResult {
            content: sql,
            format: "sql".to_string(),
        })
    }

    /// Export a data model to SQL CREATE TABLE statements (legacy method for compatibility).
    pub fn export_model(
        model: &DataModel,
        table_ids: Option<&[uuid::Uuid]>,
        dialect: Option<&str>,
    ) -> String {
        let tables_to_export: Vec<&Table> = if let Some(ids) = table_ids {
            model
                .tables
                .iter()
                .filter(|t| ids.contains(&t.id))
                .collect()
        } else {
            model.tables.iter().collect()
        };

        let mut sql = String::new();

        for table in tables_to_export {
            sql.push_str(&Self::export_table(table, dialect));
            sql.push('\n');
        }

        sql
    }

    /// Quote and escape identifier based on SQL dialect.
    ///
    /// # Security
    ///
    /// This function properly escapes quote characters within the identifier
    /// by doubling them, preventing SQL injection attacks.
    ///
    /// # Dialects
    ///
    /// - **PostgreSQL**: Uses double quotes (`"identifier"`)
    /// - **MySQL**: Uses backticks (`` `identifier` ``)
    /// - **SQL Server**: Uses brackets (`[identifier]`)
    /// - **Standard SQL**: Uses double quotes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use data_modelling_sdk::export::sql::SQLExporter;
    ///
    /// // PostgreSQL style
    /// // let quoted = SQLExporter::quote_identifier("user-name", "postgres");
    /// // Returns: "user-name"
    ///
    /// // MySQL style
    /// // let quoted = SQLExporter::quote_identifier("user-name", "mysql");
    /// // Returns: `user-name`
    /// ```
    fn quote_identifier(identifier: &str, dialect: &str) -> String {
        match dialect {
            "mysql" => {
                // MySQL uses backticks; escape internal backticks by doubling
                format!("`{}`", identifier.replace('`', "``"))
            }
            "postgres" | "postgresql" => {
                // PostgreSQL uses double quotes; escape by doubling
                format!("\"{}\"", identifier.replace('"', "\"\""))
            }
            "sqlserver" | "mssql" => {
                // SQL Server uses brackets; escape ] by doubling
                format!("[{}]", identifier.replace(']', "]]"))
            }
            _ => {
                // Standard SQL: use double quotes
                format!("\"{}\"", identifier.replace('"', "\"\""))
            }
        }
    }
}
