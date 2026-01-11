//! DMN exporter
//!
//! Provides functionality to export DMN models in their native XML format.

use crate::export::ExportError;

/// DMN Exporter
///
/// Exports DMN models in their native XML format.
#[derive(Debug, Default)]
pub struct DMNExporter;

impl DMNExporter {
    /// Create a new DMNExporter
    pub fn new() -> Self {
        Self
    }

    /// Export DMN model XML content
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The DMN XML content as a string.
    ///
    /// # Returns
    ///
    /// The XML content as a string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::export::dmn::DMNExporter;
    ///
    /// let exporter = DMNExporter::new();
    /// let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
    /// <dmn:definitions xmlns:dmn="https://www.omg.org/spec/DMN/20191111/MODEL/">
    ///   <!-- DMN content -->
    /// </dmn:definitions>"#;
    /// let exported = exporter.export(xml_content).unwrap();
    /// assert_eq!(exported, xml_content);
    /// ```
    pub fn export(&self, xml_content: &str) -> Result<String, ExportError> {
        // Since we store DMN models in their native XML format,
        // export is simply returning the content as-is
        Ok(xml_content.to_string())
    }
}
