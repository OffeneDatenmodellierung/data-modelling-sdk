//! Knowledge Base exporter
//!
//! Exports KnowledgeArticle models to YAML format.

use crate::export::ExportError;
use crate::models::knowledge::{KnowledgeArticle, KnowledgeIndex};

/// Knowledge exporter for generating YAML from KnowledgeArticle models
pub struct KnowledgeExporter;

impl KnowledgeExporter {
    /// Create a new Knowledge exporter instance
    pub fn new() -> Self {
        Self
    }

    /// Export a knowledge article to YAML format
    ///
    /// # Arguments
    ///
    /// * `article` - The KnowledgeArticle to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string, or an ExportError
    pub fn export(&self, article: &KnowledgeArticle) -> Result<String, ExportError> {
        let yaml = article.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize knowledge article: {}", e))
        })?;

        // Validate exported YAML against knowledge schema (if feature enabled)
        #[cfg(feature = "schema-validation")]
        {
            use crate::validation::schema::validate_knowledge_internal;
            validate_knowledge_internal(&yaml).map_err(ExportError::ValidationError)?;
        }

        Ok(yaml)
    }

    /// Export a knowledge article without validation
    ///
    /// Use this when you want to skip schema validation for performance
    /// or when exporting to a trusted destination.
    pub fn export_without_validation(
        &self,
        article: &KnowledgeArticle,
    ) -> Result<String, ExportError> {
        article.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize knowledge article: {}", e))
        })
    }

    /// Export a knowledge index to YAML format
    ///
    /// # Arguments
    ///
    /// * `index` - The KnowledgeIndex to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string, or an ExportError
    pub fn export_index(&self, index: &KnowledgeIndex) -> Result<String, ExportError> {
        index.to_yaml().map_err(|e| {
            ExportError::SerializationError(format!("Failed to serialize knowledge index: {}", e))
        })
    }

    /// Export multiple articles to a directory
    ///
    /// # Arguments
    ///
    /// * `articles` - The articles to export
    /// * `dir_path` - Directory to export to
    /// * `workspace_name` - Workspace name for filename generation
    ///
    /// # Returns
    ///
    /// A Result with the number of files exported, or an ExportError
    pub fn export_to_directory(
        &self,
        articles: &[KnowledgeArticle],
        dir_path: &std::path::Path,
        workspace_name: &str,
    ) -> Result<usize, ExportError> {
        // Create directory if it doesn't exist
        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path)
                .map_err(|e| ExportError::IoError(format!("Failed to create directory: {}", e)))?;
        }

        let mut count = 0;
        for article in articles {
            let filename = article.filename(workspace_name);
            let path = dir_path.join(&filename);
            let yaml = self.export(article)?;
            std::fs::write(&path, yaml).map_err(|e| {
                ExportError::IoError(format!("Failed to write {}: {}", filename, e))
            })?;
            count += 1;
        }

        Ok(count)
    }

    /// Export articles filtered by domain to a directory
    ///
    /// # Arguments
    ///
    /// * `articles` - The articles to export
    /// * `dir_path` - Directory to export to
    /// * `workspace_name` - Workspace name for filename generation
    /// * `domain` - Domain to filter by
    ///
    /// # Returns
    ///
    /// A Result with the number of files exported, or an ExportError
    pub fn export_domain_to_directory(
        &self,
        articles: &[KnowledgeArticle],
        dir_path: &std::path::Path,
        workspace_name: &str,
        domain: &str,
    ) -> Result<usize, ExportError> {
        let filtered: Vec<&KnowledgeArticle> = articles
            .iter()
            .filter(|a| a.domain.as_deref() == Some(domain))
            .collect();

        // Create domain subdirectory
        let domain_dir = dir_path.join(domain);
        if !domain_dir.exists() {
            std::fs::create_dir_all(&domain_dir)
                .map_err(|e| ExportError::IoError(format!("Failed to create directory: {}", e)))?;
        }

        let mut count = 0;
        for article in filtered {
            let filename = article.filename(workspace_name);
            let path = domain_dir.join(&filename);
            let yaml = self.export(article)?;
            std::fs::write(&path, yaml).map_err(|e| {
                ExportError::IoError(format!("Failed to write {}: {}", filename, e))
            })?;
            count += 1;
        }

        Ok(count)
    }
}

impl Default for KnowledgeExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::knowledge::KnowledgeStatus;

    #[test]
    fn test_export_knowledge_article() {
        let article = KnowledgeArticle::new(
            1,
            "Data Classification Guide",
            "This guide explains data classification.",
            "Data classification is essential for governance.",
            "data-governance@example.com",
        )
        .with_status(KnowledgeStatus::Published);

        let exporter = KnowledgeExporter::new();
        let result = exporter.export_without_validation(&article);
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("title: Data Classification Guide"));
        assert!(yaml.contains("status: published"));
    }

    #[test]
    fn test_export_knowledge_index() {
        let index = KnowledgeIndex::new();
        let exporter = KnowledgeExporter::new();
        let result = exporter.export_index(&index);
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("schemaVersion"));
        assert!(yaml.contains("nextNumber: 1"));
    }
}
