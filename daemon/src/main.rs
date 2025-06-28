use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod daemon;
mod scheduler;
mod service;
mod telemetry;
mod watcher;

use config::DaemonConfig;
use daemon::SyncDaemon;

#[derive(Parser)]
#[command(name = "sync-daemon")]
#[command(about = "Cross-platform sync daemon service")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    foreground: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon
    Start {
        /// PocketBase URL
        #[arg(long, default_value = "http://localhost:8090")]
        pocketbase_url: String,
        
        /// Admin email
        #[arg(long, default_value = "admin@example.com")]
        admin_email: String,
        
        /// Admin password
        #[arg(long)]
        admin_password: Option<String>,
    },
    /// Stop the daemon
    Stop,
    /// Restart the daemon
    Restart,
    /// Show daemon status
    Status,
    /// Install as system service
    Install {
        /// Service name
        #[arg(long, default_value = "sync-daemon")]
        service_name: String,
        
        /// Service description
        #[arg(long, default_value = "Sync Daemon Service")]
        description: String,
        
        /// Configuration file path for service
        #[arg(long)]
        config_path: Option<PathBuf>,
    },
    /// Uninstall system service
    Uninstall {
        /// Service name
        #[arg(long, default_value = "sync-daemon")]
        service_name: String,
    },
    /// Validate configuration
    Config {
        /// Check configuration validity
        #[command(subcommand)]
        action: ConfigActions,
    },
}

#[derive(Subcommand)]
enum ConfigActions {
    /// Validate configuration file
    Validate,
    /// Show current configuration
    Show,
    /// Generate default configuration
    Generate {
        /// Output path for configuration
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize basic logging (will be replaced by telemetry system)
    init_basic_logging(&cli.log_level)?;
    
    match cli.command {
        Commands::Start { pocketbase_url, admin_email, admin_password } => {
            let config = load_or_create_config(
                cli.config.as_ref(),
                &pocketbase_url,
                &admin_email,
                admin_password.as_deref(),
            ).await?;
            
            if cli.foreground {
                run_daemon(config).await
            } else {
                start_daemon_background(config).await
            }
        }
        Commands::Stop => stop_daemon().await,
        Commands::Restart => restart_daemon().await,
        Commands::Status => show_status().await,
        Commands::Install { service_name, description, config_path } => {
            service::install_service(&service_name, &description, config_path.as_ref()).await
        }
        Commands::Uninstall { service_name } => {
            service::uninstall_service(&service_name).await
        }
        Commands::Config { action } => {
            match action {
                ConfigActions::Validate => validate_config(cli.config.as_ref()).await,
                ConfigActions::Show => show_config(cli.config.as_ref()).await,
                ConfigActions::Generate { output } => generate_config(output.as_ref()).await,
            }
        }
    }
}

fn init_basic_logging(log_level: &str) -> Result<()> {
    // This is a basic fallback logging setup for early initialization
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(format!("sync_daemon={}", level))
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}

async fn load_or_create_config(
    config_path: Option<&PathBuf>,
    pocketbase_url: &str,
    admin_email: &str,
    admin_password: Option<&str>,
) -> Result<DaemonConfig> {
    if let Some(path) = config_path {
        info!("Loading configuration from {}", path.display());
        DaemonConfig::load(path).await
    } else {
        info!("Creating default configuration");
        let mut config = DaemonConfig::default();
        config.pocketbase.url = pocketbase_url.to_string();
        config.pocketbase.admin_email = admin_email.to_string();
        
        if let Some(password) = admin_password {
            config.pocketbase.admin_password = password.to_string();
        } else {
            // Prompt for password if not provided
            let password = rpassword::prompt_password("Admin password: ")?;
            config.pocketbase.admin_password = password;
        }
        
        Ok(config)
    }
}

async fn run_daemon(config: DaemonConfig) -> Result<()> {
    info!("Starting sync daemon in foreground mode");
    let daemon = SyncDaemon::new(config).await?;
    daemon.run().await
}

#[cfg(not(windows))]
async fn start_daemon_background(config: DaemonConfig) -> Result<()> {
    use daemonize::Daemonize;
    use std::fs::File;
    
    info!("Starting sync daemon in background mode");
    
    let stdout = File::create("/tmp/sync-daemon.out")?;
    let stderr = File::create("/tmp/sync-daemon.err")?;
    
    let daemonize = Daemonize::new()
        .pid_file("/tmp/sync-daemon.pid")
        .chown_pid_file(true)
        .working_directory("/tmp")
        .user("nobody")
        .group("daemon")
        .stdout(stdout)
        .stderr(stderr);
    
    match daemonize.start() {
        Ok(_) => {
            let daemon = SyncDaemon::new(config).await?;
            daemon.run().await
        }
        Err(e) => {
            eprintln!("Failed to daemonize: {}", e);
            Err(e.into())
        }
    }
}

#[cfg(windows)]
async fn start_daemon_background(config: DaemonConfig) -> Result<()> {
    info!("Starting sync daemon in background mode");
    // On Windows, we'll run in foreground mode when not installed as service
    run_daemon(config).await
}

async fn stop_daemon() -> Result<()> {
    #[cfg(unix)]
    {
        use std::fs;
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        
        if let Ok(pid_str) = fs::read_to_string("/tmp/sync-daemon.pid") {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                info!("Stopping daemon with PID {}", pid);
                signal::kill(Pid::from_raw(pid), Signal::SIGTERM)?;
                info!("Stop signal sent to daemon");
            } else {
                warn!("Invalid PID in pid file");
            }
        } else {
            warn!("No pid file found, daemon may not be running");
        }
    }
    
    #[cfg(windows)]
    {
        warn!("Stop command not implemented for Windows. Use service manager or Ctrl+C.");
    }
    
    Ok(())
}

async fn restart_daemon() -> Result<()> {
    info!("Restarting daemon");
    stop_daemon().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    // Note: restart would need to reload config and re-execute
    info!("Restart command sent");
    Ok(())
}

async fn show_status() -> Result<()> {
    #[cfg(unix)]
    {
        use std::fs;
        
        if let Ok(pid_str) = fs::read_to_string("/tmp/sync-daemon.pid") {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                use sysinfo::{System, SystemExt, ProcessExt};
                let mut sys = System::new_all();
                sys.refresh_all();
                
                if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
                    println!("Daemon Status: RUNNING");
                    println!("PID: {}", pid);
                    println!("CPU Usage: {:.2}%", process.cpu_usage());
                    println!("Memory Usage: {} KB", process.memory());
                    println!("Start Time: {:?}", process.start_time());
                } else {
                    println!("Daemon Status: NOT RUNNING (stale pid file)");
                }
            }
        } else {
            println!("Daemon Status: NOT RUNNING");
        }
    }
    
    #[cfg(windows)]
    {
        println!("Status checking not fully implemented for Windows");
        println!("Check Windows Services or Task Manager for sync-daemon process");
    }
    
    Ok(())
}

async fn validate_config(config_path: Option<&PathBuf>) -> Result<()> {
    let path = config_path
        .cloned()
        .unwrap_or_else(|| PathBuf::from("sync-daemon.toml"));
    
    info!("Validating configuration at {}", path.display());
    
    match DaemonConfig::load(&path).await {
        Ok(config) => {
            println!("✓ Configuration is valid");
            println!("PocketBase URL: {}", config.pocketbase.url);
            println!("Sync jobs configured: {}", config.sync_jobs.len());
            Ok(())
        }
        Err(e) => {
            println!("✗ Configuration validation failed: {}", e);
            Err(e)
        }
    }
}

async fn show_config(config_path: Option<&PathBuf>) -> Result<()> {
    let path = config_path
        .cloned()
        .unwrap_or_else(|| PathBuf::from("sync-daemon.toml"));
    
    match DaemonConfig::load(&path).await {
        Ok(config) => {
            println!("{}", toml::to_string_pretty(&config)?);
            Ok(())
        }
        Err(e) => {
            println!("Failed to load configuration: {}", e);
            Err(e)
        }
    }
}

async fn generate_config(output_path: Option<&PathBuf>) -> Result<()> {
    let config = DaemonConfig::default();
    let toml_content = toml::to_string_pretty(&config)?;
    
    if let Some(path) = output_path {
        tokio::fs::write(path, toml_content).await?;
        println!("Configuration generated at {}", path.display());
    } else {
        println!("{}", toml_content);
    }
    
    Ok(())
}
