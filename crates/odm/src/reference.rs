//! External reference resolution for schemas

use crate::error::CliError;
use std::path::Path;
use std::time::Duration;

/// Resolve a local file reference relative to the source file's directory
pub fn resolve_local_reference(reference: &str, source_file: &Path) -> Result<String, CliError> {
    // Remove leading './' if present
    let ref_path = reference.strip_prefix("./").unwrap_or(reference);

    // Get source file's directory
    let source_dir = source_file.parent().ok_or_else(|| {
        CliError::InvalidArgument(format!("Invalid source file path: {:?}", source_file))
    })?;

    // Resolve relative path
    let resolved_path = source_dir.join(ref_path);

    // Normalize path (resolve '..' components safely)
    let resolved_path = resolved_path.canonicalize().map_err(|e| {
        CliError::ReferenceResolutionError(format!(
            "Failed to resolve reference '{}' from {:?}: {}",
            reference, source_file, e
        ))
    })?;

    // Prevent directory traversal attacks
    let source_dir_canonical = source_dir.canonicalize().map_err(|e| {
        CliError::ReferenceResolutionError(format!(
            "Failed to canonicalize source directory {:?}: {}",
            source_dir, e
        ))
    })?;

    if !resolved_path.starts_with(&source_dir_canonical) {
        return Err(CliError::ReferenceResolutionError(format!(
            "Reference '{}' resolves outside source directory",
            reference
        )));
    }

    // Read file content
    std::fs::read_to_string(&resolved_path)
        .map_err(|e| CliError::FileReadError(resolved_path, e.to_string()))
}

/// Resolve an HTTP/HTTPS URL reference (blocking)
#[cfg(feature = "api-backend")]
pub fn resolve_http_reference(url: &str) -> Result<String, CliError> {
    // Check if URL requires authentication (basic heuristic)
    if url.contains("@") && !url.contains("@github.com") && !url.contains("@gitlab.com") {
        return Err(CliError::ReferenceResolutionError(format!(
            "Authenticated URLs are not supported. URL: {}",
            url
        )));
    }

    // Use blocking HTTP client
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| CliError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

    // Fetch URL
    let response = client
        .get(url)
        .send()
        .map_err(|e| CliError::NetworkError(format!("Failed to fetch URL {}: {}", url, e)))?;

    if !response.status().is_success() {
        return Err(CliError::NetworkError(format!(
            "HTTP error {} when fetching URL: {}",
            response.status(),
            url
        )));
    }

    response
        .text()
        .map_err(|e| CliError::NetworkError(format!("Failed to read response from {}: {}", url, e)))
}

#[cfg(not(feature = "api-backend"))]
pub fn resolve_http_reference(_url: &str) -> Result<String, CliError> {
    Err(CliError::ReferenceResolutionError(
        "HTTP reference resolution requires 'api-backend' feature".to_string(),
    ))
}

/// Resolve an external reference (local file or HTTP/HTTPS URL)
pub fn resolve_reference(reference: &str, source_file: Option<&Path>) -> Result<String, CliError> {
    if reference.starts_with("http://") || reference.starts_with("https://") {
        resolve_http_reference(reference)
    } else if let Some(source) = source_file {
        resolve_local_reference(reference, source)
    } else {
        Err(CliError::ReferenceResolutionError(format!(
            "Cannot resolve local reference '{}' without source file path",
            reference
        )))
    }
}
