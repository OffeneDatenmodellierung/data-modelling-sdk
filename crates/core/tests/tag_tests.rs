//! Tests for enhanced tag support (Simple, Pair, List formats)

use data_modelling_core::models::{Column, Table, Tag};
use std::str::FromStr;

#[test]
fn test_simple_tag_parsing() {
    let tag = Tag::from_str("finance").unwrap();
    assert_eq!(tag, Tag::Simple("finance".to_string()));
    assert_eq!(tag.to_string(), "finance");
}

#[test]
fn test_pair_tag_parsing() {
    let tag = Tag::from_str("Environment:Dev").unwrap();
    assert_eq!(tag, Tag::Pair("Environment".to_string(), "Dev".to_string()));
    assert_eq!(tag.to_string(), "Environment:Dev");
}

#[test]
fn test_list_tag_parsing() {
    let tag = Tag::from_str("SecondaryDomains:[XXXXX, PPPP]").unwrap();
    assert_eq!(
        tag,
        Tag::List(
            "SecondaryDomains".to_string(),
            vec!["XXXXX".to_string(), "PPPP".to_string()]
        )
    );
    assert_eq!(tag.to_string(), "SecondaryDomains:[XXXXX, PPPP]");
}

#[test]
fn test_list_tag_with_single_value() {
    let tag = Tag::from_str("Domain:[Finance]").unwrap();
    assert_eq!(
        tag,
        Tag::List("Domain".to_string(), vec!["Finance".to_string()])
    );
    assert_eq!(tag.to_string(), "Domain:[Finance]");
}

#[test]
fn test_list_tag_with_multiple_values() {
    let tag = Tag::from_str("Tags:[tag1, tag2, tag3]").unwrap();
    assert_eq!(
        tag,
        Tag::List(
            "Tags".to_string(),
            vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()]
        )
    );
}

#[test]
fn test_malformed_tags_treated_as_simple() {
    // Multiple colons without brackets should be treated as Simple
    let tag = Tag::from_str("key:value:extra").unwrap();
    assert_eq!(tag, Tag::Simple("key:value:extra".to_string()));

    // Colon at end - if value is empty, it falls through to Simple tag
    // (implementation requires non-empty value for Pair)
    let tag = Tag::from_str("key:").unwrap();
    assert_eq!(tag, Tag::Simple("key:".to_string()));

    // Empty string should return error (as per implementation)
    assert!(Tag::from_str("").is_err());
}

#[test]
fn test_backward_compatibility_with_simple_string_tags() {
    // Simple tags should work exactly like before
    let tag1 = Tag::Simple("production".to_string());
    let tag2 = Tag::from_str("production").unwrap();
    assert_eq!(tag1, tag2);

    // Table with simple tags should serialize correctly
    let mut table = Table::new(
        "test".to_string(),
        vec![Column::new("id".to_string(), "INT".to_string())],
    );
    table.tags.push(Tag::Simple("finance".to_string()));
    table.tags.push(Tag::Simple("production".to_string()));

    // Verify tags are preserved
    assert_eq!(table.tags.len(), 2);
    assert!(table.tags.contains(&Tag::Simple("finance".to_string())));
    assert!(table.tags.contains(&Tag::Simple("production".to_string())));
}

#[test]
fn test_tag_roundtrip_serialization() {
    // Test that tags serialize and parse back correctly
    let tags = vec![
        Tag::Simple("finance".to_string()),
        Tag::Pair("Environment".to_string(), "Dev".to_string()),
        Tag::List(
            "Domains".to_string(),
            vec!["A".to_string(), "B".to_string()],
        ),
    ];

    for tag in &tags {
        let serialized = tag.to_string();
        let parsed = Tag::from_str(&serialized).unwrap();
        assert_eq!(*tag, parsed, "Tag roundtrip failed for: {}", serialized);
    }
}

#[test]
fn test_tag_equality() {
    let tag1 = Tag::Simple("test".to_string());
    let tag2 = Tag::Simple("test".to_string());
    let tag3 = Tag::Simple("other".to_string());

    assert_eq!(tag1, tag2);
    assert_ne!(tag1, tag3);

    let pair1 = Tag::Pair("key".to_string(), "value".to_string());
    let pair2 = Tag::Pair("key".to_string(), "value".to_string());
    let pair3 = Tag::Pair("key".to_string(), "other".to_string());

    assert_eq!(pair1, pair2);
    assert_ne!(pair1, pair3);

    let list1 = Tag::List("key".to_string(), vec!["a".to_string(), "b".to_string()]);
    let list2 = Tag::List("key".to_string(), vec!["a".to_string(), "b".to_string()]);
    let list3 = Tag::List("key".to_string(), vec!["a".to_string()]);

    assert_eq!(list1, list2);
    assert_ne!(list1, list3);
}

#[test]
fn test_tag_parsing_edge_cases() {
    // Whitespace handling
    let tag = Tag::from_str("  finance  ").unwrap();
    assert_eq!(tag, Tag::Simple("finance".to_string()));

    // Pair with spaces
    let tag = Tag::from_str("Environment: Dev").unwrap();
    assert_eq!(tag, Tag::Pair("Environment".to_string(), "Dev".to_string()));

    // List with spaces
    let tag = Tag::from_str("Domains:[ A , B ]").unwrap();
    assert_eq!(
        tag,
        Tag::List(
            "Domains".to_string(),
            vec!["A".to_string(), "B".to_string()]
        )
    );
}
