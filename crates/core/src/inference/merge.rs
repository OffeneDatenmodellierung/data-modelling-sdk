//! Schema merging utilities
//!
//! Provides functionality to merge multiple inferred schemas into a single
//! unified schema, finding the minimum common structure.

use std::collections::BTreeMap;

use super::types::{InferredField, InferredSchema, InferredType};

/// Merge multiple schemas into a single unified schema
///
/// This function combines schemas by:
/// - Merging property types when compatible
/// - Marking fields as optional if not present in all schemas
/// - Promoting types when necessary (e.g., integer to number)
/// - Creating union types for incompatible types
pub fn merge_schemas(schemas: Vec<InferredSchema>) -> InferredSchema {
    if schemas.is_empty() {
        return InferredSchema::new();
    }

    if schemas.len() == 1 {
        return schemas.into_iter().next().unwrap();
    }

    let mut result = InferredSchema::new();
    result.record_count = schemas.iter().map(|s| s.record_count).sum();

    // Merge all root types
    let mut merged_root = InferredType::Unknown;
    for schema in &schemas {
        merged_root = merged_root.merge_with(schema.root.clone());
    }

    // If all schemas are objects, do a proper property merge
    if let InferredType::Object { properties: _ } = &merged_root {
        merged_root = merge_object_types(&schemas);
    }

    result.root = merged_root;

    // Merge field stats
    for schema in schemas {
        for (key, stats) in schema.field_stats {
            result
                .field_stats
                .entry(key)
                .and_modify(|existing| {
                    existing.occurrences += stats.occurrences;
                    existing.null_count += stats.null_count;
                    if let (Some(min), Some(other_min)) = (&mut existing.min, stats.min) {
                        *min = min.min(other_min);
                    }
                    if let (Some(max), Some(other_max)) = (&mut existing.max, stats.max) {
                        *max = max.max(other_max);
                    }
                })
                .or_insert(stats);
        }
    }

    result
}

/// Merge object types from multiple schemas
fn merge_object_types(schemas: &[InferredSchema]) -> InferredType {
    let mut all_properties: BTreeMap<String, Vec<InferredField>> = BTreeMap::new();
    let schema_count = schemas.len();

    // Collect all properties from all schemas
    for schema in schemas {
        if let InferredType::Object { ref properties } = schema.root {
            for (name, field) in properties {
                all_properties
                    .entry(name.clone())
                    .or_default()
                    .push(field.clone());
            }
        }
    }

    // Merge properties
    let mut merged_properties = BTreeMap::new();
    for (name, fields) in all_properties {
        let appears_in_all = fields.len() == schema_count;

        // Merge all field definitions
        let mut merged_field = fields.into_iter().reduce(|a, b| a.merge_with(b)).unwrap();

        // If field doesn't appear in all schemas, it's optional
        if !appears_in_all {
            merged_field.required = false;
        }

        merged_properties.insert(name, merged_field);
    }

    InferredType::Object {
        properties: merged_properties,
    }
}

/// Calculate schema similarity score (0.0 - 1.0)
///
/// Higher scores indicate more similar schemas that are good candidates for merging.
pub fn schema_similarity(a: &InferredSchema, b: &InferredSchema) -> f64 {
    match (&a.root, &b.root) {
        (
            InferredType::Object {
                properties: props_a,
            },
            InferredType::Object {
                properties: props_b,
            },
        ) => {
            if props_a.is_empty() && props_b.is_empty() {
                return 1.0;
            }

            let keys_a: std::collections::HashSet<_> = props_a.keys().collect();
            let keys_b: std::collections::HashSet<_> = props_b.keys().collect();

            let intersection = keys_a.intersection(&keys_b).count();
            let union = keys_a.union(&keys_b).count();

            if union == 0 {
                1.0
            } else if intersection == 0 {
                // No field overlap means completely different schemas
                0.0
            } else {
                // Jaccard similarity
                let jaccard = intersection as f64 / union as f64;

                // Also consider type compatibility for shared fields
                let mut type_matches = 0;
                for key in keys_a.intersection(&keys_b) {
                    let type_a = &props_a.get(*key).unwrap().field_type;
                    let type_b = &props_b.get(*key).unwrap().field_type;
                    if types_compatible(type_a, type_b) {
                        type_matches += 1;
                    }
                }

                let type_score = type_matches as f64 / intersection as f64;

                // Weighted average: 60% structure, 40% types
                0.6 * jaccard + 0.4 * type_score
            }
        }
        (a, b) if a == b => 1.0,
        _ => 0.0,
    }
}

/// Check if two types are compatible (can be merged without becoming Mixed)
fn types_compatible(a: &InferredType, b: &InferredType) -> bool {
    match (a, b) {
        (x, y) if x == y => true,
        (InferredType::Integer, InferredType::Number) => true,
        (InferredType::Number, InferredType::Integer) => true,
        (InferredType::Null, _) | (_, InferredType::Null) => true,
        (InferredType::String { .. }, InferredType::String { .. }) => true,
        (InferredType::Array { items: a }, InferredType::Array { items: b }) => {
            types_compatible(a, b)
        }
        (InferredType::Object { .. }, InferredType::Object { .. }) => true,
        _ => false,
    }
}

/// Group schemas by similarity using a threshold
///
/// Returns groups of schema indices that should be merged together.
pub fn group_similar_schemas(schemas: &[InferredSchema], threshold: f64) -> Vec<Vec<usize>> {
    if schemas.is_empty() {
        return Vec::new();
    }

    let n = schemas.len();
    let mut visited = vec![false; n];
    let mut groups = Vec::new();

    for i in 0..n {
        if visited[i] {
            continue;
        }

        let mut group = vec![i];
        visited[i] = true;

        for j in (i + 1)..n {
            if visited[j] {
                continue;
            }

            if schema_similarity(&schemas[i], &schemas[j]) >= threshold {
                group.push(j);
                visited[j] = true;
            }
        }

        groups.push(group);
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_object_schema(fields: &[(&str, InferredType)]) -> InferredSchema {
        let mut properties = BTreeMap::new();
        for (name, field_type) in fields {
            properties.insert(name.to_string(), InferredField::new(field_type.clone()));
        }
        InferredSchema {
            name: None,
            description: None,
            root: InferredType::Object { properties },
            record_count: 1,
            partition: None,
            field_stats: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_merge_identical_schemas() {
        let s1 = make_object_schema(&[
            ("name", InferredType::String { format: None }),
            ("age", InferredType::Integer),
        ]);
        let s2 = make_object_schema(&[
            ("name", InferredType::String { format: None }),
            ("age", InferredType::Integer),
        ]);

        let merged = merge_schemas(vec![s1, s2]);
        assert_eq!(merged.record_count, 2);

        if let InferredType::Object { properties } = merged.root {
            assert_eq!(properties.len(), 2);
            assert!(properties["name"].required);
            assert!(properties["age"].required);
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_merge_different_fields() {
        let s1 = make_object_schema(&[("name", InferredType::String { format: None })]);
        let s2 = make_object_schema(&[("age", InferredType::Integer)]);

        let merged = merge_schemas(vec![s1, s2]);

        if let InferredType::Object { properties } = merged.root {
            assert_eq!(properties.len(), 2);
            // Both fields should be optional since they don't appear in all schemas
            assert!(!properties["name"].required);
            assert!(!properties["age"].required);
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_merge_type_promotion() {
        let s1 = make_object_schema(&[("value", InferredType::Integer)]);
        let s2 = make_object_schema(&[("value", InferredType::Number)]);

        let merged = merge_schemas(vec![s1, s2]);

        if let InferredType::Object { properties } = merged.root {
            assert_eq!(properties["value"].field_type, InferredType::Number);
        } else {
            panic!("Expected object type");
        }
    }

    #[test]
    fn test_schema_similarity_identical() {
        let s1 = make_object_schema(&[
            ("a", InferredType::String { format: None }),
            ("b", InferredType::Integer),
        ]);
        let s2 = make_object_schema(&[
            ("a", InferredType::String { format: None }),
            ("b", InferredType::Integer),
        ]);

        assert_eq!(schema_similarity(&s1, &s2), 1.0);
    }

    #[test]
    fn test_schema_similarity_different() {
        let s1 = make_object_schema(&[("a", InferredType::String { format: None })]);
        let s2 = make_object_schema(&[("b", InferredType::Integer)]);

        assert_eq!(schema_similarity(&s1, &s2), 0.0);
    }

    #[test]
    fn test_group_similar_schemas() {
        let s1 = make_object_schema(&[("a", InferredType::String { format: None })]);
        let s2 = make_object_schema(&[("a", InferredType::String { format: None })]);
        let s3 = make_object_schema(&[("b", InferredType::Integer)]);

        let groups = group_similar_schemas(&[s1, s2, s3], 0.5);

        // s1 and s2 should be in one group, s3 in another
        assert_eq!(groups.len(), 2);
    }
}
