//! BPMN exporter
//!
//! Provides functionality to export BPMN models in their native XML format.

use crate::export::ExportError;

/// BPMN Exporter
///
/// Exports BPMN models in their native XML format.
#[derive(Debug, Default)]
pub struct BPMNExporter;

impl BPMNExporter {
    /// Create a new BPMNExporter
    pub fn new() -> Self {
        Self
    }

    /// Export BPMN model XML content
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The BPMN XML content as a string.
    ///
    /// # Returns
    ///
    /// The XML content as a string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::export::bpmn::BPMNExporter;
    ///
    /// let exporter = BPMNExporter::new();
    /// let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
    /// <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
    ///   <!-- BPMN content -->
    /// </bpmn:definitions>"#;
    /// let exported = exporter.export(xml_content).unwrap();
    /// assert_eq!(exported, xml_content);
    /// ```
    pub fn export(&self, xml_content: &str) -> Result<String, ExportError> {
        // Since we store BPMN models in their native XML format,
        // export is simply returning the content as-is
        Ok(xml_content.to_string())
    }
}
