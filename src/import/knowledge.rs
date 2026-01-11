//! Knowledge Base importer
//!
//! Parses Knowledge Base article YAML files (.kb.yaml) and converts them to KnowledgeArticle models.
//! Also handles the knowledge index file (knowledge.yaml).

use super::ImportError;
use crate::models::knowledge::{KnowledgeArticle, KnowledgeIndex};

#[cfg(feature = "schema-validation")]
use crate::validation::schema::validate_knowledge_internal;

/// Knowledge importer for parsing Knowledge Base article YAML files
pub struct KnowledgeImporter;

impl KnowledgeImporter {
    /// Create a new Knowledge importer instance
    pub fn new() -> Self {
        Self
    }

    /// Import a knowledge article from YAML content
    ///
    /// Optionally validates against the JSON schema if the `schema-validation` feature is enabled.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Knowledge article YAML content as a string
    ///
    /// # Returns
    ///
    /// A `KnowledgeArticle` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::knowledge::KnowledgeImporter;
    ///
    /// let importer = KnowledgeImporter::new();
    /// let yaml = r#"
    /// id: 660e8400-e29b-41d4-a716-446655440000
    /// number: 1
    /// title: "Data Classification Guide"
    /// articleType: guide
    /// status: published
    /// summary: "This guide explains data classification."
    /// content: "Data classification is essential for governance."
    /// authors:
    ///   - "data-governance@example.com"
    /// createdAt: "2024-01-15T10:00:00Z"
    /// updatedAt: "2024-01-15T10:00:00Z"
    /// "#;
    /// let article = importer.import(yaml).unwrap();
    /// assert_eq!(article.title, "Data Classification Guide");
    /// ```
    pub fn import(&self, yaml_content: &str) -> Result<KnowledgeArticle, ImportError> {
        // Validate against JSON Schema if feature is enabled
        #[cfg(feature = "schema-validation")]
        {
            validate_knowledge_internal(yaml_content).map_err(ImportError::ValidationError)?;
        }

        // Parse the YAML content
        KnowledgeArticle::from_yaml(yaml_content).map_err(|e| {
            ImportError::ParseError(format!("Failed to parse knowledge article YAML: {}", e))
        })
    }

    /// Import a knowledge article without schema validation
    ///
    /// Use this when you want to skip schema validation for performance
    /// or when importing from a trusted source.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Knowledge article YAML content as a string
    ///
    /// # Returns
    ///
    /// A `KnowledgeArticle` parsed from the YAML content
    pub fn import_without_validation(
        &self,
        yaml_content: &str,
    ) -> Result<KnowledgeArticle, ImportError> {
        KnowledgeArticle::from_yaml(yaml_content).map_err(|e| {
            ImportError::ParseError(format!("Failed to parse knowledge article YAML: {}", e))
        })
    }

    /// Import a knowledge index from YAML content
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - Knowledge index YAML content (knowledge.yaml)
    ///
    /// # Returns
    ///
    /// A `KnowledgeIndex` parsed from the YAML content
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::knowledge::KnowledgeImporter;
    ///
    /// let importer = KnowledgeImporter::new();
    /// let yaml = r#"
    /// schemaVersion: "1.0"
    /// articles: []
    /// nextNumber: 1
    /// "#;
    /// let index = importer.import_index(yaml).unwrap();
    /// assert_eq!(index.next_number, 1);
    /// ```
    pub fn import_index(&self, yaml_content: &str) -> Result<KnowledgeIndex, ImportError> {
        KnowledgeIndex::from_yaml(yaml_content).map_err(|e| {
            ImportError::ParseError(format!("Failed to parse knowledge index YAML: {}", e))
        })
    }

    /// Import multiple knowledge articles from a directory
    ///
    /// Loads all `.kb.yaml` files from the specified directory.
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory containing knowledge article files
    ///
    /// # Returns
    ///
    /// A vector of parsed `KnowledgeArticle` objects and any import errors
    pub fn import_from_directory(
        &self,
        dir_path: &std::path::Path,
    ) -> Result<(Vec<KnowledgeArticle>, Vec<ImportError>), ImportError> {
        let mut articles = Vec::new();
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

        // Read all .kb.yaml files
        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| ImportError::IoError(format!("Failed to read directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|name| name.ends_with(".kb.yaml"))
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match self.import(&content) {
                        Ok(article) => articles.push(article),
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

        // Sort articles by number
        articles.sort_by(|a, b| a.number.cmp(&b.number));

        Ok((articles, errors))
    }

    /// Import articles filtered by domain
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Path to the directory containing knowledge article files
    /// * `domain` - Domain to filter by
    ///
    /// # Returns
    ///
    /// A vector of parsed `KnowledgeArticle` objects for the specified domain
    pub fn import_by_domain(
        &self,
        dir_path: &std::path::Path,
        domain: &str,
    ) -> Result<(Vec<KnowledgeArticle>, Vec<ImportError>), ImportError> {
        let (articles, errors) = self.import_from_directory(dir_path)?;

        let filtered: Vec<KnowledgeArticle> = articles
            .into_iter()
            .filter(|a| a.domain.as_deref() == Some(domain))
            .collect();

        Ok((filtered, errors))
    }
}

impl Default for KnowledgeImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_knowledge_article() {
        let importer = KnowledgeImporter::new();
        let yaml = r#"
id: 660e8400-e29b-41d4-a716-446655440000
number: 1
title: "Data Classification Guide"
articleType: guide
status: published
summary: "This guide explains data classification."
content: "Data classification is essential for governance."
authors:
  - "data-governance@example.com"
createdAt: "2024-01-15T10:00:00Z"
updatedAt: "2024-01-15T10:00:00Z"
"#;
        let result = importer.import_without_validation(yaml);
        assert!(result.is_ok());
        let article = result.unwrap();
        assert_eq!(article.title, "Data Classification Guide");
        assert_eq!(article.number, 1);
    }

    #[test]
    fn test_import_knowledge_index() {
        let importer = KnowledgeImporter::new();
        let yaml = r#"
schemaVersion: "1.0"
articles: []
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
        let importer = KnowledgeImporter::new();
        let yaml = "not: valid: yaml: at: all";
        let result = importer.import_without_validation(yaml);
        assert!(result.is_err());
    }
}
