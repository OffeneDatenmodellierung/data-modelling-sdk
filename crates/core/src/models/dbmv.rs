//! Databricks Metric Views (DBMV) model
//!
//! Defines the data structures for Databricks Metric Views — a semantic layer
//! format that transforms raw tables into standardised business metrics.
//!
//! ## File Format
//!
//! DBMV documents use the `.dbmv.yaml` extension and contain one or more
//! metric view definitions per file, wrapped in an SDK envelope format.
//!
//! The envelope uses **camelCase** keys (`apiVersion`, `kind`, `metricViews`)
//! while the inner Databricks-native content uses **snake_case** keys
//! (`display_name`, `materialized_views`).
//!
//! ## Example
//!
//! ```yaml
//! apiVersion: v1.0.0
//! kind: MetricViews
//! system: my-databricks-system
//! metricViews:
//!   - name: orders_metrics
//!     source: catalog.schema.orders
//!     dimensions:
//!       - name: order_date
//!         expr: order_date
//!     measures:
//!       - name: total_revenue
//!         expr: SUM(revenue)
//! ```

use serde::{Deserialize, Serialize};

/// Default version for metric views
fn default_version() -> String {
    "1.1".to_string()
}

/// Default API version
fn default_api_version() -> String {
    "v1.0.0".to_string()
}

/// Default kind
fn default_kind() -> String {
    "MetricViews".to_string()
}

/// DBMV Document — wrapper envelope for multiple metric views
///
/// Uses camelCase for the envelope fields to match SDK conventions.
/// One document per system, containing multiple metric view definitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DBMVDocument {
    /// API version of the DBMV format (e.g., "v1.0.0")
    #[serde(default = "default_api_version")]
    pub api_version: String,
    /// Document kind — always "MetricViews"
    #[serde(default = "default_kind")]
    pub kind: String,
    /// System name this document belongs to
    pub system: String,
    /// Optional description of the metric views collection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Metric view definitions
    #[serde(default)]
    pub metric_views: Vec<DBMVMetricView>,
}

impl Default for DBMVDocument {
    fn default() -> Self {
        Self {
            api_version: default_api_version(),
            kind: default_kind(),
            system: String::new(),
            description: None,
            metric_views: Vec::new(),
        }
    }
}

impl DBMVDocument {
    /// Create a new DBMV document for a system
    pub fn new(system: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            ..Default::default()
        }
    }

    /// Add a metric view to the document
    pub fn add_metric_view(&mut self, view: DBMVMetricView) {
        self.metric_views.push(view);
    }

    /// Get a metric view by name
    pub fn get_metric_view(&self, name: &str) -> Option<&DBMVMetricView> {
        self.metric_views.iter().find(|v| v.name == name)
    }

    /// Import from YAML
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }

    /// Export to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

/// Databricks Metric View definition
///
/// Uses snake_case (Rust default) to match Databricks native YAML format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVMetricView {
    /// Metric view name
    pub name: String,
    /// Version of the metric view definition
    #[serde(default = "default_version")]
    pub version: String,
    /// Fully qualified source table (e.g., "catalog.schema.table")
    pub source: String,
    /// Optional SQL filter expression applied to the source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Optional comment/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Dimension definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dimensions: Vec<DBMVDimension>,
    /// Measure definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measures: Vec<DBMVMeasure>,
    /// Join definitions (supports nested joins for snowflake schemas)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joins: Vec<DBMVJoin>,
    /// Materialization configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materialization: Option<DBMVMaterialization>,
}

impl Default for DBMVMetricView {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: default_version(),
            source: String::new(),
            filter: None,
            comment: None,
            dimensions: Vec::new(),
            measures: Vec::new(),
            joins: Vec::new(),
            materialization: None,
        }
    }
}

/// Dimension definition in a metric view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVDimension {
    /// Dimension name
    pub name: String,
    /// SQL expression for the dimension
    pub expr: String,
    /// Human-readable display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Optional comment/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Measure definition in a metric view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVMeasure {
    /// Measure name
    pub name: String,
    /// SQL aggregation expression (e.g., "SUM(revenue)")
    pub expr: String,
    /// Human-readable display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Optional comment/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Format specification for the measure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<DBMVMeasureFormat>,
    /// Window function specifications
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window: Vec<DBMVWindow>,
}

/// Format specification for a measure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVMeasureFormat {
    /// Format type (e.g., "currency", "percentage", "number")
    #[serde(rename = "type")]
    pub format_type: String,
}

/// Window function specification for a measure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVWindow {
    /// Column to order by
    pub order: String,
    /// Window range (e.g., "cumulative", "unbounded")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    /// Semi-additive behaviour (e.g., "last", "first")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semiadditive: Option<String>,
}

/// Join definition (supports recursive nesting for snowflake schemas)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVJoin {
    /// Join alias name
    pub name: String,
    /// Fully qualified source table for the join
    pub source: String,
    /// Join condition expression (e.g., "source.customer_id = customers.id")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
    /// Column names for equi-join (alternative to `on`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub using: Vec<String>,
    /// Nested joins (for snowflake schema patterns)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joins: Vec<DBMVJoin>,
}

/// Materialization configuration for a metric view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVMaterialization {
    /// Refresh schedule (e.g., "every 6 hours", "daily")
    pub schedule: String,
    /// Materialization mode (e.g., "relaxed", "strict")
    pub mode: String,
    /// Pre-computed materialized views
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materialized_views: Vec<DBMVMaterializedView>,
}

/// Pre-computed materialized view definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DBMVMaterializedView {
    /// Materialized view name
    pub name: String,
    /// View type: "aggregated" or "unaggregated"
    #[serde(rename = "type")]
    pub view_type: String,
    /// Dimensions to include (for aggregated views)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dimensions: Vec<String>,
    /// Measures to include (for aggregated views)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measures: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_new() {
        let doc = DBMVDocument::new("my-system");
        assert_eq!(doc.system, "my-system");
        assert_eq!(doc.api_version, "v1.0.0");
        assert_eq!(doc.kind, "MetricViews");
        assert!(doc.metric_views.is_empty());
    }

    #[test]
    fn test_document_add_metric_view() {
        let mut doc = DBMVDocument::new("test-system");
        doc.add_metric_view(DBMVMetricView {
            name: "orders".to_string(),
            source: "catalog.schema.orders".to_string(),
            ..Default::default()
        });
        assert_eq!(doc.metric_views.len(), 1);
        assert_eq!(doc.get_metric_view("orders").unwrap().name, "orders");
        assert!(doc.get_metric_view("nonexistent").is_none());
    }

    #[test]
    fn test_default_version() {
        let view = DBMVMetricView::default();
        assert_eq!(view.version, "1.1");
    }

    #[test]
    fn test_measure_format_type_rename() {
        let format = DBMVMeasureFormat {
            format_type: "currency".to_string(),
        };
        let yaml = serde_yaml::to_string(&format).unwrap();
        assert!(yaml.contains("type: currency"));
    }

    #[test]
    fn test_materialized_view_type_rename() {
        let mv = DBMVMaterializedView {
            name: "test".to_string(),
            view_type: "aggregated".to_string(),
            dimensions: vec![],
            measures: vec![],
        };
        let yaml = serde_yaml::to_string(&mv).unwrap();
        assert!(yaml.contains("type: aggregated"));
    }

    #[test]
    fn test_document_yaml_roundtrip() {
        let mut doc = DBMVDocument::new("test-system");
        doc.description = Some("Test metrics".to_string());
        doc.add_metric_view(DBMVMetricView {
            name: "orders_metrics".to_string(),
            source: "catalog.schema.orders".to_string(),
            dimensions: vec![DBMVDimension {
                name: "order_date".to_string(),
                expr: "order_date".to_string(),
                display_name: Some("Order Date".to_string()),
                comment: None,
            }],
            measures: vec![DBMVMeasure {
                name: "total_revenue".to_string(),
                expr: "SUM(revenue)".to_string(),
                display_name: Some("Total Revenue".to_string()),
                comment: None,
                format: Some(DBMVMeasureFormat {
                    format_type: "currency".to_string(),
                }),
                window: vec![],
            }],
            ..Default::default()
        });

        let yaml = doc.to_yaml().unwrap();
        let parsed = DBMVDocument::from_yaml(&yaml).unwrap();
        assert_eq!(doc, parsed);
    }

    #[test]
    fn test_camel_case_envelope_snake_case_inner() {
        let doc = DBMVDocument::new("test");
        let yaml = doc.to_yaml().unwrap();

        // Envelope fields should be camelCase
        assert!(yaml.contains("apiVersion:"));
        assert!(yaml.contains("metricViews:"));

        // These should NOT appear (wrong casing)
        assert!(!yaml.contains("api_version:"));
        assert!(!yaml.contains("metric_views:"));
    }

    #[test]
    fn test_inner_fields_snake_case() {
        let mut doc = DBMVDocument::new("test");
        doc.add_metric_view(DBMVMetricView {
            name: "test_view".to_string(),
            source: "catalog.schema.table".to_string(),
            dimensions: vec![DBMVDimension {
                name: "dim1".to_string(),
                expr: "col1".to_string(),
                display_name: Some("Dimension 1".to_string()),
                comment: None,
            }],
            measures: vec![DBMVMeasure {
                name: "measure1".to_string(),
                expr: "SUM(col2)".to_string(),
                display_name: None,
                comment: None,
                format: None,
                window: vec![],
            }],
            ..Default::default()
        });

        let yaml = doc.to_yaml().unwrap();
        // Inner fields should be snake_case (Rust default, no rename)
        assert!(yaml.contains("display_name:"));
    }

    #[test]
    fn test_nested_joins() {
        let join = DBMVJoin {
            name: "customers".to_string(),
            source: "catalog.schema.customers".to_string(),
            on: Some("source.customer_id = customers.id".to_string()),
            using: vec![],
            joins: vec![DBMVJoin {
                name: "nation".to_string(),
                source: "catalog.schema.nations".to_string(),
                on: Some("customers.nation_id = nation.id".to_string()),
                using: vec![],
                joins: vec![],
            }],
        };

        let yaml = serde_yaml::to_string(&join).unwrap();
        assert!(yaml.contains("nation"));
        assert!(yaml.contains("customers.nation_id"));

        // Roundtrip
        let parsed: DBMVJoin = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(join, parsed);
    }

    #[test]
    fn test_window_measure() {
        let measure = DBMVMeasure {
            name: "ytd_revenue".to_string(),
            expr: "SUM(revenue)".to_string(),
            display_name: None,
            comment: None,
            format: None,
            window: vec![DBMVWindow {
                order: "order_date".to_string(),
                range: Some("cumulative".to_string()),
                semiadditive: Some("last".to_string()),
            }],
        };

        let yaml = serde_yaml::to_string(&measure).unwrap();
        let parsed: DBMVMeasure = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(measure, parsed);
    }

    #[test]
    fn test_materialization() {
        let mat = DBMVMaterialization {
            schedule: "every 6 hours".to_string(),
            mode: "relaxed".to_string(),
            materialized_views: vec![
                DBMVMaterializedView {
                    name: "baseline".to_string(),
                    view_type: "unaggregated".to_string(),
                    dimensions: vec![],
                    measures: vec![],
                },
                DBMVMaterializedView {
                    name: "revenue_by_date".to_string(),
                    view_type: "aggregated".to_string(),
                    dimensions: vec!["order_date".to_string()],
                    measures: vec!["total_revenue".to_string()],
                },
            ],
        };

        let yaml = serde_yaml::to_string(&mat).unwrap();
        assert!(yaml.contains("materialized_views:"));

        let parsed: DBMVMaterialization = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(mat, parsed);
    }

    #[test]
    fn test_optional_fields_omitted() {
        let view = DBMVMetricView {
            name: "simple".to_string(),
            source: "catalog.schema.table".to_string(),
            ..Default::default()
        };

        let yaml = serde_yaml::to_string(&view).unwrap();
        assert!(!yaml.contains("filter:"));
        assert!(!yaml.contains("comment:"));
        assert!(!yaml.contains("dimensions:"));
        assert!(!yaml.contains("measures:"));
        assert!(!yaml.contains("joins:"));
        assert!(!yaml.contains("materialization:"));
    }
}
