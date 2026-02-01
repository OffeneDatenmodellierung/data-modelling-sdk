//! Sketch importer
//!
//! Parses Excalidraw sketch YAML files (.sketch.yaml) and converts them to Sketch models.
//! Also handles the sketch index file (sketches.yaml).

use super::ImportError;
use crate::models::sketch::{Sketch, SketchIndex};

#[cfg(feature = "schema-validation")]
use crate::validation::schema::validate_sketch_internal;

/// Sketch importer for parsing Excalidraw sketch YAML files
pub struct SketchImporter;

impl SketchImporter {
    /// Create a new Sketch importer instance
    pub fn new() -> Self {
        Self
    }

    /// Import a sketch from YAML content
    ///
    /// Optionally validates against the JSON schema if the `schema-validation` feature is enabled.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Sketch YAML content as a string
    ///
    /// # Returns
    ///
    /// A `Sketch` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::import::sketch::SketchImporter;
    ///
    /// let importer = SketchImporter::new();
    /// let yaml = r#"
    /// id: 770e8400-e29b-41d4-a716-446655440001
    /// number: 1
    /// title: "Architecture Diagram"
    /// sketchType: architecture
    /// status: published
    /// excalidrawData: "{}"
    /// createdAt: "2024-01-15T10:00:00Z"
    /// updatedAt: "2024-01-15T10:00:00Z"
    /// "#;
    /// let sketch = importer.import(yaml).unwrap();
    /// assert_eq!(sketch.title, "Architecture Diagram");
    /// ```
    pub fn import(&self, yaml_content: &str) -> Result<Sketch, ImportError> {
        // Validate against JSON Schema if feature is enabled
        #[cfg(feature = "schema-validation")]
        {
            validate_sketch_internal(yaml_content).map_err(ImportError::ValidationError)?;
        }

        // Parse the YAML content
        Sketch::from_yaml(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse sketch YAML: {}", e)))
    }

    /// Import a sketch without schema validation
    ///
    /// Use this when you want to skip schema validation for performance
    /// or when importing from a trusted source.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Sketch YAML content as a string
    ///
    /// # Returns
    ///
    /// A `Sketch` parsed from the YAML content
    pub fn import_without_validation(&self, yaml_content: &str) -> Result<Sketch, ImportError> {
        Sketch::from_yaml(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse sketch YAML: {}", e)))
    }

    /// Import a sketch index from YAML content
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Sketch index YAML content (sketches.yaml)
    ///
    /// # Returns
    ///
    /// A `SketchIndex` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::import::sketch::SketchImporter;
    ///
    /// let importer = SketchImporter::new();
    /// let yaml = r#"
    /// schemaVersion: "1.0"
    /// sketches: []
    /// nextNumber: 1
    /// "#;
    /// let index = importer.import_index(yaml).unwrap();
    /// assert_eq!(index.next_number, 1);
    /// ```
    pub fn import_index(&self, yaml_content: &str) -> Result<SketchIndex, ImportError> {
        SketchIndex::from_yaml(yaml_content).map_err(|e| {
            ImportError::ParseError(format!("Failed to parse sketch index YAML: {}", e))
        })
    }

    /// Import multiple sketches from a directory
    ///
    /// Loads all `.sketch.yaml` files from the specified directory.
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory containing sketch files
    ///
    /// # Returns
    ///
    /// A vector of parsed `Sketch` objects and any import errors
    pub fn import_from_directory(
        &self,
        dir_path: &std::path::Path,
    ) -> Result<(Vec<Sketch>, Vec<ImportError>), ImportError> {
        let mut sketches = Vec::new();
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

        // Read all .sketch.yaml files
        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| ImportError::IoError(format!("Failed to read directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|name| name.ends_with(".sketch.yaml"))
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match self.import(&content) {
                        Ok(sketch) => sketches.push(sketch),
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

        // Sort sketches by number
        sketches.sort_by(|a, b| a.number.cmp(&b.number));

        Ok((sketches, errors))
    }

    /// Import sketches filtered by domain
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory containing sketch files
    /// * `domain` - Domain to filter by
    ///
    /// # Returns
    ///
    /// A vector of parsed `Sketch` objects for the specified domain
    pub fn import_by_domain(
        &self,
        dir_path: &std::path::Path,
        domain: &str,
    ) -> Result<(Vec<Sketch>, Vec<ImportError>), ImportError> {
        let (sketches, errors) = self.import_from_directory(dir_path)?;

        let filtered: Vec<Sketch> = sketches
            .into_iter()
            .filter(|s| s.domain.as_deref() == Some(domain))
            .collect();

        Ok((filtered, errors))
    }
}

impl Default for SketchImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_sketch() {
        let importer = SketchImporter::new();
        let yaml = r#"
id: 770e8400-e29b-41d4-a716-446655440001
number: 1
title: "Architecture Diagram"
sketchType: architecture
status: published
excalidrawData: "{}"
createdAt: "2024-01-15T10:00:00Z"
updatedAt: "2024-01-15T10:00:00Z"
"#;
        let result = importer.import_without_validation(yaml);
        assert!(result.is_ok());
        let sketch = result.unwrap();
        assert_eq!(sketch.title, "Architecture Diagram");
        assert_eq!(sketch.number, 1);
    }

    #[test]
    fn test_import_sketch_index() {
        let importer = SketchImporter::new();
        let yaml = r#"
schemaVersion: "1.0"
sketches: []
nextNumber: 1
"#;
        let result = importer.import_index(yaml);
        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.next_number, 1);
        assert_eq!(index.schema_version, "1.0");
    }

    #[test]
    fn test_import_invalid_yaml() {
        let importer = SketchImporter::new();
        let yaml = "not: valid: yaml: at: all";
        let result = importer.import_without_validation(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_sketch_with_all_fields() {
        let importer = SketchImporter::new();
        let yaml = r#"
id: 770e8400-e29b-41d4-a716-446655440001
number: 1
title: "Sales Domain Architecture"
sketchType: architecture
status: published
domain: sales
description: "High-level architecture diagram"
excalidrawData: '{"elements":[]}'
thumbnailPath: thumbnails/sketch-0001.png
authors:
  - architect@company.com
tags:
  - architecture
  - sales
createdAt: "2024-01-15T10:00:00Z"
updatedAt: "2024-01-15T10:00:00Z"
"#;
        let result = importer.import_without_validation(yaml);
        assert!(result.is_ok());
        let sketch = result.unwrap();
        assert_eq!(sketch.title, "Sales Domain Architecture");
        assert_eq!(sketch.domain, Some("sales".to_string()));
        assert_eq!(
            sketch.thumbnail_path,
            Some("thumbnails/sketch-0001.png".to_string())
        );
        assert_eq!(sketch.authors.len(), 1);
    }
}
