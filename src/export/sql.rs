//! SQL exporter for generating CREATE TABLE statements from data models.

use crate::models::{DataModel, Table};
use crate::export::{ExportResult, ExportError};

/// Exporter for SQL CREATE TABLE format.
pub struct SQLExporter;

impl SQLExporter {
    /// Export a table to SQL CREATE TABLE statement.
    pub fn export_table(table: &Table, dialect: Option<&str>) -> String {
        let dialect = dialect.unwrap_or("standard");
        let mut sql = String::new();

        // CREATE TABLE statement
        sql.push_str(&format!(
            "CREATE TABLE {}",
            Self::quote_identifier(&table.name, dialect)
        ));

        // Add catalog and schema if specified
        if let Some(ref catalog) = table.catalog_name {
            sql = format!(
                "CREATE TABLE {}.{}",
                Self::quote_identifier(catalog, dialect),
                Self::quote_identifier(&table.name, dialect)
            );
        }

        if let Some(ref schema) = table.schema_name {
            if table.catalog_name.is_none() {
                sql = format!(
                    "CREATE TABLE {}.{}",
                    Self::quote_identifier(schema, dialect),
                    Self::quote_identifier(&table.name, dialect)
                );
            } else {
                sql = format!(
                    "CREATE TABLE {}.{}.{}",
                    Self::quote_identifier(table.catalog_name.as_ref().unwrap(), dialect),
                    Self::quote_identifier(schema, dialect),
                    Self::quote_identifier(&table.name, dialect)
                );
            }
        }

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
    pub fn export(&self, tables: &[Table], dialect: Option<&str>) -> Result<ExportResult, ExportError> {
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

    /// Quote identifier based on SQL dialect.
    fn quote_identifier(identifier: &str, dialect: &str) -> String {
        match dialect {
            "mysql" => format!("`{}`", identifier),
            "postgres" | "postgresql" => format!("\"{}\"", identifier),
            "sqlserver" | "mssql" => format!("[{}]", identifier),
            _ => identifier.to_string(), // Standard SQL doesn't require quoting for simple identifiers
        }
    }
}
