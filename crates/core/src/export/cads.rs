//! CADS (Compute Asset Description Specification) exporter
//!
//! Exports CADSAsset models to CADS v1.0 YAML format.

use crate::export::ExportError;
use crate::models::cads::*;

/// CADS exporter for generating CADS v1.0 YAML from CADSAsset models
pub struct CADSExporter;

impl CADSExporter {
    /// Export a CADS asset to CADS v1.0 YAML format (instance method for WASM compatibility)
    ///
    /// # Arguments
    ///
    /// * `asset` - The CADS asset to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string in CADS v1.0 format, or an ExportError
    pub fn export(&self, asset: &CADSAsset) -> Result<String, ExportError> {
        let yaml = Self::export_asset(asset);

        // Validate exported YAML against CADS schema (if feature enabled)
        #[cfg(feature = "schema-validation")]
        {
            use crate::validation::schema::validate_cads_internal;
            validate_cads_internal(&yaml).map_err(ExportError::ValidationError)?;
        }

        Ok(yaml)
    }

    /// Export a CADS asset to CADS v1.0 YAML format
    ///
    /// # Arguments
    ///
    /// * `asset` - The CADS asset to export
    ///
    /// # Returns
    ///
    /// A YAML string in CADS v1.0 format
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::export::cads::CADSExporter;
    /// use data_modelling_core::models::cads::*;
    ///
    /// let asset = CADSAsset {
    ///     api_version: "v1.0".to_string(),
    ///     kind: CADSKind::AIModel,
    ///     id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    ///     name: "sentiment-analysis-model".to_string(),
    ///     version: "1.0.0".to_string(),
    ///     status: CADSStatus::Production,
    ///     domain: None,
    ///     tags: vec![],
    ///     description: None,
    ///     runtime: None,
    ///     sla: None,
    ///     pricing: None,
    ///     team: None,
    ///     risk: None,
    ///     compliance: None,
    ///     validation_profiles: None,
    ///     bpmn_models: None,
    ///     dmn_models: None,
    ///     openapi_specs: None,
    ///     custom_properties: None,
    ///     created_at: None,
    ///     updated_at: None,
    /// };
    ///
    /// let yaml = CADSExporter::export_asset(&asset);
    /// assert!(yaml.contains("apiVersion: v1.0"));
    /// assert!(yaml.contains("kind: AIModel"));
    /// ```
    pub fn export_asset(asset: &CADSAsset) -> String {
        // Use direct struct serialization - serde handles all field naming and optional fields
        match serde_yaml::to_string(asset) {
            Ok(yaml) => yaml,
            Err(e) => format!("# Error serializing asset: {}\n", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_asset_basic() {
        let asset = CADSAsset {
            api_version: "v1.0".to_string(),
            kind: CADSKind::AIModel,
            id: "test-id".to_string(),
            name: "Test Model".to_string(),
            version: "1.0.0".to_string(),
            status: CADSStatus::Production,
            domain: Some("ml".to_string()),
            tags: vec![],
            description: None,
            runtime: None,
            sla: None,
            pricing: None,
            team: None,
            risk: None,
            compliance: None,
            validation_profiles: None,
            bpmn_models: None,
            dmn_models: None,
            openapi_specs: None,
            custom_properties: None,
            created_at: None,
            updated_at: None,
        };

        let yaml = CADSExporter::export_asset(&asset);
        assert!(yaml.contains("apiVersion: v1.0"));
        assert!(yaml.contains("kind: AIModel"));
        assert!(yaml.contains("id: test-id"));
        assert!(yaml.contains("status: production"));
        assert!(yaml.contains("name: Test Model"));
        assert!(yaml.contains("domain: ml"));
    }

    #[test]
    fn test_export_asset_with_description() {
        let asset = CADSAsset {
            api_version: "v1.0".to_string(),
            kind: CADSKind::MLPipeline,
            id: "test-id".to_string(),
            name: "Test Pipeline".to_string(),
            version: "1.0.0".to_string(),
            status: CADSStatus::Draft,
            domain: None,
            tags: vec![],
            description: Some(CADSDescription {
                purpose: Some("Data processing pipeline".to_string()),
                limitations: Some("Max 1TB per day".to_string()),
                usage: None,
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
            dmn_models: None,
            openapi_specs: None,
            custom_properties: None,
            created_at: None,
            updated_at: None,
        };

        let yaml = CADSExporter::export_asset(&asset);
        assert!(yaml.contains("description:"));
        assert!(yaml.contains("purpose: Data processing pipeline"));
        assert!(yaml.contains("limitations: Max 1TB per day"));
    }

    #[test]
    fn test_export_asset_with_runtime() {
        let asset = CADSAsset {
            api_version: "v1.0".to_string(),
            kind: CADSKind::Application,
            id: "test-id".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            status: CADSStatus::Validated,
            domain: None,
            tags: vec![],
            description: None,
            runtime: Some(CADSRuntime {
                environment: Some("kubernetes".to_string()),
                endpoints: Some(vec!["https://api.example.com".to_string()]),
                container: Some(CADSRuntimeContainer {
                    image: Some("myapp:latest".to_string()),
                }),
                resources: Some(CADSRuntimeResources {
                    cpu: Some("2".to_string()),
                    memory: Some("4Gi".to_string()),
                    gpu: None,
                }),
            }),
            sla: None,
            pricing: None,
            team: None,
            risk: None,
            compliance: None,
            validation_profiles: None,
            bpmn_models: None,
            dmn_models: None,
            openapi_specs: None,
            custom_properties: None,
            created_at: None,
            updated_at: None,
        };

        let yaml = CADSExporter::export_asset(&asset);
        assert!(yaml.contains("runtime:"));
        assert!(yaml.contains("environment: kubernetes"));
        assert!(yaml.contains("container:"));
        assert!(yaml.contains("image: myapp:latest"));
    }

    #[test]
    fn test_export_asset_with_team() {
        let asset = CADSAsset {
            api_version: "v1.0".to_string(),
            kind: CADSKind::AIModel,
            id: "test-id".to_string(),
            name: "Test Model".to_string(),
            version: "1.0.0".to_string(),
            status: CADSStatus::Production,
            domain: None,
            tags: vec![],
            description: None,
            runtime: None,
            sla: None,
            pricing: None,
            team: Some(vec![CADSTeamMember {
                role: "owner".to_string(),
                name: "John Doe".to_string(),
                contact: Some("user@example.com".to_string()),
            }]),
            risk: None,
            compliance: None,
            validation_profiles: None,
            bpmn_models: None,
            dmn_models: None,
            openapi_specs: None,
            custom_properties: None,
            created_at: None,
            updated_at: None,
        };

        let yaml = CADSExporter::export_asset(&asset);
        assert!(yaml.contains("team:"));
        assert!(yaml.contains("name: John Doe"));
        assert!(yaml.contains("role: owner"));
    }

    #[test]
    fn test_export_all_kinds() {
        let kinds = vec![
            (CADSKind::AIModel, "AIModel"),
            (CADSKind::MLPipeline, "MLPipeline"),
            (CADSKind::Application, "Application"),
            (CADSKind::ETLPipeline, "ETLPipeline"),
            (CADSKind::SourceSystem, "SourceSystem"),
            (CADSKind::DestinationSystem, "DestinationSystem"),
            (CADSKind::DataPipeline, "DataPipeline"),
            (CADSKind::ETLProcess, "ETLProcess"),
        ];

        for (kind, expected_str) in kinds {
            let asset = CADSAsset {
                api_version: "v1.0".to_string(),
                kind,
                id: "test-id".to_string(),
                name: "Test".to_string(),
                version: "1.0.0".to_string(),
                status: CADSStatus::Draft,
                domain: None,
                tags: vec![],
                description: None,
                runtime: None,
                sla: None,
                pricing: None,
                team: None,
                risk: None,
                compliance: None,
                validation_profiles: None,
                bpmn_models: None,
                dmn_models: None,
                openapi_specs: None,
                custom_properties: None,
                created_at: None,
                updated_at: None,
            };

            let yaml = CADSExporter::export_asset(&asset);
            assert!(
                yaml.contains(&format!("kind: {}", expected_str)),
                "Expected kind '{}' not found in YAML",
                expected_str
            );
        }
    }
}
