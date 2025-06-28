//! Progress reporting functionality for sync operations

use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::error::{Result, SyncError};

/// Progress event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    /// Sync operation started
    SyncStarted {
        session_id: Uuid,
        total_files: usize,
        total_bytes: u64,
    },
    /// Directory scan started
    ScanStarted {
        path: String,
    },
    /// Directory scan completed
    ScanCompleted {
        path: String,
        files_found: usize,
        duration: Duration,
    },
    /// File operation started
    FileOperationStarted {
        operation: FileOperation,
        source_path: String,
        destination_path: Option<String>,
        file_size: u64,
    },
    /// File operation completed
    FileOperationCompleted {
        operation: FileOperation,
        source_path: String,
        destination_path: Option<String>,
        file_size: u64,
        duration: Duration,
    },
    /// File operation failed
    FileOperationFailed {
        operation: FileOperation,
        source_path: String,
        destination_path: Option<String>,
        error: String,
    },
    /// Progress update
    ProgressUpdate {
        files_processed: usize,
        bytes_processed: u64,
        files_total: usize,
        bytes_total: u64,
        current_file: Option<String>,
        elapsed_time: Duration,
        estimated_remaining: Option<Duration>,
        transfer_rate: f64, // bytes per second
    },
    /// Sync operation completed
    SyncCompleted {
        session_id: Uuid,
        files_processed: usize,
        bytes_processed: u64,
        duration: Duration,
        errors: Vec<String>,
    },
    /// Sync operation failed
    SyncFailed {
        session_id: Uuid,
        error: String,
        files_processed: usize,
        bytes_processed: u64,
        duration: Duration,
    },
    /// Warning message
    Warning {
        message: String,
        file_path: Option<String>,
    },
    /// Info message
    Info {
        message: String,
    },
}

/// File operation types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FileOperation {
    Copy,
    Update,
    Delete,
    CreateDirectory,
    Skip,
    Conflict,
}

impl std::fmt::Display for FileOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOperation::Copy => write!(f, "Copy"),
            FileOperation::Update => write!(f, "Update"),
            FileOperation::Delete => write!(f, "Delete"),
            FileOperation::CreateDirectory => write!(f, "Create Directory"),
            FileOperation::Skip => write!(f, "Skip"),
            FileOperation::Conflict => write!(f, "Conflict"),
        }
    }
}

/// Progress channel for receiving progress updates
pub struct ProgressChannel {
    receiver: mpsc::UnboundedReceiver<ProgressEvent>,
}

impl ProgressChannel {
    /// Create a new progress channel
    pub fn new() -> (ProgressReporter, Self) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let reporter = ProgressReporter::new(sender);
        let channel = Self { receiver };
        (reporter, channel)
    }

    /// Receive the next progress event
    pub async fn recv(&mut self) -> Option<ProgressEvent> {
        self.receiver.recv().await
    }

    /// Try to receive a progress event without blocking
    pub fn try_recv(&mut self) -> Result<ProgressEvent> {
        self.receiver.try_recv().map_err(|e| match e {
            mpsc::error::TryRecvError::Empty => SyncError::Progress("No progress events available".to_string()),
            mpsc::error::TryRecvError::Disconnected => SyncError::Progress("Progress channel disconnected".to_string()),
        })
    }

    /// Close the channel
    pub fn close(&mut self) {
        self.receiver.close();
    }
}

/// Progress reporter for sending progress updates
#[derive(Clone)]
pub struct ProgressReporter {
    sender: mpsc::UnboundedSender<ProgressEvent>,
    session_id: Uuid,
    start_time: Instant,
    state: Arc<RwLock<ProgressState>>,
}

#[derive(Debug)]
struct ProgressState {
    files_processed: usize,
    bytes_processed: u64,
    files_total: usize,
    bytes_total: u64,
    current_file: Option<String>,
    errors: Vec<String>,
}

impl ProgressReporter {
    /// Create a new progress reporter
    fn new(sender: mpsc::UnboundedSender<ProgressEvent>) -> Self {
        Self {
            sender,
            session_id: Uuid::new_v4(),
            start_time: Instant::now(),
            state: Arc::new(RwLock::new(ProgressState {
                files_processed: 0,
                bytes_processed: 0,
                files_total: 0,
                bytes_total: 0,
                current_file: None,
                errors: Vec::new(),
            })),
        }
    }

    /// Report sync started
    pub async fn sync_started(&self, total_files: usize, total_bytes: u64) -> Result<()> {
        {
            let mut state = self.state.write().await;
            state.files_total = total_files;
            state.bytes_total = total_bytes;
        }

        self.send(ProgressEvent::SyncStarted {
            session_id: self.session_id,
            total_files,
            total_bytes,
        })
    }

    /// Report scan started
    pub fn scan_started(&self, path: impl Into<String>) -> Result<()> {
        self.send(ProgressEvent::ScanStarted {
            path: path.into(),
        })
    }

    /// Report scan completed
    pub fn scan_completed(&self, path: impl Into<String>, files_found: usize, duration: Duration) -> Result<()> {
        self.send(ProgressEvent::ScanCompleted {
            path: path.into(),
            files_found,
            duration,
        })
    }

    /// Report file operation started
    pub fn file_operation_started(
        &self,
        operation: FileOperation,
        source_path: impl Into<String>,
        destination_path: Option<String>,
        file_size: u64,
    ) -> Result<()> {
        self.send(ProgressEvent::FileOperationStarted {
            operation,
            source_path: source_path.into(),
            destination_path,
            file_size,
        })
    }

    /// Report file operation completed
    pub async fn file_operation_completed(
        &self,
        operation: FileOperation,
        source_path: impl Into<String>,
        destination_path: Option<String>,
        file_size: u64,
        duration: Duration,
    ) -> Result<()> {
        let source_path = source_path.into();
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.files_processed += 1;
            state.bytes_processed += file_size;
            state.current_file = Some(source_path.clone());
        }

        // Send completion event
        self.send(ProgressEvent::FileOperationCompleted {
            operation,
            source_path,
            destination_path,
            file_size,
            duration,
        })?;

        // Send progress update
        self.send_progress_update().await
    }

    /// Report file operation failed
    pub async fn file_operation_failed(
        &self,
        operation: FileOperation,
        source_path: impl Into<String>,
        destination_path: Option<String>,
        error: impl Into<String>,
    ) -> Result<()> {
        let source_path = source_path.into();
        let error_msg = error.into();

        // Update state
        {
            let mut state = self.state.write().await;
            state.files_processed += 1;
            state.errors.push(error_msg.clone());
        }

        // Send failure event
        self.send(ProgressEvent::FileOperationFailed {
            operation,
            source_path,
            destination_path,
            error: error_msg,
        })?;

        // Send progress update
        self.send_progress_update().await
    }

    /// Report sync completed
    pub async fn sync_completed(&self) -> Result<()> {
        let state = self.state.read().await;
        let duration = self.start_time.elapsed();

        self.send(ProgressEvent::SyncCompleted {
            session_id: self.session_id,
            files_processed: state.files_processed,
            bytes_processed: state.bytes_processed,
            duration,
            errors: state.errors.clone(),
        })
    }

    /// Report sync failed
    pub async fn sync_failed(&self, error: impl Into<String>) -> Result<()> {
        let state = self.state.read().await;
        let duration = self.start_time.elapsed();

        self.send(ProgressEvent::SyncFailed {
            session_id: self.session_id,
            error: error.into(),
            files_processed: state.files_processed,
            bytes_processed: state.bytes_processed,
            duration,
        })
    }

    /// Report warning
    pub fn warning(&self, message: impl Into<String>, file_path: Option<String>) -> Result<()> {
        self.send(ProgressEvent::Warning {
            message: message.into(),
            file_path,
        })
    }

    /// Report info
    pub fn info(&self, message: impl Into<String>) -> Result<()> {
        self.send(ProgressEvent::Info {
            message: message.into(),
        })
    }

    /// Send progress update
    pub async fn send_progress_update(&self) -> Result<()> {
        let state = self.state.read().await;
        let elapsed_time = self.start_time.elapsed();

        // Calculate transfer rate
        let transfer_rate = if elapsed_time.as_secs_f64() > 0.0 {
            state.bytes_processed as f64 / elapsed_time.as_secs_f64()
        } else {
            0.0
        };

        // Estimate remaining time
        let estimated_remaining = if state.files_processed > 0 && state.files_total > state.files_processed {
            let rate = state.files_processed as f64 / elapsed_time.as_secs_f64();
            if rate > 0.0 {
                let remaining_files = (state.files_total - state.files_processed) as f64;
                Some(Duration::from_secs_f64(remaining_files / rate))
            } else {
                None
            }
        } else {
            None
        };

        self.send(ProgressEvent::ProgressUpdate {
            files_processed: state.files_processed,
            bytes_processed: state.bytes_processed,
            files_total: state.files_total,
            bytes_total: state.bytes_total,
            current_file: state.current_file.clone(),
            elapsed_time,
            estimated_remaining,
            transfer_rate,
        })
    }

    /// Send a progress event
    fn send(&self, event: ProgressEvent) -> Result<()> {
        self.sender.send(event).map_err(|_| {
            SyncError::Progress("Progress channel disconnected".to_string())
        })
    }

    /// Get current progress state
    pub async fn get_progress(&self) -> ProgressSnapshot {
        let state = self.state.read().await;
        let elapsed_time = self.start_time.elapsed();

        ProgressSnapshot {
            session_id: self.session_id,
            files_processed: state.files_processed,
            bytes_processed: state.bytes_processed,
            files_total: state.files_total,
            bytes_total: state.bytes_total,
            current_file: state.current_file.clone(),
            elapsed_time,
            error_count: state.errors.len(),
        }
    }
}

/// Snapshot of current progress state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressSnapshot {
    pub session_id: Uuid,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub files_total: usize,
    pub bytes_total: u64,
    pub current_file: Option<String>,
    pub elapsed_time: Duration,
    pub error_count: usize,
}

impl ProgressSnapshot {
    /// Calculate completion percentage (0.0 to 1.0)
    pub fn completion_percentage(&self) -> f64 {
        if self.files_total == 0 {
            1.0
        } else {
            self.files_processed as f64 / self.files_total as f64
        }
    }

    /// Calculate transfer rate in bytes per second
    pub fn transfer_rate(&self) -> f64 {
        if self.elapsed_time.as_secs_f64() > 0.0 {
            self.bytes_processed as f64 / self.elapsed_time.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Format transfer rate as human-readable string
    pub fn transfer_rate_human(&self) -> String {
        let rate = self.transfer_rate();
        format_bytes_per_second(rate)
    }

    /// Format bytes processed as human-readable string
    pub fn bytes_processed_human(&self) -> String {
        format_bytes(self.bytes_processed)
    }

    /// Format total bytes as human-readable string
    pub fn bytes_total_human(&self) -> String {
        format_bytes(self.bytes_total)
    }
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Format bytes per second as human-readable string
pub fn format_bytes_per_second(bytes_per_second: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_second as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_progress_channel() {
        let (reporter, mut channel) = ProgressChannel::new();

        // Send some events
        reporter.sync_started(10, 1000).await.unwrap();
        reporter.info("Starting sync").unwrap();

        // Receive events
        let event1 = channel.recv().await.unwrap();
        let event2 = channel.recv().await.unwrap();

        match event1 {
            ProgressEvent::SyncStarted { total_files, total_bytes, .. } => {
                assert_eq!(total_files, 10);
                assert_eq!(total_bytes, 1000);
            }
            _ => panic!("Expected SyncStarted event"),
        }

        match event2 {
            ProgressEvent::Info { message } => {
                assert_eq!(message, "Starting sync");
            }
            _ => panic!("Expected Info event"),
        }
    }

    #[tokio::test]
    async fn test_file_operation_progress() {
        let (reporter, mut channel) = ProgressChannel::new();

        reporter.sync_started(2, 200).await.unwrap();
        
        reporter.file_operation_completed(
            FileOperation::Copy,
            "test1.txt",
            Some("dest1.txt".to_string()),
            100,
            Duration::from_millis(50),
        ).await.unwrap();

        // Should receive file operation completed and progress update
        let _sync_started = channel.recv().await.unwrap();
        let _file_completed = channel.recv().await.unwrap();
        let progress_update = channel.recv().await.unwrap();

        match progress_update {
            ProgressEvent::ProgressUpdate { files_processed, bytes_processed, .. } => {
                assert_eq!(files_processed, 1);
                assert_eq!(bytes_processed, 100);
            }
            _ => panic!("Expected ProgressUpdate event"),
        }
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_progress_snapshot() {
        let snapshot = ProgressSnapshot {
            session_id: Uuid::new_v4(),
            files_processed: 5,
            bytes_processed: 500,
            files_total: 10,
            bytes_total: 1000,
            current_file: Some("test.txt".to_string()),
            elapsed_time: Duration::from_secs(10),
            error_count: 0,
        };

        assert_eq!(snapshot.completion_percentage(), 0.5);
        assert_eq!(snapshot.transfer_rate(), 50.0);
        assert_eq!(snapshot.bytes_processed_human(), "500 B");
        assert_eq!(snapshot.transfer_rate_human(), "50 B/s");
    }
}
