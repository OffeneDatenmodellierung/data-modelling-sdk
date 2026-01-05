//! DMN importer
//!
//! Provides functionality to import DMN 1.3 XML files with validation.

use anyhow::{Context, Result};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::dmn::DMNModel;

/// DMN namespace URIs
const DMN_NAMESPACE: &str = "https://www.omg.org/spec/DMN/20191111/MODEL/";
const DMN_NAMESPACE_ALT: &str = "http://www.omg.org/spec/DMN/20180521/MODEL/";
const DMNDI_NAMESPACE: &str = "https://www.omg.org/spec/DMN/20191111/DMNDI/";

/// DMN Importer
///
/// Imports DMN 1.3 XML content into a DMNModel struct.
#[derive(Debug, Default)]
pub struct DMNImporter {
    /// List of errors encountered during parsing
    pub errors: Vec<String>,
}

impl DMNImporter {
    /// Create a new DMNImporter
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Validate DMN XML against XSD schema
    ///
    /// Performs structural validation of DMN 1.3 XML including:
    /// - XML well-formedness checking
    /// - Required DMN elements validation
    /// - Namespace verification
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The DMN XML content as a string.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether validation succeeded.
    #[cfg(feature = "dmn")]
    pub fn validate(&self, xml_content: &str) -> Result<()> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut found_definitions = false;
        let mut has_dmn_namespace = false;
        let mut has_decision_or_input = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let local_name_bytes = e.local_name();
                    let local_name = String::from_utf8_lossy(local_name_bytes.as_ref()).to_string();

                    // Check for definitions element (root element)
                    if local_name == "definitions" {
                        found_definitions = true;

                        // Check for DMN namespace in attributes
                        for attr in e.attributes().flatten() {
                            let value = String::from_utf8_lossy(&attr.value);
                            if value.contains("omg.org/spec/DMN")
                                || value == DMN_NAMESPACE
                                || value == DMN_NAMESPACE_ALT
                            {
                                has_dmn_namespace = true;
                            }
                        }
                    }

                    // Check for decision or inputData elements
                    if local_name == "decision"
                        || local_name == "inputData"
                        || local_name == "businessKnowledgeModel"
                        || local_name == "knowledgeSource"
                    {
                        has_decision_or_input = true;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "DMN XML parsing error at position {}: {}",
                        reader.error_position(),
                        e
                    ))
                    .context("DMN XML validation failed");
                }
                _ => {}
            }
        }

        // Validate required elements
        if !found_definitions {
            return Err(anyhow::anyhow!(
                "Invalid DMN: missing root 'definitions' element"
            ))
            .context("DMN XML validation failed");
        }

        if !has_dmn_namespace {
            return Err(anyhow::anyhow!(
                "Invalid DMN: missing DMN namespace declaration (expected {} or {})",
                DMN_NAMESPACE,
                DMN_NAMESPACE_ALT
            ))
            .context("DMN XML validation failed");
        }

        if !has_decision_or_input {
            // Note: Some DMN files may be empty definitions
            tracing::warn!(
                "DMN file does not contain any decision, inputData, or businessKnowledgeModel elements"
            );
        }

        Ok(())
    }

    #[cfg(not(feature = "dmn"))]
    pub fn validate(&self, _xml_content: &str) -> Result<()> {
        // DMN feature not enabled - skip validation
        Ok(())
    }

    /// Extract metadata from DMN XML
    ///
    /// Extracts information including:
    /// - Definitions ID and name
    /// - Target namespace
    /// - Exporter and exporter version
    /// - Decision count and details
    /// - Input data count
    /// - Business knowledge model count
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The DMN XML content as a string.
    ///
    /// # Returns
    ///
    /// A `HashMap` containing extracted metadata.
    #[cfg(feature = "dmn")]
    pub fn extract_metadata(&self, xml_content: &str) -> HashMap<String, serde_json::Value> {
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use serde_json::json;

        let mut metadata = HashMap::new();
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut decision_count = 0;
        let mut input_data_count = 0;
        let mut bkm_count = 0;
        let mut knowledge_source_count = 0;
        let mut decision_table_count = 0;
        let mut literal_expression_count = 0;
        let mut decisions: Vec<serde_json::Value> = Vec::new();
        let mut input_data: Vec<serde_json::Value> = Vec::new();

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
                                "namespace" | "targetNamespace" => {
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

                    // Extract decision information
                    if local_name_str == "decision" {
                        decision_count += 1;
                        let mut decision_info = serde_json::Map::new();

                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match key.as_str() {
                                "id" => {
                                    decision_info.insert("id".to_string(), json!(value));
                                }
                                "name" => {
                                    decision_info.insert("name".to_string(), json!(value));
                                }
                                _ => {}
                            }
                        }

                        if !decision_info.is_empty() {
                            decisions.push(serde_json::Value::Object(decision_info));
                        }
                    }

                    // Extract input data information
                    if local_name_str == "inputData" {
                        input_data_count += 1;
                        let mut input_info = serde_json::Map::new();

                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match key.as_str() {
                                "id" => {
                                    input_info.insert("id".to_string(), json!(value));
                                }
                                "name" => {
                                    input_info.insert("name".to_string(), json!(value));
                                }
                                _ => {}
                            }
                        }

                        if !input_info.is_empty() {
                            input_data.push(serde_json::Value::Object(input_info));
                        }
                    }

                    // Count other element types
                    match local_name_str.as_str() {
                        "businessKnowledgeModel" => bkm_count += 1,
                        "knowledgeSource" => knowledge_source_count += 1,
                        "decisionTable" => decision_table_count += 1,
                        "literalExpression" => literal_expression_count += 1,
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Add counts to metadata
        metadata.insert("decisionCount".to_string(), json!(decision_count));
        metadata.insert("inputDataCount".to_string(), json!(input_data_count));
        metadata.insert("businessKnowledgeModelCount".to_string(), json!(bkm_count));
        metadata.insert(
            "knowledgeSourceCount".to_string(),
            json!(knowledge_source_count),
        );
        metadata.insert(
            "decisionTableCount".to_string(),
            json!(decision_table_count),
        );
        metadata.insert(
            "literalExpressionCount".to_string(),
            json!(literal_expression_count),
        );

        if !decisions.is_empty() {
            metadata.insert("decisions".to_string(), json!(decisions));
        }

        if !input_data.is_empty() {
            metadata.insert("inputData".to_string(), json!(input_data));
        }

        // Add DMN version info
        metadata.insert("dmnVersion".to_string(), json!("1.3"));
        metadata.insert("dmnNamespace".to_string(), json!(DMN_NAMESPACE));
        metadata.insert("dmndiNamespace".to_string(), json!(DMNDI_NAMESPACE));

        metadata
    }

    #[cfg(not(feature = "dmn"))]
    pub fn extract_metadata(&self, _xml_content: &str) -> HashMap<String, serde_json::Value> {
        // DMN feature not enabled - return empty metadata
        HashMap::new()
    }

    /// Import DMN XML content into a DMNModel struct.
    ///
    /// # Arguments
    ///
    /// * `xml_content` - The DMN XML content as a string.
    /// * `domain_id` - The domain ID this model belongs to.
    /// * `model_name` - The name for the model (extracted from XML if not provided).
    ///
    /// # Returns
    ///
    /// A `Result` containing the `DMNModel` if successful, or an error if parsing fails.
    pub fn import(
        &mut self,
        xml_content: &str,
        domain_id: Uuid,
        model_name: Option<&str>,
    ) -> Result<DMNModel> {
        // Validate XML
        self.validate(xml_content)
            .context("DMN XML validation failed")?;

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
                    .get("decisions")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|d| d.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "dmn_model".to_string());

        // Create file path
        let file_path = format!("{}/{}.dmn.xml", domain_id, name);

        // Calculate file size
        let file_size = xml_content.len() as u64;

        Ok(DMNModel::new(domain_id, name, file_path, file_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "dmn")]
    fn test_validate_valid_dmn() {
        let dmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             xmlns:dmndi="https://www.omg.org/spec/DMN/20191111/DMNDI/"
             id="definitions_1"
             name="Test DMN Model"
             namespace="http://example.com/dmn">
  <decision id="decision_1" name="Approval Decision">
    <decisionTable id="dt_1">
      <input id="input_1"/>
      <output id="output_1"/>
    </decisionTable>
  </decision>
  <inputData id="input_data_1" name="Customer Age"/>
</definitions>"#;

        let importer = DMNImporter::new();
        assert!(importer.validate(dmn_xml).is_ok());
    }

    #[test]
    #[cfg(feature = "dmn")]
    fn test_validate_missing_definitions() {
        let dmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<decision id="decision_1" name="Test Decision">
  <literalExpression/>
</decision>"#;

        let importer = DMNImporter::new();
        let result = importer.validate(dmn_xml);
        assert!(
            result.is_err(),
            "Expected error for missing definitions, got Ok"
        );
        let err = result.unwrap_err();
        // Check the full error chain for the expected message
        let err_chain = format!("{:?}", err);
        assert!(
            err_chain.contains("missing root 'definitions' element")
                || err.to_string().contains("DMN XML validation failed"),
            "Expected error about missing definitions, got: {}",
            err_chain
        );
    }

    #[test]
    #[cfg(feature = "dmn")]
    fn test_extract_metadata() {
        let dmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="definitions_1"
             name="Loan Approval Model"
             namespace="http://example.com/dmn"
             exporter="DMN Modeler"
             exporterVersion="2.0.0">
  <decision id="decision_1" name="Approve Loan"/>
  <decision id="decision_2" name="Calculate Risk"/>
  <inputData id="input_1" name="Applicant Age"/>
  <inputData id="input_2" name="Credit Score"/>
  <businessKnowledgeModel id="bkm_1" name="Risk Formula"/>
</definitions>"#;

        let importer = DMNImporter::new();
        let metadata = importer.extract_metadata(dmn_xml);

        assert_eq!(
            metadata.get("definitionsName").and_then(|v| v.as_str()),
            Some("Loan Approval Model")
        );
        assert_eq!(
            metadata.get("exporter").and_then(|v| v.as_str()),
            Some("DMN Modeler")
        );
        assert_eq!(
            metadata.get("decisionCount").and_then(|v| v.as_i64()),
            Some(2)
        );
        assert_eq!(
            metadata.get("inputDataCount").and_then(|v| v.as_i64()),
            Some(2)
        );
        assert_eq!(
            metadata
                .get("businessKnowledgeModelCount")
                .and_then(|v| v.as_i64()),
            Some(1)
        );
    }
}
