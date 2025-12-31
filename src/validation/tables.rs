//! Table validation functionality
//!
//! Validates tables for naming conflicts, pattern exclusivity, etc.
//!
//! This module implements SDK-native validation against SDK models.

use crate::models::Table;
use uuid::Uuid;

/// Result of table validation.
///
/// Contains any naming conflicts or pattern violations found during validation.
#[derive(Debug)]
#[must_use = "validation results should be checked for conflicts and violations"]
pub struct TableValidationResult {
    /// Naming conflicts found
    pub naming_conflicts: Vec<NamingConflict>,
    /// Pattern exclusivity violations
    pub pattern_violations: Vec<PatternViolation>,
}

/// Naming conflict between two tables
#[derive(Debug, Clone)]
pub struct NamingConflict {
    pub new_table_id: Uuid,
    pub new_table_name: String,
    pub existing_table_id: Uuid,
    pub existing_table_name: String,
}

/// Pattern exclusivity violation
#[derive(Debug, Clone)]
pub struct PatternViolation {
    pub table_id: Uuid,
    pub table_name: String,
    pub message: String,
}

/// Error during table validation
#[derive(Debug, thiserror::Error)]
pub enum TableValidationError {
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Table validator
#[derive(Default)]
pub struct TableValidator;

impl TableValidator {
    /// Create a new table validator
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::validation::tables::TableValidator;
    ///
    /// let validator = TableValidator::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Detect naming conflicts between new tables and existing tables
    ///
    /// The logic checks for conflicts using unique keys:
    /// (database_type, name, catalog_name, schema_name)
    ///
    /// # Arguments
    ///
    /// * `existing_tables` - Tables that already exist
    /// * `new_tables` - New tables to check for conflicts
    ///
    /// # Returns
    ///
    /// A vector of `NamingConflict` structs for each conflict found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::validation::tables::TableValidator;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let validator = TableValidator::new();
    /// let existing = vec![Table::new("users".to_string(), vec![])];
    /// let new_tables = vec![Table::new("users".to_string(), vec![])];
    ///
    /// let conflicts = validator.detect_naming_conflicts(&existing, &new_tables);
    /// assert_eq!(conflicts.len(), 1);
    /// ```
    pub fn detect_naming_conflicts(
        &self,
        existing_tables: &[Table],
        new_tables: &[Table],
    ) -> Vec<NamingConflict> {
        let mut conflicts = Vec::new();

        // Build a map of existing tables by unique key
        let mut existing_map = std::collections::HashMap::new();
        for table in existing_tables {
            let key = table.get_unique_key();
            existing_map.insert(key, table);
        }

        // Check new tables against existing
        for new_table in new_tables {
            let key = new_table.get_unique_key();

            if let Some(existing) = existing_map.get(&key) {
                conflicts.push(NamingConflict {
                    new_table_id: new_table.id,
                    new_table_name: new_table.name.clone(),
                    existing_table_id: existing.id,
                    existing_table_name: existing.name.clone(),
                });
            }
        }

        conflicts
    }

    /// Validate pattern exclusivity (SCD pattern and Data Vault classification are mutually exclusive)
    ///
    /// # Arguments
    ///
    /// * `table` - The table to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(PatternViolation)` if both SCD pattern and Data Vault classification are set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::validation::tables::TableValidator;
    /// use data_modelling_sdk::models::{Table, Column};
    /// use data_modelling_sdk::models::enums::{SCDPattern, DataVaultClassification};
    ///
    /// let validator = TableValidator::new();
    /// let mut table = Table::new("test".to_string(), vec![]);
    /// table.scd_pattern = Some(SCDPattern::Type2);
    /// table.data_vault_classification = Some(DataVaultClassification::Hub);
    ///
    /// let result = validator.validate_pattern_exclusivity(&table);
    /// assert!(result.is_err());
    /// ```
    pub fn validate_pattern_exclusivity(
        &self,
        table: &Table,
    ) -> std::result::Result<(), PatternViolation> {
        if table.scd_pattern.is_some() && table.data_vault_classification.is_some() {
            return Err(PatternViolation {
                table_id: table.id,
                table_name: table.name.clone(),
                message: "SCD pattern and Data Vault classification are mutually exclusive"
                    .to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::column::Column;
    use crate::models::table::Table as SdkTable;

    #[test]
    fn detects_naming_conflicts_using_unique_key() {
        let t1 = SdkTable::new(
            "users".to_string(),
            vec![Column::new("id".to_string(), "int".to_string())],
        );
        let t2 = SdkTable {
            id: Uuid::new_v4(),
            ..t1.clone()
        };

        let v = TableValidator::new().detect_naming_conflicts(&[t1], &[t2]);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].new_table_name, "users");
    }

    #[test]
    fn enforces_pattern_exclusivity() {
        let mut t = SdkTable::new("t".to_string(), vec![]);
        t.scd_pattern = Some(crate::models::enums::SCDPattern::Type2);
        t.data_vault_classification = Some(crate::models::enums::DataVaultClassification::Hub);

        let err = TableValidator::new()
            .validate_pattern_exclusivity(&t)
            .unwrap_err();
        assert!(err.message.contains("mutually exclusive"));
    }
}
