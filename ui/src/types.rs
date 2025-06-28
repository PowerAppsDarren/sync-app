use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Job status enumeration
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "Pending"),
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Paused => write!(f, "Paused"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
            JobStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Job priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl std::fmt::Display for JobPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobPriority::Low => write!(f, "Low"),
            JobPriority::Normal => write!(f, "Normal"),
            JobPriority::High => write!(f, "High"),
            JobPriority::Critical => write!(f, "Critical"),
        }
    }
}

/// Represents a synchronization job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub progress: f64, // 0.0 to 1.0
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub source_path: String,
    pub target_path: String,
    pub files_total: u64,
    pub files_processed: u64,
    pub bytes_total: u64,
    pub bytes_processed: u64,
    pub conflicts: Vec<Conflict>,
    pub errors: Vec<JobError>,
}

impl Job {
    pub fn new(name: String, source_path: String, target_path: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            status: JobStatus::Pending,
            priority: JobPriority::Normal,
            progress: 0.0,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            source_path,
            target_path,
            files_total: 0,
            files_processed: 0,
            bytes_total: 0,
            bytes_processed: 0,
            conflicts: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, JobStatus::Running | JobStatus::Paused)
    }

    pub fn can_start(&self) -> bool {
        matches!(self.status, JobStatus::Pending | JobStatus::Paused | JobStatus::Failed)
    }

    pub fn can_pause(&self) -> bool {
        matches!(self.status, JobStatus::Running)
    }

    pub fn can_stop(&self) -> bool {
        matches!(self.status, JobStatus::Running | JobStatus::Paused)
    }

    pub fn progress_percentage(&self) -> u16 {
        (self.progress * 100.0) as u16
    }

    pub fn files_progress_text(&self) -> String {
        format!("{}/{}", self.files_processed, self.files_total)
    }

    pub fn bytes_progress_text(&self) -> String {
        format!(
            "{}/{}",
            humanize_bytes(self.bytes_processed),
            humanize_bytes(self.bytes_total)
        )
    }
}

/// Represents a file conflict that needs resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: Uuid,
    pub job_id: Uuid,
    pub file_path: String,
    pub conflict_type: ConflictType,
    pub source_modified: DateTime<Utc>,
    pub target_modified: DateTime<Utc>,
    pub source_size: u64,
    pub target_size: u64,
    pub resolution: Option<ConflictResolution>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    ModificationTime,
    FileSize,
    ContentHash,
    FileType,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictType::ModificationTime => write!(f, "Modification Time"),
            ConflictType::FileSize => write!(f, "File Size"),
            ConflictType::ContentHash => write!(f, "Content Hash"),
            ConflictType::FileType => write!(f, "File Type"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    KeepSource,
    KeepTarget,
    Merge,
    Skip,
}

impl std::fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictResolution::KeepSource => write!(f, "Keep Source"),
            ConflictResolution::KeepTarget => write!(f, "Keep Target"),
            ConflictResolution::Merge => write!(f, "Merge"),
            ConflictResolution::Skip => write!(f, "Skip"),
        }
    }
}

/// Job error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobError {
    pub id: Uuid,
    pub job_id: Uuid,
    pub message: String,
    pub file_path: Option<String>,
    pub error_type: String,
    pub created_at: DateTime<Utc>,
}

/// Action log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    pub id: Uuid,
    pub job_id: Uuid,
    pub action: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Debug => write!(f, "DEBUG"),
        }
    }
}

/// Current view in the TUI application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    Home,
    JobDetail(Uuid),
    ConflictResolution(Uuid),
    Settings,
    Help,
}

/// Application state
#[derive(Debug)]
pub struct AppState {
    pub jobs: HashMap<Uuid, Job>,
    pub action_logs: HashMap<Uuid, Vec<ActionLogEntry>>,
    pub current_view: ViewMode,
    pub selected_job_index: usize,
    pub selected_conflict_index: usize,
    pub selected_log_index: usize,
    pub should_quit: bool,
    pub websocket_connected: bool,
    pub last_update: DateTime<Utc>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            action_logs: HashMap::new(),
            current_view: ViewMode::Home,
            selected_job_index: 0,
            selected_conflict_index: 0,
            selected_log_index: 0,
            should_quit: false,
            websocket_connected: false,
            last_update: Utc::now(),
        }
    }

    pub fn get_jobs_sorted(&self) -> Vec<&Job> {
        let mut jobs: Vec<&Job> = self.jobs.values().collect();
        jobs.sort_by(|a, b| {
            // Sort by priority first, then by status, then by created_at
            b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal)
                .then(a.status.partial_cmp(&b.status).unwrap_or(std::cmp::Ordering::Equal))
                .then(b.created_at.cmp(&a.created_at))
        });
        jobs
    }

    pub fn get_selected_job(&self) -> Option<&Job> {
        let jobs = self.get_jobs_sorted();
        jobs.get(self.selected_job_index).copied()
    }

    pub fn get_selected_job_mut(&mut self) -> Option<&mut Job> {
        let jobs: Vec<Uuid> = self.get_jobs_sorted().iter().map(|j| j.id).collect();
        if let Some(job_id) = jobs.get(self.selected_job_index) {
            self.jobs.get_mut(job_id)
        } else {
            None
        }
    }

    pub fn add_job(&mut self, job: Job) {
        self.action_logs.insert(job.id, Vec::new());
        self.jobs.insert(job.id, job);
        self.last_update = Utc::now();
    }

    pub fn update_job(&mut self, job: Job) {
        self.jobs.insert(job.id, job);
        self.last_update = Utc::now();
    }

    pub fn add_action_log(&mut self, entry: ActionLogEntry) {
        if let Some(logs) = self.action_logs.get_mut(&entry.job_id) {
            logs.push(entry);
            // Keep only the last 1000 entries
            if logs.len() > 1000 {
                logs.drain(0..logs.len() - 1000);
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        match self.current_view {
            ViewMode::Home => {
                if self.selected_job_index > 0 {
                    self.selected_job_index -= 1;
                }
            }
            ViewMode::JobDetail(_) => {
                if self.selected_log_index > 0 {
                    self.selected_log_index -= 1;
                }
            }
            ViewMode::ConflictResolution(_) => {
                if self.selected_conflict_index > 0 {
                    self.selected_conflict_index -= 1;
                }
            }
            _ => {}
        }
    }

    pub fn move_selection_down(&mut self) {
        match self.current_view {
            ViewMode::Home => {
                let job_count = self.jobs.len();
                if job_count > 0 && self.selected_job_index < job_count - 1 {
                    self.selected_job_index += 1;
                }
            }
            ViewMode::JobDetail(job_id) => {
                if let Some(logs) = self.action_logs.get(&job_id) {
                    if !logs.is_empty() && self.selected_log_index < logs.len() - 1 {
                        self.selected_log_index += 1;
                    }
                }
            }
            ViewMode::ConflictResolution(job_id) => {
                if let Some(job) = self.jobs.get(&job_id) {
                    if !job.conflicts.is_empty() && self.selected_conflict_index < job.conflicts.len() - 1 {
                        self.selected_conflict_index += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to humanize byte sizes
fn humanize_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if bytes < THRESHOLD {
        return format!("{} B", bytes);
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanize_bytes() {
        assert_eq!(humanize_bytes(0), "0 B");
        assert_eq!(humanize_bytes(512), "512 B");
        assert_eq!(humanize_bytes(1024), "1.0 KB");
        assert_eq!(humanize_bytes(1536), "1.5 KB");
        assert_eq!(humanize_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_job_creation() {
        let job = Job::new(
            "Test Job".to_string(),
            "/source".to_string(),
            "/target".to_string(),
        );
        
        assert_eq!(job.name, "Test Job");
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.progress, 0.0);
        assert!(job.can_start());
        assert!(!job.can_pause());
    }
}
