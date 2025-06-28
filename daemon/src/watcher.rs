use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::daemon::{SyncRequest, TriggerSource};

#[derive(Debug)]
pub struct FileWatcherManager {
    watchers: Arc<RwLock<HashMap<String, WatcherInstance>>>,
    running: Arc<RwLock<bool>>,
    event_tx: Option<mpsc::Sender<WatcherEvent>>,
    event_rx: Option<mpsc::Receiver<WatcherEvent>>,
}

#[derive(Debug)]
struct WatcherInstance {
    id: String,
    name: String,
    path: PathBuf,
    sync_job_id: String,
    watcher: RecommendedWatcher,
    debounce_ms: u64,
    last_event: Option<Instant>,
    sync_tx: mpsc::Sender<SyncRequest>,
}

#[derive(Debug, Clone)]
struct WatcherEvent {
    watcher_id: String,
    event: Event,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Create,
    Write,
    Remove,
    Rename,
    Chmod,
}

impl FileWatcherManager {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);
        
        Self {
            watchers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            event_tx: Some(event_tx),
            event_rx: Some(event_rx),
        }
    }

    pub async fn add_watcher(
        &self,
        id: String,
        path: PathBuf,
        recursive: bool,
        debounce_ms: u64,
        sync_job_id: String,
        sync_tx: mpsc::Sender<SyncRequest>,
    ) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::anyhow!("Watch path does not exist: {}", path.display()));
        }

        let event_tx = self.event_tx.as_ref().unwrap().clone();
        let watcher_id = id.clone();

        // Create the notify watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    let watcher_event = WatcherEvent {
                        watcher_id: watcher_id.clone(),
                        event,
                        timestamp: Instant::now(),
                    };
                    
                    if let Err(e) = event_tx.try_send(watcher_event) {
                        warn!("Failed to send watcher event: {}", e);
                    }
                }
                Err(e) => {
                    error!("File watcher error: {}", e);
                }
            }
        })?;

        // Start watching the path
        let watch_mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(&path, watch_mode)?;

        let instance = WatcherInstance {
            id: id.clone(),
            name: format!("Watcher for {}", path.display()),
            path: path.clone(),
            sync_job_id,
            watcher,
            debounce_ms,
            last_event: None,
            sync_tx,
        };

        let mut watchers = self.watchers.write().await;
        watchers.insert(id.clone(), instance);

        info!("Added file watcher '{}' for path: {}", id, path.display());
        Ok(())
    }

    pub async fn remove_watcher(&self, id: &str) -> Result<()> {
        let mut watchers = self.watchers.write().await;
        
        if let Some(_instance) = watchers.remove(id) {
            // The watcher will be automatically dropped and stopped
            info!("Removed file watcher: {}", id);
        } else {
            warn!("File watcher not found: {}", id);
        }
        
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            warn!("File watcher manager is already running");
            return Ok(());
        }

        *running = true;
        info!("Starting file watcher manager");

        // Start the event processing task
        if let Some(event_rx) = self.event_rx.take() {
            let watchers = self.watchers.clone();
            let running_flag = self.running.clone();

            tokio::spawn(async move {
                Self::process_events(event_rx, watchers, running_flag).await;
            });
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        *running = false;
        info!("Stopping file watcher manager");

        // Clear all watchers (they will be dropped and stopped automatically)
        let mut watchers = self.watchers.write().await;
        watchers.clear();

        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn list_watchers(&self) -> Vec<WatcherInfo> {
        let watchers = self.watchers.read().await;
        watchers.values()
            .map(|instance| WatcherInfo {
                id: instance.id.clone(),
                name: instance.name.clone(),
                path: instance.path.clone(),
                sync_job_id: instance.sync_job_id.clone(),
                debounce_ms: instance.debounce_ms,
                last_event: instance.last_event,
            })
            .collect()
    }

    async fn process_events(
        mut event_rx: mpsc::Receiver<WatcherEvent>,
        watchers: Arc<RwLock<HashMap<String, WatcherInstance>>>,
        running: Arc<RwLock<bool>>,
    ) {
        info!("File watcher event processor started");

        while *running.read().await {
            tokio::select! {
                event = event_rx.recv() => {
                    if let Some(event) = event {
                        Self::handle_watcher_event(event, &watchers).await;
                    } else {
                        debug!("File watcher event channel closed");
                        break;
                    }
                }
                _ = sleep(Duration::from_millis(100)) => {
                    // Periodic check to see if we should continue running
                    continue;
                }
            }
        }

        info!("File watcher event processor stopped");
    }

    async fn handle_watcher_event(
        watcher_event: WatcherEvent,
        watchers: &Arc<RwLock<HashMap<String, WatcherInstance>>>,
    ) {
        let mut watchers_write = watchers.write().await;
        
        let instance = match watchers_write.get_mut(&watcher_event.watcher_id) {
            Some(instance) => instance,
            None => {
                warn!("Received event for unknown watcher: {}", watcher_event.watcher_id);
                return;
            }
        };

        // Check debouncing
        if let Some(last_event) = instance.last_event {
            let elapsed = watcher_event.timestamp.duration_since(last_event);
            if elapsed.as_millis() < instance.debounce_ms as u128 {
                debug!(
                    "Ignoring debounced event for watcher '{}' (elapsed: {:?}, debounce: {}ms)",
                    instance.id, elapsed, instance.debounce_ms
                );
                return;
            }
        }

        // Update last event time
        instance.last_event = Some(watcher_event.timestamp);

        // Check if this is an event we care about
        if Self::should_trigger_sync(&watcher_event.event) {
            debug!(
                "File watcher '{}' triggered by event: {:?}",
                instance.id, watcher_event.event.kind
            );

            // Send sync request
            let sync_request = SyncRequest {
                job_id: instance.sync_job_id.clone(),
                triggered_by: TriggerSource::FileWatcher {
                    watcher_id: instance.id.clone(),
                },
                timestamp: tokio::time::Instant::now(),
            };

            if let Err(e) = instance.sync_tx.send(sync_request).await {
                error!("Failed to send sync request from file watcher '{}': {}", instance.id, e);
            }
        } else {
            debug!(
                "File watcher '{}' ignoring event: {:?}",
                instance.id, watcher_event.event.kind
            );
        }
    }

    fn should_trigger_sync(event: &Event) -> bool {
        match &event.kind {
            EventKind::Create(_) => true,
            EventKind::Modify(_) => true,
            EventKind::Remove(_) => true,
            EventKind::Access(_) => false, // Usually don't care about access events
            EventKind::Other => false,
            _ => false,
        }
    }

    pub async fn get_watcher_info(&self, id: &str) -> Option<WatcherInfo> {
        let watchers = self.watchers.read().await;
        watchers.get(id).map(|instance| WatcherInfo {
            id: instance.id.clone(),
            name: instance.name.clone(),
            path: instance.path.clone(),
            sync_job_id: instance.sync_job_id.clone(),
            debounce_ms: instance.debounce_ms,
            last_event: instance.last_event,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WatcherInfo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub sync_job_id: String,
    pub debounce_ms: u64,
    pub last_event: Option<Instant>,
}

impl Default for FileWatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for FileWatcherManager {
    fn drop(&mut self) {
        // Watchers will be automatically dropped and stopped
    }
}

// Utility functions for converting between different event types
impl From<notify::EventKind> for WatchEvent {
    fn from(kind: notify::EventKind) -> Self {
        match kind {
            notify::EventKind::Create(_) => WatchEvent::Create,
            notify::EventKind::Modify(_) => WatchEvent::Write,
            notify::EventKind::Remove(_) => WatchEvent::Remove,
            _ => WatchEvent::Write, // Default fallback
        }
    }
}

impl WatchEvent {
    pub fn matches_kind(&self, kind: &notify::EventKind) -> bool {
        match (self, kind) {
            (WatchEvent::Create, notify::EventKind::Create(_)) => true,
            (WatchEvent::Write, notify::EventKind::Modify(_)) => true,
            (WatchEvent::Remove, notify::EventKind::Remove(_)) => true,
            _ => false,
        }
    }
}
