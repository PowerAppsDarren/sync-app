use anyhow::Result;
use sync_core::{SyncClient, SyncConfig};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting sync server...");

    let config = SyncConfig::default();
    let client = SyncClient::new(config);

    // Check if PocketBase is available
    match client.health_check().await {
        Ok(true) => info!("PocketBase is healthy and accessible"),
        Ok(false) => {
            tracing::warn!("PocketBase health check failed");
        }
        Err(e) => {
            tracing::error!("Failed to connect to PocketBase: {}", e);
        }
    }

    info!("🚧 Server functionality not yet implemented");

    // Keep the server running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down sync server...");

    Ok(())
}
