//! Progress reporting for ingestion operations
//!
//! This module provides progress bars and spinners for long-running operations
//! using the `indicatif` crate.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

/// Progress reporter for file ingestion
pub struct IngestProgress {
    multi: MultiProgress,
    files_bar: ProgressBar,
    records_bar: ProgressBar,
    bytes_bar: ProgressBar,
}

impl IngestProgress {
    /// Create a new progress reporter for ingestion
    ///
    /// # Arguments
    /// * `total_files` - Total number of files to process
    /// * `show_bytes` - Whether to show a bytes progress bar
    pub fn new(total_files: u64, show_bytes: bool) -> Self {
        let multi = MultiProgress::new();

        // File progress bar
        let files_bar = multi.add(ProgressBar::new(total_files));
        files_bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} files ({eta})"
            )
            .unwrap()
            .progress_chars("█▓▒░  ")
        );
        files_bar.enable_steady_tick(Duration::from_millis(100));

        // Records progress bar (indeterminate until we start processing)
        let records_bar = multi.add(ProgressBar::new_spinner());
        records_bar.set_style(ProgressStyle::with_template("{spinner:.yellow} {msg}").unwrap());
        records_bar.set_message("Records: 0");
        records_bar.enable_steady_tick(Duration::from_millis(100));

        // Bytes progress bar (optional)
        let bytes_bar = if show_bytes {
            let bar = multi.add(ProgressBar::new_spinner());
            bar.set_style(ProgressStyle::with_template("{spinner:.magenta} {msg}").unwrap());
            bar.set_message("Bytes: 0 B");
            bar.enable_steady_tick(Duration::from_millis(100));
            bar
        } else {
            ProgressBar::hidden()
        };

        Self {
            multi,
            files_bar,
            records_bar,
            bytes_bar,
        }
    }

    /// Update file progress
    pub fn update_files(&self, processed: u64) {
        self.files_bar.set_position(processed);
    }

    /// Increment file progress by one
    pub fn inc_files(&self) {
        self.files_bar.inc(1);
    }

    /// Update records count
    pub fn update_records(&self, count: u64) {
        self.records_bar
            .set_message(format!("Records: {}", format_number(count)));
    }

    /// Update bytes processed
    pub fn update_bytes(&self, bytes: u64) {
        self.bytes_bar
            .set_message(format!("Bytes: {}", format_bytes(bytes)));
    }

    /// Set a status message
    pub fn set_message(&self, msg: &str) {
        self.files_bar.set_message(msg.to_string());
    }

    /// Mark a file as skipped
    pub fn skip_file(&self, reason: &str) {
        self.files_bar.println(format!("  ⊘ Skipped: {}", reason));
        self.files_bar.inc(1);
    }

    /// Report an error
    pub fn error(&self, msg: &str) {
        self.files_bar.println(format!("  ✗ Error: {}", msg));
    }

    /// Report a warning
    pub fn warn(&self, msg: &str) {
        self.files_bar.println(format!("  ⚠ Warning: {}", msg));
    }

    /// Finish with success message
    pub fn finish_success(&self, msg: &str) {
        self.files_bar.finish_with_message(format!("✓ {}", msg));
        self.records_bar.finish_and_clear();
        self.bytes_bar.finish_and_clear();
    }

    /// Finish with error message
    pub fn finish_error(&self, msg: &str) {
        self.files_bar.abandon_with_message(format!("✗ {}", msg));
        self.records_bar.finish_and_clear();
        self.bytes_bar.finish_and_clear();
    }

    /// Get the multi-progress handle for spawning additional bars
    pub fn multi(&self) -> &MultiProgress {
        &self.multi
    }
}

/// Progress reporter for schema inference
pub struct InferenceProgress {
    bar: ProgressBar,
}

impl InferenceProgress {
    /// Create a new progress reporter for inference
    ///
    /// # Arguments
    /// * `total_records` - Total number of records to analyze (0 for unknown)
    pub fn new(total_records: u64) -> Self {
        let bar = if total_records > 0 {
            let bar = ProgressBar::new(total_records);
            bar.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} records ({eta})"
                )
                .unwrap()
                .progress_chars("█▓▒░  ")
            );
            bar
        } else {
            let bar = ProgressBar::new_spinner();
            bar.set_style(
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}").unwrap(),
            );
            bar
        };
        bar.enable_steady_tick(Duration::from_millis(100));

        Self { bar }
    }

    /// Update progress
    pub fn update(&self, processed: u64) {
        if self.bar.length().unwrap_or(0) > 0 {
            self.bar.set_position(processed);
        } else {
            self.bar
                .set_message(format!("Processed {} records", format_number(processed)));
        }
    }

    /// Increment progress by one
    pub fn inc(&self) {
        self.bar.inc(1);
    }

    /// Set a status message
    pub fn set_message(&self, msg: &str) {
        self.bar.set_message(msg.to_string());
    }

    /// Finish with success message
    pub fn finish_success(&self, msg: &str) {
        self.bar.finish_with_message(format!("✓ {}", msg));
    }

    /// Finish with error message
    pub fn finish_error(&self, msg: &str) {
        self.bar.abandon_with_message(format!("✗ {}", msg));
    }
}

/// Simple spinner for indeterminate operations
pub struct Spinner {
    bar: ProgressBar,
}

impl Spinner {
    /// Create a new spinner with a message
    pub fn new(msg: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
        bar.set_message(msg.to_string());
        bar.enable_steady_tick(Duration::from_millis(100));

        Self { bar }
    }

    /// Update the spinner message
    pub fn set_message(&self, msg: &str) {
        self.bar.set_message(msg.to_string());
    }

    /// Finish with success
    pub fn finish_success(&self, msg: &str) {
        self.bar.finish_with_message(format!("✓ {}", msg));
    }

    /// Finish with error
    pub fn finish_error(&self, msg: &str) {
        self.bar.abandon_with_message(format!("✗ {}", msg));
    }

    /// Finish and clear
    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }
}

/// Format a number with thousand separators
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }
}
