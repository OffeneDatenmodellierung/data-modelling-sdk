//! Databricks Metric Views (DBMV) exporter
//!
//! Exports DBMVDocument models to DBMV YAML format.

use crate::export::ExportError;
use crate::models::dbmv::{DBMVDocument, DBMVMetricView};

/// DBMV exporter for generating Databricks Metric Views YAML
pub struct DBMVExporter;

impl DBMVExporter {
    /// Export a DBMV document to YAML format (instance method for WASM compatibility)
    ///
    /// Validates against the DBMV JSON Schema if the `schema-validation` feature is enabled.
    pub fn export(&self, doc: &DBMVDocument) -> Result<String, ExportError> {
        let yaml = Self::export_document(doc);

        #[cfg(feature = "schema-validation")]
        {
            use crate::validation::schema::validate_dbmv_internal;
            validate_dbmv_internal(&yaml).map_err(ExportError::ValidationError)?;
        }

        Ok(yaml)
    }

    /// Export a full multi-view DBMV document to YAML
    pub fn export_document(doc: &DBMVDocument) -> String {
        match serde_yaml::to_string(doc) {
            Ok(yaml) => yaml,
            Err(e) => format!("# Error serializing DBMV document: {}\n", e),
        }
    }

    /// Export a single standalone Databricks metric view to YAML (no wrapper envelope)
    pub fn export_single_view(view: &DBMVMetricView) -> String {
        match serde_yaml::to_string(view) {
            Ok(yaml) => yaml,
            Err(e) => format!("# Error serializing metric view: {}\n", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dbmv::*;

    #[test]
    fn test_export_document_basic() {
        let doc = DBMVDocument::new("test-system");
        let yaml = DBMVExporter::export_document(&doc);
        assert!(yaml.contains("apiVersion: v1.0.0"));
        assert!(yaml.contains("kind: MetricViews"));
        assert!(yaml.contains("system: test-system"));
    }

    #[test]
    fn test_export_document_with_views() {
        let mut doc = DBMVDocument::new("sales-system");
        doc.add_metric_view(DBMVMetricView {
            name: "orders".to_string(),
            source: "catalog.schema.orders".to_string(),
            dimensions: vec![DBMVDimension {
                name: "order_date".to_string(),
                expr: "order_date".to_string(),
                display_name: None,
                comment: None,
            }],
            measures: vec![DBMVMeasure {
                name: "total".to_string(),
                expr: "SUM(amount)".to_string(),
                display_name: None,
                comment: None,
                format: None,
                window: vec![],
            }],
            ..Default::default()
        });

        let yaml = DBMVExporter::export_document(&doc);
        assert!(yaml.contains("orders"));
        assert!(yaml.contains("SUM(amount)"));
    }

    #[test]
    fn test_export_single_view() {
        let view = DBMVMetricView {
            name: "test_view".to_string(),
            source: "catalog.schema.table".to_string(),
            ..Default::default()
        };

        let yaml = DBMVExporter::export_single_view(&view);
        assert!(yaml.contains("name: test_view"));
        assert!(yaml.contains("source: catalog.schema.table"));
        // Should NOT contain envelope fields
        assert!(!yaml.contains("apiVersion"));
        assert!(!yaml.contains("kind"));
    }
}
