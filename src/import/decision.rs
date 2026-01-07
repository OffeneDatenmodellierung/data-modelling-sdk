//! Decision (MADR) importer
//!
//! Parses MADR-compliant decision YAML files (.madr.yaml) and converts them to Decision models.
//! Also handles the decisions index file (decisions.yaml).

use super::ImportError;
use crate::models::decision::{Decision, DecisionIndex};

#[cfg(feature = "schema-validation")]
use crate::validation::schema::validate_decision_internal;

/// Decision importer for parsing MADR-compliant YAML files
pub struct DecisionImporter;

impl DecisionImporter {
    /// Create a new Decision importer instance
    pub fn new() -> Self {
        Self
    }

    /// Import a decision from YAML content
    ///
    /// Optionally validates against the JSON schema if the `schema-validation` feature is enabled.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Decision YAML content as a string
    ///
    /// # Returns
    ///
    /// A `Decision` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::decision::DecisionImporter;
    ///
    /// let importer = DecisionImporter::new();
    /// let yaml = r#"
    /// id: 550e8400-e29b-41d4-a716-446655440000
    /// number: 1
    /// title: "Use ODCS Format for Data Contracts"
    /// status: accepted
    /// category: datadesign
    /// date: "2024-01-15T10:00:00Z"
    /// context: "We need a standard format for data contracts."
    /// decision: "Use ODCS v3.1.0 format."
    /// created_at: "2024-01-15T10:00:00Z"
    /// updated_at: "2024-01-15T10:00:00Z"
    /// "#;
    /// let decision = importer.import(yaml).unwrap();
    /// assert_eq!(decision.title, "Use ODCS Format for Data Contracts");
    /// ```
    pub fn import(&self, yaml_content: &str) -> Result<Decision, ImportError> {
        // Validate against JSON Schema if feature is enabled
        #[cfg(feature = "schema-validation")]
        {
            validate_decision_internal(yaml_content).map_err(ImportError::ValidationError)?;
        }

        // Parse the YAML content
        Decision::from_yaml(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse decision YAML: {}", e)))
    }

    /// Import a decision without schema validation
    ///
    /// Use this when you want to skip schema validation for performance
    /// or when importing from a trusted source.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Decision YAML content as a string
    ///
    /// # Returns
    ///
    /// A `Decision` parsed from the YAML content
    pub fn import_without_validation(&self, yaml_content: &str) -> Result<Decision, ImportError> {
        Decision::from_yaml(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse decision YAML: {}", e)))
    }

    /// Import a decisions index from YAML content
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Decisions index YAML content (decisions.yaml)
    ///
    /// # Returns
    ///
    /// A `DecisionIndex` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::decision::DecisionImporter;
    ///
    /// let importer = DecisionImporter::new();
    /// let yaml = r#"
    /// schema_version: "1.0"
    /// decisions: []
    /// next_number: 1
    /// "#;
    /// let index = importer.import_index(yaml).unwrap();
    /// assert_eq!(index.next_number, 1);
    /// ```
    pub fn import_index(&self, yaml_content: &str) -> Result<DecisionIndex, ImportError> {
        DecisionIndex::from_yaml(yaml_content).map_err(|e| {
            ImportError::ParseError(format!("Failed to parse decisions index YAML: {}", e))
        })
    }

    /// Import multiple decisions from a directory
    ///
    /// Loads all `.madr.yaml` files from the specified directory.
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory containing decision files
    ///
    /// # Returns
    ///
    /// A vector of parsed `Decision` objects and any import errors
    pub fn import_from_directory(
        &self,
        dir_path: &std::path::Path,
    ) -> Result<(Vec<Decision>, Vec<ImportError>), ImportError> {
        let mut decisions = Vec::new();
        let mut errors = Vec::new();

        if !dir_path.exists() {
            return Err(ImportError::IoError(format!(
                "Directory does not exist: {}",
                dir_path.display()
            )));
        }

        if !dir_path.is_dir() {
            return Err(ImportError::IoError(format!(
                "Path is not a directory: {}",
                dir_path.display()
            )));
        }

        // Read all .madr.yaml files
        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| ImportError::IoError(format!("Failed to read directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|name| name.ends_with(".madr.yaml"))
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match self.import(&content) {
                        Ok(decision) => decisions.push(decision),
                        Err(e) => errors.push(ImportError::ParseError(format!(
                            "Failed to import {}: {}",
                            path.display(),
                            e
                        ))),
                    },
                    Err(e) => errors.push(ImportError::IoError(format!(
                        "Failed to read {}: {}",
                        path.display(),
                        e
                    ))),
                }
            }
        }

        // Sort decisions by number
        decisions.sort_by_key(|d| d.number);

        Ok((decisions, errors))
    }
}

impl Default for DecisionImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_decision() {
        let importer = DecisionImporter::new();
        let yaml = r#"
id: 550e8400-e29b-41d4-a716-446655440000
number: 1
title: "Use ODCS Format for Data Contracts"
status: accepted
category: datadesign
date: "2024-01-15T10:00:00Z"
context: "We need a standard format for data contracts."
decision: "Use ODCS v3.1.0 format."
created_at: "2024-01-15T10:00:00Z"
updated_at: "2024-01-15T10:00:00Z"
"#;
        let result = importer.import_without_validation(yaml);
        assert!(result.is_ok());
        let decision = result.unwrap();
        assert_eq!(decision.title, "Use ODCS Format for Data Contracts");
        assert_eq!(decision.number, 1);
    }

    #[test]
    fn test_import_decision_index() {
        let importer = DecisionImporter::new();
        let yaml = r#"
schema_version: "1.0"
decisions: []
next_number: 1
"#;
        let result = importer.import_index(yaml);
        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.next_number, 1);
        assert_eq!(index.schema_version, "1.0");
    }

    #[test]
    fn test_import_invalid_yaml() {
        let importer = DecisionImporter::new();
        let yaml = "not: valid: yaml: at: all";
        let result = importer.import_without_validation(yaml);
        assert!(result.is_err());
    }
}
