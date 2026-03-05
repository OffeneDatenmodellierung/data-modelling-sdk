//! Databricks Metric Views (DBMV) importer
//!
//! Parses DBMV YAML files (.dbmv.yaml) and converts them to DBMVDocument models.

use super::ImportError;
use crate::models::dbmv::{DBMVDocument, DBMVMetricView};

/// DBMV importer for parsing Databricks Metric Views YAML files
pub struct DBMVImporter;

impl DBMVImporter {
    /// Create a new DBMV importer instance
    pub fn new() -> Self {
        Self
    }

    /// Import a DBMV document from YAML content
    ///
    /// Optionally validates against the JSON schema if the `schema-validation` feature is enabled.
    pub fn import(&self, yaml_content: &str) -> Result<DBMVDocument, ImportError> {
        #[cfg(feature = "schema-validation")]
        {
            use crate::validation::schema::validate_dbmv_internal;
            validate_dbmv_internal(yaml_content).map_err(ImportError::ValidationError)?;
        }

        DBMVDocument::from_yaml(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse DBMV YAML: {}", e)))
    }

    /// Import a DBMV document without schema validation
    pub fn import_without_validation(
        &self,
        yaml_content: &str,
    ) -> Result<DBMVDocument, ImportError> {
        DBMVDocument::from_yaml(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse DBMV YAML: {}", e)))
    }

    /// Import a single standalone Databricks metric view from YAML content (no wrapper envelope)
    pub fn import_single_view(&self, yaml_content: &str) -> Result<DBMVMetricView, ImportError> {
        serde_yaml::from_str(yaml_content).map_err(|e| {
            ImportError::ParseError(format!("Failed to parse metric view YAML: {}", e))
        })
    }
}

impl Default for DBMVImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_document() {
        let importer = DBMVImporter::new();
        let yaml = r#"
apiVersion: v1.0.0
kind: MetricViews
system: test-system
metricViews:
  - name: orders_metrics
    source: catalog.schema.orders
    dimensions:
      - name: order_date
        expr: order_date
    measures:
      - name: total_revenue
        expr: SUM(revenue)
"#;
        let result = importer.import_without_validation(yaml);
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.system, "test-system");
        assert_eq!(doc.metric_views.len(), 1);
        assert_eq!(doc.metric_views[0].name, "orders_metrics");
    }

    #[test]
    fn test_import_single_view() {
        let importer = DBMVImporter::new();
        let yaml = r#"
name: orders_metrics
source: catalog.schema.orders
dimensions:
  - name: order_date
    expr: order_date
measures:
  - name: total_revenue
    expr: SUM(revenue)
"#;
        let result = importer.import_single_view(yaml);
        assert!(result.is_ok());
        let view = result.unwrap();
        assert_eq!(view.name, "orders_metrics");
        assert_eq!(view.dimensions.len(), 1);
        assert_eq!(view.measures.len(), 1);
    }

    #[test]
    fn test_import_invalid_yaml() {
        let importer = DBMVImporter::new();
        let result = importer.import_without_validation("not: valid: yaml: at: all");
        assert!(result.is_err());
    }
}
