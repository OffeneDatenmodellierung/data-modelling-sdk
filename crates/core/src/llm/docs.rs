//! Documentation loading for LLM context
//!
//! This module provides utilities for loading documentation files
//! to provide context to the LLM during schema refinement.

use std::path::Path;

use super::error::{LlmError, LlmResult};

/// Supported documentation file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    /// Plain text (.txt)
    PlainText,
    /// Markdown (.md)
    Markdown,
    /// Word document (.docx) - basic text extraction
    Word,
    /// Unknown format
    Unknown,
}

impl DocFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("txt") => DocFormat::PlainText,
            Some("md") | Some("markdown") => DocFormat::Markdown,
            Some("docx") => DocFormat::Word,
            _ => DocFormat::Unknown,
        }
    }

    /// Check if the format is supported
    pub fn is_supported(&self) -> bool {
        !matches!(self, DocFormat::Unknown)
    }
}

/// Load documentation from a file
///
/// Supports:
/// - `.txt` - Plain text
/// - `.md` - Markdown (treated as text)
/// - `.docx` - Word documents (basic text extraction)
///
/// # Arguments
/// * `path` - Path to the documentation file
///
/// # Returns
/// The documentation text
pub fn load_documentation(path: &Path) -> LlmResult<String> {
    let format = DocFormat::from_path(path);

    if !path.exists() {
        return Err(LlmError::DocumentationError(format!(
            "File not found: {}",
            path.display()
        )));
    }

    match format {
        DocFormat::PlainText | DocFormat::Markdown => load_text_file(path),
        DocFormat::Word => load_docx_file(path),
        DocFormat::Unknown => {
            // Try to load as text anyway
            load_text_file(path).map_err(|_| {
                LlmError::DocumentationError(format!("Unsupported file format: {}", path.display()))
            })
        }
    }
}

/// Load a plain text or markdown file
fn load_text_file(path: &Path) -> LlmResult<String> {
    std::fs::read_to_string(path).map_err(|e| {
        LlmError::DocumentationError(format!("Failed to read {}: {}", path.display(), e))
    })
}

/// Load and extract text from a Word document
///
/// This provides basic text extraction from .docx files.
/// For full Word document support, consider using a dedicated library.
fn load_docx_file(path: &Path) -> LlmResult<String> {
    // .docx files are ZIP archives containing XML
    let file = std::fs::File::open(path).map_err(|e| {
        LlmError::DocumentationError(format!("Failed to open {}: {}", path.display(), e))
    })?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| LlmError::DocumentationError(format!("Failed to read docx archive: {}", e)))?;

    // The main document content is in word/document.xml
    let mut document = archive.by_name("word/document.xml").map_err(|e| {
        LlmError::DocumentationError(format!("Failed to find document.xml in docx: {}", e))
    })?;

    let mut xml_content = String::new();
    std::io::Read::read_to_string(&mut document, &mut xml_content)
        .map_err(|e| LlmError::DocumentationError(format!("Failed to read document.xml: {}", e)))?;

    // Extract text from XML (basic extraction)
    Ok(extract_text_from_docx_xml(&xml_content))
}

/// Extract plain text from Word XML content
///
/// This is a basic extraction that gets text from <w:t> elements
fn extract_text_from_docx_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut in_text = false;
    let mut current_text = String::new();

    for c in xml.chars() {
        if c == '<' {
            if in_text && !current_text.is_empty() {
                result.push_str(&current_text);
                current_text.clear();
            }
            in_text = false;
        } else if c == '>' {
            // Check if we just closed a text tag
            // Simple heuristic: if we see w:t or w:p
        } else if in_text {
            current_text.push(c);
        }
    }

    // Better approach: use regex or simple state machine
    // to find <w:t>...</w:t> and <w:p>...</w:p> for paragraphs
    let text_pattern = regex::Regex::new(r"<w:t[^>]*>([^<]*)</w:t>").unwrap();
    let mut extracted = Vec::new();

    for cap in text_pattern.captures_iter(xml) {
        if let Some(text) = cap.get(1) {
            extracted.push(text.as_str().to_string());
        }
    }

    // Also detect paragraph breaks
    let para_pattern = regex::Regex::new(r"</w:p>").unwrap();
    let mut last_end = 0;
    let mut output = String::new();

    for m in para_pattern.find_iter(xml) {
        // Find all text in this paragraph
        let para_xml = &xml[last_end..m.end()];
        for cap in text_pattern.captures_iter(para_xml) {
            if let Some(text) = cap.get(1) {
                output.push_str(text.as_str());
            }
        }
        output.push('\n');
        last_end = m.end();
    }

    // Handle any remaining text
    let remaining = &xml[last_end..];
    for cap in text_pattern.captures_iter(remaining) {
        if let Some(text) = cap.get(1) {
            output.push_str(text.as_str());
        }
    }

    output.trim().to_string()
}

/// Load documentation from multiple files
///
/// Concatenates content from multiple files with separators
pub fn load_documentation_files(paths: &[&Path]) -> LlmResult<String> {
    let mut combined = Vec::new();

    for path in paths {
        let content = load_documentation(path)?;
        combined.push(format!("--- {} ---\n{}", path.display(), content));
    }

    Ok(combined.join("\n\n"))
}

/// Truncate documentation to fit within token limit
///
/// Tries to preserve complete sentences/paragraphs
pub fn truncate_documentation(text: &str, max_tokens: usize) -> String {
    // Rough estimate: 4 chars per token
    let max_chars = max_tokens * 4;

    if text.len() <= max_chars {
        return text.to_string();
    }

    // Try to find a good break point
    let truncated = &text[..max_chars];

    // Try paragraph break
    if let Some(pos) = truncated.rfind("\n\n") {
        if pos > max_chars / 2 {
            return format!("{}...\n\n[Documentation truncated]", &truncated[..pos]);
        }
    }

    // Try sentence break
    if let Some(pos) = truncated.rfind(". ") {
        if pos > max_chars / 2 {
            return format!("{}.\n\n[Documentation truncated]", &truncated[..pos]);
        }
    }

    // Fall back to word break
    if let Some(pos) = truncated.rfind(' ') {
        return format!("{}...\n\n[Documentation truncated]", &truncated[..pos]);
    }

    format!("{}...\n\n[Documentation truncated]", truncated)
}

/// Extract relevant sections from documentation based on field names
///
/// Searches for sections that mention the given field names
pub fn extract_relevant_sections(text: &str, field_names: &[&str], max_chars: usize) -> String {
    let mut relevant_lines = Vec::new();
    let lines: Vec<&str> = text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();
        for field in field_names {
            if line_lower.contains(&field.to_lowercase()) {
                // Include some context (previous and next lines)
                let start = i.saturating_sub(1);
                let end = (i + 2).min(lines.len());
                for j in start..end {
                    let context_line = lines[j];
                    if !relevant_lines.contains(&context_line) {
                        relevant_lines.push(context_line);
                    }
                }
                break;
            }
        }
    }

    let result = relevant_lines.join("\n");
    if result.len() > max_chars {
        truncate_documentation(&result, max_chars / 4)
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_doc_format_from_path() {
        assert_eq!(
            DocFormat::from_path(Path::new("doc.txt")),
            DocFormat::PlainText
        );
        assert_eq!(
            DocFormat::from_path(Path::new("doc.md")),
            DocFormat::Markdown
        );
        assert_eq!(
            DocFormat::from_path(Path::new("doc.markdown")),
            DocFormat::Markdown
        );
        assert_eq!(DocFormat::from_path(Path::new("doc.docx")), DocFormat::Word);
        assert_eq!(
            DocFormat::from_path(Path::new("doc.pdf")),
            DocFormat::Unknown
        );
    }

    #[test]
    fn test_doc_format_is_supported() {
        assert!(DocFormat::PlainText.is_supported());
        assert!(DocFormat::Markdown.is_supported());
        assert!(DocFormat::Word.is_supported());
        assert!(!DocFormat::Unknown.is_supported());
    }

    #[test]
    fn test_load_text_file() {
        let mut temp = NamedTempFile::with_suffix(".txt").unwrap();
        writeln!(temp, "This is test documentation.").unwrap();
        writeln!(temp, "It has multiple lines.").unwrap();

        let content = load_documentation(temp.path()).unwrap();
        assert!(content.contains("test documentation"));
        assert!(content.contains("multiple lines"));
    }

    #[test]
    fn test_load_markdown_file() {
        let mut temp = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(temp, "# Header").unwrap();
        writeln!(temp, "").unwrap();
        writeln!(temp, "Some **bold** text.").unwrap();

        let content = load_documentation(temp.path()).unwrap();
        assert!(content.contains("# Header"));
        assert!(content.contains("**bold**"));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_documentation(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_truncate_documentation_short() {
        let text = "Short text.";
        let result = truncate_documentation(text, 100);
        assert_eq!(result, text);
    }

    #[test]
    fn test_truncate_documentation_long() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph with more content that makes this longer.";
        let result = truncate_documentation(text, 10); // ~40 chars
        assert!(result.contains("[Documentation truncated]"));
        assert!(result.len() < text.len());
    }

    #[test]
    fn test_extract_relevant_sections() {
        let text = "Line 1: Introduction\nLine 2: The customer_id field is important.\nLine 3: Other info.\nLine 4: The order_date represents when the order was placed.\nLine 5: Conclusion.";

        let fields = vec!["customer_id", "order_date"];
        let result = extract_relevant_sections(text, &fields, 1000);

        assert!(result.contains("customer_id"));
        assert!(result.contains("order_date"));
    }

    #[test]
    fn test_extract_relevant_sections_no_match() {
        let text = "This documentation has no relevant field mentions.";
        let fields = vec!["nonexistent_field"];
        let result = extract_relevant_sections(text, &fields, 1000);

        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_text_from_docx_xml() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:t> World</w:t></w:r></w:p><w:p><w:r><w:t>Second paragraph</w:t></w:r></w:p></w:body></w:document>"#;
        let result = extract_text_from_docx_xml(xml);

        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
        assert!(result.contains("Second paragraph"));
    }
}
