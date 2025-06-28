//! Metrics and statistics for sync operations

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::progress::FileOperation;
use tracing::{info, warn, error, debug, span, Span, Level};

/// Comprehensive metrics for sync operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetrics {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Start time of the sync operation
    pub start_time: SystemTime,
    /// End time of the sync operation
    pub end_time: Option<SystemTime>,
    /// Total duration of the sync operation
    pub duration: Duration,
    /// File statistics
    pub files: FileStats,
    /// Data transfer statistics
    pub transfer: TransferStats,
    /// Performance statistics
    pub performance: PerformanceStats,
    /// Error statistics
    pub errors: ErrorStats,
    /// Operation breakdown
    pub operations: OperationStats,
    /// Conflict statistics
    pub conflicts: ConflictStats,
}

/// File-related statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStats {
    /// Total files scanned in source
    pub scanned: usize,
    /// Total files processed
    pub processed: usize,
    /// Files copied
    pub copied: usize,
    /// Files updated
    pub updated: usize,
    /// Files deleted
    pub deleted: usize,
    /// Files skipped
    pub skipped: usize,
    /// Directories created
    pub directories_created: usize,
    /// Files with conflicts
    pub conflicts: usize,
    /// Files that failed processing
    pub failed: usize,
}

/// Data transfer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStats {
    /// Total bytes scanned
    pub bytes_scanned: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Bytes copied
    pub bytes_copied: u64,
    /// Bytes updated
    pub bytes_updated: u64,
    /// Largest file transferred
    pub largest_file_size: u64,
    /// Smallest file transferred
    pub smallest_file_size: u64,
    /// Average file size
    pub average_file_size: u64,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Transfer rate in bytes per second
    pub transfer_rate: f64,
    /// Files processed per second
    pub files_per_second: f64,
    /// Time spent scanning directories
    pub scan_time: Duration,
    /// Time spent comparing files
    pub comparison_time: Duration,
    /// Time spent transferring files
    pub transfer_time: Duration,
    /// Peak memory usage (if available)
    pub peak_memory_usage: Option<u64>,
}

/// Error and warning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    /// Total number of errors
    pub total_errors: usize,
    /// Total number of warnings
    pub total_warnings: usize,
    /// Errors by type
    pub errors_by_type: HashMap<String, usize>,
    /// Critical errors that stopped the sync
    pub critical_errors: Vec<String>,
    /// Non-critical errors that were skipped
    pub recoverable_errors: Vec<String>,
}

/// Operation statistics breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    /// Time spent on each operation type
    pub operation_times: HashMap<String, Duration>,
    /// Count of each operation type
    pub operation_counts: HashMap<String, usize>,
    /// Bytes transferred per operation type
    pub operation_bytes: HashMap<String, u64>,
}

/// Conflict resolution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictStats {
    /// Total conflicts encountered
    pub total_conflicts: usize,
    /// Conflicts resolved automatically
    pub auto_resolved: usize,
    /// Conflicts requiring manual intervention
    pub manual_intervention: usize,
    /// Conflicts by resolution strategy
    pub resolution_strategies: HashMap<String, usize>,
}

impl Default for SyncMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncMetrics {
    /// Create new sync metrics
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            start_time: SystemTime::now(),
            end_time: None,
            duration: Duration::default(),
            files: FileStats::default(),
            transfer: TransferStats::default(),
            performance: PerformanceStats::default(),
            errors: ErrorStats::default(),
            operations: OperationStats::default(),
            conflicts: ConflictStats::default(),
        }
    }

    /// Mark the sync operation as started
    pub fn start(&mut self) {
        self.start_time = SystemTime::now();
    }

    /// Mark the sync operation as completed
    pub fn complete(&mut self) {
        self.end_time = Some(SystemTime::now());
        self.duration = self.end_time.unwrap()
            .duration_since(self.start_time)
            .unwrap_or_default();
        
        self.calculate_performance_stats();
        
        // Log completion with comprehensive metrics
        info!(
            session_id = %self.session_id,
            duration_secs = self.duration.as_secs_f64(),
            files_processed = self.files.processed,
            files_copied = self.files.copied,
            files_updated = self.files.updated,
            files_deleted = self.files.deleted,
            files_failed = self.files.failed,
            bytes_transferred = self.transfer.bytes_transferred,
            transfer_rate_mbps = self.performance.transfer_rate / (1024.0 * 1024.0),
            success_rate = self.success_rate(),
            total_errors = self.errors.total_errors,
            total_conflicts = self.conflicts.total_conflicts,
            "Sync operation completed"
        );
    }

    /// Record a file operation
    pub fn record_file_operation(&mut self, operation: FileOperation, file_size: u64, duration: Duration) {
        self.files.processed += 1;
        
        // Log the operation with structured data
        let span = span!(Level::DEBUG, "file_operation",
            operation = ?operation,
            file_size = file_size,
            duration_ms = duration.as_millis()
        );
        let _enter = span.enter();
        
        match operation {
            FileOperation::Copy => {
                self.files.copied += 1;
                self.transfer.bytes_copied += file_size;
            }
            FileOperation::Update => {
                self.files.updated += 1;
                self.transfer.bytes_updated += file_size;
            }
            FileOperation::Delete => {
                self.files.deleted += 1;
            }
            FileOperation::CreateDirectory => {
                self.files.directories_created += 1;
            }
            FileOperation::Skip => {
                self.files.skipped += 1;
            }
            FileOperation::Conflict => {
                self.files.conflicts += 1;
                self.conflicts.total_conflicts += 1;
            }
        }

        if file_size > 0 {
            self.transfer.bytes_transferred += file_size;
            
            // Update file size statistics
            if self.transfer.largest_file_size == 0 || file_size > self.transfer.largest_file_size {
                self.transfer.largest_file_size = file_size;
            }
            
            if self.transfer.smallest_file_size == 0 || file_size < self.transfer.smallest_file_size {
                self.transfer.smallest_file_size = file_size;
            }
        }

        // Record operation timing
        let operation_name = format!("{:?}", operation);
        *self.operations.operation_counts.entry(operation_name.clone()).or_insert(0) += 1;
        *self.operations.operation_times.entry(operation_name.clone()).or_insert(Duration::default()) += duration;
        *self.operations.operation_bytes.entry(operation_name).or_insert(0) += file_size;
    }

    /// Record an error
    pub fn record_error(&mut self, error_type: impl Into<String>, message: impl Into<String>, is_critical: bool) {
        let error_type = error_type.into();
        let message = message.into();
        
        // Log error with structured data
        if is_critical {
            error!(
                error_type = %error_type,
                message = %message,
                "Critical sync error occurred"
            );
        } else {
            warn!(
                error_type = %error_type,
                message = %message,
                "Recoverable sync error occurred"
            );
        }
        
        self.errors.total_errors += 1;
        *self.errors.errors_by_type.entry(error_type).or_insert(0) += 1;

        if is_critical {
            self.errors.critical_errors.push(message);
        } else {
            self.errors.recoverable_errors.push(message);
        }
    }

    /// Record a warning
    pub fn record_warning(&mut self) {
        self.errors.total_warnings += 1;
    }

    /// Record conflict resolution
    pub fn record_conflict_resolution(&mut self, strategy: impl Into<String>, auto_resolved: bool) {
        let strategy = strategy.into();
        
        if auto_resolved {
            self.conflicts.auto_resolved += 1;
        } else {
            self.conflicts.manual_intervention += 1;
        }
        
        *self.conflicts.resolution_strategies.entry(strategy).or_insert(0) += 1;
    }

    /// Record scan metrics
    pub fn record_scan(&mut self, files_found: usize, bytes_scanned: u64, duration: Duration) {
        self.files.scanned += files_found;
        self.transfer.bytes_scanned += bytes_scanned;
        self.performance.scan_time += duration;
    }

    /// Record comparison time
    pub fn record_comparison_time(&mut self, duration: Duration) {
        self.performance.comparison_time += duration;
    }

    /// Record transfer time
    pub fn record_transfer_time(&mut self, duration: Duration) {
        self.performance.transfer_time += duration;
    }

    /// Calculate performance statistics
    fn calculate_performance_stats(&mut self) {
        if self.duration.as_secs_f64() > 0.0 {
            self.performance.transfer_rate = self.transfer.bytes_transferred as f64 / self.duration.as_secs_f64();
            self.performance.files_per_second = self.files.processed as f64 / self.duration.as_secs_f64();
        }

        if self.files.processed > 0 {
            self.transfer.average_file_size = self.transfer.bytes_transferred / self.files.processed as u64;
        }
    }

    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.files.processed == 0 {
            100.0
        } else {
            let successful = self.files.processed - self.files.failed;
            (successful as f64 / self.files.processed as f64) * 100.0
        }
    }

    /// Check if the sync operation was successful
    pub fn is_successful(&self) -> bool {
        self.errors.critical_errors.is_empty() && self.files.failed == 0
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "Sync completed in {:.2}s: {} files processed ({} copied, {} updated, {} deleted), {} bytes transferred at {:.2} MB/s",
            self.duration.as_secs_f64(),
            self.files.processed,
            self.files.copied,
            self.files.updated,
            self.files.deleted,
            self.transfer.bytes_transferred,
            self.performance.transfer_rate / (1024.0 * 1024.0)
        )
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Merge metrics from another sync operation
    pub fn merge(&mut self, other: &SyncMetrics) {
        self.files.scanned += other.files.scanned;
        self.files.processed += other.files.processed;
        self.files.copied += other.files.copied;
        self.files.updated += other.files.updated;
        self.files.deleted += other.files.deleted;
        self.files.skipped += other.files.skipped;
        self.files.directories_created += other.files.directories_created;
        self.files.conflicts += other.files.conflicts;
        self.files.failed += other.files.failed;

        self.transfer.bytes_scanned += other.transfer.bytes_scanned;
        self.transfer.bytes_transferred += other.transfer.bytes_transferred;
        self.transfer.bytes_copied += other.transfer.bytes_copied;
        self.transfer.bytes_updated += other.transfer.bytes_updated;
        
        self.transfer.largest_file_size = self.transfer.largest_file_size.max(other.transfer.largest_file_size);
        if self.transfer.smallest_file_size == 0 {
            self.transfer.smallest_file_size = other.transfer.smallest_file_size;
        } else if other.transfer.smallest_file_size > 0 {
            self.transfer.smallest_file_size = self.transfer.smallest_file_size.min(other.transfer.smallest_file_size);
        }

        self.performance.scan_time += other.performance.scan_time;
        self.performance.comparison_time += other.performance.comparison_time;
        self.performance.transfer_time += other.performance.transfer_time;

        self.errors.total_errors += other.errors.total_errors;
        self.errors.total_warnings += other.errors.total_warnings;
        
        for (error_type, count) in &other.errors.errors_by_type {
            *self.errors.errors_by_type.entry(error_type.clone()).or_insert(0) += count;
        }

        self.errors.critical_errors.extend(other.errors.critical_errors.clone());
        self.errors.recoverable_errors.extend(other.errors.recoverable_errors.clone());

        self.conflicts.total_conflicts += other.conflicts.total_conflicts;
        self.conflicts.auto_resolved += other.conflicts.auto_resolved;
        self.conflicts.manual_intervention += other.conflicts.manual_intervention;

        for (strategy, count) in &other.conflicts.resolution_strategies {
            *self.conflicts.resolution_strategies.entry(strategy.clone()).or_insert(0) += count;
        }

        // Recalculate performance stats
        self.calculate_performance_stats();
    }
}

impl Default for FileStats {
    fn default() -> Self {
        Self {
            scanned: 0,
            processed: 0,
            copied: 0,
            updated: 0,
            deleted: 0,
            skipped: 0,
            directories_created: 0,
            conflicts: 0,
            failed: 0,
        }
    }
}

impl Default for TransferStats {
    fn default() -> Self {
        Self {
            bytes_scanned: 0,
            bytes_transferred: 0,
            bytes_copied: 0,
            bytes_updated: 0,
            largest_file_size: 0,
            smallest_file_size: 0,
            average_file_size: 0,
        }
    }
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            transfer_rate: 0.0,
            files_per_second: 0.0,
            scan_time: Duration::default(),
            comparison_time: Duration::default(),
            transfer_time: Duration::default(),
            peak_memory_usage: None,
        }
    }
}

impl Default for ErrorStats {
    fn default() -> Self {
        Self {
            total_errors: 0,
            total_warnings: 0,
            errors_by_type: HashMap::new(),
            critical_errors: Vec::new(),
            recoverable_errors: Vec::new(),
        }
    }
}

impl Default for OperationStats {
    fn default() -> Self {
        Self {
            operation_times: HashMap::new(),
            operation_counts: HashMap::new(),
            operation_bytes: HashMap::new(),
        }
    }
}

impl Default for ConflictStats {
    fn default() -> Self {
        Self {
            total_conflicts: 0,
            auto_resolved: 0,
            manual_intervention: 0,
            resolution_strategies: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_new_metrics() {
        let metrics = SyncMetrics::new();
        assert_eq!(metrics.files.processed, 0);
        assert_eq!(metrics.transfer.bytes_transferred, 0);
        assert_eq!(metrics.errors.total_errors, 0);
    }

    #[test]
    fn test_record_file_operation() {
        let mut metrics = SyncMetrics::new();
        
        metrics.record_file_operation(FileOperation::Copy, 1024, Duration::from_millis(100));
        
        assert_eq!(metrics.files.processed, 1);
        assert_eq!(metrics.files.copied, 1);
        assert_eq!(metrics.transfer.bytes_transferred, 1024);
        assert_eq!(metrics.transfer.bytes_copied, 1024);
        assert_eq!(metrics.transfer.largest_file_size, 1024);
        assert_eq!(metrics.transfer.smallest_file_size, 1024);
    }

    #[test]
    fn test_record_error() {
        let mut metrics = SyncMetrics::new();
        
        metrics.record_error("IO", "File not found", false);
        metrics.record_error("Permission", "Access denied", true);
        
        assert_eq!(metrics.errors.total_errors, 2);
        assert_eq!(metrics.errors.errors_by_type.get("IO"), Some(&1));
        assert_eq!(metrics.errors.errors_by_type.get("Permission"), Some(&1));
        assert_eq!(metrics.errors.critical_errors.len(), 1);
        assert_eq!(metrics.errors.recoverable_errors.len(), 1);
    }

    #[test]
    fn test_success_rate() {
        let mut metrics = SyncMetrics::new();
        
        // Record successful operations
        for _ in 0..8 {
            metrics.record_file_operation(FileOperation::Copy, 100, Duration::from_millis(10));
        }
        
        // Record failed operations
        metrics.files.failed = 2;
        
        assert_eq!(metrics.success_rate(), 80.0); // 8/10 = 80%
    }

    #[test]
    fn test_merge_metrics() {
        let mut metrics1 = SyncMetrics::new();
        let mut metrics2 = SyncMetrics::new();
        
        metrics1.record_file_operation(FileOperation::Copy, 1000, Duration::from_millis(100));
        metrics2.record_file_operation(FileOperation::Update, 2000, Duration::from_millis(200));
        
        metrics1.merge(&metrics2);
        
        assert_eq!(metrics1.files.processed, 2);
        assert_eq!(metrics1.files.copied, 1);
        assert_eq!(metrics1.files.updated, 1);
        assert_eq!(metrics1.transfer.bytes_transferred, 3000);
    }

    #[test]
    fn test_json_serialization() {
        let metrics = SyncMetrics::new();
        let json = metrics.to_json().unwrap();
        let deserialized = SyncMetrics::from_json(&json).unwrap();
        
        assert_eq!(metrics.session_id, deserialized.session_id);
        assert_eq!(metrics.files.processed, deserialized.files.processed);
    }
}
