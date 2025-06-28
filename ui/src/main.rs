mod events;
mod types;
mod ui;
mod websocket;

use crate::events::{handle_key_event, should_quit};
use crate::types::{AppState, Job, JobStatus, JobPriority, ActionLogEntry, LogLevel, Conflict, ConflictType};
use crate::ui::draw;
use crate::websocket::{WebSocketClient, handle_websocket_message};

use anyhow::{Context, Result};
use chrono::Utc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use sync_core::SyncConfig;
use tracing::{error, info, warn, Level};
use tracing_subscriber;
use uuid::Uuid;

/// Main application struct
pub struct App {
    state: AppState,
    ws_client: Option<WebSocketClient>,
    last_tick: Instant,
    tick_rate: Duration,
}

impl App {
    /// Create a new App instance
    pub fn new(tick_rate: Duration) -> Self {
        Self {
            state: AppState::new(),
            ws_client: None,
            last_tick: Instant::now(),
            tick_rate,
        }
    }

    /// Initialize WebSocket connection
    pub async fn init_websocket(&mut self, config: &SyncConfig) -> Result<()> {
        info!("Initializing WebSocket connection to PocketBase...");
        
        match WebSocketClient::new(config).await {
            Ok(client) => {
                self.ws_client = Some(client);
                self.state.websocket_connected = true;
                info!("WebSocket connection established");
            }
            Err(e) => {
                error!("Failed to establish WebSocket connection: {}", e);
                self.state.websocket_connected = false;
                // Continue without WebSocket - allow offline mode
            }
        }

        Ok(())
    }

    /// Add some demo jobs for testing
    pub fn add_demo_jobs(&mut self) {
        let job1 = Job {
            id: Uuid::new_v4(),
            name: "Documents Backup".to_string(),
            description: "Backup documents folder to external drive".to_string(),
            status: JobStatus::Running,
            priority: JobPriority::High,
            progress: 0.65,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            source_path: "/home/user/Documents".to_string(),
            target_path: "/media/backup/Documents".to_string(),
            files_total: 1250,
            files_processed: 812,
            bytes_total: 2_147_483_648, // 2GB
            bytes_processed: 1_395_864_371, // ~1.3GB
            conflicts: vec![],
            errors: vec![],
        };

        let job2 = Job {
            id: Uuid::new_v4(),
            name: "Photos Sync".to_string(),
            description: "Sync photos to cloud storage".to_string(),
            status: JobStatus::Paused,
            priority: JobPriority::Normal,
            progress: 0.25,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            source_path: "/home/user/Pictures".to_string(),
            target_path: "cloud://photos".to_string(),
            files_total: 3420,
            files_processed: 855,
            bytes_total: 15_032_385_536, // ~14GB
            bytes_processed: 3_758_096_384, // ~3.5GB
            conflicts: vec![
                Conflict {
                    id: Uuid::new_v4(),
                    job_id: Uuid::new_v4(),
                    file_path: "/home/user/Pictures/vacation/IMG_001.jpg".to_string(),
                    conflict_type: ConflictType::ModificationTime,
                    source_modified: Utc::now(),
                    target_modified: Utc::now(),
                    source_size: 2_048_576,
                    target_size: 2_097_152,
                    resolution: None,
                    created_at: Utc::now(),
                }
            ],
            errors: vec![],
        };

        let job3 = Job {
            id: Uuid::new_v4(),
            name: "Code Repository Backup".to_string(),
            description: "Backup development projects".to_string(),
            status: JobStatus::Failed,
            priority: JobPriority::Critical,
            progress: 0.15,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            source_path: "/home/user/dev".to_string(),
            target_path: "/backup/dev".to_string(),
            files_total: 5680,
            files_processed: 852,
            bytes_total: 1_073_741_824, // 1GB
            bytes_processed: 161_061_273, // ~150MB
            conflicts: vec![],
            errors: vec![],
        };

        let job4 = Job {
            id: Uuid::new_v4(),
            name: "Music Library".to_string(),
            description: "Sync music collection".to_string(),
            status: JobStatus::Completed,
            priority: JobPriority::Low,
            progress: 1.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            source_path: "/home/user/Music".to_string(),
            target_path: "/media/music".to_string(),
            files_total: 892,
            files_processed: 892,
            bytes_total: 4_294_967_296, // 4GB
            bytes_processed: 4_294_967_296, // 4GB
            conflicts: vec![],
            errors: vec![],
        };

        // Add demo action logs
        let log1 = ActionLogEntry {
            id: Uuid::new_v4(),
            job_id: job1.id,
            action: "sync_file".to_string(),
            message: "Processing file: report.pdf".to_string(),
            timestamp: Utc::now(),
            level: LogLevel::Info,
        };

        let log2 = ActionLogEntry {
            id: Uuid::new_v4(),
            job_id: job2.id,
            action: "pause".to_string(),
            message: "Job paused by user".to_string(),
            timestamp: Utc::now(),
            level: LogLevel::Warning,
        };

        let log3 = ActionLogEntry {
            id: Uuid::new_v4(),
            job_id: job3.id,
            action: "error".to_string(),
            message: "Permission denied: /backup/dev".to_string(),
            timestamp: Utc::now(),
            level: LogLevel::Error,
        };

        self.state.add_job(job1);
        self.state.add_job(job2);
        self.state.add_job(job3);
        self.state.add_job(job4);

        self.state.add_action_log(log1);
        self.state.add_action_log(log2);
        self.state.add_action_log(log3);
    }

    /// Run the main application loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Handle WebSocket messages
            if let Some(ref mut ws_client) = self.ws_client {
                if let Some(message) = ws_client.recv_message().await {
                    handle_websocket_message(&mut self.state, message);
                }
            }

            // Draw UI
            terminal.draw(|f| draw(f, &mut self.state))?;

            // Handle input events with timeout
            let timeout = self.tick_rate.saturating_sub(self.last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        if should_quit(key) {
                            break;
                        }
                        
                        if let Some(ref ws_client) = self.ws_client {
                            if let Err(e) = handle_key_event(&mut self.state, key, ws_client) {
                                error!("Error handling key event: {}", e);
                            }
                        }

                        if self.state.should_quit {
                            break;
                        }
                    }
                    Event::Mouse(_) => {
                        // Mouse events are not handled in this implementation
                    }
                    Event::Resize(_, _) => {
                        // Terminal was resized, no action needed as ratatui handles this
                    }
                    _ => {}
                }
            }

            // Check if it's time for a tick
            if self.last_tick.elapsed() >= self.tick_rate {
                self.last_tick = Instant::now();
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Starting Sync Dashboard TUI...");

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to setup terminal")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Create and run app
    let mut app = App::new(Duration::from_millis(250));
    
    // Load configuration
    let config = SyncConfig::default();
    
    // Try to initialize WebSocket connection
    if let Err(e) = app.init_websocket(&config).await {
        warn!("WebSocket initialization failed, continuing in offline mode: {}", e);
    }

    // Add demo jobs for testing
    app.add_demo_jobs();

    let result = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).context("Failed to restore terminal")?;
    terminal.show_cursor().context("Failed to show cursor")?;

    // Print any error that occurred
    if let Err(e) = result {
        error!("Application error: {}", e);
        return Err(e);
    }

    info!("Sync Dashboard TUI exited cleanly");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new(Duration::from_millis(100));
        assert!(!app.state.should_quit);
        assert!(!app.state.websocket_connected);
        assert!(app.ws_client.is_none());
    }

    #[test]
    fn test_demo_jobs() {
        let mut app = App::new(Duration::from_millis(100));
        app.add_demo_jobs();
        
        assert_eq!(app.state.jobs.len(), 4);
        
        let jobs = app.state.get_jobs_sorted();
        // Should be sorted by priority (Critical first)
        assert!(matches!(jobs[0].priority, JobPriority::Critical));
    }
}
