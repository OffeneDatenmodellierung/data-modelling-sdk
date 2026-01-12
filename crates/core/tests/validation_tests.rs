//! Comprehensive validation tests

use data_modelling_core::models::enums::{DataVaultClassification, DatabaseType, SCDPattern};
use data_modelling_core::models::{Relationship, Table};
use data_modelling_core::validation::input::{
    ValidationError, sanitize_description, sanitize_sql_identifier, validate_column_name,
    validate_data_type, validate_table_name, validate_uuid,
};
use data_modelling_core::validation::relationships::RelationshipValidator;
use data_modelling_core::validation::tables::TableValidator;
use uuid::Uuid;

mod input_validation_tests {
    use super::*;

    #[test]
    fn test_validate_table_name_edge_cases() {
        // Exactly at max length
        let max_name = "a".repeat(255);
        assert!(validate_table_name(&max_name).is_ok());

        // One over max length
        let too_long = "a".repeat(256);
        assert!(matches!(
            validate_table_name(&too_long),
            Err(ValidationError::TooLong { .. })
        ));

        // Unicode characters
        assert!(validate_table_name("tëst_täblë").is_ok());

        // Starts with underscore
        assert!(validate_table_name("_private").is_ok());

        // Contains hyphen
        assert!(validate_table_name("my-table").is_ok());
    }

    #[test]
    fn test_validate_column_name_edge_cases() {
        // Exactly at max length
        let max_name = "a".repeat(255);
        assert!(validate_column_name(&max_name).is_ok());

        // One over max length
        let too_long = "a".repeat(256);
        assert!(matches!(
            validate_column_name(&too_long),
            Err(ValidationError::TooLong { .. })
        ));

        // Deeply nested columns
        assert!(validate_column_name("a.b.c.d.e.f").is_ok());

        // Nested column with reserved word (should be allowed)
        assert!(validate_column_name("data.select").is_ok());

        // Starts with digit (should fail)
        assert!(matches!(
            validate_column_name("123column"),
            Err(ValidationError::InvalidFormat(..))
        ));
    }

    #[test]
    fn test_validate_data_type_edge_cases() {
        // Complex nested types
        assert!(validate_data_type("STRUCT<id INT, name STRING>").is_ok());
        assert!(validate_data_type("ARRAY<STRUCT<field STRING>>").is_ok());
        assert!(validate_data_type("MAP<STRING, INT>").is_ok());

        // Decimal with precision
        assert!(validate_data_type("DECIMAL(10, 2)").is_ok());
        assert!(validate_data_type("NUMERIC(18, 4)").is_ok());

        // SQL injection attempts
        assert!(matches!(
            validate_data_type("'; DROP TABLE users;--"),
            Err(ValidationError::InvalidCharacters { .. })
        ));
        assert!(matches!(
            validate_data_type("INT); DELETE FROM users;--"),
            Err(ValidationError::InvalidCharacters { .. })
        ));
    }

    #[test]
    fn test_validate_uuid_edge_cases() {
        // Valid UUIDs
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_uuid("00000000-0000-0000-0000-000000000000").is_ok());
        assert!(validate_uuid("ffffffff-ffff-ffff-ffff-ffffffffffff").is_ok());

        // Invalid formats
        assert!(validate_uuid("not-a-uuid").is_err());
        assert!(validate_uuid("550e8400-e29b-41d4-a716").is_err());
        // Note: UUID parser may accept format without dashes, so we test a clearly invalid one
        assert!(validate_uuid("not-a-valid-uuid-format-at-all").is_err());
    }

    #[test]
    fn test_sanitize_sql_identifier_edge_cases() {
        // Special characters
        assert_eq!(
            sanitize_sql_identifier("user-name", "postgres"),
            "\"user-name\""
        );
        assert_eq!(sanitize_sql_identifier("user_name", "mysql"), "`user_name`");

        // Already quoted (should handle gracefully)
        let quoted = sanitize_sql_identifier("users", "postgres");
        assert!(quoted.starts_with('"') && quoted.ends_with('"'));

        // Empty string
        assert_eq!(sanitize_sql_identifier("", "postgres"), "\"\"");

        // Very long identifier
        let long = "a".repeat(100);
        let sanitized = sanitize_sql_identifier(&long, "postgres");
        assert!(sanitized.len() > long.len()); // Should be quoted
    }

    #[test]
    fn test_sanitize_description_edge_cases() {
        // Empty description
        assert_eq!(sanitize_description(""), "");

        // Very long description
        let long_desc = "a".repeat(10000);
        let sanitized = sanitize_description(&long_desc);
        assert_eq!(sanitized.len(), 10000);

        // Description with control characters
        assert_eq!(
            sanitize_description("Line1\nLine2\r\nLine3"),
            "Line1\nLine2\r\nLine3"
        );

        // Description with tabs
        assert_eq!(sanitize_description("Col1\tCol2\tCol3"), "Col1\tCol2\tCol3");
    }
}

mod table_validation_tests {
    use super::*;

    #[test]
    fn test_detect_naming_conflicts_with_database_type() {
        let validator = TableValidator::new();

        let mut existing = Table::new("users".to_string(), vec![]);
        existing.database_type = Some(DatabaseType::Postgres);

        let mut new_table_same_db = Table::new("users".to_string(), vec![]);
        new_table_same_db.database_type = Some(DatabaseType::Postgres);

        let mut new_table_different_db = Table::new("users".to_string(), vec![]);
        new_table_different_db.database_type = Some(DatabaseType::Mysql);

        let conflicts =
            validator.detect_naming_conflicts(&[existing.clone()], &[new_table_same_db]);
        assert_eq!(conflicts.len(), 1);

        let conflicts = validator.detect_naming_conflicts(&[existing], &[new_table_different_db]);
        assert_eq!(conflicts.len(), 0); // Different database type, no conflict
    }

    #[test]
    fn test_detect_naming_conflicts_with_schema() {
        let validator = TableValidator::new();

        let mut existing = Table::new("users".to_string(), vec![]);
        existing.schema_name = Some("public".to_string());

        let mut new_table_same_schema = Table::new("users".to_string(), vec![]);
        new_table_same_schema.schema_name = Some("public".to_string());

        let mut new_table_different_schema = Table::new("users".to_string(), vec![]);
        new_table_different_schema.schema_name = Some("private".to_string());

        let conflicts =
            validator.detect_naming_conflicts(&[existing.clone()], &[new_table_same_schema]);
        assert_eq!(conflicts.len(), 1);

        let conflicts =
            validator.detect_naming_conflicts(&[existing], &[new_table_different_schema]);
        assert_eq!(conflicts.len(), 0); // Different schema, no conflict
    }

    #[test]
    fn test_detect_naming_conflicts_multiple_conflicts() {
        let validator = TableValidator::new();

        let existing1 = Table::new("users".to_string(), vec![]);
        let existing2 = Table::new("orders".to_string(), vec![]);

        let new1 = Table::new("users".to_string(), vec![]);
        let new2 = Table::new("orders".to_string(), vec![]);
        let new3 = Table::new("products".to_string(), vec![]);

        let conflicts =
            validator.detect_naming_conflicts(&[existing1, existing2], &[new1, new2, new3]);
        assert_eq!(conflicts.len(), 2); // users and orders conflict
    }

    #[test]
    fn test_validate_pattern_exclusivity_valid_cases() {
        let validator = TableValidator::new();

        // Only SCD pattern
        let mut table_scd = Table::new("test".to_string(), vec![]);
        table_scd.scd_pattern = Some(SCDPattern::Type2);
        table_scd.data_vault_classification = None;
        assert!(validator.validate_pattern_exclusivity(&table_scd).is_ok());

        // Only Data Vault
        let mut table_dv = Table::new("test".to_string(), vec![]);
        table_dv.scd_pattern = None;
        table_dv.data_vault_classification = Some(DataVaultClassification::Hub);
        assert!(validator.validate_pattern_exclusivity(&table_dv).is_ok());

        // Neither
        let table_none = Table::new("test".to_string(), vec![]);
        assert!(validator.validate_pattern_exclusivity(&table_none).is_ok());
    }

    #[test]
    fn test_validate_pattern_exclusivity_violation() {
        let validator = TableValidator::new();

        let mut table = Table::new("test".to_string(), vec![]);
        table.scd_pattern = Some(SCDPattern::Type2);
        table.data_vault_classification = Some(DataVaultClassification::Hub);

        let result = validator.validate_pattern_exclusivity(&table);
        assert!(result.is_err());
        let violation = result.unwrap_err();
        assert_eq!(violation.table_id, table.id);
        assert_eq!(violation.table_name, "test");
        assert!(violation.message.contains("mutually exclusive"));
    }
}

mod relationship_validation_tests {
    use super::*;

    #[test]
    fn test_check_circular_dependency_no_cycle() {
        let validator = RelationshipValidator::new();

        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();
        let table_c = Uuid::new_v4();

        let rels = vec![
            Relationship::new(table_a, table_b),
            Relationship::new(table_b, table_c),
        ];

        // Adding C -> D should not create a cycle
        let table_d = Uuid::new_v4();
        let (has_cycle, _) = validator
            .check_circular_dependency(&rels, table_c, table_d)
            .unwrap();
        assert!(!has_cycle);
    }

    #[test]
    fn test_check_circular_dependency_simple_cycle() {
        let validator = RelationshipValidator::new();

        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();

        let rels = vec![Relationship::new(table_a, table_b)];

        // Adding B -> A creates a cycle
        let (has_cycle, path) = validator
            .check_circular_dependency(&rels, table_b, table_a)
            .unwrap();
        assert!(has_cycle);
        assert!(path.is_some());
    }

    #[test]
    fn test_check_circular_dependency_complex_cycle() {
        let validator = RelationshipValidator::new();

        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();
        let table_c = Uuid::new_v4();
        let table_d = Uuid::new_v4();

        let rels = vec![
            Relationship::new(table_a, table_b),
            Relationship::new(table_b, table_c),
            Relationship::new(table_c, table_d),
        ];

        // Adding D -> A creates a cycle: A -> B -> C -> D -> A
        let (has_cycle, path) = validator
            .check_circular_dependency(&rels, table_d, table_a)
            .unwrap();
        assert!(has_cycle);
        assert!(path.is_some());
    }

    #[test]
    fn test_check_circular_dependency_self_reference() {
        let validator = RelationshipValidator::new();

        let table_a = Uuid::new_v4();
        let rels = vec![];

        // Self-reference creates a cycle
        let (has_cycle, _) = validator
            .check_circular_dependency(&rels, table_a, table_a)
            .unwrap();
        assert!(has_cycle);
    }

    #[test]
    fn test_validate_no_self_reference_valid() {
        let validator = RelationshipValidator::new();

        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();

        assert!(
            validator
                .validate_no_self_reference(table_a, table_b)
                .is_ok()
        );
    }

    #[test]
    fn test_validate_no_self_reference_violation() {
        let validator = RelationshipValidator::new();

        let table_id = Uuid::new_v4();
        let result = validator.validate_no_self_reference(table_id, table_id);
        assert!(result.is_err());
        let self_ref = result.unwrap_err();
        assert_eq!(self_ref.table_id, table_id);
    }

    #[test]
    fn test_check_circular_dependency_empty_graph() {
        let validator = RelationshipValidator::new();

        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();

        // No existing relationships, adding A -> B should not create a cycle
        let (has_cycle, _) = validator
            .check_circular_dependency(&[], table_a, table_b)
            .unwrap();
        assert!(!has_cycle);
    }

    #[test]
    fn test_check_circular_dependency_multiple_paths_no_cycle() {
        let validator = RelationshipValidator::new();

        let table_a = Uuid::new_v4();
        let table_b = Uuid::new_v4();
        let table_c = Uuid::new_v4();
        let table_d = Uuid::new_v4();

        // A -> B -> C
        // A -> D -> C
        // No cycle, multiple paths to C
        let rels = vec![
            Relationship::new(table_a, table_b),
            Relationship::new(table_b, table_c),
            Relationship::new(table_a, table_d),
            Relationship::new(table_d, table_c),
        ];

        // Adding C -> E should not create a cycle
        let table_e = Uuid::new_v4();
        let (has_cycle, _) = validator
            .check_circular_dependency(&rels, table_c, table_e)
            .unwrap();
        assert!(!has_cycle);
    }
}
