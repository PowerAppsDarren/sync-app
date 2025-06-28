use anyhow::Result;
use clap::{Parser, Subcommand};
use sync_core::{SyncClient, SyncConfig};
use tracing::{info, Level};

#[derive(Parser)]
#[command(name = "sync")]
#[command(about = "A CLI tool for synchronization with PocketBase")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check health of the sync service
    Health {
        /// PocketBase URL
        #[arg(long, default_value = "http://localhost:8090")]
        url: String,
    },
    /// Sync data
    Sync {
        /// Source path
        #[arg(short, long)]
        source: String,
        /// Destination path
        #[arg(short, long)]
        dest: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Health { url } => {
            info!("Checking health of sync service at {}", url);

            let config = SyncConfig {
                pocketbase_url: url,
                ..Default::default()
            };

            let client = SyncClient::new(config);

            match client.health_check().await {
                Ok(true) => {
                    println!("✅ Sync service is healthy");
                    Ok(())
                }
                Ok(false) => {
                    println!("❌ Sync service is not responding correctly");
                    std::process::exit(1);
                }
                Err(e) => {
                    println!("❌ Failed to check health: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Sync { source, dest } => {
            info!("Syncing from {} to {}", source, dest);
            println!("🚧 Sync functionality not yet implemented");
            Ok(())
        }
    }
}
