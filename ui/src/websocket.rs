use crate::types::{Job, ActionLogEntry, AppState};
use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sync_core::SyncConfig;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use url::Url;

/// WebSocket message types from PocketBase
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PocketBaseMessage {
    #[serde(rename = "job_update")]
    JobUpdate { data: Job },
    #[serde(rename = "job_created")]
    JobCreated { data: Job },
    #[serde(rename = "job_deleted")]
    JobDeleted { id: String },
    #[serde(rename = "action_log")]
    ActionLog { data: ActionLogEntry },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

/// Commands that can be sent to the WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketCommand {
    #[serde(rename = "start_job")]
    StartJob { job_id: String },
    #[serde(rename = "pause_job")]
    PauseJob { job_id: String },
    #[serde(rename = "stop_job")]
    StopJob { job_id: String },
    #[serde(rename = "retry_job")]
    RetryJob { job_id: String },
    #[serde(rename = "resolve_conflict")]
    ResolveConflict { 
        job_id: String, 
        conflict_id: String, 
        resolution: String 
    },
    #[serde(rename = "subscribe")]
    Subscribe { collection: String },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { collection: String },
}

/// WebSocket client for communicating with PocketBase
pub struct WebSocketClient {
    sender: mpsc::UnboundedSender<WebSocketCommand>,
    receiver: mpsc::UnboundedReceiver<PocketBaseMessage>,
}

impl WebSocketClient {
    /// Create a new WebSocket client and start the connection
    pub async fn new(config: &SyncConfig) -> Result<Self> {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        // Start the WebSocket connection task
        let ws_url = format!("{}/ws", config.pocketbase_url.replace("http", "ws"));
        tokio::spawn(websocket_task(
            ws_url,
            command_receiver,
            message_sender,
        ));

        Ok(Self {
            sender: command_sender,
            receiver: message_receiver,
        })
    }

    /// Send a command to the WebSocket
    pub fn send_command(&self, command: WebSocketCommand) -> Result<()> {
        self.sender.send(command)
            .map_err(|e| anyhow!("Failed to send WebSocket command: {}", e))
    }

    /// Receive the next message from the WebSocket
    pub async fn recv_message(&mut self) -> Option<PocketBaseMessage> {
        self.receiver.recv().await
    }

    /// Subscribe to job updates
    pub fn subscribe_to_jobs(&self) -> Result<()> {
        self.send_command(WebSocketCommand::Subscribe {
            collection: "jobs".to_string(),
        })
    }

    /// Subscribe to action logs
    pub fn subscribe_to_logs(&self) -> Result<()> {
        self.send_command(WebSocketCommand::Subscribe {
            collection: "action_logs".to_string(),
        })
    }

    /// Start a job
    pub fn start_job(&self, job_id: &str) -> Result<()> {
        self.send_command(WebSocketCommand::StartJob {
            job_id: job_id.to_string(),
        })
    }

    /// Pause a job
    pub fn pause_job(&self, job_id: &str) -> Result<()> {
        self.send_command(WebSocketCommand::PauseJob {
            job_id: job_id.to_string(),
        })
    }

    /// Stop a job
    pub fn stop_job(&self, job_id: &str) -> Result<()> {
        self.send_command(WebSocketCommand::StopJob {
            job_id: job_id.to_string(),
        })
    }

    /// Retry a failed job
    pub fn retry_job(&self, job_id: &str) -> Result<()> {
        self.send_command(WebSocketCommand::RetryJob {
            job_id: job_id.to_string(),
        })
    }

    /// Resolve a conflict
    pub fn resolve_conflict(&self, job_id: &str, conflict_id: &str, resolution: &str) -> Result<()> {
        self.send_command(WebSocketCommand::ResolveConflict {
            job_id: job_id.to_string(),
            conflict_id: conflict_id.to_string(),
            resolution: resolution.to_string(),
        })
    }
}

/// Background task that manages the WebSocket connection
async fn websocket_task(
    url: String,
    mut command_receiver: mpsc::UnboundedReceiver<WebSocketCommand>,
    message_sender: mpsc::UnboundedSender<PocketBaseMessage>,
) {
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY: u64 = 1000; // milliseconds

    loop {
        match connect_and_handle(&url, &mut command_receiver, &message_sender).await {
            Ok(_) => {
                info!("WebSocket connection closed normally");
                break;
            }
            Err(e) => {
                error!("WebSocket connection error: {}", e);
                retry_count += 1;
                
                if retry_count >= MAX_RETRIES {
                    error!("Max WebSocket reconnection attempts reached, giving up");
                    break;
                }

                let delay = RETRY_DELAY * (2_u64.pow(retry_count.min(5)));
                warn!("Retrying WebSocket connection in {}ms (attempt {}/{})", delay, retry_count, MAX_RETRIES);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
        }
    }
}

/// Connect to WebSocket and handle messages
async fn connect_and_handle(
    url: &str,
    command_receiver: &mut mpsc::UnboundedReceiver<WebSocketCommand>,
    message_sender: &mpsc::UnboundedSender<PocketBaseMessage>,
) -> Result<()> {
    let url = Url::parse(url)?;
    info!("Connecting to WebSocket: {}", url);

    let (ws_stream, _) = connect_async(url).await?;
    info!("WebSocket connected successfully");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send initial subscriptions
    let subscribe_jobs = WebSocketCommand::Subscribe {
        collection: "jobs".to_string(),
    };
    let subscribe_logs = WebSocketCommand::Subscribe {
        collection: "action_logs".to_string(),
    };

    if let Ok(msg) = serde_json::to_string(&subscribe_jobs) {
        if let Err(e) = ws_sender.send(Message::Text(msg)).await {
            error!("Failed to send initial subscription: {}", e);
        }
    }

    if let Ok(msg) = serde_json::to_string(&subscribe_logs) {
        if let Err(e) = ws_sender.send(Message::Text(msg)).await {
            error!("Failed to send initial subscription: {}", e);
        }
    }

    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received WebSocket message: {}", text);
                        match serde_json::from_str::<PocketBaseMessage>(&text) {
                            Ok(message) => {
                                if let Err(e) = message_sender.send(message) {
                                    error!("Failed to forward message to application: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse WebSocket message: {} - {}", e, text);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        debug!("Received ping, sending pong");
                        if let Err(e) = ws_sender.send(Message::Pong(payload)).await {
                            error!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed by server");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Handle outgoing commands
            command = command_receiver.recv() => {
                match command {
                    Some(cmd) => {
                        match serde_json::to_string(&cmd) {
                            Ok(msg) => {
                                debug!("Sending WebSocket command: {}", msg);
                                if let Err(e) = ws_sender.send(Message::Text(msg)).await {
                                    error!("Failed to send WebSocket command: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to serialize command: {}", e);
                            }
                        }
                    }
                    None => {
                        info!("Command channel closed, ending WebSocket task");
                        break;
                    }
                }
            }
        }
    }

    // Clean shutdown
    if let Err(e) = ws_sender.send(Message::Close(None)).await {
        debug!("Error closing WebSocket: {}", e);
    }

    Ok(())
}

/// Handle incoming WebSocket messages and update application state
pub fn handle_websocket_message(app_state: &mut AppState, message: PocketBaseMessage) {
    match message {
        PocketBaseMessage::JobUpdate { data: job } => {
            debug!("Updating job: {}", job.id);
            app_state.update_job(job);
        }
        PocketBaseMessage::JobCreated { data: job } => {
            debug!("Adding new job: {}", job.id);
            app_state.add_job(job);
        }
        PocketBaseMessage::JobDeleted { id } => {
            if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
                debug!("Removing job: {}", uuid);
                app_state.jobs.remove(&uuid);
                app_state.action_logs.remove(&uuid);
            }
        }
        PocketBaseMessage::ActionLog { data: log_entry } => {
            debug!("Adding action log entry for job: {}", log_entry.job_id);
            app_state.add_action_log(log_entry);
        }
        PocketBaseMessage::Ping => {
            debug!("Received WebSocket ping");
            // Ping is handled automatically by the WebSocket task
        }
        PocketBaseMessage::Pong => {
            debug!("Received WebSocket pong");
        }
    }
}
