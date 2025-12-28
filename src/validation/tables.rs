//! Table validation functionality
//! 
//! Validates tables for naming conflicts, pattern exclusivity, etc.
//! 
//! This module wraps validation logic from the parent crate.
//! In a full migration, the validation logic would be moved here.

use anyhow::Result;
use uuid::Uuid;

/// Result of table validation
#[derive(Debug)]
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
pub struct TableValidator;

impl TableValidator {
    /// Create a new table validator
    pub fn new() -> Self {
        Self
    }

    /// Detect naming conflicts between new tables and existing tables
    /// 
    /// This wraps ModelService::detect_naming_conflicts from the parent crate.
    /// In a full migration, the logic would be moved here.
    /// 
    /// The logic checks for conflicts using unique keys:
    /// (database_type, name, catalog_name, schema_name)
    pub fn detect_naming_conflicts(
        &self,
        existing_tables: &[TableData],
        new_tables: &[TableData],
    ) -> Vec<NamingConflict> {
        // Placeholder - will delegate to parent crate logic
        // TODO: Migrate detect_naming_conflicts logic from rust/src/api/services/model_service.rs
        
        let mut conflicts = Vec::new();
        
        // Build a map of existing tables by unique key
        let mut existing_map = std::collections::HashMap::new();
        for table in existing_tables {
            let key = (
                table.database_type.clone(),
                table.name.clone(),
                table.catalog_name.clone(),
                table.schema_name.clone(),
            );
            existing_map.insert(key, table);
        }
        
        // Check new tables against existing
        for new_table in new_tables {
            let key = (
                new_table.database_type.clone(),
                new_table.name.clone(),
                new_table.catalog_name.clone(),
                new_table.schema_name.clone(),
            );
            
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
    pub fn validate_pattern_exclusivity(&self, table: &TableData) -> Result<(), PatternViolation> {
        // Placeholder - will implement actual validation
        // TODO: Migrate validate_pattern_exclusivity logic from rust/src/api/models/table.rs
        
        if table.scd_pattern.is_some() && table.data_vault_classification.is_some() {
            return Err(PatternViolation {
                table_id: table.id,
                table_name: table.name.clone(),
                message: "SCD pattern and Data Vault classification are mutually exclusive".to_string(),
            });
        }
        
        Ok(())
    }
}

// Placeholder types - these would use actual model types
#[derive(Debug, Clone)]
pub struct TableData {
    pub id: Uuid,
    pub name: String,
    pub database_type: Option<String>,
    pub catalog_name: Option<String>,
    pub schema_name: Option<String>,
    pub scd_pattern: Option<String>,
    pub data_vault_classification: Option<String>,
}
