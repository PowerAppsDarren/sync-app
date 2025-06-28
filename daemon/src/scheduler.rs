use anyhow::Result;
use cron::Schedule;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Instant};
use tracing::{debug, error, info, warn};

use crate::daemon::{SyncRequest, TriggerSource};

#[derive(Debug)]
pub struct JobScheduler {
    jobs: Arc<RwLock<HashMap<String, ScheduledJob>>>,
    running: Arc<RwLock<bool>>,
    tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub schedule_type: ScheduleType,
    pub sync_tx: mpsc::Sender<SyncRequest>,
    pub enabled: bool,
    pub last_run: Option<Instant>,
    pub next_run: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum ScheduleType {
    Interval { interval: Duration },
    Cron { schedule: Schedule },
}

impl JobScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_job(&self, job: ScheduledJob) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        info!("Adding scheduled job: {} ({})", job.name, job.id);
        jobs.insert(job.id.clone(), job);
        Ok(())
    }

    pub async fn remove_job(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        let mut tasks = self.tasks.write().await;
        
        if jobs.remove(job_id).is_some() {
            info!("Removed scheduled job: {}", job_id);
            
            // Stop the associated task if it exists
            if let Some(handle) = tasks.remove(job_id) {
                handle.abort();
                debug!("Stopped task for job: {}", job_id);
            }
        }
        
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            warn!("Job scheduler is already running");
            return Ok(());
        }
        
        *running = true;
        info!("Starting job scheduler");
        
        // Start scheduler tasks for each job
        let jobs = self.jobs.read().await;
        for job in jobs.values() {
            self.start_job_task(job.clone()).await?;
        }
        
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        *running = false;
        info!("Stopping job scheduler");
        
        // Stop all job tasks
        let mut tasks = self.tasks.write().await;
        for (job_id, handle) in tasks.drain() {
            debug!("Stopping scheduler task for job: {}", job_id);
            handle.abort();
        }
        
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn get_job_status(&self, job_id: &str) -> Option<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).map(|job| JobStatus {
            id: job.id.clone(),
            name: job.name.clone(),
            enabled: job.enabled,
            last_run: job.last_run,
            next_run: job.next_run,
            schedule_type: job.schedule_type.clone(),
        })
    }

    pub async fn list_jobs(&self) -> Vec<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.values()
            .map(|job| JobStatus {
                id: job.id.clone(),
                name: job.name.clone(),
                enabled: job.enabled,
                last_run: job.last_run,
                next_run: job.next_run,
                schedule_type: job.schedule_type.clone(),
            })
            .collect()
    }

    async fn start_job_task(&self, job: ScheduledJob) -> Result<()> {
        if !job.enabled {
            debug!("Job '{}' is disabled, not starting task", job.name);
            return Ok(());
        }

        let running = self.running.clone();
        let jobs = self.jobs.clone();
        
        let handle = match &job.schedule_type {
            ScheduleType::Interval { interval } => {
                let interval = *interval;
                let job_clone = job.clone();
                
                tokio::spawn(async move {
                    Self::run_interval_job(job_clone, interval, running, jobs).await;
                })
            }
            ScheduleType::Cron { schedule } => {
                let schedule = schedule.clone();
                let job_clone = job.clone();
                
                tokio::spawn(async move {
                    Self::run_cron_job(job_clone, schedule, running, jobs).await;
                })
            }
        };

        let mut tasks = self.tasks.write().await;
        tasks.insert(job.id.clone(), handle);
        
        debug!("Started scheduler task for job: {}", job.name);
        Ok(())
    }

    async fn run_interval_job(
        job: ScheduledJob,
        interval: Duration,
        running: Arc<RwLock<bool>>,
        jobs: Arc<RwLock<HashMap<String, ScheduledJob>>>,
    ) {
        info!("Starting interval job '{}' with interval {:?}", job.name, interval);
        
        let mut ticker = tokio::time::interval(interval);
        ticker.tick().await; // Skip the first immediate tick
        
        while *running.read().await {
            ticker.tick().await;
            
            // Check if job is still enabled
            let job_enabled = {
                let jobs_read = jobs.read().await;
                jobs_read.get(&job.id).map(|j| j.enabled).unwrap_or(false)
            };
            
            if !job_enabled {
                debug!("Job '{}' is disabled, skipping execution", job.name);
                continue;
            }
            
            debug!("Triggering interval job: {}", job.name);
            
            let request = SyncRequest {
                job_id: job.id.clone(),
                triggered_by: TriggerSource::Schedule,
                timestamp: Instant::now(),
            };
            
            if let Err(e) = job.sync_tx.send(request).await {
                error!("Failed to send sync request for job '{}': {}", job.name, e);
                break;
            }
            
            // Update last run time
            {
                let mut jobs_write = jobs.write().await;
                if let Some(mut job_mut) = jobs_write.get_mut(&job.id) {
                    job_mut.last_run = Some(Instant::now());
                    job_mut.next_run = Some(Instant::now() + interval);
                }
            }
        }
        
        debug!("Interval job '{}' task stopped", job.name);
    }

    async fn run_cron_job(
        job: ScheduledJob,
        schedule: Schedule,
        running: Arc<RwLock<bool>>,
        jobs: Arc<RwLock<HashMap<String, ScheduledJob>>>,
    ) {
        info!("Starting cron job '{}' with schedule", job.name);
        
        while *running.read().await {
            let now = chrono::Utc::now();
            let next_run = match schedule.after(&now).next() {
                Some(datetime) => datetime,
                None => {
                    error!("Failed to calculate next run time for cron job '{}'", job.name);
                    break;
                }
            };
            
            let duration_until_next = (next_run - now).to_std().unwrap_or(Duration::from_secs(60));
            
            // Update next run time
            {
                let mut jobs_write = jobs.write().await;
                if let Some(mut job_mut) = jobs_write.get_mut(&job.id) {
                    job_mut.next_run = Some(Instant::now() + duration_until_next);
                }
            }
            
            debug!(
                "Cron job '{}' scheduled to run at {} (in {:?})",
                job.name, next_run, duration_until_next
            );
            
            // Wait until it's time to run
            sleep(duration_until_next).await;
            
            if !*running.read().await {
                break;
            }
            
            // Check if job is still enabled
            let job_enabled = {
                let jobs_read = jobs.read().await;
                jobs_read.get(&job.id).map(|j| j.enabled).unwrap_or(false)
            };
            
            if !job_enabled {
                debug!("Cron job '{}' is disabled, skipping execution", job.name);
                continue;
            }
            
            debug!("Triggering cron job: {}", job.name);
            
            let request = SyncRequest {
                job_id: job.id.clone(),
                triggered_by: TriggerSource::Schedule,
                timestamp: Instant::now(),
            };
            
            if let Err(e) = job.sync_tx.send(request).await {
                error!("Failed to send sync request for cron job '{}': {}", job.name, e);
                break;
            }
            
            // Update last run time
            {
                let mut jobs_write = jobs.write().await;
                if let Some(mut job_mut) = jobs_write.get_mut(&job.id) {
                    job_mut.last_run = Some(Instant::now());
                }
            }
        }
        
        debug!("Cron job '{}' task stopped", job.name);
    }
}

impl ScheduledJob {
    pub fn new_interval(
        id: String,
        name: String,
        interval: Duration,
        sync_tx: mpsc::Sender<SyncRequest>,
    ) -> Self {
        Self {
            id,
            name,
            schedule_type: ScheduleType::Interval { interval },
            sync_tx,
            enabled: true,
            last_run: None,
            next_run: Some(Instant::now() + interval),
        }
    }

    pub fn new_cron(
        id: String,
        cron_expression: String,
        sync_tx: mpsc::Sender<SyncRequest>,
    ) -> Result<Self> {
        let name = format!("Cron job {}", id);
        let schedule = Schedule::from_str(&cron_expression)
            .map_err(|e| anyhow::anyhow!("Invalid cron expression '{}': {}", cron_expression, e))?;
        
        // Calculate next run time
        let now = chrono::Utc::now();
        let next_run = schedule.after(&now).next()
            .map(|datetime| {
                let duration = (datetime - now).to_std().unwrap_or(Duration::from_secs(60));
                Instant::now() + duration
            });

        Ok(Self {
            id,
            name,
            schedule_type: ScheduleType::Cron { schedule },
            sync_tx,
            enabled: true,
            last_run: None,
            next_run,
        })
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

#[derive(Debug, Clone)]
pub struct JobStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub last_run: Option<Instant>,
    pub next_run: Option<Instant>,
    pub schedule_type: ScheduleType,
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JobScheduler {
    fn drop(&mut self) {
        // Note: This is a synchronous drop, so we can't await the async stop method
        // The tasks will be aborted when the handles are dropped
    }
}
