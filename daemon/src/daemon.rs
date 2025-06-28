use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn, instrument};
use crate::config::{DaemonConfig, SyncJob, ScheduleType};
use crate::scheduler::{JobScheduler, ScheduledJob};
use crate::telemetry::TelemetrySystem;
use crate::watcher::FileWatcherManager;
use sync_core::api::client::PocketBaseClient;
use sync::{SyncEngine, SyncOptions, ComparisonMethod};

pub struct SyncDaemon {
    config: Arc<RwLock<DaemonConfig>>,
    pocketbase_client: Arc<PocketBaseClient>,
    job_scheduler: JobScheduler,
    file_watcher_manager: FileWatcherManager,
    sync_semaphore: Arc<Semaphore>,
    telemetry: TelemetrySystem,
    shutdown_tx: Option<mpsc::Sender<()>>,
    tasks: HashMap<String, tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct SyncRequest {
    pub job_id: String,
    pub triggered_by: TriggerSource,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum TriggerSource {
    Schedule,
    FileWatcher { watcher_id: String },
    Manual,
    ConfigReload,
}

impl SyncDaemon {
    pub async fn new(config: DaemonConfig) -> Result<Self> {
        info!("Initializing sync daemon");
        
        // Validate configuration
        config.validate()?;
        
        // Initialize PocketBase client
        let pocketbase_client = PocketBaseClient::new(config.pocketbase.url.clone())?;
        
        // Test connection
        if let Err(e) = pocketbase_client.health_check().await {
            warn!("PocketBase health check failed: {}. Continuing anyway.", e);
        } else {
            info!("PocketBase connection established");
        }
        
        let pocketbase_arc = Arc::new(pocketbase_client);
        
        // Initialize telemetry system
        let mut telemetry = TelemetrySystem::new(
            config.telemetry.clone(),
            Some(pocketbase_arc.clone()),
        )?;
        
        // Initialize the telemetry logging (this will replace the basic logging)
        telemetry.initialize_logging()?;
        
        info!(
            daemon_id = %telemetry.daemon_id(),
            session_id = %telemetry.session_id(),
            "Telemetry system initialized"
        );
        
        // Initialize semaphore for concurrency control
        let sync_semaphore = Arc::new(Semaphore::new(config.concurrency.max_concurrent_syncs));
        
        // Initialize scheduler
        let job_scheduler = JobScheduler::new();
        
        // Initialize file watcher manager
        let file_watcher_manager = FileWatcherManager::new();
        
        let daemon = Self {
            config: Arc::new(RwLock::new(config)),
            pocketbase_client: pocketbase_arc,
            job_scheduler,
            file_watcher_manager,
            sync_semaphore,
            telemetry,
            shutdown_tx: None,
            tasks: HashMap::new(),
        };
        
        Ok(daemon)
    }
    
    #[instrument(skip(self))]
    pub async fn run(mut self) -> Result<()> {
        info!("Starting sync daemon");
        
        // Start telemetry background tasks
        self.telemetry.start_background_tasks()?;
        
        // Start metrics server if enabled
        if self.config.read().await.telemetry.metrics.enabled {
            self.start_metrics_server().await?;
        }
        
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        // Setup sync request channel
        let (sync_tx, mut sync_rx) = mpsc::channel::<SyncRequest>(1000);
        
        // Start configuration reload task
        self.start_config_reload_task(sync_tx.clone()).await?;
        
        // Start sync jobs
        self.start_sync_jobs(sync_tx.clone()).await?;
        
        // Start file watchers
        self.start_file_watchers(sync_tx.clone()).await?;
        
        // Start sync processing task
        self.start_sync_processor(sync_rx).await?;
        
        // Setup signal handlers
        self.setup_signal_handlers().await?;
        
        info!("Sync daemon started successfully");
        
        // Main event loop
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Ctrl+C received, shutting down");
                    break;
                }
            }
        }
        
        self.shutdown().await?;
        info!("Sync daemon stopped");
        Ok(())
    }
    
    async fn start_config_reload_task(&mut self, sync_tx: mpsc::Sender<SyncRequest>) -> Result<()> {
        let config = self.config.clone();
        let pocketbase_client = self.pocketbase_client.clone(); // Already Arc, no need to clone
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                config.read().await.daemon.config_refresh_interval_secs
            ));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = Self::reload_config_from_pocketbase(&config, &pocketbase_client, &sync_tx).await {
                    error!("Failed to reload configuration: {}", e);
                }
            }
        });
        
        self.tasks.insert("config_reload".to_string(), handle);
        Ok(())
    }
    
    async fn reload_config_from_pocketbase(
        config: &Arc<RwLock<DaemonConfig>>,
        pocketbase_client: &PocketBaseClient,
        sync_tx: &mpsc::Sender<SyncRequest>,
    ) -> Result<()> {
        debug!("Reloading configuration from PocketBase");
        
        // Try to fetch updated configuration from PocketBase
        // This is a placeholder - you would implement the actual API call
        // to fetch configuration updates from your PocketBase collections
        
        // For now, we'll just validate the current config and trigger a reload
        let current_config = config.read().await.clone();
        if let Err(e) = current_config.validate() {
            warn!("Configuration validation failed: {}", e);
            return Ok(());
        }
        
        // If configuration changed, send reload signal
        if current_config.daemon.auto_restart_on_config_change {
            let _ = sync_tx.send(SyncRequest {
                job_id: "config_reload".to_string(),
                triggered_by: TriggerSource::ConfigReload,
                timestamp: Instant::now(),
            }).await;
        }
        
        Ok(())
    }
    
    async fn start_sync_jobs(&mut self, sync_tx: mpsc::Sender<SyncRequest>) -> Result<()> {
        let config = self.config.read().await;
        
        for job in &config.sync_jobs {
            if !job.enabled {
                continue;
            }
            
            match &job.schedule.schedule_type {
                ScheduleType::Interval { interval } => {
                    let scheduled_job = ScheduledJob::new_interval(
                        job.id.clone(),
                        job.name.clone(),
                        *interval,
                        sync_tx.clone(),
                    );
                    self.job_scheduler.add_job(scheduled_job).await?;
                }
                ScheduleType::Cron { expression } => {
                    let scheduled_job = ScheduledJob::new_cron(
                        job.id.clone(), expression.clone(), sync_tx.clone(),
                    )?;
                    self.job_scheduler.add_job(scheduled_job).await?;
                }
                ScheduleType::Manual => {
                    info!("Sync job '{}' is set to manual scheduling", job.name);
                }
            }
        }
        
        self.job_scheduler.start().await?;
        info!("Started {} sync jobs", config.sync_jobs.len());
        Ok(())
    }
    
    async fn start_file_watchers(&mut self, sync_tx: mpsc::Sender<SyncRequest>) -> Result<()> {
        let config = self.config.read().await;
        
        for watcher in &config.file_watchers {
            if !watcher.enabled {
                continue;
            }
            
            self.file_watcher_manager.add_watcher(
                watcher.id.clone(),
                watcher.watch_path.clone(),
                watcher.recursive,
                watcher.debounce_ms,
                watcher.sync_job_id.clone(),
                sync_tx.clone(),
            ).await?;
        }
        
        self.file_watcher_manager.start().await?;
        info!("Started {} file watchers", config.file_watchers.len());
        Ok(())
    }
    
    async fn start_sync_processor(&mut self, mut sync_rx: mpsc::Receiver<SyncRequest>) -> Result<()> {
        let config = self.config.clone();
        let semaphore = Arc::clone(&self.sync_semaphore);
        
        let handle = tokio::spawn(async move {
            while let Some(request) = sync_rx.recv().await {
                let semaphore_clone = Arc::clone(&semaphore);
                let config_clone = config.clone();
                let request_clone = request.clone();
                
                // Spawn a task to handle concurrency control and processing
                tokio::spawn(async move {
                    // Try to acquire permit within the spawned task
                    let _permit = match semaphore_clone.acquire().await {
                        Ok(permit) => permit,
                        Err(_) => {
                            warn!("Failed to acquire semaphore permit");
                            return;
                        }
                    };
                    
                    if let Err(e) = Self::process_sync_request(config_clone, request_clone).await {
                        error!("Error in sync request processing: {}", e);
                    }
                    // permit is automatically dropped when task completes
                });
            }
        });
        
        self.tasks.insert("sync_processor".to_string(), handle);
        Ok(())
    }
    
    #[instrument(skip(config), fields(job_id = %request.job_id))]
    async fn process_sync_request(
        config: Arc<RwLock<DaemonConfig>>,
        request: SyncRequest,
    ) -> Result<()> {
        let config_read = config.read().await;
        
        // Find the sync job
        let job = config_read
            .sync_jobs
            .iter()
            .find(|j| j.id == request.job_id)
            .ok_or_else(|| anyhow::anyhow!("Sync job not found: {}", request.job_id))?;
        
        if !job.enabled {
            debug!("Sync job '{}' is disabled, skipping", job.name);
            return Ok(());
        }
        
        info!(
            job_name = %job.name,
            trigger_source = ?request.triggered_by,
            source_path = %job.source_path.display(),
            destination_path = %job.destination_path.display(),
            "Starting sync job"
        );
        
        // Build sync options
        let sync_options = Self::build_sync_options(job)?;
        
        // Create sync engine
        let mut sync_engine = SyncEngine::new(sync_options);
        
        // Perform the sync
        let start_time = Instant::now();
        match sync_engine.sync(&job.source_path, &job.destination_path).await {
            Ok(metrics) => {
                let duration = start_time.elapsed();
                info!(
                    job_name = %job.name,
                    duration_ms = duration.as_millis(),
                    files_processed = metrics.files.processed,
                    bytes_transferred = metrics.transfer.bytes_transferred,
                    success_rate = metrics.success_rate(),
                    "Sync job completed successfully"
                );
            }
            Err(e) => {
                error!(
                    job_name = %job.name,
                    error = %e,
                    "Sync job failed"
                );
            }
        }
        
        Ok(())
    }
    
    fn build_sync_options(job: &SyncJob) -> Result<SyncOptions> {
        let comparison_method = match job.sync_options.comparison_method.as_str() {
            "size" => ComparisonMethod::Size,
            "sha256" => ComparisonMethod::Sha256,
            "timestamp" => ComparisonMethod::Timestamp,
            _ => ComparisonMethod::SizeAndTimestamp,
        };
        
        let mut options = SyncOptions {
            dry_run: job.sync_options.dry_run,
            comparison_method,
            continue_on_error: true,
            ..Default::default()
        };
        
        // Note: Filters would be applied through FilterOptions
        // This is a placeholder for proper filter configuration
        
        Ok(options)
    }
    
    async fn start_metrics_server(&mut self) -> Result<()> {
        let config = self.config.read().await;
        if !config.telemetry.metrics.enabled {
            return Ok(());
        }

        let bind_addr = format!(
            "{}:{}",
            config.telemetry.metrics.bind_address,
            config.telemetry.metrics.port
        );

        let app = self.telemetry.create_metrics_server();
        
        let handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
            info!("Metrics server listening on {}", bind_addr);
            axum::serve(listener, app).await.unwrap();
        });

        self.tasks.insert("metrics_server".to_string(), handle);
        Ok(())
    }

    async fn setup_signal_handlers(&self) -> Result<()> {
        // Signal handling is platform-specific and would be implemented here
        // For now, we rely on tokio::signal::ctrl_c() in the main loop
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down sync daemon");
        
        // Stop all tasks
        for (name, handle) in self.tasks.drain() {
            debug!("Stopping task: {}", name);
            handle.abort();
        }
        
        // Stop scheduler
        self.job_scheduler.stop().await?;
        
        // Stop file watchers
        self.file_watcher_manager.stop().await?;
        
        Ok(())
    }
}

impl Drop for SyncDaemon {
    fn drop(&mut self) {
        // Cleanup any remaining resources
        for (_, handle) in self.tasks.drain() {
            handle.abort();
        }
    }
}
