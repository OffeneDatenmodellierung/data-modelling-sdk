//! Tests for CADS (Compute Asset Description Specification) import/export

use data_modelling_core::export::cads::CADSExporter;
use data_modelling_core::import::cads::CADSImporter;
use data_modelling_core::models::Tag;
use data_modelling_core::models::cads::*;

#[test]
fn test_cads_import_basic() {
    let yaml = r#"
apiVersion: v1.0
kind: AIModel
id: 550e8400-e29b-41d4-a716-446655440000
name: sentiment-analysis-model
version: 1.0.0
status: production
domain: ai-ml
tags:
  - ai
  - nlp
description:
  purpose: Sentiment analysis for customer feedback
  usage: Analyze customer reviews and feedback
  limitations: Only supports English language
"#;

    let importer = CADSImporter::new();
    let asset = importer.import(yaml).unwrap();

    assert_eq!(asset.api_version, "v1.0");
    assert_eq!(asset.kind, CADSKind::AIModel);
    assert_eq!(asset.id, "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(asset.name, "sentiment-analysis-model");
    assert_eq!(asset.version, "1.0.0");
    assert_eq!(asset.status, CADSStatus::Production);
    assert_eq!(asset.domain, Some("ai-ml".to_string()));
    assert_eq!(asset.tags.len(), 2);

    if let Some(description) = &asset.description {
        assert_eq!(
            description.purpose,
            Some("Sentiment analysis for customer feedback".to_string())
        );
        assert_eq!(
            description.usage,
            Some("Analyze customer reviews and feedback".to_string())
        );
        assert_eq!(
            description.limitations,
            Some("Only supports English language".to_string())
        );
    } else {
        panic!("Description should be present");
    }
}

#[test]
fn test_cads_export_basic() {
    let asset = CADSAsset {
        dmn_models: None,
        openapi_specs: None,
        api_version: "v1.0".to_string(),
        kind: CADSKind::AIModel,
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        name: "sentiment-analysis-model".to_string(),
        version: "1.0.0".to_string(),
        status: CADSStatus::Production,
        domain: Some("ai-ml".to_string()),
        domain_id: None,
        tags: vec![
            Tag::Simple("ai".to_string()),
            Tag::Simple("nlp".to_string()),
        ],
        description: Some(CADSDescription {
            purpose: Some("Sentiment analysis for customer feedback".to_string()),
            usage: Some("Analyze customer reviews and feedback".to_string()),
            limitations: Some("Only supports English language".to_string()),
            external_links: None,
        }),
        runtime: None,
        sla: None,
        pricing: None,
        team: None,
        risk: None,
        compliance: None,
        validation_profiles: None,
        bpmn_models: None,
        custom_properties: None,
        created_at: None,
        updated_at: None,
    };

    let yaml = CADSExporter::export_asset(&asset);

    assert!(yaml.contains("apiVersion: v1.0"));
    assert!(yaml.contains("kind: AIModel"));
    assert!(yaml.contains("id: 550e8400-e29b-41d4-a716-446655440000"));
    assert!(yaml.contains("name: sentiment-analysis-model"));
    assert!(yaml.contains("version: 1.0.0"));
    assert!(yaml.contains("status: production"));
    assert!(yaml.contains("domain: ai-ml"));
    assert!(yaml.contains("tags:"));
    assert!(yaml.contains("description:"));
}

#[test]
fn test_cads_round_trip() {
    let original_yaml = r#"
apiVersion: v1.0
kind: MLPipeline
id: 660e8400-e29b-41d4-a716-446655440001
name: data-preprocessing-pipeline
version: 2.1.0
status: validated
domain: data-engineering
tags:
  - ml
  - preprocessing
description:
  purpose: Preprocess raw data for ML training
  usage: Clean and transform data before model training
runtime:
  environment: kubernetes
  endpoints:
    - https://api.example.com/preprocess
  resources:
    cpu: "2"
    memory: "4Gi"
sla:
  properties:
    - element: latency
      value: 100
      unit: milliseconds
      driver: operational
team:
  - role: Data Engineer
    name: John Doe
    contact: john.doe@example.com
"#;

    // Import
    let importer = CADSImporter::new();
    let asset = importer.import(original_yaml).unwrap();

    // Verify import
    assert_eq!(asset.kind, CADSKind::MLPipeline);
    assert_eq!(asset.name, "data-preprocessing-pipeline");
    assert!(asset.runtime.is_some());
    assert!(asset.sla.is_some());
    assert!(asset.team.is_some());

    // Export
    let exported_yaml = CADSExporter::export_asset(&asset);

    // Import again
    let asset2 = importer.import(&exported_yaml).unwrap();

    // Verify round-trip
    assert_eq!(asset.kind, asset2.kind);
    assert_eq!(asset.name, asset2.name);
    assert_eq!(asset.version, asset2.version);
    assert_eq!(asset.status, asset2.status);
    assert_eq!(asset.domain, asset2.domain);
}

#[test]
fn test_cads_all_asset_kinds() {
    let kinds = vec![
        ("AIModel", CADSKind::AIModel),
        ("MLPipeline", CADSKind::MLPipeline),
        ("Application", CADSKind::Application),
        ("ETLPipeline", CADSKind::ETLPipeline),
        ("SourceSystem", CADSKind::SourceSystem),
        ("DestinationSystem", CADSKind::DestinationSystem),
    ];

    for (kind_str, kind_enum) in kinds {
        let yaml = format!(
            r#"
apiVersion: v1.0
kind: {}
id: 550e8400-e29b-41d4-a716-446655440000
name: test-asset
version: 1.0.0
status: draft
"#,
            kind_str
        );

        let importer = CADSImporter::new();
        let asset = importer.import(&yaml).unwrap();
        assert_eq!(asset.kind, kind_enum);
    }
}

#[test]
fn test_cads_enhanced_tags() {
    let yaml = r#"
apiVersion: v1.0
kind: Application
id: 550e8400-e29b-41d4-a716-446655440000
name: test-app
version: 1.0.0
status: production
tags:
  - simple-tag
  - Environment:Production
  - SecondaryDomains:[finance, operations]
"#;

    let importer = CADSImporter::new();
    let asset = importer.import(yaml).unwrap();

    assert_eq!(asset.tags.len(), 3);
    assert_eq!(asset.tags[0], Tag::Simple("simple-tag".to_string()));
    assert_eq!(
        asset.tags[1],
        Tag::Pair("Environment".to_string(), "Production".to_string())
    );
    assert_eq!(
        asset.tags[2],
        Tag::List(
            "SecondaryDomains".to_string(),
            vec!["finance".to_string(), "operations".to_string()]
        )
    );
}
