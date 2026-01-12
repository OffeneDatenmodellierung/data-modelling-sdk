//! Batch tracking for ingestion resume support

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a processing batch
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchStatus {
    /// Batch is currently running
    Running,
    /// Batch completed successfully
    Completed,
    /// Batch failed with an error
    Failed,
    /// Batch was cancelled
    Cancelled,
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchStatus::Running => write!(f, "running"),
            BatchStatus::Completed => write!(f, "completed"),
            BatchStatus::Failed => write!(f, "failed"),
            BatchStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for BatchStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "running" => Ok(BatchStatus::Running),
            "completed" => Ok(BatchStatus::Completed),
            "failed" => Ok(BatchStatus::Failed),
            "cancelled" => Ok(BatchStatus::Cancelled),
            _ => Err(format!("Invalid batch status: {}", s)),
        }
    }
}

/// A processing batch record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingBatch {
    /// Unique batch identifier
    pub id: String,
    /// Source path being processed
    pub source_path: String,
    /// Source type (local, s3, unity_volume)
    pub source_type: String,
    /// Partition key for this batch
    pub partition_key: Option<String>,
    /// File pattern used
    pub pattern: String,
    /// Current status
    pub status: BatchStatus,
    /// Total number of files to process
    pub files_total: i32,
    /// Number of files processed
    pub files_processed: i32,
    /// Number of files skipped (duplicates)
    pub files_skipped: i32,
    /// Total records ingested
    pub records_ingested: i64,
    /// Total bytes processed
    pub bytes_processed: i64,
    /// Number of errors encountered
    pub errors_count: i32,
    /// Last file path processed (for resume)
    pub last_file_path: Option<String>,
    /// Last record index in the last file (for resume)
    pub last_record_index: Option<i32>,
    /// When the batch started
    pub started_at: Option<DateTime<Utc>>,
    /// When the batch was last updated
    pub updated_at: Option<DateTime<Utc>>,
    /// When the batch completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl ProcessingBatch {
    /// Create a new batch
    pub fn new(
        id: String,
        source_path: String,
        source_type: String,
        partition_key: Option<String>,
        pattern: String,
    ) -> Self {
        Self {
            id,
            source_path,
            source_type,
            partition_key,
            pattern,
            status: BatchStatus::Running,
            files_total: 0,
            files_processed: 0,
            files_skipped: 0,
            records_ingested: 0,
            bytes_processed: 0,
            errors_count: 0,
            last_file_path: None,
            last_record_index: None,
            started_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            completed_at: None,
            error_message: None,
        }
    }

    /// Generate a new batch ID
    pub fn generate_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Mark the batch as completed
    pub fn complete(&mut self) {
        self.status = BatchStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Some(Utc::now());
    }

    /// Mark the batch as failed
    pub fn fail(&mut self, error: &str) {
        self.status = BatchStatus::Failed;
        self.error_message = Some(error.to_string());
        self.completed_at = Some(Utc::now());
        self.updated_at = Some(Utc::now());
    }

    /// Update progress
    pub fn update_progress(
        &mut self,
        files_processed: i32,
        files_skipped: i32,
        records_ingested: i64,
        bytes_processed: i64,
        last_file: Option<&str>,
        last_record: Option<i32>,
    ) {
        self.files_processed = files_processed;
        self.files_skipped = files_skipped;
        self.records_ingested = records_ingested;
        self.bytes_processed = bytes_processed;
        self.last_file_path = last_file.map(|s| s.to_string());
        self.last_record_index = last_record;
        self.updated_at = Some(Utc::now());
    }

    /// Increment error count
    pub fn increment_errors(&mut self) {
        self.errors_count += 1;
        self.updated_at = Some(Utc::now());
    }

    /// Check if the batch can be resumed
    pub fn can_resume(&self) -> bool {
        matches!(self.status, BatchStatus::Running | BatchStatus::Failed)
    }

    /// Get duration in seconds (if started)
    pub fn duration_seconds(&self) -> Option<i64> {
        let started = self.started_at?;
        let ended = self.completed_at.unwrap_or_else(Utc::now);
        Some((ended - started).num_seconds())
    }

    /// Get records per second throughput
    pub fn throughput(&self) -> Option<f64> {
        let duration = self.duration_seconds()?;
        if duration == 0 {
            return None;
        }
        Some(self.records_ingested as f64 / duration as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_status_display() {
        assert_eq!(BatchStatus::Running.to_string(), "running");
        assert_eq!(BatchStatus::Completed.to_string(), "completed");
        assert_eq!(BatchStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_batch_status_from_str() {
        assert_eq!(
            "running".parse::<BatchStatus>().unwrap(),
            BatchStatus::Running
        );
        assert_eq!(
            "completed".parse::<BatchStatus>().unwrap(),
            BatchStatus::Completed
        );
        assert_eq!(
            "FAILED".parse::<BatchStatus>().unwrap(),
            BatchStatus::Failed
        );
    }

    #[test]
    fn test_processing_batch_new() {
        let batch = ProcessingBatch::new(
            "test-id".to_string(),
            "./data".to_string(),
            "local".to_string(),
            Some("2024-01".to_string()),
            "*.json".to_string(),
        );

        assert_eq!(batch.id, "test-id");
        assert_eq!(batch.status, BatchStatus::Running);
        assert!(batch.started_at.is_some());
        assert!(batch.can_resume());
    }

    #[test]
    fn test_batch_complete() {
        let mut batch = ProcessingBatch::new(
            "test-id".to_string(),
            "./data".to_string(),
            "local".to_string(),
            None,
            "*.json".to_string(),
        );

        batch.complete();
        assert_eq!(batch.status, BatchStatus::Completed);
        assert!(batch.completed_at.is_some());
        assert!(!batch.can_resume());
    }

    #[test]
    fn test_batch_fail() {
        let mut batch = ProcessingBatch::new(
            "test-id".to_string(),
            "./data".to_string(),
            "local".to_string(),
            None,
            "*.json".to_string(),
        );

        batch.fail("Something went wrong");
        assert_eq!(batch.status, BatchStatus::Failed);
        assert_eq!(
            batch.error_message,
            Some("Something went wrong".to_string())
        );
        assert!(batch.can_resume());
    }
}
