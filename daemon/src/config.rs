use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::str::FromStr;

use crate::telemetry::TelemetryConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub pocketbase: PocketBaseConfig,
    pub daemon: DaemonSettings,
    pub sync_jobs: Vec<SyncJob>,
    pub file_watchers: Vec<FileWatcher>,
    pub concurrency: ConcurrencyConfig,
    pub cache: CacheConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PocketBaseConfig {
    pub url: String,
    pub admin_email: String,
    pub admin_password: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_delay_secs")]
    pub retry_delay_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSettings {
    #[serde(default = "default_pid_file")]
    pub pid_file: PathBuf,
    #[serde(default = "default_log_file")]
    pub log_file: Option<PathBuf>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_config_refresh_interval")]
    pub config_refresh_interval_secs: u64,
    #[serde(default)]
    pub auto_restart_on_config_change: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: String,
    pub name: String,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub schedule: ScheduleConfig,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub filters: Vec<String>,
    #[serde(default)]
    pub sync_options: SyncJobOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    #[serde(flatten)]
    pub schedule_type: ScheduleType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScheduleType {
    #[serde(rename = "interval")]
    Interval {
        #[serde(with = "humantime_serde")]
        interval: Duration,
    },
    #[serde(rename = "cron")]
    Cron {
        expression: String,
    },
    #[serde(rename = "manual")]
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobOptions {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_preserve_permissions")]
    pub preserve_permissions: bool,
    #[serde(default = "default_preserve_timestamps")]
    pub preserve_timestamps: bool,
    #[serde(default)]
    pub delete_destination_files: bool,
    #[serde(default = "default_comparison_method")]
    pub comparison_method: String,
    #[serde(default)]
    pub ignore_hidden_files: bool,
    #[serde(default)]
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatcher {
    pub id: String,
    pub name: String,
    pub watch_path: PathBuf,
    pub sync_job_id: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub debounce_ms: u64,
    #[serde(default)]
    pub watch_events: Vec<WatchEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchEvent {
    Create,
    Write,
    Remove,
    Rename,
    Chmod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    #[serde(default = "default_max_concurrent_syncs")]
    pub max_concurrent_syncs: usize,
    #[serde(default = "default_max_file_operations")]
    pub max_file_operations: usize,
    #[serde(default = "default_sync_queue_size")]
    pub sync_queue_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    #[serde(default = "default_config_cache_ttl")]
    pub config_cache_ttl_secs: u64,
    #[serde(default = "default_file_metadata_cache_ttl")]
    pub file_metadata_cache_ttl_secs: u64,
    #[serde(default)]
    pub enable_persistent_cache: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            pocketbase: PocketBaseConfig::default(),
            daemon: DaemonSettings::default(),
            sync_jobs: vec![Self::default_sync_job()],
            file_watchers: vec![Self::default_file_watcher()],
            concurrency: ConcurrencyConfig::default(),
            cache: CacheConfig::default(),
            telemetry: TelemetryConfig::default(),
        }
    }
}

impl Default for PocketBaseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8090".to_string(),
            admin_email: "admin@example.com".to_string(),
            admin_password: "admin123456".to_string(),
            timeout_secs: default_timeout_secs(),
            retry_attempts: default_retry_attempts(),
            retry_delay_secs: default_retry_delay_secs(),
        }
    }
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            pid_file: default_pid_file(),
            log_file: default_log_file(),
            log_level: default_log_level(),
            config_refresh_interval_secs: default_config_refresh_interval(),
            auto_restart_on_config_change: false,
        }
    }
}

impl Default for SyncJobOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            preserve_permissions: default_preserve_permissions(),
            preserve_timestamps: default_preserve_timestamps(),
            delete_destination_files: false,
            comparison_method: default_comparison_method(),
            ignore_hidden_files: false,
            continue_on_error: false,
        }
    }
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent_syncs: default_max_concurrent_syncs(),
            max_file_operations: default_max_file_operations(),
            sync_queue_size: default_sync_queue_size(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: default_cache_dir(),
            config_cache_ttl_secs: default_config_cache_ttl(),
            file_metadata_cache_ttl_secs: default_file_metadata_cache_ttl(),
            enable_persistent_cache: false,
        }
    }
}

impl DaemonConfig {
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: DaemonConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        // Validate PocketBase URL
        url::Url::parse(&self.pocketbase.url)?;
        
        // Validate sync jobs
        for job in &self.sync_jobs {
            if job.id.is_empty() {
                anyhow::bail!("Sync job ID cannot be empty");
            }
            if !job.source_path.exists() {
                anyhow::bail!("Source path does not exist: {}", job.source_path.display());
            }
            
            // Validate cron expressions
            if let ScheduleType::Cron { expression } = &job.schedule.schedule_type {
                let _ = cron::Schedule::from_str(expression)?;
            }
        }
        
        // Validate file watchers
        for watcher in &self.file_watchers {
            if watcher.id.is_empty() {
                anyhow::bail!("File watcher ID cannot be empty");
            }
            if !watcher.watch_path.exists() {
                anyhow::bail!("Watch path does not exist: {}", watcher.watch_path.display());
            }
            
            // Check if referenced sync job exists
            if !self.sync_jobs.iter().any(|job| job.id == watcher.sync_job_id) {
                anyhow::bail!("File watcher references non-existent sync job: {}", watcher.sync_job_id);
            }
        }
        
        Ok(())
    }

    fn default_sync_job() -> SyncJob {
        SyncJob {
            id: "default".to_string(),
            name: "Default Sync Job".to_string(),
            source_path: PathBuf::from("./source"),
            destination_path: PathBuf::from("./destination"),
            schedule: ScheduleConfig {
                schedule_type: ScheduleType::Interval {
                    interval: Duration::from_secs(300), // 5 minutes
                },
            },
            enabled: true,
            filters: vec!["*.tmp".to_string(), "*.log".to_string()],
            sync_options: SyncJobOptions::default(),
        }
    }

    fn default_file_watcher() -> FileWatcher {
        FileWatcher {
            id: "default-watcher".to_string(),
            name: "Default File Watcher".to_string(),
            watch_path: PathBuf::from("./source"),
            sync_job_id: "default".to_string(),
            enabled: true,
            recursive: true,
            debounce_ms: 1000,
            watch_events: vec![
                WatchEvent::Create,
                WatchEvent::Write,
                WatchEvent::Remove,
                WatchEvent::Rename,
            ],
        }
    }
}

// Default value functions
fn default_timeout_secs() -> u64 { 30 }
fn default_retry_attempts() -> u32 { 3 }
fn default_retry_delay_secs() -> u64 { 5 }

fn default_pid_file() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\ProgramData\sync-daemon\sync-daemon.pid")
    } else {
        PathBuf::from("/var/run/sync-daemon.pid")
    }
}

fn default_log_file() -> Option<PathBuf> {
    Some(if cfg!(windows) {
        PathBuf::from(r"C:\ProgramData\sync-daemon\sync-daemon.log")
    } else {
        PathBuf::from("/var/log/sync-daemon.log")
    })
}

fn default_log_level() -> String { "info".to_string() }
fn default_config_refresh_interval() -> u64 { 300 } // 5 minutes

fn default_preserve_permissions() -> bool { true }
fn default_preserve_timestamps() -> bool { true }
fn default_comparison_method() -> String { "checksum".to_string() }

fn default_max_concurrent_syncs() -> usize { 4 }
fn default_max_file_operations() -> usize { 100 }
fn default_sync_queue_size() -> usize { 1000 }

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sync-daemon")
}

fn default_config_cache_ttl() -> u64 { 300 } // 5 minutes
fn default_file_metadata_cache_ttl() -> u64 { 60 } // 1 minute
