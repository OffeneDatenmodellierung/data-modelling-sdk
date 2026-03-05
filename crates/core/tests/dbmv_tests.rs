//! Integration tests for Databricks Metric Views (DBMV) support

use data_modelling_core::export::DBMVExporter;
use data_modelling_core::import::DBMVImporter;
use data_modelling_core::models::dbmv::*;

const FULL_EXAMPLE_YAML: &str = r#"
apiVersion: v1.0.0
kind: MetricViews
system: sales-databricks
description: Sales domain metrics
metricViews:
  - name: orders_metrics
    version: '1.1'
    source: catalog.schema.orders
    filter: "status = 'completed'"
    comment: Order metrics
    dimensions:
      - name: order_date
        expr: order_date
        display_name: Order Date
        comment: Date the order was placed
    measures:
      - name: total_revenue
        expr: SUM(revenue)
        display_name: Total Revenue
        format:
          type: currency
      - name: ytd_revenue
        expr: SUM(revenue)
        window:
          - order: order_date
            range: cumulative
            semiadditive: last
    joins:
      - name: customers
        source: catalog.schema.customers
        on: "source.customer_id = customers.id"
        joins:
          - name: nation
            source: catalog.schema.nations
            on: "customers.nation_id = nation.id"
    materialization:
      schedule: every 6 hours
      mode: relaxed
      materialized_views:
        - name: baseline
          type: unaggregated
        - name: revenue_by_date
          type: aggregated
          dimensions:
            - order_date
          measures:
            - total_revenue
"#;

#[test]
fn test_import_export_roundtrip() {
    let importer = DBMVImporter::new();
    let doc = importer
        .import_without_validation(FULL_EXAMPLE_YAML)
        .unwrap();

    // Export
    let yaml = DBMVExporter::export_document(&doc);

    // Re-import
    let doc2 = importer.import_without_validation(&yaml).unwrap();

    assert_eq!(doc, doc2);
}

#[test]
fn test_import_full_document() {
    let importer = DBMVImporter::new();
    let doc = importer
        .import_without_validation(FULL_EXAMPLE_YAML)
        .unwrap();

    assert_eq!(doc.api_version, "v1.0.0");
    assert_eq!(doc.kind, "MetricViews");
    assert_eq!(doc.system, "sales-databricks");
    assert_eq!(doc.description, Some("Sales domain metrics".to_string()));
    assert_eq!(doc.metric_views.len(), 1);

    let view = &doc.metric_views[0];
    assert_eq!(view.name, "orders_metrics");
    assert_eq!(view.version, "1.1");
    assert_eq!(view.source, "catalog.schema.orders");
    assert_eq!(view.filter, Some("status = 'completed'".to_string()));
    assert_eq!(view.comment, Some("Order metrics".to_string()));

    // Dimensions
    assert_eq!(view.dimensions.len(), 1);
    assert_eq!(view.dimensions[0].name, "order_date");
    assert_eq!(view.dimensions[0].expr, "order_date");
    assert_eq!(
        view.dimensions[0].display_name,
        Some("Order Date".to_string())
    );

    // Measures
    assert_eq!(view.measures.len(), 2);
    assert_eq!(view.measures[0].name, "total_revenue");
    assert_eq!(view.measures[0].expr, "SUM(revenue)");
    assert_eq!(
        view.measures[0].format.as_ref().unwrap().format_type,
        "currency"
    );
    assert_eq!(view.measures[1].name, "ytd_revenue");
    assert_eq!(view.measures[1].window.len(), 1);
    assert_eq!(view.measures[1].window[0].order, "order_date");
    assert_eq!(
        view.measures[1].window[0].range,
        Some("cumulative".to_string())
    );
    assert_eq!(
        view.measures[1].window[0].semiadditive,
        Some("last".to_string())
    );

    // Joins (nested)
    assert_eq!(view.joins.len(), 1);
    assert_eq!(view.joins[0].name, "customers");
    assert_eq!(view.joins[0].joins.len(), 1);
    assert_eq!(view.joins[0].joins[0].name, "nation");

    // Materialization
    let mat = view.materialization.as_ref().unwrap();
    assert_eq!(mat.schedule, "every 6 hours");
    assert_eq!(mat.mode, "relaxed");
    assert_eq!(mat.materialized_views.len(), 2);
    assert_eq!(mat.materialized_views[0].view_type, "unaggregated");
    assert_eq!(mat.materialized_views[1].view_type, "aggregated");
    assert_eq!(mat.materialized_views[1].dimensions, vec!["order_date"]);
    assert_eq!(mat.materialized_views[1].measures, vec!["total_revenue"]);
}

#[test]
fn test_import_single_view() {
    let importer = DBMVImporter::new();
    let yaml = r#"
name: standalone_metrics
source: catalog.schema.table
dimensions:
  - name: dim1
    expr: col1
measures:
  - name: measure1
    expr: COUNT(*)
"#;
    let view = importer.import_single_view(yaml).unwrap();
    assert_eq!(view.name, "standalone_metrics");
    assert_eq!(view.version, "1.1"); // default
    assert_eq!(view.dimensions.len(), 1);
    assert_eq!(view.measures.len(), 1);
}

#[test]
fn test_export_single_view() {
    let view = DBMVMetricView {
        name: "test_view".to_string(),
        source: "catalog.schema.table".to_string(),
        dimensions: vec![DBMVDimension {
            name: "dim1".to_string(),
            expr: "col1".to_string(),
            display_name: None,
            comment: None,
        }],
        measures: vec![DBMVMeasure {
            name: "m1".to_string(),
            expr: "SUM(col2)".to_string(),
            display_name: None,
            comment: None,
            format: None,
            window: vec![],
        }],
        ..Default::default()
    };

    let yaml = DBMVExporter::export_single_view(&view);
    assert!(yaml.contains("name: test_view"));
    assert!(yaml.contains("source: catalog.schema.table"));
    // Should NOT have envelope fields
    assert!(!yaml.contains("apiVersion"));
    assert!(!yaml.contains("kind"));
}

#[test]
fn test_multiple_views_in_document() {
    let yaml = r#"
apiVersion: v1.0.0
kind: MetricViews
system: multi-view-system
metricViews:
  - name: view_one
    source: catalog.schema.table1
    dimensions:
      - name: d1
        expr: col1
    measures:
      - name: m1
        expr: SUM(col2)
  - name: view_two
    source: catalog.schema.table2
    dimensions:
      - name: d2
        expr: col3
    measures:
      - name: m2
        expr: COUNT(*)
"#;
    let importer = DBMVImporter::new();
    let doc = importer.import_without_validation(yaml).unwrap();

    assert_eq!(doc.metric_views.len(), 2);
    assert_eq!(doc.metric_views[0].name, "view_one");
    assert_eq!(doc.metric_views[1].name, "view_two");

    // Test get_metric_view
    assert!(doc.get_metric_view("view_one").is_some());
    assert!(doc.get_metric_view("view_two").is_some());
    assert!(doc.get_metric_view("view_three").is_none());
}

#[test]
fn test_camel_case_envelope_snake_case_inner() {
    let mut doc = DBMVDocument::new("test-system");
    doc.add_metric_view(DBMVMetricView {
        name: "test".to_string(),
        source: "cat.sch.tbl".to_string(),
        dimensions: vec![DBMVDimension {
            name: "d".to_string(),
            expr: "c".to_string(),
            display_name: Some("Display".to_string()),
            comment: None,
        }],
        ..Default::default()
    });

    let yaml = DBMVExporter::export_document(&doc);

    // Envelope: camelCase
    assert!(yaml.contains("apiVersion:"));
    assert!(yaml.contains("metricViews:"));
    assert!(!yaml.contains("api_version:"));
    assert!(!yaml.contains("metric_views:"));

    // Inner: snake_case
    assert!(yaml.contains("display_name:"));
}

#[test]
fn test_optional_fields_omitted_in_yaml() {
    let doc = DBMVDocument {
        api_version: "v1.0.0".to_string(),
        kind: "MetricViews".to_string(),
        system: "test".to_string(),
        description: None,
        metric_views: vec![DBMVMetricView {
            name: "simple".to_string(),
            source: "cat.sch.tbl".to_string(),
            ..Default::default()
        }],
    };

    let yaml = DBMVExporter::export_document(&doc);

    // Optional fields should be omitted
    assert!(!yaml.contains("description:"));
    assert!(!yaml.contains("filter:"));
    assert!(!yaml.contains("comment:"));
    assert!(!yaml.contains("materialization:"));
}

#[test]
fn test_join_with_using() {
    let yaml = r#"
apiVersion: v1.0.0
kind: MetricViews
system: test
metricViews:
  - name: test_view
    source: cat.sch.tbl
    joins:
      - name: other
        source: cat.sch.other
        using:
          - id
          - code
"#;
    let importer = DBMVImporter::new();
    let doc = importer.import_without_validation(yaml).unwrap();

    let join = &doc.metric_views[0].joins[0];
    assert_eq!(join.using, vec!["id", "code"]);
    assert!(join.on.is_none());
}

#[test]
fn test_default_version_applied() {
    let yaml = r#"
apiVersion: v1.0.0
kind: MetricViews
system: test
metricViews:
  - name: no_version_specified
    source: cat.sch.tbl
"#;
    let importer = DBMVImporter::new();
    let doc = importer.import_without_validation(yaml).unwrap();
    assert_eq!(doc.metric_views[0].version, "1.1");
}

#[test]
fn test_example_file_parseable() {
    let content = include_str!("../../../examples/sales-metrics.dbmv.yaml");
    let importer = DBMVImporter::new();
    let doc = importer.import_without_validation(content).unwrap();

    assert_eq!(doc.system, "sales-databricks");
    assert_eq!(doc.metric_views.len(), 2);
    assert_eq!(doc.metric_views[0].name, "orders_metrics");
    assert_eq!(doc.metric_views[1].name, "customer_metrics");
}

#[test]
fn test_document_convenience_methods() {
    let mut doc = DBMVDocument::new("my-system");
    assert_eq!(doc.system, "my-system");
    assert_eq!(doc.api_version, "v1.0.0");
    assert_eq!(doc.kind, "MetricViews");

    doc.add_metric_view(DBMVMetricView {
        name: "view1".to_string(),
        source: "src".to_string(),
        ..Default::default()
    });
    doc.add_metric_view(DBMVMetricView {
        name: "view2".to_string(),
        source: "src2".to_string(),
        ..Default::default()
    });

    assert_eq!(doc.metric_views.len(), 2);
    assert_eq!(doc.get_metric_view("view1").unwrap().source, "src");
    assert_eq!(doc.get_metric_view("view2").unwrap().source, "src2");
}
