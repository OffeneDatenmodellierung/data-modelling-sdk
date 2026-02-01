//! Sketch exporter
//!
//! Exports Sketch models to YAML format.

use crate::export::ExportError;
use crate::models::sketch::{Sketch, SketchIndex};

/// Sketch exporter for generating YAML from Sketch models
pub struct SketchExporter;

impl SketchExporter {
    /// Create a new Sketch exporter instance
    pub fn new() -> Self {
        Self
    }

    /// Export a sketch to YAML format
    ///
    /// # Arguments
    ///
    /// * `sketch` - The Sketch to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string, or an ExportError
    pub fn export(&self, sketch: &Sketch) -> Result<String, ExportError> {
        let yaml = sketch.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize sketch: {}", e))
        })?;

        // Validate exported YAML against sketch schema (if feature enabled)
        #[cfg(feature = "schema-validation")]
        {
            use crate::validation::schema::validate_sketch_internal;
            validate_sketch_internal(&yaml).map_err(ExportError::ValidationError)?;
        }

        Ok(yaml)
    }

    /// Export a sketch without validation
    ///
    /// Use this when you want to skip schema validation for performance
    /// or when exporting to a trusted destination.
    pub fn export_without_validation(&self, sketch: &Sketch) -> Result<String, ExportError> {
        sketch.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize sketch: {}", e))
        })
    }

    /// Export a sketch index to YAML format
    ///
    /// # Arguments
    ///
    /// * `index` - The SketchIndex to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string, or an ExportError
    pub fn export_index(&self, index: &SketchIndex) -> Result<String, ExportError> {
        index.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize sketch index: {}", e))
        })
    }

    /// Export multiple sketches to a directory
    ///
    /// # Arguments
    ///
    /// * `sketches` - The sketches to export
    /// * `dir_path` - Directory to export to
    /// * `workspace_name` - Workspace name for filename generation
    ///
    /// # Returns
    ///
    /// A Result with the number of files exported, or an ExportError
    pub fn export_to_directory(
        &self,
        sketches: &[Sketch],
        dir_path: &std::path::Path,
        workspace_name: &str,
    ) -> Result<usize, ExportError> {
        // Create directory if it doesn't exist
        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path)
                .map_err(|e| ExportError::IoError(format!("Failed to create directory: {}", e)))?;
        }

        let mut count = 0;
        for sketch in sketches {
            let filename = sketch.filename(workspace_name);
            let path = dir_path.join(&filename);
            let yaml = self.export(sketch)?;
            std::fs::write(&path, yaml).map_err(|e| {
                ExportError::IoError(format!("Failed to write {}: {}", filename, e))
            })?;
            count += 1;
        }

        Ok(count)
    }

    /// Export sketches filtered by domain to a directory
    ///
    /// # Arguments
    ///
    /// * `sketches` - The sketches to export
    /// * `dir_path` - Directory to export to
    /// * `workspace_name` - Workspace name for filename generation
    /// * `domain` - Domain to filter by
    ///
    /// # Returns
    ///
    /// A Result with the number of files exported, or an ExportError
    pub fn export_domain_to_directory(
        &self,
        sketches: &[Sketch],
        dir_path: &std::path::Path,
        workspace_name: &str,
        domain: &str,
    ) -> Result<usize, ExportError> {
        let filtered: Vec<&Sketch> = sketches
            .iter()
            .filter(|s| s.domain.as_deref() == Some(domain))
            .collect();

        // Create domain subdirectory
        let domain_dir = dir_path.join(domain);
        if !domain_dir.exists() {
            std::fs::create_dir_all(&domain_dir)
                .map_err(|e| ExportError::IoError(format!("Failed to create directory: {}", e)))?;
        }

        let mut count = 0;
        for sketch in filtered {
            let filename = sketch.filename(workspace_name);
            let path = domain_dir.join(&filename);
            let yaml = self.export(sketch)?;
            std::fs::write(&path, yaml).map_err(|e| {
                ExportError::IoError(format!("Failed to write {}: {}", filename, e))
            })?;
            count += 1;
        }

        Ok(count)
    }
}

impl Default for SketchExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sketch::SketchStatus;

    #[test]
    fn test_export_sketch() {
        let sketch = Sketch::new(1, "Architecture Diagram", r#"{"elements":[]}"#)
            .with_status(SketchStatus::Published);

        let exporter = SketchExporter::new();
        let result = exporter.export_without_validation(&sketch);
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("title: Architecture Diagram"));
        assert!(yaml.contains("status: published"));
    }

    #[test]
    fn test_export_sketch_index() {
        let index = SketchIndex::new();
        let exporter = SketchExporter::new();
        let result = exporter.export_index(&index);
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("schemaVersion"));
        assert!(yaml.contains("nextNumber: 1"));
    }

    #[test]
    fn test_export_sketch_with_all_fields() {
        let sketch = Sketch::new(1, "Sales Domain Architecture", r#"{"elements":[]}"#)
            .with_status(SketchStatus::Published)
            .with_domain("sales")
            .with_description("High-level architecture diagram")
            .with_thumbnail("thumbnails/sketch-0001.png")
            .add_author("architect@company.com");

        let exporter = SketchExporter::new();
        let result = exporter.export_without_validation(&sketch);
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("domain: sales"));
        assert!(yaml.contains("thumbnailPath: thumbnails/sketch-0001.png"));
    }
}
