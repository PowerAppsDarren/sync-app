use super::{error::*, types::*};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc, RwLock},
    time::{interval, timeout},
};
use tokio_tungstenite::{
    connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsReceiver = SplitStream<WsStream>;

#[derive(Debug, Clone)]
pub struct RealtimeEvent {
    pub collection: String,
    pub action: RealtimeAction,
    pub record: Option<Record>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct RealtimeSubscription {
    pub id: String,
    pub collection: String,
    pub filter: Option<String>,
    pub receiver: broadcast::Receiver<RealtimeEvent>,
}

impl RealtimeSubscription {
    pub async fn next(&mut self) -> Option<RealtimeEvent> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    warn!("Realtime subscription lagged, some events may have been missed");
                    // Continue the loop to try again
                    continue;
                }
            }
        }
    }
}

pub struct RealtimeClient {
    base_url: String,
    client_id: String,
    auth_token: Arc<RwLock<Option<String>>>,
    
    // Connection management
    connected: Arc<AtomicBool>,
    reconnect_attempts: Arc<AtomicU64>,
    
    // Subscription management
    subscriptions: Arc<RwLock<HashMap<String, broadcast::Sender<RealtimeEvent>>>>,
    
    // Communication channels
    command_sender: Option<mpsc::UnboundedSender<RealtimeCommand>>,
    
    // Background task handle
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug)]
enum RealtimeCommand {
    Subscribe {
        id: String,
        collection: String,
        filter: Option<String>,
        sender: broadcast::Sender<RealtimeEvent>,
    },
    Unsubscribe {
        id: String,
    },
    UpdateAuth {
        token: Option<String>,
    },
    Reconnect,
    Shutdown,
}

impl RealtimeClient {
    pub fn new(base_url: String) -> Self {
        let client_id = Uuid::new_v4().to_string();
        
        Self {
            base_url,
            client_id,
            auth_token: Arc::new(RwLock::new(None)),
            connected: Arc::new(AtomicBool::new(false)),
            reconnect_attempts: Arc::new(AtomicU64::new(0)),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            command_sender: None,
            task_handle: None,
        }
    }
    
    pub async fn connect(&mut self, auth_token: Option<String>) -> Result<()> {
        if self.is_connected() {
            return Ok(());
        }
        
        // Update auth token
        {
            let mut token = self.auth_token.write().await;
            *token = auth_token;
        }
        
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        self.command_sender = Some(command_sender);
        
        let task_handle = self.spawn_connection_task(command_receiver).await?;
        self.task_handle = Some(task_handle);
        
        Ok(())
    }
    
    pub async fn disconnect(&mut self) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(RealtimeCommand::Shutdown);
        }
        
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
        
        self.connected.store(false, Ordering::Relaxed);
        self.command_sender = None;
    }
    
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
    
    pub async fn subscribe(
        &self,
        collection: &str,
        filter: Option<String>,
    ) -> Result<RealtimeSubscription> {
        let subscription_id = Uuid::new_v4().to_string();
        let (sender, receiver) = broadcast::channel(1000);
        
        if let Some(command_sender) = &self.command_sender {
            command_sender
                .send(RealtimeCommand::Subscribe {
                    id: subscription_id.clone(),
                    collection: collection.to_string(),
                    filter: filter.clone(),
                    sender,
                })
                .map_err(|_| PocketBaseError::WebSocket("Connection closed".to_string()))?;
        } else {
            return Err(PocketBaseError::WebSocket("Not connected".to_string()));
        }
        
        Ok(RealtimeSubscription {
            id: subscription_id,
            collection: collection.to_string(),
            filter,
            receiver,
        })
    }
    
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<()> {
        if let Some(command_sender) = &self.command_sender {
            command_sender
                .send(RealtimeCommand::Unsubscribe {
                    id: subscription_id.to_string(),
                })
                .map_err(|_| PocketBaseError::WebSocket("Connection closed".to_string()))?;
        }
        
        Ok(())
    }
    
    pub async fn update_auth_token(&self, token: Option<String>) -> Result<()> {
        {
            let mut auth_token = self.auth_token.write().await;
            *auth_token = token.clone();
        }
        
        if let Some(command_sender) = &self.command_sender {
            command_sender
                .send(RealtimeCommand::UpdateAuth { token })
                .map_err(|_| PocketBaseError::WebSocket("Connection closed".to_string()))?;
        }
        
        Ok(())
    }
    
    async fn spawn_connection_task(
        &self,
        mut command_receiver: mpsc::UnboundedReceiver<RealtimeCommand>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let base_url = self.base_url.clone();
        let client_id = self.client_id.clone();
        let auth_token = self.auth_token.clone();
        let connected = self.connected.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();
        let subscriptions = self.subscriptions.clone();
        
        let handle = tokio::spawn(async move {
            let mut reconnect_interval = interval(Duration::from_secs(5));
            let mut ws_sink: Option<WsSink> = None;
            let mut ws_receiver: Option<WsReceiver> = None;
            
            loop {
                tokio::select! {
                    // Handle commands
                    command = command_receiver.recv() => {
                        match command {
                            Some(RealtimeCommand::Subscribe { id, collection, filter, sender }) => {
                                {
                                    let mut subs = subscriptions.write().await;
                                    subs.insert(id.clone(), sender);
                                }
                                
                                if let Some(sink) = &mut ws_sink {
                                    let subscription_msg = Self::create_subscription_message(
                                        &client_id, &collection, filter.as_deref()
                                    );
                                    if let Err(e) = sink.send(subscription_msg).await {
                                        error!("Failed to send subscription: {}", e);
                                    }
                                }
                            }
                            Some(RealtimeCommand::Unsubscribe { id }) => {
                                let mut subs = subscriptions.write().await;
                                subs.remove(&id);
                            }
                            Some(RealtimeCommand::UpdateAuth { token: _ }) => {
                                // Force reconnection with new auth
                                if ws_sink.is_some() {
                                    ws_sink = None;
                                    ws_receiver = None;
                                    connected.store(false, Ordering::Relaxed);
                                }
                            }
                            Some(RealtimeCommand::Reconnect) => {
                                if ws_sink.is_some() {
                                    ws_sink = None;
                                    ws_receiver = None;
                                    connected.store(false, Ordering::Relaxed);
                                }
                            }
                            Some(RealtimeCommand::Shutdown) => {
                                debug!("Shutting down realtime connection");
                                break;
                            }
                            None => break,
                        }
                    }
                    
                    // Handle WebSocket messages
                    msg = async {
                        if let Some(receiver) = &mut ws_receiver {
                            receiver.next().await
                        } else {
                            futures_util::future::pending().await
                        }
                    } => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Err(e) = Self::handle_message(&text, &subscriptions).await {
                                    warn!("Failed to handle realtime message: {}", e);
                                }
                            }
                            Some(Ok(Message::Close(_))) => {
                                info!("WebSocket connection closed by server");
                                ws_sink = None;
                                ws_receiver = None;
                                connected.store(false, Ordering::Relaxed);
                            }
                            Some(Err(e)) => {
                                error!("WebSocket error: {}", e);
                                ws_sink = None;
                                ws_receiver = None;
                                connected.store(false, Ordering::Relaxed);
                            }
                            _ => {}
                        }
                    }
                    
                    // Reconnection logic
                    _ = reconnect_interval.tick() => {
                        if ws_sink.is_none() && !connected.load(Ordering::Relaxed) {
                            match Self::establish_connection(&base_url, &auth_token).await {
                                Ok((sink, receiver)) => {
                                    info!("WebSocket connection established");
                                    ws_sink = Some(sink);
                                    ws_receiver = Some(receiver);
                                    connected.store(true, Ordering::Relaxed);
                                    reconnect_attempts.store(0, Ordering::Relaxed);
                                    
                                    // Re-subscribe to all active subscriptions
                                    let subs = subscriptions.read().await;
                                    for (id, _) in subs.iter() {
                                        // In a real implementation, you'd need to store
                                        // subscription details to re-subscribe properly
                                        debug!("Would re-subscribe to: {}", id);
                                    }
                                }
                                Err(e) => {
                                    let attempts = reconnect_attempts.fetch_add(1, Ordering::Relaxed);
                                    warn!("Failed to connect to WebSocket (attempt {}): {}", attempts + 1, e);
                                    
                                    // Exponential backoff
                                    if attempts < 10 {
                                        let delay = Duration::from_secs(2_u64.pow(attempts.min(6) as u32));
                                        tokio::time::sleep(delay).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Cleanup
            connected.store(false, Ordering::Relaxed);
            if let Some(mut sink) = ws_sink {
                let _ = sink.close().await;
            }
        });
        
        Ok(handle)
    }
    
    async fn establish_connection(
        base_url: &str,
        _auth_token: &Arc<RwLock<Option<String>>>,
    ) -> Result<(WsSink, WsReceiver)> {
        let ws_url = base_url.replace("http", "ws") + "/api/realtime";
        
        debug!("Connecting to WebSocket: {}", ws_url);
        
        let (ws_stream, _) = timeout(Duration::from_secs(10), connect_async(&ws_url))
            .await
            .map_err(|_| PocketBaseError::WebSocket("Connection timeout".to_string()))?
            .map_err(|e| PocketBaseError::WebSocket(format!("Connection failed: {}", e)))?;
        
        let (sink, receiver) = ws_stream.split();
        
        Ok((sink, receiver))
    }
    
    fn create_subscription_message(
        client_id: &str,
        collection: &str,
        filter: Option<&str>,
    ) -> Message {
        let mut subscription = json!({
            "clientId": client_id,
            "command": "subscribe",
            "data": {
                "channel": format!("collections/{}", collection)
            }
        });
        
        if let Some(filter) = filter {
            subscription["data"]["filter"] = json!(filter);
        }
        
        Message::Text(subscription.to_string())
    }
    
    async fn handle_message(
        text: &str,
        subscriptions: &Arc<RwLock<HashMap<String, broadcast::Sender<RealtimeEvent>>>>,
    ) -> Result<()> {
        let message: serde_json::Value = serde_json::from_str(text)?;
        
        if let Some(action_str) = message.get("action").and_then(|v| v.as_str()) {
            let action = match action_str {
                "create" => RealtimeAction::Create,
                "update" => RealtimeAction::Update,
                "delete" => RealtimeAction::Delete,
                _ => return Ok(()),
            };
            
            if let Some(record_data) = message.get("record") {
                let record: Record = serde_json::from_value(record_data.clone())?;
                
                // Extract collection from the record or message
                let collection = message
                    .get("collection")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                let event = RealtimeEvent {
                    collection,
                    action,
                    record: Some(record),
                    timestamp: chrono::Utc::now(),
                };
                
                // Broadcast to all relevant subscriptions
                let subs = subscriptions.read().await;
                for (_, sender) in subs.iter() {
                    if let Err(e) = sender.send(event.clone()) {
                        debug!("Failed to send event to subscription: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
}
