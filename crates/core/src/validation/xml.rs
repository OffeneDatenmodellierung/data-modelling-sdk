//! XML validation utilities
//!
//! Provides functionality for validating XML files against XSD schemas.

use anyhow::{Context, Result};
use std::path::Path;

/// Validate XML content against an XSD schema file.
///
/// # Arguments
///
/// * `xml_content` - The XML content to validate
/// * `xsd_path` - Path to the XSD schema file (relative to schemas/ directory)
///
/// # Returns
///
/// A `Result` indicating whether validation succeeded.
///
/// # Note
///
/// This is a placeholder implementation. Full XSD validation requires
/// a proper XSD validation library or external tool integration.
pub fn validate_xml_against_xsd(xml_content: &str, xsd_path: &str) -> Result<()> {
    // TODO: Implement full XSD validation
    // For now, we'll do basic XML well-formedness checking
    // using quick-xml if available

    #[cfg(feature = "bpmn")]
    {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event() {
                Ok(Event::Eof) => break,
                Ok(_) => continue,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "XML parsing error: {}. Schema: {}",
                        e,
                        xsd_path
                    ))
                    .context("XML validation failed");
                }
            }
        }
    }

    #[cfg(not(feature = "bpmn"))]
    {
        // Basic check: ensure XML starts with <?xml
        if !xml_content.trim_start().starts_with("<?xml") {
            return Err(anyhow::anyhow!(
                "Invalid XML: missing XML declaration. Schema: {}",
                xsd_path
            ))
            .context("XML validation failed");
        }
    }

    // Verify XSD file exists (for future full validation)
    let schemas_dir = Path::new("schemas");
    let full_xsd_path = schemas_dir.join(xsd_path);
    if !full_xsd_path.exists() {
        return Err(anyhow::anyhow!(
            "XSD schema file not found: {}. Please ensure schema files are downloaded.",
            full_xsd_path.display()
        ))
        .context("XSD schema file missing");
    }

    Ok(())
}

/// Load XSD schema content from the schemas directory.
///
/// # Arguments
///
/// * `xsd_path` - Path to the XSD schema file (relative to schemas/ directory)
///
/// # Returns
///
/// The XSD schema content as a string.
pub fn load_xsd_schema(xsd_path: &str) -> Result<String> {
    use std::fs;

    let schemas_dir = Path::new("schemas");
    let full_xsd_path = schemas_dir.join(xsd_path);

    fs::read_to_string(&full_xsd_path)
        .with_context(|| format!("Failed to read XSD schema: {}", full_xsd_path.display()))
}
