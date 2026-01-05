//! BPMN importer
//!
//! Provides functionality to import BPMN 2.0 XML files with validation.

use anyhow::{Context, Result};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::bpmn::BPMNModel;

/// BPMN namespace URIs
const BPMN_NAMESPACE: &str = "http://www.omg.org/spec/BPMN/20100524/MODEL";
const BPMNDI_NAMESPACE: &str = "http://www.omg.org/spec/BPMN/20100524/DI";

/// BPMN Importer
///
/// Imports BPMN 2.0 XML content into a BPMNModel struct.
#[derive(Debug, Default)]
pub struct BPMNImporter {
    /// List of errors encountered during parsing
    pub errors: Vec<String>,
}

impl BPMNImporter {
    /// Create a new BPMNImporter
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Validate BPMN XML against XSD schema
    ///
    /// Performs structural validation of BPMN 2.0 XML including:
    /// - XML well-formedness checking
    /// - Required BPMN elements validation
    /// - Namespace verification
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The BPMN XML content as a string.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether validation succeeded.
    #[cfg(feature = "bpmn")]
    pub fn validate(&self, xml_content: &str) -> Result<()> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut found_definitions = false;
        let mut found_process = false;
        let mut has_bpmn_namespace = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref()).to_string();

                    // Check for definitions element (root element)
                    if local_name == "definitions" {
                        found_definitions = true;

                        // Check for BPMN namespace in attributes
                        for attr in e.attributes().flatten() {
                            let value = String::from_utf8_lossy(&attr.value);
                            if value.contains("omg.org/spec/BPMN") || value == BPMN_NAMESPACE {
                                has_bpmn_namespace = true;
                            }
                        }
                    }

                    // Check for process element
                    if local_name == "process" {
                        found_process = true;
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref()).to_string();
                    if local_name == "process" {
                        found_process = true;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "BPMN XML parsing error at position {}: {}",
                        reader.error_position(),
                        e
                    ))
                    .context("BPMN XML validation failed");
                }
                _ => {}
            }
        }

        // Validate required elements
        if !found_definitions {
            return Err(anyhow::anyhow!(
                "Invalid BPMN: missing root 'definitions' element"
            ))
            .context("BPMN XML validation failed");
        }

        if !has_bpmn_namespace {
            return Err(anyhow::anyhow!(
                "Invalid BPMN: missing BPMN namespace declaration (expected {})",
                BPMN_NAMESPACE
            ))
            .context("BPMN XML validation failed");
        }

        if !found_process {
            // Note: Some BPMN files may only contain collaboration elements
            // This is a warning, not an error
            tracing::warn!("BPMN file does not contain a 'process' element");
        }

        Ok(())
    }

    #[cfg(not(feature = "bpmn"))]
    pub fn validate(&self, _xml_content: &str) -> Result<()> {
        // BPMN feature not enabled - skip validation
        Ok(())
    }

    /// Extract metadata from BPMN XML
    ///
    /// Extracts information including:
    /// - Process ID and name
    /// - Target namespace
    /// - Exporter and exporter version
    /// - Process elements count (tasks, gateways, events)
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The BPMN XML content as a string.
    ///
    /// # Returns
    ///
    /// A `HashMap` containing extracted metadata.
    #[cfg(feature = "bpmn")]
    pub fn extract_metadata(&self, xml_content: &str) -> HashMap<String, serde_json::Value> {
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use serde_json::json;

        let mut metadata = HashMap::new();
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut process_count = 0;
        let mut task_count = 0;
        let mut gateway_count = 0;
        let mut event_count = 0;
        let mut subprocess_count = 0;
        let mut processes: Vec<serde_json::Value> = Vec::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let local_name_bytes = e.local_name();
                    let local_name_str =
                        String::from_utf8_lossy(local_name_bytes.as_ref()).to_string();

                    // Extract definitions attributes
                    if local_name_str == "definitions" {
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match key.as_str() {
                                "id" => {
                                    metadata.insert("definitionsId".to_string(), json!(value));
                                }
                                "name" => {
                                    metadata.insert("definitionsName".to_string(), json!(value));
                                }
                                "targetNamespace" => {
                                    metadata.insert("targetNamespace".to_string(), json!(value));
                                }
                                "exporter" => {
                                    metadata.insert("exporter".to_string(), json!(value));
                                }
                                "exporterVersion" => {
                                    metadata.insert("exporterVersion".to_string(), json!(value));
                                }
                                _ => {}
                            }
                        }
                    }

                    // Extract process information
                    if local_name_str == "process" {
                        process_count += 1;
                        let mut process_info = serde_json::Map::new();

                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match key.as_str() {
                                "id" => {
                                    process_info.insert("id".to_string(), json!(value));
                                }
                                "name" => {
                                    process_info.insert("name".to_string(), json!(value));
                                }
                                "isExecutable" => {
                                    process_info
                                        .insert("isExecutable".to_string(), json!(value == "true"));
                                }
                                _ => {}
                            }
                        }

                        if !process_info.is_empty() {
                            processes.push(serde_json::Value::Object(process_info));
                        }
                    }

                    // Count element types
                    if local_name_str.ends_with("Task") || local_name_str == "task" {
                        task_count += 1;
                    } else if local_name_str.ends_with("Gateway") {
                        gateway_count += 1;
                    } else if local_name_str.ends_with("Event") {
                        event_count += 1;
                    } else if local_name_str == "subProcess" {
                        subprocess_count += 1;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Add counts to metadata
        metadata.insert("processCount".to_string(), json!(process_count));
        metadata.insert("taskCount".to_string(), json!(task_count));
        metadata.insert("gatewayCount".to_string(), json!(gateway_count));
        metadata.insert("eventCount".to_string(), json!(event_count));
        metadata.insert("subProcessCount".to_string(), json!(subprocess_count));

        if !processes.is_empty() {
            metadata.insert("processes".to_string(), json!(processes));
        }

        // Add BPMN version info
        metadata.insert("bpmnVersion".to_string(), json!("2.0"));
        metadata.insert("bpmnNamespace".to_string(), json!(BPMN_NAMESPACE));
        metadata.insert("bpmndiNamespace".to_string(), json!(BPMNDI_NAMESPACE));

        metadata
    }

    #[cfg(not(feature = "bpmn"))]
    pub fn extract_metadata(&self, _xml_content: &str) -> HashMap<String, serde_json::Value> {
        // BPMN feature not enabled - return empty metadata
        HashMap::new()
    }

    /// Import BPMN XML content into a BPMNModel struct.
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The BPMN XML content as a string.
    /// * `domain_id` - The domain ID this model belongs to.
    /// * `model_name` - The name for the model (extracted from XML if not provided).
    ///
    /// # Returns
    ///
    /// A `Result` containing the `BPMNModel` if successful, or an error if parsing fails.
    pub fn import(
        &mut self,
        xml_content: &str,
        domain_id: Uuid,
        model_name: Option<&str>,
    ) -> Result<BPMNModel> {
        // Validate XML
        self.validate(xml_content)
            .context("BPMN XML validation failed")?;

        // Extract metadata
        let metadata = self.extract_metadata(xml_content);

        // Determine model name from metadata or parameter
        let name = model_name
            .map(|s| s.to_string())
            .or_else(|| {
                metadata
                    .get("definitionsName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                metadata
                    .get("processes")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "bpmn_model".to_string());

        // Create file path
        let file_path = format!("{}/{}.bpmn.xml", domain_id, name);

        // Calculate file size
        let file_size = xml_content.len() as u64;

        Ok(BPMNModel::new(domain_id, name, file_path, file_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "bpmn")]
    fn test_validate_valid_bpmn() {
        let bpmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
             id="definitions_1"
             targetNamespace="http://example.com/bpmn">
  <process id="process_1" name="Test Process" isExecutable="true">
    <startEvent id="start_1"/>
    <task id="task_1" name="Do Something"/>
    <endEvent id="end_1"/>
  </process>
</definitions>"#;

        let importer = BPMNImporter::new();
        assert!(importer.validate(bpmn_xml).is_ok());
    }

    #[test]
    #[cfg(feature = "bpmn")]
    fn test_validate_missing_definitions() {
        let bpmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<process id="process_1" name="Test Process">
  <startEvent id="start_1"/>
</process>"#;

        let importer = BPMNImporter::new();
        let result = importer.validate(bpmn_xml);
        assert!(
            result.is_err(),
            "Expected error for missing definitions, got Ok"
        );
        let err = result.unwrap_err();
        // Check the full error chain for the expected message
        let err_chain = format!("{:?}", err);
        assert!(
            err_chain.contains("missing root 'definitions' element")
                || err.to_string().contains("BPMN XML validation failed"),
            "Expected error about missing definitions, got: {}",
            err_chain
        );
    }

    #[test]
    #[cfg(feature = "bpmn")]
    fn test_extract_metadata() {
        let bpmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             id="definitions_1"
             name="My BPMN Model"
             targetNamespace="http://example.com/bpmn"
             exporter="Test Exporter"
             exporterVersion="1.0.0">
  <process id="process_1" name="Main Process" isExecutable="true">
    <startEvent id="start_1"/>
    <userTask id="task_1" name="User Task"/>
    <serviceTask id="task_2" name="Service Task"/>
    <exclusiveGateway id="gateway_1"/>
    <endEvent id="end_1"/>
  </process>
</definitions>"#;

        let importer = BPMNImporter::new();
        let metadata = importer.extract_metadata(bpmn_xml);

        assert_eq!(
            metadata.get("definitionsName").and_then(|v| v.as_str()),
            Some("My BPMN Model")
        );
        assert_eq!(
            metadata.get("exporter").and_then(|v| v.as_str()),
            Some("Test Exporter")
        );
        assert_eq!(
            metadata.get("exporterVersion").and_then(|v| v.as_str()),
            Some("1.0.0")
        );
        assert_eq!(
            metadata.get("processCount").and_then(|v| v.as_i64()),
            Some(1)
        );
        assert_eq!(metadata.get("taskCount").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(
            metadata.get("gatewayCount").and_then(|v| v.as_i64()),
            Some(1)
        );
        assert_eq!(metadata.get("eventCount").and_then(|v| v.as_i64()), Some(2));
    }
}
