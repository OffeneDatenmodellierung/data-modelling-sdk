//! Enhanced tag support with Simple, Pair, and List formats
//!
//! Tags support three formats:
//! - Simple: "finance" (single word)
//! - Pair: "Environment:Dev" (key:value)
//! - List: "SecondaryDomains:[XXXXX, PPPP]" (key:[value1, value2, ...])

use std::fmt;
use std::str::FromStr;

/// Tag enum supporting Simple, Pair, and List formats
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tag {
    /// Simple tag: single word (e.g., "finance")
    Simple(String),
    /// Pair tag: key:value format (e.g., "Environment:Dev")
    Pair(String, String),
    /// List tag: key:[value1, value2, ...] format (e.g., "SecondaryDomains:[XXXXX, PPPP]")
    List(String, Vec<String>),
}

impl FromStr for Tag {
    type Err = ();

    /// Parse a tag string into a Tag enum with auto-detection
    ///
    /// Parsing logic:
    /// - No colon = Simple tag
    /// - Single colon (not followed by bracket, and no more colons) = Pair tag
    /// - Colon followed by bracket = List tag
    /// - Multiple colons without brackets = Simple tag (malformed, graceful degradation)
    ///
    /// Malformed tags are treated as Simple tags (graceful degradation)
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Check for List format: "Key:[Value1, Value2, ...]"
        if let Some(colon_pos) = s.find(':') {
            let key = s[..colon_pos].trim().to_string();
            let value_part = s[colon_pos + 1..].trim();

            // Check if value part starts with '[' (List format)
            if value_part.starts_with('[') && value_part.ends_with(']') {
                // Extract values between brackets
                let values_str = &value_part[1..value_part.len() - 1];
                let values: Vec<String> = values_str
                    .split(',')
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .collect();

                if !key.is_empty() && !values.is_empty() {
                    return Ok(Tag::List(key, values));
                }
            } else {
                // Check if there are multiple colons (malformed - treat as Simple)
                if value_part.contains(':') {
                    // Multiple colons without brackets -> Simple tag
                    return Ok(Tag::Simple(s.to_string()));
                }

                // Pair format: "Key:Value" (single colon, no brackets)
                let value = value_part.to_string();
                if !key.is_empty() && !value.is_empty() {
                    return Ok(Tag::Pair(key, value));
                }
            }
        }

        // Simple tag: no colon, or malformed (fallback to Simple)
        if !s.is_empty() {
            Ok(Tag::Simple(s.to_string()))
        } else {
            Err(())
        }
    }
}

impl fmt::Display for Tag {
    /// Serialize Tag enum to string format
    ///
    /// Formats:
    /// - Simple: "finance"
    /// - Pair: "Environment:Dev"
    /// - List: "SecondaryDomains:[XXXXX, PPPP]"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tag::Simple(s) => write!(f, "{}", s),
            Tag::Pair(key, value) => write!(f, "{}:{}", key, value),
            Tag::List(key, values) => {
                let values_str = values.join(", ");
                write!(f, "{}:[{}]", key, values_str)
            }
        }
    }
}

impl serde::Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Tag::from_str(&s).map_err(|_| serde::de::Error::custom("Invalid tag format"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_list_tag_with_spaces() {
        let tag = Tag::from_str("SecondaryDomains:[XXXXX,  PPPP  ,  QQQQ]").unwrap();
        assert_eq!(
            tag,
            Tag::List(
                "SecondaryDomains".to_string(),
                vec!["XXXXX".to_string(), "PPPP".to_string(), "QQQQ".to_string()]
            )
        );
    }

    #[test]
    fn test_malformed_tag_fallback() {
        // Multiple colons without brackets -> Simple tag
        let tag = Tag::from_str("Key:Value1:Value2").unwrap();
        assert_eq!(tag, Tag::Simple("Key:Value1:Value2".to_string()));
    }

    #[test]
    fn test_empty_tag_error() {
        assert!(Tag::from_str("").is_err());
        assert!(Tag::from_str("   ").is_err());
    }

    #[test]
    fn test_tag_serialization() {
        let simple = Tag::Simple("finance".to_string());
        let pair = Tag::Pair("Environment".to_string(), "Dev".to_string());
        let list = Tag::List(
            "SecondaryDomains".to_string(),
            vec!["XXXXX".to_string(), "PPPP".to_string()],
        );

        assert_eq!(simple.to_string(), "finance");
        assert_eq!(pair.to_string(), "Environment:Dev");
        assert_eq!(list.to_string(), "SecondaryDomains:[XXXXX, PPPP]");
    }
}
