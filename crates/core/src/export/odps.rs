//! ODPS (Open Data Product Standard) exporter
//!
//! Exports ODPSDataProduct models to ODPS YAML format.

use crate::export::ExportError;
use crate::models::odps::*;

/// ODPS exporter for generating ODPS YAML from ODPSDataProduct models
pub struct ODPSExporter;

impl ODPSExporter {
    /// Export a Data Product to ODPS YAML format (instance method for WASM compatibility)
    ///
    /// # Arguments
    ///
    /// * `product` - The Data Product to export
    ///
    /// # Returns
    ///
    /// A Result containing the YAML string in ODPS format, or an ExportError
    pub fn export(&self, product: &ODPSDataProduct) -> Result<String, ExportError> {
        let yaml = Self::export_product(product);

        // Validate exported YAML against ODPS schema (if feature enabled)
        #[cfg(feature = "odps-validation")]
        {
            use crate::validation::schema::validate_odps_internal;
            validate_odps_internal(&yaml).map_err(ExportError::ValidationError)?;
        }

        Ok(yaml)
    }

    /// Export a Data Product to ODPS YAML format
    ///
    /// # Arguments
    ///
    /// * `product` - The Data Product to export
    ///
    /// # Returns
    ///
    /// A YAML string in ODPS format
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::export::odps::ODPSExporter;
    /// use data_modelling_core::models::odps::*;
    ///
    /// let product = ODPSDataProduct {
    ///     api_version: "v1.0.0".to_string(),
    ///     kind: "DataProduct".to_string(),
    ///     id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    ///     name: Some("customer-data-product".to_string()),
    ///     version: Some("1.0.0".to_string()),
    ///     status: ODPSStatus::Active,
    ///     domain: None,
    ///     tenant: None,
    ///     authoritative_definitions: None,
    ///     description: None,
    ///     custom_properties: None,
    ///     tags: vec![],
    ///     input_ports: None,
    ///     output_ports: None,
    ///     management_ports: None,
    ///     support: None,
    ///     team: None,
    ///     product_created_ts: None,
    ///     created_at: None,
    ///     updated_at: None,
    /// };
    ///
    /// let yaml = ODPSExporter::export_product(&product);
    /// assert!(yaml.contains("apiVersion: v1.0.0"));
    /// assert!(yaml.contains("kind: DataProduct"));
    /// ```
    pub fn export_product(product: &ODPSDataProduct) -> String {
        // Use direct struct serialization - serde handles all field naming and optional fields
        match serde_yaml::to_string(product) {
            Ok(yaml) => yaml,
            Err(e) => format!("# Error serializing product: {}\n", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_product_basic() {
        let product = ODPSDataProduct {
            api_version: "v1.0.0".to_string(),
            kind: "DataProduct".to_string(),
            id: "test-id".to_string(),
            name: Some("Test Product".to_string()),
            version: Some("1.0.0".to_string()),
            status: ODPSStatus::Active,
            domain: Some("test-domain".to_string()),
            tenant: None,
            authoritative_definitions: None,
            description: None,
            custom_properties: None,
            tags: vec![],
            input_ports: None,
            output_ports: None,
            management_ports: None,
            support: None,
            team: None,
            product_created_ts: None,
            created_at: None,
            updated_at: None,
        };

        let yaml = ODPSExporter::export_product(&product);
        assert!(yaml.contains("apiVersion: v1.0.0"));
        assert!(yaml.contains("kind: DataProduct"));
        assert!(yaml.contains("id: test-id"));
        assert!(yaml.contains("status: active"));
        assert!(yaml.contains("name: Test Product"));
        assert!(yaml.contains("domain: test-domain"));
    }

    #[test]
    fn test_export_product_with_ports() {
        let product = ODPSDataProduct {
            api_version: "v1.0.0".to_string(),
            kind: "DataProduct".to_string(),
            id: "test-id".to_string(),
            name: None,
            version: None,
            status: ODPSStatus::Draft,
            domain: None,
            tenant: None,
            authoritative_definitions: None,
            description: None,
            custom_properties: None,
            tags: vec![],
            input_ports: Some(vec![ODPSInputPort {
                name: "input-1".to_string(),
                version: "1.0.0".to_string(),
                contract_id: "contract-123".to_string(),
                tags: vec![],
                custom_properties: None,
                authoritative_definitions: None,
            }]),
            output_ports: Some(vec![ODPSOutputPort {
                name: "output-1".to_string(),
                description: Some("Output port description".to_string()),
                r#type: Some("dataset".to_string()),
                version: "1.0.0".to_string(),
                contract_id: Some("contract-456".to_string()),
                sbom: None,
                input_contracts: None,
                tags: vec![],
                custom_properties: None,
                authoritative_definitions: None,
            }]),
            management_ports: None,
            support: None,
            team: None,
            product_created_ts: None,
            created_at: None,
            updated_at: None,
        };

        let yaml = ODPSExporter::export_product(&product);
        assert!(yaml.contains("inputPorts:"));
        assert!(yaml.contains("name: input-1"));
        assert!(yaml.contains("contractId: contract-123"));
        assert!(yaml.contains("outputPorts:"));
        assert!(yaml.contains("name: output-1"));
    }

    #[test]
    fn test_export_product_with_team() {
        let product = ODPSDataProduct {
            api_version: "v1.0.0".to_string(),
            kind: "DataProduct".to_string(),
            id: "test-id".to_string(),
            name: None,
            version: None,
            status: ODPSStatus::Active,
            domain: None,
            tenant: None,
            authoritative_definitions: None,
            description: None,
            custom_properties: None,
            tags: vec![],
            input_ports: None,
            output_ports: None,
            management_ports: None,
            support: None,
            team: Some(ODPSTeam {
                name: Some("Data Team".to_string()),
                description: Some("The data team".to_string()),
                members: Some(vec![ODPSTeamMember {
                    username: "user@example.com".to_string(),
                    name: Some("John Doe".to_string()),
                    description: None,
                    role: Some("Lead".to_string()),
                    date_in: Some("2024-01-01".to_string()),
                    date_out: None,
                    replaced_by_username: None,
                    tags: vec![],
                    custom_properties: None,
                    authoritative_definitions: None,
                }]),
                tags: vec![],
                custom_properties: None,
                authoritative_definitions: None,
            }),
            product_created_ts: None,
            created_at: None,
            updated_at: None,
        };

        let yaml = ODPSExporter::export_product(&product);
        assert!(yaml.contains("team:"));
        assert!(yaml.contains("name: Data Team"));
        assert!(yaml.contains("members:"));
        assert!(yaml.contains("username: user@example.com"));
        assert!(yaml.contains("role: Lead"));
    }
}
