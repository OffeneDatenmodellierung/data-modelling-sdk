//! S3 ingestion support
//!
//! This module provides file discovery and ingestion from Amazon S3 buckets.

use std::path::PathBuf;

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;

use super::error::IngestError;
use super::ingest::DiscoveredFile;

/// S3 source configuration
#[derive(Debug, Clone)]
pub struct S3Source {
    /// S3 bucket name
    pub bucket: String,
    /// Prefix (folder path) within the bucket
    pub prefix: String,
    /// AWS region (optional, uses default if not specified)
    pub region: Option<String>,
    /// AWS profile name (optional)
    pub profile: Option<String>,
    /// Endpoint URL (for S3-compatible storage like MinIO)
    pub endpoint_url: Option<String>,
}

impl S3Source {
    /// Create a new S3 source
    pub fn new(bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: prefix.into(),
            region: None,
            profile: None,
            endpoint_url: None,
        }
    }

    /// Set the AWS region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the AWS profile
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Set a custom endpoint URL (for S3-compatible storage)
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint_url = Some(endpoint.into());
        self
    }

    /// Get the display string for this source
    pub fn display(&self) -> String {
        format!("s3://{}/{}", self.bucket, self.prefix)
    }
}

/// S3 client wrapper with secure credential handling
pub struct S3Ingester {
    client: S3Client,
    source: S3Source,
}

impl S3Ingester {
    /// Create a new S3 ingester with default credentials
    ///
    /// Credentials are loaded from the environment in this order:
    /// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// 2. AWS credentials file (~/.aws/credentials)
    /// 3. IAM role (if running on AWS infrastructure)
    pub async fn new(source: S3Source) -> Result<Self, IngestError> {
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());

        // Set region if specified
        if let Some(region) = &source.region {
            config_loader = config_loader.region(aws_config::Region::new(region.clone()));
        }

        // Set profile if specified
        if let Some(profile) = &source.profile {
            config_loader = config_loader.profile_name(profile);
        }

        let config = config_loader.load().await;

        let mut s3_config = aws_sdk_s3::config::Builder::from(&config);

        // Set custom endpoint if specified
        if let Some(endpoint) = &source.endpoint_url {
            s3_config = s3_config.endpoint_url(endpoint);
            s3_config = s3_config.force_path_style(true);
        }

        let client = S3Client::from_conf(s3_config.build());

        Ok(Self { client, source })
    }

    /// Discover files matching a pattern in the S3 bucket
    ///
    /// # Arguments
    /// * `pattern` - Glob pattern to match (e.g., "*.json", "data/*.jsonl")
    pub async fn discover_files(&self, pattern: &str) -> Result<Vec<DiscoveredFile>, IngestError> {
        let mut files = Vec::new();
        let mut continuation_token: Option<String> = None;

        // Compile the glob pattern
        let glob_pattern = glob::Pattern::new(pattern)
            .map_err(|e| IngestError::InvalidPattern(format!("{}: {}", pattern, e)))?;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.source.bucket)
                .prefix(&self.source.prefix);

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| IngestError::SourceNotAccessible {
                    path: self.source.display(),
                    reason: e.to_string(),
                })?;

            // Process objects
            if let Some(contents) = response.contents {
                for object in contents {
                    if let Some(key) = object.key {
                        // Extract filename from key
                        let filename = key
                            .strip_prefix(&self.source.prefix)
                            .unwrap_or(&key)
                            .trim_start_matches('/');

                        // Check if filename matches pattern
                        if glob_pattern.matches(filename) {
                            let size = object.size.unwrap_or(0) as u64;
                            files.push(DiscoveredFile::new(PathBuf::from(&key), size));
                        }
                    }
                }
            }

            // Check for more results
            if response.is_truncated.unwrap_or(false) {
                continuation_token = response.next_continuation_token;
            } else {
                break;
            }
        }

        // Sort by key for consistent ordering
        files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(files)
    }

    /// Download a file from S3 and return its contents
    pub async fn download_file(&self, key: &str) -> Result<Vec<u8>, IngestError> {
        let response = self
            .client
            .get_object()
            .bucket(&self.source.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| IngestError::SourceNotAccessible {
                path: format!("s3://{}/{}", self.source.bucket, key),
                reason: e.to_string(),
            })?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| IngestError::Io(std::io::Error::other(e.to_string())))?;

        Ok(bytes.into_bytes().to_vec())
    }

    /// Get the source configuration
    pub fn source(&self) -> &S3Source {
        &self.source
    }
}

/// Secure credential provider that never logs secrets
#[derive(Debug)]
pub struct SecureCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

impl SecureCredentials {
    /// Create new secure credentials
    pub fn new(access_key_id: impl Into<String>, secret_access_key: impl Into<String>) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            session_token: None,
        }
    }

    /// Set a session token for temporary credentials
    pub fn with_session_token(mut self, token: impl Into<String>) -> Self {
        self.session_token = Some(token.into());
        self
    }

    /// Get the access key ID (safe to log)
    pub fn access_key_id(&self) -> &str {
        &self.access_key_id
    }

    /// Get a redacted version of the access key for logging
    pub fn redacted_access_key(&self) -> String {
        redact_secret(&self.access_key_id, 4)
    }

    /// Check if session token is present
    pub fn has_session_token(&self) -> bool {
        self.session_token.is_some()
    }
}

// Implement Display to prevent accidental secret logging
impl std::fmt::Display for SecureCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SecureCredentials {{ access_key_id: {}, secret: [REDACTED] }}",
            self.redacted_access_key()
        )
    }
}

/// Redact a secret string, showing only the first N characters
pub fn redact_secret(secret: &str, visible_chars: usize) -> String {
    if secret.len() <= visible_chars {
        "[REDACTED]".to_string()
    } else {
        format!("{}...[REDACTED]", &secret[..visible_chars])
    }
}

/// Redact potential secrets from a string (URLs, keys, tokens)
pub fn redact_secrets_in_string(s: &str) -> String {
    let mut result = s.to_string();

    // Redact AWS access keys (AKIA...)
    let aws_key_pattern = regex::Regex::new(r"AKIA[0-9A-Z]{16}").unwrap();
    result = aws_key_pattern
        .replace_all(&result, "AKIA...[REDACTED]")
        .to_string();

    // Redact AWS secret keys (40 char base64-like strings after = or :)
    let secret_pattern = regex::Regex::new(
        r#"(?i)(secret[_-]?(?:access[_-]?)?key["']?\s*[:=]\s*["']?)([A-Za-z0-9+/]{40})"#,
    )
    .unwrap();
    result = secret_pattern
        .replace_all(&result, "${1}[REDACTED]")
        .to_string();

    // Redact bearer tokens
    let bearer_pattern = regex::Regex::new(r"(?i)(bearer\s+)([A-Za-z0-9._\-]+)").unwrap();
    result = bearer_pattern
        .replace_all(&result, "${1}[REDACTED]")
        .to_string();

    // Redact passwords in URLs
    let url_password_pattern = regex::Regex::new(r"(://[^:]+:)([^@]+)(@)").unwrap();
    result = url_password_pattern
        .replace_all(&result, "${1}[REDACTED]${3}")
        .to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_source_display() {
        let source = S3Source::new("my-bucket", "data/json");
        assert_eq!(source.display(), "s3://my-bucket/data/json");
    }

    #[test]
    fn test_s3_source_builder() {
        let source = S3Source::new("bucket", "prefix")
            .with_region("us-west-2")
            .with_profile("prod")
            .with_endpoint("http://localhost:9000");

        assert_eq!(source.bucket, "bucket");
        assert_eq!(source.prefix, "prefix");
        assert_eq!(source.region, Some("us-west-2".to_string()));
        assert_eq!(source.profile, Some("prod".to_string()));
        assert_eq!(
            source.endpoint_url,
            Some("http://localhost:9000".to_string())
        );
    }

    #[test]
    fn test_redact_secret() {
        assert_eq!(redact_secret("short", 10), "[REDACTED]");
        assert_eq!(redact_secret("longsecretkey123", 4), "long...[REDACTED]");
    }

    #[test]
    fn test_secure_credentials_display() {
        let creds = SecureCredentials::new("AKIAIOSFODNN7EXAMPLE", "secret123");
        let display = format!("{}", creds);
        assert!(display.contains("AKIA"));
        assert!(display.contains("[REDACTED]"));
        assert!(!display.contains("secret123"));
    }

    #[test]
    fn test_redact_secrets_in_string() {
        // AWS access key
        let s = "Using key AKIAIOSFODNN7EXAMPLE for access";
        assert_eq!(
            redact_secrets_in_string(s),
            "Using key AKIA...[REDACTED] for access"
        );

        // Password in URL
        let s = "postgres://user:password123@host:5432/db";
        assert!(redact_secrets_in_string(s).contains("[REDACTED]"));
        assert!(!redact_secrets_in_string(s).contains("password123"));

        // Bearer token
        let s = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        assert!(redact_secrets_in_string(s).contains("[REDACTED]"));
    }
}
