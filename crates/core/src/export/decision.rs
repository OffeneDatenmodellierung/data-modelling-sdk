//! Decision (MADR) exporter
//!
//! Exports Decision models to MADR-compliant YAML format.

use crate::export::ExportError;
use crate::models::decision::{Decision, DecisionIndex};

/// Decision exporter for generating MADR-compliant YAML from Decision models
pub struct DecisionExporter;

impl DecisionExporter {
    /// Create a new Decision exporter instance
    pub fn new() -> Self {
        Self
    }

    /// Export a decision to MADR YAML format
    ///
    /// # Arguments
    ///
    /// * `decision` - The Decision to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string, or an ExportError
    pub fn export(&self, decision: &Decision) -> Result<String, ExportError> {
        let yaml = decision.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize decision: {}", e))
        })?;

        // Validate exported YAML against decision schema (if feature enabled)
        #[cfg(feature = "schema-validation")]
        {
            use crate::validation::schema::validate_decision_internal;
            validate_decision_internal(&yaml).map_err(ExportError::ValidationError)?;
        }

        Ok(yaml)
    }

    /// Export a decision without validation
    ///
    /// Use this when you want to skip schema validation for performance
    /// or when exporting to a trusted destination.
    pub fn export_without_validation(&self, decision: &Decision) -> Result<String, ExportError> {
        decision.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize decision: {}", e))
        })
    }

    /// Export a decisions index to YAML format
    ///
    /// # Arguments
    ///
    /// * `index` - The DecisionIndex to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string, or an ExportError
    pub fn export_index(&self, index: &DecisionIndex) -> Result<String, ExportError> {
        index.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize decision index: {}", e))
        })
    }

    /// Export multiple decisions to a directory
    ///
    /// # Arguments
    ///
    /// * `decisions` - The decisions to export
    /// * `dir_path` - Directory to export to
    /// * `workspace_name` - Workspace name for filename generation
    ///
    /// # Returns
    ///
    /// A Result with the number of files exported, or an ExportError
    pub fn export_to_directory(
        &self,
        decisions: &[Decision],
        dir_path: &std::path::Path,
        workspace_name: &str,
    ) -> Result<usize, ExportError> {
        // Create directory if it doesn't exist
        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path)
                .map_err(|e| ExportError::IoError(format!("Failed to create directory: {}", e)))?;
        }

        let mut count = 0;
        for decision in decisions {
            let filename = decision.filename(workspace_name);
            let path = dir_path.join(&filename);
            let yaml = self.export(decision)?;
            std::fs::write(&path, yaml).map_err(|e| {
                ExportError::IoError(format!("Failed to write {}: {}", filename, e))
            })?;
            count += 1;
        }

        Ok(count)
    }
}

impl Default for DecisionExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::decision::{DecisionCategory, DecisionStatus};

    #[test]
    fn test_export_decision() {
        let decision = Decision::new(
            1,
            "Use ODCS Format",
            "We need a standard format.",
            "Use ODCS v3.1.0.",
            "author@example.com",
        )
        .with_status(DecisionStatus::Accepted)
        .with_category(DecisionCategory::DataDesign);

        let exporter = DecisionExporter::new();
        let result = exporter.export_without_validation(&decision);
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("title: Use ODCS Format"));
        assert!(yaml.contains("status: accepted"));
    }

    #[test]
    fn test_export_decision_index() {
        let index = DecisionIndex::new();
        let exporter = DecisionExporter::new();
        let result = exporter.export_index(&index);
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("schema_version"));
        assert!(yaml.contains("next_number: 1"));
    }
}
