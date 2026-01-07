//! CLI-specific error types

use crate::export::ExportError;
use crate::import::ImportError;
use std::path::PathBuf;
use thiserror::Error;

/// CLI-specific error type
#[derive(Error, Debug)]
pub enum CliError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to read file {0}: {1}")]
    FileReadError(PathBuf, String),

    #[error("Failed to write file {0}: {1}")]
    FileWriteError(PathBuf, String),

    #[error("Invalid UUID format: {0}")]
    InvalidUuid(String),

    #[error("UUID override is only supported when importing a single table. Found {0} tables.")]
    MultipleTablesWithUuid(usize),

    #[error(
        "protoc not found. The Protocol Buffers compiler is required for protobuf-descriptor export.\n\nInstallation:\n  macOS:    brew install protobuf\n  Linux:    sudo apt-get install protobuf-compiler (Debian/Ubuntu) or sudo yum install protobuf-compiler (RHEL/CentOS)\n  Windows:  Download from https://protobuf.dev/downloads/ or choco install protoc\n  Other:    https://protobuf.dev/downloads/\n\nAlternatively, use --protoc-path to specify a custom protoc location."
    )]
    ProtocNotFound,

    #[error("protoc execution error: {0}")]
    ProtocError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Failed to resolve external reference: {0}")]
    ReferenceResolutionError(String),

    #[error("Schema validation error: {0}")]
    ValidationError(String),

    #[error("Import error: {0}")]
    ImportError(#[from] ImportError),

    #[error("Export error: {0}")]
    ExportError(#[from] ExportError),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
