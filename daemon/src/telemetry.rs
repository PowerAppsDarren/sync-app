//! Telemetry module for logging, tracing, and metrics collection
//!
//! This module provides:
//! - Structured logging with JSON and pretty formatters
//! - Log persistence to PocketBase and local files
//! - Prometheus metrics collection and exposure
//! - Error tracking and telemetry

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use prometheus::{
    CounterVec, Gauge, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, Opts, Registry,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tracing::{error, info, warn};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use uuid::Uuid;
use sysinfo::{System, Process};

use sync_core::api::client::PocketBaseClient;

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    /// Enable JSON logging
    pub json_logging: bool,
    /// Enable console logging
    pub console_logging: bool,
    /// Local log file path
    pub log_file_path: Option<PathBuf>,
    /// Log file rotation settings
    pub log_rotation: LogRotationConfig,
    /// PocketBase logging settings
    pub pocketbase_logging: PocketBaseLoggingConfig,
    /// Metrics settings
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Enable log rotation
    pub enabled: bool,
    /// Rotation frequency (daily, hourly)
    pub frequency: String,
    /// Keep this many log files
    pub keep_files: u32,
    /// Maximum file size before rotation (in MB)
    pub max_size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PocketBaseLoggingConfig {
    /// Enable PocketBase logging
    pub enabled: bool,
    /// Collection name for logs
    pub collection: String,
    /// Batch size for log uploads
    pub batch_size: u32,
    /// Flush interval in seconds
    pub flush_interval_secs: u64,
    /// Maximum retries for failed uploads
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable Prometheus metrics
    pub enabled: bool,
    /// Metrics server bind address
    pub bind_address: String,
    /// Metrics server port
    pub port: u16,
    /// Metrics collection interval in seconds
    pub collection_interval_secs: u64,
}

/// Log entry for PocketBase storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub target: String,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub fields: HashMap<String, serde_json::Value>,
    pub spans: Vec<SpanInfo>,
    pub daemon_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanInfo {
    pub name: String,
    pub target: String,
    pub level: String,
    pub fields: HashMap<String, serde_json::Value>,
}

/// Prometheus metrics registry and collectors
#[derive(Clone)]
pub struct DaemonMetrics {
    registry: Registry,
    
    // Sync operation metrics
    pub sync_operations_total: IntCounterVec,
    pub sync_operations_duration: HistogramVec,
    pub sync_files_processed: IntCounterVec,
    pub sync_bytes_transferred: CounterVec,
    pub sync_errors_total: IntCounterVec,
    
    // Daemon health metrics
    pub daemon_uptime_seconds: IntGauge,
    pub daemon_memory_usage_bytes: IntGauge,
    pub daemon_cpu_usage_percent: Gauge,
    pub active_sync_jobs: IntGauge,
    pub file_watchers_active: IntGauge,
    
    // PocketBase metrics
    pub pocketbase_requests_total: IntCounterVec,
    pub pocketbase_request_duration: HistogramVec,
    pub pocketbase_connection_errors: IntCounter,
    
    // Log metrics
    pub log_entries_total: IntCounterVec,
    pub log_upload_errors: IntCounter,
    pub log_buffer_size: IntGauge,
}

/// Telemetry system for the daemon
pub struct TelemetrySystem {
    config: TelemetryConfig,
    metrics: DaemonMetrics,
    pocketbase_client: Option<Arc<PocketBaseClient>>,
    log_buffer: Arc<Mutex<Vec<LogEntry>>>,
    _file_guard: Option<WorkerGuard>,
    daemon_id: String,
    session_id: String,
    start_time: SystemTime,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            json_logging: true,
            console_logging: true,
            log_file_path: Some(PathBuf::from("logs/daemon.log")),
            log_rotation: LogRotationConfig {
                enabled: true,
                frequency: "daily".to_string(),
                keep_files: 7,
                max_size_mb: 100,
            },
            pocketbase_logging: PocketBaseLoggingConfig {
                enabled: true,
                collection: "daemon_logs".to_string(),
                batch_size: 100,
                flush_interval_secs: 30,
                max_retries: 3,
            },
            metrics: MetricsConfig {
                enabled: true,
                bind_address: "127.0.0.1".to_string(),
                port: 9090,
                collection_interval_secs: 15,
            },
        }
    }
}

impl DaemonMetrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        // Sync operation metrics
        let sync_operations_total = IntCounterVec::new(
            Opts::new("sync_operations_total", "Total number of sync operations"),
            &["job_id", "status", "trigger_source"],
        )?;

        let sync_operations_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "sync_operations_duration_seconds",
                "Duration of sync operations in seconds",
            ),
            &["job_id", "status"],
        )?;

        let sync_files_processed = IntCounterVec::new(
            Opts::new("sync_files_processed_total", "Total number of files processed"),
            &["job_id", "operation"],
        )?;

        let sync_bytes_transferred = CounterVec::new(
            Opts::new("sync_bytes_transferred_total", "Total bytes transferred"),
            &["job_id", "direction"],
        )?;

        let sync_errors_total = IntCounterVec::new(
            Opts::new("sync_errors_total", "Total number of sync errors"),
            &["job_id", "error_type"],
        )?;

        // Daemon health metrics
        let daemon_uptime_seconds = IntGauge::new(
            "daemon_uptime_seconds",
            "Daemon uptime in seconds",
        )?;

        let daemon_memory_usage_bytes = IntGauge::new(
            "daemon_memory_usage_bytes",
            "Memory usage in bytes",
        )?;

        let daemon_cpu_usage_percent = Gauge::new(
            "daemon_cpu_usage_percent",
            "CPU usage percentage",
        )?;

        let active_sync_jobs = IntGauge::new(
            "active_sync_jobs",
            "Number of currently active sync jobs",
        )?;

        let file_watchers_active = IntGauge::new(
            "file_watchers_active",
            "Number of active file watchers",
        )?;

        // PocketBase metrics
        let pocketbase_requests_total = IntCounterVec::new(
            Opts::new("pocketbase_requests_total", "Total PocketBase requests"),
            &["method", "status"],
        )?;

        let pocketbase_request_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "pocketbase_request_duration_seconds",
                "PocketBase request duration in seconds",
            ),
            &["method", "status"],
        )?;

        let pocketbase_connection_errors = IntCounter::new(
            "pocketbase_connection_errors_total",
            "Total PocketBase connection errors",
        )?;

        // Log metrics
        let log_entries_total = IntCounterVec::new(
            Opts::new("log_entries_total", "Total log entries"),
            &["level", "target"],
        )?;

        let log_upload_errors = IntCounter::new(
            "log_upload_errors_total",
            "Total log upload errors",
        )?;

        let log_buffer_size = IntGauge::new(
            "log_buffer_size",
            "Current log buffer size",
        )?;

        // Register all metrics
        registry.register(Box::new(sync_operations_total.clone()))?;
        registry.register(Box::new(sync_operations_duration.clone()))?;
        registry.register(Box::new(sync_files_processed.clone()))?;
        registry.register(Box::new(sync_bytes_transferred.clone()))?;
        registry.register(Box::new(sync_errors_total.clone()))?;
        registry.register(Box::new(daemon_uptime_seconds.clone()))?;
        registry.register(Box::new(daemon_memory_usage_bytes.clone()))?;
        registry.register(Box::new(daemon_cpu_usage_percent.clone()))?;
        registry.register(Box::new(active_sync_jobs.clone()))?;
        registry.register(Box::new(file_watchers_active.clone()))?;
        registry.register(Box::new(pocketbase_requests_total.clone()))?;
        registry.register(Box::new(pocketbase_request_duration.clone()))?;
        registry.register(Box::new(pocketbase_connection_errors.clone()))?;
        registry.register(Box::new(log_entries_total.clone()))?;
        registry.register(Box::new(log_upload_errors.clone()))?;
        registry.register(Box::new(log_buffer_size.clone()))?;

        Ok(Self {
            registry,
            sync_operations_total,
            sync_operations_duration,
            sync_files_processed,
            sync_bytes_transferred,
            sync_errors_total,
            daemon_uptime_seconds,
            daemon_memory_usage_bytes,
            daemon_cpu_usage_percent,
            active_sync_jobs,
            file_watchers_active,
            pocketbase_requests_total,
            pocketbase_request_duration,
            pocketbase_connection_errors,
            log_entries_total,
            log_upload_errors,
            log_buffer_size,
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl TelemetrySystem {
    pub fn new(
        config: TelemetryConfig,
        pocketbase_client: Option<Arc<PocketBaseClient>>,
    ) -> Result<Self> {
        let metrics = DaemonMetrics::new()?;
        let daemon_id = Uuid::new_v4().to_string();
        let session_id = Uuid::new_v4().to_string();

        Ok(Self {
            config,
            metrics,
            pocketbase_client,
            log_buffer: Arc::new(Mutex::new(Vec::new())),
            _file_guard: None,
            daemon_id,
            session_id,
            start_time: SystemTime::now(),
        })
    }

    pub fn initialize_logging(&mut self) -> Result<()> {
        let level = match self.config.log_level.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        };

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(format!("sync_daemon={}", level)));

        let registry = tracing_subscriber::registry().with(filter);

        // Console layer (pretty formatting)
        let console_layer = if self.config.console_logging {
            Some(
                fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
                    .pretty(),
            )
        } else {
            None
        };

        // File layer - simplified to always use JSON if file logging is enabled
        let (file_layer, guard) = if let Some(log_path) = &self.config.log_file_path {
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file_appender = if self.config.log_rotation.enabled {
                match self.config.log_rotation.frequency.as_str() {
                    "daily" => rolling::daily(
                        log_path.parent().unwrap_or_else(|| std::path::Path::new(".")),
                        log_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("daemon.log")),
                    ),
                    "hourly" => rolling::hourly(
                        log_path.parent().unwrap_or_else(|| std::path::Path::new(".")),
                        log_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("daemon.log")),
                    ),
                    _ => rolling::never(
                        log_path.parent().unwrap_or_else(|| std::path::Path::new(".")),
                        log_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("daemon.log")),
                    ),
                }
            } else {
                rolling::never(
                    log_path.parent().unwrap_or_else(|| std::path::Path::new(".")),
                    log_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("daemon.log")),
                )
            };

            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            let layer = Some(
                fmt::layer()
                    .with_writer(non_blocking)
                    .json()
                    .with_current_span(true)
                    .with_span_list(true),
            );
            (layer, Some(guard))
        } else {
            (None, None)
        };

        // PocketBase logging layer
        let pocketbase_layer = if self.config.pocketbase_logging.enabled {
            Some(PocketBaseLayer::new(
                self.log_buffer.clone(),
                self.daemon_id.clone(),
                self.session_id.clone(),
                self.metrics.log_entries_total.clone(),
            ))
        } else {
            None
        };

        // Combine all layers
        let subscriber = registry
            .with(console_layer)
            .with(file_layer)
            .with(pocketbase_layer);

        subscriber.init();
        self._file_guard = guard;

        info!(
            daemon_id = %self.daemon_id,
            session_id = %self.session_id,
            "Telemetry system initialized"
        );

        Ok(())
    }

    pub fn start_background_tasks(&self) -> Result<()> {
        if self.config.pocketbase_logging.enabled && self.pocketbase_client.is_some() {
            self.start_log_upload_task()?;
        }

        if self.config.metrics.enabled {
            self.start_metrics_collection_task()?;
        }

        Ok(())
    }

    fn start_log_upload_task(&self) -> Result<()> {
        let buffer = self.log_buffer.clone();
        let client = self.pocketbase_client.as_ref().unwrap().clone();
        let config = self.config.pocketbase_logging.clone();
        let upload_errors = self.metrics.log_upload_errors.clone();
        let buffer_size_gauge = self.metrics.log_buffer_size.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.flush_interval_secs));

            loop {
                interval.tick().await;

                let logs_to_upload = {
                    let mut buffer_guard = buffer.lock().unwrap();
                    if buffer_guard.is_empty() {
                        continue;
                    }

                    let batch_size = config.batch_size.min(buffer_guard.len() as u32) as usize;
                    let logs: Vec<LogEntry> = buffer_guard.drain(..batch_size).collect();
                    buffer_size_gauge.set(buffer_guard.len() as i64);
                    logs
                };

                if let Err(e) = upload_logs_to_pocketbase(&client, &config.collection, logs_to_upload).await {
                    error!("Failed to upload logs to PocketBase: {}", e);
                    upload_errors.inc();
                }
            }
        });

        Ok(())
    }

    fn start_metrics_collection_task(&self) -> Result<()> {
        let uptime_gauge = self.metrics.daemon_uptime_seconds.clone();
        let memory_gauge = self.metrics.daemon_memory_usage_bytes.clone();
        let cpu_gauge = self.metrics.daemon_cpu_usage_percent.clone();
        let start_time = self.start_time;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(15));
            let mut system = sysinfo::System::new_all();

            loop {
                interval.tick().await;

                // Update uptime
                if let Ok(duration) = start_time.elapsed() {
                    uptime_gauge.set(duration.as_secs() as i64);
                }

                // Update system metrics
                system.refresh_all();
                if let Some(process) = system.processes().get(&sysinfo::Pid::from(std::process::id() as usize)) {
                    memory_gauge.set(process.memory() as i64 * 1024); // Convert KB to bytes
                    cpu_gauge.set(process.cpu_usage() as f64);
                }
            }
        });

        Ok(())
    }

    pub fn create_metrics_server(&self) -> Router {
        let registry = self.metrics.registry.clone();

        Router::new()
            .route("/metrics", get(metrics_handler))
            .with_state(registry)
    }

    pub fn metrics(&self) -> &DaemonMetrics {
        &self.metrics
    }

    pub fn daemon_id(&self) -> &str {
        &self.daemon_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

async fn upload_logs_to_pocketbase(
    client: &PocketBaseClient,
    collection: &str,
    logs: Vec<LogEntry>,
) -> Result<()> {
    for log in logs {
        if let Err(e) = client.create_record::<LogEntry, serde_json::Value>(collection, &log).await {
            warn!("Failed to upload log entry: {}", e);
        }
    }
    Ok(())
}

async fn metrics_handler(State(registry): State<Registry>) -> Response {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry.gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(output) => (StatusCode::OK, output).into_response(),
        Err(e) => {
            error!("Failed to encode metrics: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response()
        }
    }
}

/// Custom tracing layer for PocketBase logging
struct PocketBaseLayer {
    buffer: Arc<Mutex<Vec<LogEntry>>>,
    daemon_id: String,
    session_id: String,
    log_counter: IntCounterVec,
}

impl PocketBaseLayer {
    fn new(
        buffer: Arc<Mutex<Vec<LogEntry>>>,
        daemon_id: String,
        session_id: String,
        log_counter: IntCounterVec,
    ) -> Self {
        Self {
            buffer,
            daemon_id,
            session_id,
            log_counter,
        }
    }
}

impl<S> Layer<S> for PocketBaseLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let mut fields = HashMap::new();
        let mut visitor = FieldVisitor::new(&mut fields);
        event.record(&mut visitor);

        let log_entry = LogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level: metadata.level().to_string(),
            message: fields.remove("message").unwrap_or_else(|| "".into()).to_string(),
            target: metadata.target().to_string(),
            module_path: metadata.module_path().map(|s| s.to_string()),
            file: metadata.file().map(|s| s.to_string()),
            line: metadata.line(),
            fields,
            spans: Vec::new(), // TODO: Collect span information
            daemon_id: self.daemon_id.clone(),
            session_id: self.session_id.clone(),
        };

        // Update metrics
        self.log_counter
            .with_label_values(&[&log_entry.level, &log_entry.target])
            .inc();

        // Add to buffer
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push(log_entry);
        }
    }
}

struct FieldVisitor<'a> {
    fields: &'a mut HashMap<String, serde_json::Value>,
}

impl<'a> FieldVisitor<'a> {
    fn new(fields: &'a mut HashMap<String, serde_json::Value>) -> Self {
        Self { fields }
    }
}

impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{:?}", value)),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(serde_json::Number::from(value)),
        );
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(serde_json::Number::from(value)),
        );
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Bool(value),
        );
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        );
    }
}
