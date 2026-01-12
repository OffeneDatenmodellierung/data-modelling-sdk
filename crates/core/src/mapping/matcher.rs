//! Field matching algorithms for schema mapping

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::config::MappingConfig;
use super::error::{MappingError, MappingResult};
use super::types::{
    FieldGap, FieldMapping, MappingStats, MatchMethod, SchemaMapping, TransformMapping,
    TransformType,
};

/// Match fields between source and target schemas
pub struct SchemaMatcher {
    config: MappingConfig,
}

impl SchemaMatcher {
    /// Create a new schema matcher with default config
    pub fn new() -> Self {
        Self {
            config: MappingConfig::default(),
        }
    }

    /// Create a schema matcher with custom config
    pub fn with_config(config: MappingConfig) -> Self {
        Self { config }
    }

    /// Match source schema fields to target schema fields
    pub fn match_schemas(&self, source: &Value, target: &Value) -> MappingResult<SchemaMapping> {
        let source_fields = extract_fields(source)?;
        let target_fields = extract_fields(target)?;

        let mut mapping = SchemaMapping::empty();
        let mut matched_sources: HashSet<String> = HashSet::new();
        let mut matched_targets: HashSet<String> = HashSet::new();

        // Phase 1: Exact matches
        for (target_path, target_info) in &target_fields {
            if let Some(source_info) = source_fields.get(target_path) {
                let type_compatible =
                    types_compatible(&source_info.field_type, &target_info.field_type);

                mapping.direct_mappings.push(
                    FieldMapping::new(target_path.clone(), target_path.clone())
                        .with_confidence(1.0)
                        .with_type_compatible(type_compatible)
                        .with_match_method(MatchMethod::Exact),
                );
                matched_sources.insert(target_path.clone());
                matched_targets.insert(target_path.clone());
            }
        }

        // Phase 2: Case-insensitive matches
        if self.config.case_insensitive {
            let source_lower: HashMap<String, &String> = source_fields
                .keys()
                .map(|k| (k.to_lowercase(), k))
                .collect();

            for (target_path, target_info) in &target_fields {
                if matched_targets.contains(target_path) {
                    continue;
                }

                let target_lower = target_path.to_lowercase();
                if let Some(source_path) = source_lower.get(&target_lower) {
                    if !matched_sources.contains(*source_path) {
                        let source_info = &source_fields[*source_path];
                        let type_compatible =
                            types_compatible(&source_info.field_type, &target_info.field_type);

                        mapping.direct_mappings.push(
                            FieldMapping::new((*source_path).clone(), target_path.clone())
                                .with_confidence(0.95)
                                .with_type_compatible(type_compatible)
                                .with_match_method(MatchMethod::CaseInsensitive),
                        );
                        matched_sources.insert((*source_path).clone());
                        matched_targets.insert(target_path.clone());
                    }
                }
            }
        }

        // Phase 3: Fuzzy matches
        if self.config.fuzzy_matching {
            for (target_path, target_info) in &target_fields {
                if matched_targets.contains(target_path) {
                    continue;
                }

                let mut best_match: Option<(String, usize, f64)> = None;

                for (source_path, _source_info) in &source_fields {
                    if matched_sources.contains(source_path) {
                        continue;
                    }

                    let distance = levenshtein_distance(
                        &source_path.to_lowercase(),
                        &target_path.to_lowercase(),
                    );

                    if distance <= self.config.max_edit_distance {
                        let max_len = source_path.len().max(target_path.len());
                        let similarity = 1.0 - (distance as f64 / max_len as f64);

                        if similarity >= self.config.min_confidence {
                            match &best_match {
                                Some((_, best_dist, _)) if distance < *best_dist => {
                                    best_match = Some((source_path.clone(), distance, similarity));
                                }
                                None => {
                                    best_match = Some((source_path.clone(), distance, similarity));
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if let Some((source_path, _, similarity)) = best_match {
                    let source_info = &source_fields[&source_path];
                    let type_compatible =
                        types_compatible(&source_info.field_type, &target_info.field_type);

                    mapping.direct_mappings.push(
                        FieldMapping::new(source_path.clone(), target_path.clone())
                            .with_confidence(similarity)
                            .with_type_compatible(type_compatible)
                            .with_match_method(MatchMethod::Fuzzy),
                    );
                    matched_sources.insert(source_path);
                    matched_targets.insert(target_path.clone());
                }
            }
        }

        // Phase 4: Type coercion suggestions
        if self.config.suggest_type_coercions {
            for mapping_item in &mut mapping.direct_mappings {
                if !mapping_item.type_compatible {
                    let source_info = &source_fields[&mapping_item.source_path];
                    let target_info = &target_fields[&mapping_item.target_path];

                    if can_coerce(&source_info.field_type, &target_info.field_type) {
                        // Convert to transformation
                        let transform = TransformMapping::new(
                            vec![mapping_item.source_path.clone()],
                            mapping_item.target_path.clone(),
                            TransformType::TypeCast {
                                from_type: source_info.field_type.clone(),
                                to_type: target_info.field_type.clone(),
                            },
                        )
                        .with_confidence(mapping_item.confidence * 0.9);

                        mapping.transformations.push(transform);
                    }
                }
            }

            // Remove type-incompatible direct mappings that now have transforms
            let transform_targets: HashSet<_> = mapping
                .transformations
                .iter()
                .map(|t| t.target_path.clone())
                .collect();

            mapping
                .direct_mappings
                .retain(|m| m.type_compatible || !transform_targets.contains(&m.target_path));
        }

        // Phase 5: Identify gaps
        if self.config.track_gaps {
            for (target_path, target_info) in &target_fields {
                if !matched_targets.contains(target_path) {
                    let mut gap = FieldGap::new(
                        target_path.clone(),
                        target_info.field_type.clone(),
                        target_info.required,
                    );

                    // Find similar unmatched source fields as suggestions
                    for (source_path, _) in &source_fields {
                        if !matched_sources.contains(source_path) {
                            let distance = levenshtein_distance(
                                &source_path.to_lowercase(),
                                &target_path.to_lowercase(),
                            );
                            if distance <= self.config.max_edit_distance + 2 {
                                gap.suggestions.push(source_path.clone());
                            }
                        }
                    }

                    // Suggest default values for common types
                    gap.suggested_default = suggest_default(&target_info.field_type);

                    mapping.gaps.push(gap);
                }
            }
        }

        // Phase 6: Identify extras
        if self.config.track_extras {
            for source_path in source_fields.keys() {
                if !matched_sources.contains(source_path) {
                    mapping.extras.push(source_path.clone());
                }
            }
        }

        // Calculate statistics
        mapping.stats = MappingStats {
            source_fields: source_fields.len(),
            target_fields: target_fields.len(),
            direct_mapped: mapping.direct_mappings.len(),
            transform_mapped: mapping.transformations.len(),
            gaps_count: mapping.gaps.len(),
            required_gaps: mapping.gaps.iter().filter(|g| g.required).count(),
            extras_count: mapping.extras.len(),
        };

        // Calculate compatibility score
        mapping.compatibility_score = calculate_compatibility_score(&mapping);

        Ok(mapping)
    }
}

impl Default for SchemaMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a field extracted from a schema
#[derive(Debug, Clone)]
struct FieldInfo {
    field_type: String,
    required: bool,
    #[allow(dead_code)]
    format: Option<String>,
}

/// Extract fields from a JSON Schema
fn extract_fields(schema: &Value) -> MappingResult<HashMap<String, FieldInfo>> {
    let mut fields = HashMap::new();

    let properties = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .ok_or_else(|| {
            MappingError::InvalidSchema("Schema must have 'properties' object".to_string())
        })?;

    let required_fields: HashSet<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    extract_properties_recursive(properties, &required_fields, "", &mut fields);

    Ok(fields)
}

fn extract_properties_recursive(
    properties: &serde_json::Map<String, Value>,
    required: &HashSet<&str>,
    prefix: &str,
    fields: &mut HashMap<String, FieldInfo>,
) {
    for (name, prop) in properties {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", prefix, name)
        };

        let field_type = prop
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("any")
            .to_string();

        let format = prop
            .get("format")
            .and_then(|f| f.as_str())
            .map(String::from);

        let is_required = required.contains(name.as_str());

        fields.insert(
            path.clone(),
            FieldInfo {
                field_type: field_type.clone(),
                required: is_required,
                format,
            },
        );

        // Recurse into nested objects
        if field_type == "object" {
            if let Some(nested_props) = prop.get("properties").and_then(|p| p.as_object()) {
                let nested_required: HashSet<&str> = prop
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();

                extract_properties_recursive(nested_props, &nested_required, &path, fields);
            }
        }
    }
}

/// Check if two types are directly compatible
fn types_compatible(source: &str, target: &str) -> bool {
    if source == target {
        return true;
    }

    // Integer is compatible with number
    if source == "integer" && target == "number" {
        return true;
    }

    // Any type matches anything
    if source == "any" || target == "any" {
        return true;
    }

    false
}

/// Check if a type can be coerced to another
fn can_coerce(from: &str, to: &str) -> bool {
    match (from, to) {
        // Numeric conversions
        ("string", "integer") => true,
        ("string", "number") => true,
        ("number", "integer") => true,
        ("integer", "string") => true,
        ("number", "string") => true,
        // Boolean conversions
        ("string", "boolean") => true,
        ("boolean", "string") => true,
        ("integer", "boolean") => true,
        // Null handling
        (_, "null") => false,
        ("null", _) => false,
        _ => false,
    }
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}

/// Calculate overall compatibility score
fn calculate_compatibility_score(mapping: &SchemaMapping) -> f64 {
    if mapping.stats.target_fields == 0 {
        return 1.0;
    }

    let direct_score: f64 = mapping
        .direct_mappings
        .iter()
        .map(|m| m.confidence * if m.type_compatible { 1.0 } else { 0.8 })
        .sum();

    let transform_score: f64 = mapping
        .transformations
        .iter()
        .map(|t| t.confidence * 0.9)
        .sum();

    let total_mapped = direct_score + transform_score;
    let gap_penalty = mapping.stats.required_gaps as f64 * 0.2;

    let raw_score = total_mapped / mapping.stats.target_fields as f64 - gap_penalty;
    raw_score.clamp(0.0, 1.0)
}

/// Suggest a default value for a type
fn suggest_default(field_type: &str) -> Option<Value> {
    match field_type {
        "string" => Some(Value::String(String::new())),
        "integer" => Some(Value::Number(0.into())),
        "number" => Some(Value::Number(serde_json::Number::from_f64(0.0)?)),
        "boolean" => Some(Value::Bool(false)),
        "array" => Some(Value::Array(Vec::new())),
        "object" => Some(Value::Object(serde_json::Map::new())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("abc", "abcd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_types_compatible() {
        assert!(types_compatible("string", "string"));
        assert!(types_compatible("integer", "number"));
        assert!(!types_compatible("string", "integer"));
        assert!(types_compatible("any", "string"));
    }

    #[test]
    fn test_can_coerce() {
        assert!(can_coerce("string", "integer"));
        assert!(can_coerce("number", "string"));
        assert!(!can_coerce("array", "string"));
    }

    #[test]
    fn test_exact_matching() {
        let source = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });

        let matcher = SchemaMatcher::new();
        let result = matcher.match_schemas(&source, &target).unwrap();

        assert_eq!(result.direct_mappings.len(), 2);
        assert!(result.gaps.is_empty());
        assert!(result.extras.is_empty());
        assert_eq!(result.compatibility_score, 1.0);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let source = json!({
            "type": "object",
            "properties": {
                "FirstName": {"type": "string"},
                "LastName": {"type": "string"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "firstname": {"type": "string"},
                "lastname": {"type": "string"}
            }
        });

        let matcher = SchemaMatcher::with_config(MappingConfig::new().with_case_insensitive(true));
        let result = matcher.match_schemas(&source, &target).unwrap();

        assert_eq!(result.direct_mappings.len(), 2);
        assert!(
            result
                .direct_mappings
                .iter()
                .all(|m| m.match_method == MatchMethod::CaseInsensitive)
        );
    }

    #[test]
    fn test_fuzzy_matching() {
        let source = json!({
            "type": "object",
            "properties": {
                "customer_name": {"type": "string"},
                "customer_email": {"type": "string"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "customerName": {"type": "string"},
                "customerEmail": {"type": "string"}
            }
        });

        let matcher = SchemaMatcher::with_config(
            MappingConfig::new()
                .with_fuzzy_matching(true)
                .with_max_edit_distance(5),
        );
        let result = matcher.match_schemas(&source, &target).unwrap();

        assert_eq!(result.direct_mappings.len(), 2);
    }

    #[test]
    fn test_gap_detection() {
        let source = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"},
                "phone": {"type": "string"}
            },
            "required": ["name", "email"]
        });

        let matcher = SchemaMatcher::new();
        let result = matcher.match_schemas(&source, &target).unwrap();

        assert_eq!(result.direct_mappings.len(), 1);
        assert_eq!(result.gaps.len(), 2);
        assert_eq!(result.stats.required_gaps, 1);
    }

    #[test]
    fn test_extras_detection() {
        let source = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "internal_id": {"type": "string"},
                "debug_flag": {"type": "boolean"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let matcher = SchemaMatcher::new();
        let result = matcher.match_schemas(&source, &target).unwrap();

        assert_eq!(result.direct_mappings.len(), 1);
        assert_eq!(result.extras.len(), 2);
    }

    #[test]
    fn test_type_coercion_detection() {
        let source = json!({
            "type": "object",
            "properties": {
                "count": {"type": "string"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        });

        let matcher = SchemaMatcher::with_config(MappingConfig::new().with_fuzzy_matching(false));
        let result = matcher.match_schemas(&source, &target).unwrap();

        // Should have a transformation for type cast
        assert_eq!(result.transformations.len(), 1);
        assert!(matches!(
            result.transformations[0].transform_type,
            TransformType::TypeCast { .. }
        ));
    }

    #[test]
    fn test_nested_properties() {
        let source = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "email": {"type": "string"}
                    }
                }
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "email": {"type": "string"}
                    }
                }
            }
        });

        let matcher = SchemaMatcher::new();
        let result = matcher.match_schemas(&source, &target).unwrap();

        // Should match: user, user.name, user.email
        assert_eq!(result.direct_mappings.len(), 3);
    }
}
