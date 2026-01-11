//! Format detection for string values

#![allow(dead_code)]
#![allow(clippy::manual_is_multiple_of)]

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Detected string format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    /// ISO 8601 date (YYYY-MM-DD)
    Date,
    /// ISO 8601 date-time (YYYY-MM-DDTHH:MM:SS)
    DateTime,
    /// Time (HH:MM:SS)
    Time,
    /// Email address
    Email,
    /// URI/URL
    Uri,
    /// UUID/GUID
    Uuid,
    /// IPv4 address
    Ipv4,
    /// IPv6 address
    Ipv6,
    /// Hostname
    Hostname,
    /// JSON Pointer
    JsonPointer,
    /// Regex pattern
    Regex,
    /// Base64 encoded string
    Base64,
    /// Phone number (E.164 format)
    Phone,
    /// Credit card number
    CreditCard,
    /// ISO 3166-1 alpha-2 country code
    CountryCode,
    /// ISO 4217 currency code
    CurrencyCode,
    /// Semantic version
    Semver,
    /// No specific format detected
    None,
}

impl Format {
    /// Get the JSON Schema format string for this format
    pub fn as_json_schema_format(&self) -> Option<&'static str> {
        match self {
            Format::Date => Some("date"),
            Format::DateTime => Some("date-time"),
            Format::Time => Some("time"),
            Format::Email => Some("email"),
            Format::Uri => Some("uri"),
            Format::Uuid => Some("uuid"),
            Format::Ipv4 => Some("ipv4"),
            Format::Ipv6 => Some("ipv6"),
            Format::Hostname => Some("hostname"),
            Format::JsonPointer => Some("json-pointer"),
            Format::Regex => Some("regex"),
            Format::None => None,
            // Non-standard formats (could be custom)
            Format::Base64 => Some("byte"),
            Format::Phone => Some("phone"),
            Format::CreditCard => None,
            Format::CountryCode => None,
            Format::CurrencyCode => None,
            Format::Semver => None,
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Date => write!(f, "date"),
            Format::DateTime => write!(f, "date-time"),
            Format::Time => write!(f, "time"),
            Format::Email => write!(f, "email"),
            Format::Uri => write!(f, "uri"),
            Format::Uuid => write!(f, "uuid"),
            Format::Ipv4 => write!(f, "ipv4"),
            Format::Ipv6 => write!(f, "ipv6"),
            Format::Hostname => write!(f, "hostname"),
            Format::JsonPointer => write!(f, "json-pointer"),
            Format::Regex => write!(f, "regex"),
            Format::Base64 => write!(f, "base64"),
            Format::Phone => write!(f, "phone"),
            Format::CreditCard => write!(f, "credit-card"),
            Format::CountryCode => write!(f, "country-code"),
            Format::CurrencyCode => write!(f, "currency-code"),
            Format::Semver => write!(f, "semver"),
            Format::None => write!(f, "none"),
        }
    }
}

// Regex patterns for format detection
static DATE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());

static DATETIME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})?$").unwrap()
});

static TIME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{2}:\d{2}:\d{2}(\.\d+)?$").unwrap());

static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
        .unwrap()
});

static URI_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(https?|ftp|file)://[^\s/$.?#].[^\s]*$").unwrap());

static IPV4_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$",
    )
    .unwrap()
});

static IPV6_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$|^::$|^([0-9a-fA-F]{1,4}:){1,7}:$")
        .unwrap()
});

static HOSTNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]*[a-zA-Z0-9])(\.[a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]*[a-zA-Z0-9])*$").unwrap()
});

static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap());

static BASE64_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9+/]+=*$").unwrap());

static COUNTRY_CODE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Z]{2}$").unwrap());

static CURRENCY_CODE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Z]{3}$").unwrap());

static SEMVER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?(\+[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?$").unwrap()
});

/// Detect the format of a string value
///
/// Returns the most specific format that matches the value.
/// Checks are ordered from most specific to least specific.
pub fn detect_format(value: &str) -> Format {
    // Skip empty or whitespace-only strings
    let value = value.trim();
    if value.is_empty() {
        return Format::None;
    }

    // Check formats in order of specificity

    // UUID is very specific
    if UUID_REGEX.is_match(value) {
        return Format::Uuid;
    }

    // DateTime before Date (more specific)
    if DATETIME_REGEX.is_match(value) {
        return Format::DateTime;
    }

    // Date
    if DATE_REGEX.is_match(value) {
        return Format::Date;
    }

    // Time
    if TIME_REGEX.is_match(value) {
        return Format::Time;
    }

    // Email
    if EMAIL_REGEX.is_match(value) {
        return Format::Email;
    }

    // URI/URL
    if URI_REGEX.is_match(value) {
        return Format::Uri;
    }

    // IPv4
    if IPV4_REGEX.is_match(value) {
        return Format::Ipv4;
    }

    // IPv6
    if IPV6_REGEX.is_match(value) {
        return Format::Ipv6;
    }

    // Semver
    if SEMVER_REGEX.is_match(value) {
        return Format::Semver;
    }

    // Phone (E.164)
    if PHONE_REGEX.is_match(value) && value.len() >= 8 && value.len() <= 15 {
        return Format::Phone;
    }

    // Country code (must be uppercase letters only)
    if COUNTRY_CODE_REGEX.is_match(value) {
        return Format::CountryCode;
    }

    // Currency code (must be uppercase letters only)
    if CURRENCY_CODE_REGEX.is_match(value) {
        return Format::CurrencyCode;
    }

    // Base64 (only if long enough and valid pattern)
    if value.len() >= 4 && value.len() % 4 == 0 && BASE64_REGEX.is_match(value) {
        return Format::Base64;
    }

    // Hostname (check after URI since URIs contain hostnames)
    if HOSTNAME_REGEX.is_match(value) && value.contains('.') {
        return Format::Hostname;
    }

    Format::None
}

/// Calculate the confidence that a set of values matches a format
pub fn format_confidence(values: &[&str], format: Format) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let matches = values.iter().filter(|v| detect_format(v) == format).count();

    matches as f64 / values.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_date() {
        assert_eq!(detect_format("2024-01-15"), Format::Date);
        assert_eq!(detect_format("2024-12-31"), Format::Date);
        assert_ne!(detect_format("2024-1-15"), Format::Date); // Invalid format
    }

    #[test]
    fn test_detect_datetime() {
        assert_eq!(detect_format("2024-01-15T10:30:00"), Format::DateTime);
        assert_eq!(detect_format("2024-01-15T10:30:00Z"), Format::DateTime);
        assert_eq!(detect_format("2024-01-15T10:30:00+05:00"), Format::DateTime);
        assert_eq!(detect_format("2024-01-15 10:30:00"), Format::DateTime);
    }

    #[test]
    fn test_detect_time() {
        assert_eq!(detect_format("10:30:00"), Format::Time);
        assert_eq!(detect_format("23:59:59.999"), Format::Time);
    }

    #[test]
    fn test_detect_email() {
        assert_eq!(detect_format("user@example.com"), Format::Email);
        assert_eq!(detect_format("user.name+tag@domain.co.uk"), Format::Email);
    }

    #[test]
    fn test_detect_uuid() {
        assert_eq!(
            detect_format("550e8400-e29b-41d4-a716-446655440000"),
            Format::Uuid
        );
        assert_eq!(
            detect_format("550E8400-E29B-41D4-A716-446655440000"),
            Format::Uuid
        );
    }

    #[test]
    fn test_detect_uri() {
        assert_eq!(detect_format("https://example.com"), Format::Uri);
        assert_eq!(detect_format("http://localhost:8080/path"), Format::Uri);
        assert_eq!(
            detect_format("ftp://files.example.com/file.txt"),
            Format::Uri
        );
    }

    #[test]
    fn test_detect_ipv4() {
        assert_eq!(detect_format("192.168.1.1"), Format::Ipv4);
        assert_eq!(detect_format("255.255.255.255"), Format::Ipv4);
        assert_eq!(detect_format("0.0.0.0"), Format::Ipv4);
    }

    #[test]
    fn test_detect_semver() {
        assert_eq!(detect_format("1.0.0"), Format::Semver);
        assert_eq!(detect_format("2.1.3-alpha"), Format::Semver);
        assert_eq!(detect_format("0.0.1+build.123"), Format::Semver);
    }

    #[test]
    fn test_detect_country_code() {
        assert_eq!(detect_format("US"), Format::CountryCode);
        assert_eq!(detect_format("GB"), Format::CountryCode);
        assert_eq!(detect_format("DE"), Format::CountryCode);
    }

    #[test]
    fn test_detect_currency_code() {
        assert_eq!(detect_format("USD"), Format::CurrencyCode);
        assert_eq!(detect_format("EUR"), Format::CurrencyCode);
        assert_eq!(detect_format("GBP"), Format::CurrencyCode);
    }

    #[test]
    fn test_format_confidence() {
        let dates = vec!["2024-01-01", "2024-02-15", "2024-03-20"];
        assert_eq!(format_confidence(&dates, Format::Date), 1.0);

        let mixed = vec!["2024-01-01", "not-a-date", "2024-03-20"];
        assert!((format_confidence(&mixed, Format::Date) - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_empty_and_whitespace() {
        assert_eq!(detect_format(""), Format::None);
        assert_eq!(detect_format("   "), Format::None);
    }
}
