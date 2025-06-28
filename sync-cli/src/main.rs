use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use sync_core::{SyncClient, SyncConfig};
use tracing::{debug, info, Level};
use uuid::Uuid;

/// Output format options
#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

/// Global CLI options
#[derive(Parser)]
#[command(name = "sync")]
#[command(about = "A CLI tool for synchronization with PocketBase")]
#[command(version)]
struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Quiet output (suppress non-essential messages)
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Output format
    #[arg(long, global = true, default_value = "human")]
    format: OutputFormat,

    /// Config file path
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncConfigEntry {
    pub id: String,
    pub name: String,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub pocketbase_url: String,
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
    pub filters: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub dry_run: bool,
    pub preserve_permissions: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for SyncConfigEntry {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: "default".to_string(),
            source_path: PathBuf::from("."),
            destination_path: PathBuf::from("./backup"),
            pocketbase_url: "http://localhost:8090".to_string(),
            admin_email: None,
            admin_password: None,
            filters: vec![],
            exclude_patterns: vec![".git".to_string(), "node_modules".to_string()],
            dry_run: false,
            preserve_permissions: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Configuration storage
#[derive(Debug, Serialize, Deserialize)]
struct ConfigStorage {
    configs: HashMap<String, SyncConfigEntry>,
    default_config: Option<String>,
}

impl Default for ConfigStorage {
    fn default() -> Self {
        Self {
            configs: HashMap::new(),
            default_config: None,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Run synchronization with a specific config
    Run {
        /// Configuration ID to use
        config_id: String,
        /// Override dry-run setting
        #[arg(long)]
        dry_run: Option<bool>,
        /// Force sync even if conflicts exist
        #[arg(long)]
        force: bool,
    },
    /// List all available sync configurations
    List {
        /// Show detailed information
        #[arg(long)]
        detailed: bool,
    },
    /// Add a new sync configuration
    Add {
        /// Configuration name
        #[arg(long)]
        name: String,
        /// Source directory path
        #[arg(long)]
        source: PathBuf,
        /// Destination directory path
        #[arg(long)]
        dest: PathBuf,
        /// PocketBase URL
        #[arg(long, default_value = "http://localhost:8090")]
        pocketbase_url: String,
        /// Admin email for PocketBase
        #[arg(long)]
        admin_email: Option<String>,
        /// Admin password for PocketBase
        #[arg(long)]
        admin_password: Option<String>,
        /// Save to PocketBase instead of local config
        #[arg(long)]
        remote: bool,
    },
    /// Edit an existing sync configuration
    Edit {
        /// Configuration ID to edit
        config_id: String,
        /// New configuration name
        #[arg(long)]
        name: Option<String>,
        /// New source directory path
        #[arg(long)]
        source: Option<PathBuf>,
        /// New destination directory path
        #[arg(long)]
        dest: Option<PathBuf>,
        /// New PocketBase URL
        #[arg(long)]
        pocketbase_url: Option<String>,
        /// Edit in PocketBase instead of local config
        #[arg(long)]
        remote: bool,
    },
    /// Remove a sync configuration
    Remove {
        /// Configuration ID to remove
        config_id: String,
        /// Remove from PocketBase instead of local config
        #[arg(long)]
        remote: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Perform a dry run to show planned actions
    DryRun {
        /// Configuration ID to use
        config_id: String,
        /// Show file-level details
        #[arg(long)]
        detailed: bool,
    },
    /// Import configurations from JSON/YAML file
    Import {
        /// File path to import from
        file: PathBuf,
        /// Import to PocketBase instead of local storage
        #[arg(long)]
        remote: bool,
        /// Overwrite existing configurations
        #[arg(long)]
        overwrite: bool,
    },
    /// Export configurations to JSON/YAML file
    Export {
        /// File path to export to
        file: PathBuf,
        /// Export format (json or yaml)
        #[arg(long, default_value = "json")]
        export_format: String,
        /// Export from PocketBase instead of local storage
        #[arg(long)]
        remote: bool,
        /// Specific config ID to export (exports all if not specified)
        #[arg(long)]
        config_id: Option<String>,
    },
    /// Check health of the sync service
    Health {
        /// PocketBase URL
        #[arg(long, default_value = "http://localhost:8090")]
        url: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity
    let level = if cli.verbose {
        Level::DEBUG
    } else if cli.quiet {
        Level::ERROR
    } else {
        Level::INFO
    };
    
    tracing_subscriber::fmt().with_max_level(level).init();

    let config_manager = ConfigManager::new(cli.config.clone())?;

    match cli.command {
        Commands::Run { config_id, dry_run, force } => {
            run_sync(config_manager, &config_id, dry_run, force, &cli.format).await
        }
        Commands::List { detailed } => {
            list_configs(config_manager, detailed, &cli.format).await
        }
        Commands::Add {
            name,
            source,
            dest,
            pocketbase_url,
            admin_email,
            admin_password,
            remote,
        } => {
            add_config(
                config_manager,
                name,
                source,
                dest,
                pocketbase_url,
                admin_email,
                admin_password,
                remote,
                &cli.format,
            ).await
        }
        Commands::Edit {
            config_id,
            name,
            source,
            dest,
            pocketbase_url,
            remote,
        } => {
            edit_config(
                config_manager,
                config_id,
                name,
                source,
                dest,
                pocketbase_url,
                remote,
                &cli.format,
            ).await
        }
        Commands::Remove { config_id, remote, yes } => {
            remove_config(config_manager, config_id, remote, yes, &cli.format).await
        }
        Commands::DryRun { config_id, detailed } => {
            dry_run_sync(config_manager, config_id, detailed, &cli.format).await
        }
        Commands::Import { file, remote, overwrite } => {
            import_configs(config_manager, file, remote, overwrite, &cli.format).await
        }
        Commands::Export { file, export_format, remote, config_id } => {
            export_configs(config_manager, file, export_format, remote, config_id, &cli.format).await
        }
        Commands::Health { url } => {
            health_check(url, &cli.format).await
        }
    }
}

/// Configuration manager for handling local and remote configs
struct ConfigManager {
    config_path: PathBuf,
    storage: ConfigStorage,
}

impl ConfigManager {
    fn new(config_path: Option<PathBuf>) -> Result<Self> {
        let config_path = config_path.unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("sync")
                .join("config.json")
        });

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let storage = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            ConfigStorage::default()
        };

        Ok(Self {
            config_path,
            storage,
        })
    }

    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.storage)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    fn add_config(&mut self, config: SyncConfigEntry) -> Result<()> {
        self.storage.configs.insert(config.id.clone(), config);
        self.save()
    }

    fn get_config(&self, id: &str) -> Option<&SyncConfigEntry> {
        self.storage.configs.get(id)
    }

    fn update_config(&mut self, id: &str, mut config: SyncConfigEntry) -> Result<()> {
        config.updated_at = Utc::now();
        self.storage.configs.insert(id.to_string(), config);
        self.save()
    }

    fn remove_config(&mut self, id: &str) -> Result<bool> {
        let removed = self.storage.configs.remove(id).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    fn list_configs(&self) -> Vec<&SyncConfigEntry> {
        self.storage.configs.values().collect()
    }

    fn import_configs(&mut self, configs: Vec<SyncConfigEntry>, overwrite: bool) -> Result<usize> {
        let mut imported = 0;
        for config in configs {
            if !self.storage.configs.contains_key(&config.id) || overwrite {
                self.storage.configs.insert(config.id.clone(), config);
                imported += 1;
            }
        }
        if imported > 0 {
            self.save()?;
        }
        Ok(imported)
    }

    fn export_configs(&self, config_id: Option<&str>) -> Vec<&SyncConfigEntry> {
        match config_id {
            Some(id) => self.storage.configs.get(id).into_iter().collect(),
            None => self.storage.configs.values().collect(),
        }
    }
}

/// Output helper functions
fn output_json<T: Serialize>(data: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

fn output_success(message: &str, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => println!("✅ {}", message),
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "success",
                "message": message
            });
            output_json(&result)?
        }
    }
    Ok(())
}

fn output_error(message: &str, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => eprintln!("❌ {}", message),
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "error",
                "message": message
            });
            output_json(&result)?
        }
    }
    Ok(())
}

/// Command handler functions
async fn run_sync(
    config_manager: ConfigManager,
    config_id: &str,
    dry_run_override: Option<bool>,
    force: bool,
    format: &OutputFormat,
) -> Result<()> {
    let config = config_manager
        .get_config(config_id)
        .ok_or_else(|| anyhow!("Configuration '{}' not found", config_id))?;

    let is_dry_run = dry_run_override.unwrap_or(config.dry_run);

    if is_dry_run {
        info!("Running dry-run sync for config: {}", config.name);
        return dry_run_sync(config_manager, config_id.to_string(), false, format).await;
    }

    info!("Running sync for config: {}", config.name);
    debug!("Source: {:?}", config.source_path);
    debug!("Destination: {:?}", config.destination_path);
    debug!("Force: {}", force);

    // TODO: Implement actual sync logic using sync-core
    match format {
        OutputFormat::Human => {
            println!("🚧 Sync functionality is not yet fully implemented");
            println!("Would sync from {:?} to {:?}", config.source_path, config.destination_path);
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "not_implemented",
                "message": "Sync functionality is not yet fully implemented",
                "config_id": config_id,
                "source": config.source_path,
                "destination": config.destination_path,
                "dry_run": is_dry_run,
                "force": force
            });
            output_json(&result)?;
        }
    }

    Ok(())
}

async fn list_configs(
    config_manager: ConfigManager,
    detailed: bool,
    format: &OutputFormat,
) -> Result<()> {
    let configs = config_manager.list_configs();

    match format {
        OutputFormat::Human => {
            if configs.is_empty() {
                println!("No configurations found.");
                return Ok(());
            }

            println!("Available sync configurations:");
            for config in configs {
                if detailed {
                    println!(
                        "\n🔧 {} ({})",
                        config.name,
                        config.id
                    );
                    println!("   Source: {:?}", config.source_path);
                    println!("   Destination: {:?}", config.destination_path);
                    println!("   PocketBase: {}", config.pocketbase_url);
                    println!("   Created: {}", config.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    println!("   Updated: {}", config.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
                } else {
                    println!("  🔧 {} ({})", config.name, config.id);
                }
            }
        }
        OutputFormat::Json => {
            let result = if detailed {
                serde_json::json!({
                    "configs": configs,
                    "count": configs.len()
                })
            } else {
                let simple_configs: Vec<_> = configs
                    .iter()
                    .map(|c| serde_json::json!({
                        "id": c.id,
                        "name": c.name
                    }))
                    .collect();
                serde_json::json!({
                    "configs": simple_configs,
                    "count": configs.len()
                })
            };
            output_json(&result)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add_config(
    mut config_manager: ConfigManager,
    name: String,
    source: PathBuf,
    dest: PathBuf,
    pocketbase_url: String,
    admin_email: Option<String>,
    admin_password: Option<String>,
    remote: bool,
    format: &OutputFormat,
) -> Result<()> {
    let config = SyncConfigEntry {
        name,
        source_path: source,
        destination_path: dest,
        pocketbase_url,
        admin_email,
        admin_password,
        ..Default::default()
    };

    if remote {
        // TODO: Implement PocketBase storage
        return Err(anyhow!("Remote configuration storage not yet implemented"));
    }

    config_manager.add_config(config.clone())?;
    output_success(&format!("Added configuration '{}' with ID: {}", config.name, config.id), format)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn edit_config(
    mut config_manager: ConfigManager,
    config_id: String,
    name: Option<String>,
    source: Option<PathBuf>,
    dest: Option<PathBuf>,
    pocketbase_url: Option<String>,
    remote: bool,
    format: &OutputFormat,
) -> Result<()> {
    if remote {
        return Err(anyhow!("Remote configuration editing not yet implemented"));
    }

    let mut config = config_manager
        .get_config(&config_id)
        .ok_or_else(|| anyhow!("Configuration '{}' not found", config_id))?
        .clone();

    // Update fields if provided
    if let Some(n) = name {
        config.name = n;
    }
    if let Some(s) = source {
        config.source_path = s;
    }
    if let Some(d) = dest {
        config.destination_path = d;
    }
    if let Some(url) = pocketbase_url {
        config.pocketbase_url = url;
    }

    config_manager.update_config(&config_id, config)?;
    output_success(&format!("Updated configuration '{}'.", config_id), format)?;

    Ok(())
}

async fn remove_config(
    mut config_manager: ConfigManager,
    config_id: String,
    remote: bool,
    yes: bool,
    format: &OutputFormat,
) -> Result<()> {
    if remote {
        return Err(anyhow!("Remote configuration removal not yet implemented"));
    }

    let config = config_manager
        .get_config(&config_id)
        .ok_or_else(|| anyhow!("Configuration '{}' not found", config_id))?;

    if !yes {
        match format {
            OutputFormat::Human => {
                println!("Are you sure you want to remove configuration '{}' ({})? [y/N]", config.name, config_id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Cancelled.");
                    return Ok(());
                }
            }
            OutputFormat::Json => {
                return Err(anyhow!("Interactive confirmation not supported in JSON mode. Use --yes flag."));
            }
        }
    }

    if config_manager.remove_config(&config_id)? {
        output_success(&format!("Removed configuration '{}'.", config_id), format)?;
    } else {
        output_error(&format!("Configuration '{}' not found.", config_id), format)?;
    }

    Ok(())
}

async fn dry_run_sync(
    config_manager: ConfigManager,
    config_id: String,
    detailed: bool,
    format: &OutputFormat,
) -> Result<()> {
    let config = config_manager
        .get_config(&config_id)
        .ok_or_else(|| anyhow!("Configuration '{}' not found", config_id))?;

    // TODO: Implement actual dry-run logic
    match format {
        OutputFormat::Human => {
            println!("🔍 Dry run for configuration: {}", config.name);
            println!("   Source: {:?}", config.source_path);
            println!("   Destination: {:?}", config.destination_path);
            
            if detailed {
                println!("\n📋 Planned actions:");
                println!("   • Create directory structure");
                println!("   • Copy new files: 0");
                println!("   • Update existing files: 0");
                println!("   • Delete obsolete files: 0");
                println!("\n🚧 Dry-run analysis not yet fully implemented");
            } else {
                println!("🚧 Dry-run analysis not yet fully implemented");
            }
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "not_implemented",
                "message": "Dry-run analysis not yet fully implemented",
                "config_id": config_id,
                "config_name": config.name,
                "source": config.source_path,
                "destination": config.destination_path,
                "detailed": detailed,
                "planned_actions": {
                    "create_directories": 0,
                    "copy_files": 0,
                    "update_files": 0,
                    "delete_files": 0
                }
            });
            output_json(&result)?;
        }
    }

    Ok(())
}

async fn import_configs(
    mut config_manager: ConfigManager,
    file: PathBuf,
    remote: bool,
    overwrite: bool,
    format: &OutputFormat,
) -> Result<()> {
    if remote {
        return Err(anyhow!("Remote configuration import not yet implemented"));
    }

    let content = fs::read_to_string(&file)?;
    let configs: Vec<SyncConfigEntry> = if file.extension().and_then(|s| s.to_str()) == Some("yaml") || 
                                               file.extension().and_then(|s| s.to_str()) == Some("yml") {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    let imported = config_manager.import_configs(configs, overwrite)?;
    output_success(&format!("Imported {} configuration(s) from {:?}.", imported, file), format)?;

    Ok(())
}

async fn export_configs(
    config_manager: ConfigManager,
    file: PathBuf,
    export_format: String,
    remote: bool,
    config_id: Option<String>,
    format: &OutputFormat,
) -> Result<()> {
    if remote {
        return Err(anyhow!("Remote configuration export not yet implemented"));
    }

    let configs = config_manager.export_configs(config_id.as_deref());
    
    if configs.is_empty() {
        return Err(anyhow!("No configurations found to export"));
    }

    let content = match export_format.as_str() {
        "yaml" | "yml" => serde_yaml::to_string(&configs)?,
        "json" => serde_json::to_string_pretty(&configs)?,
        _ => return Err(anyhow!("Unsupported export format: {}. Use 'json' or 'yaml'.", export_format)),
    };

    // Ensure directory exists
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&file, content)?;
    output_success(&format!("Exported {} configuration(s) to {:?}.", configs.len(), file), format)?;

    Ok(())
}

async fn health_check(url: String, format: &OutputFormat) -> Result<()> {
    info!("Checking health of sync service at {}", url);

    let config = SyncConfig {
        pocketbase_url: url.clone(),
        ..Default::default()
    };

    let client = SyncClient::new(config);

    match client.health_check().await {
        Ok(true) => {
            match format {
                OutputFormat::Human => println!("✅ Sync service is healthy at {}", url),
                OutputFormat::Json => {
                    let result = serde_json::json!({
                        "status": "healthy",
                        "url": url,
                        "message": "Sync service is responding correctly"
                    });
                    output_json(&result)?;
                }
            }
            Ok(())
        }
        Ok(false) => {
            let message = format!("Sync service is not responding correctly at {}", url);
            output_error(&message, format)?;
            std::process::exit(1);
        }
        Err(e) => {
            let message = format!("Failed to check health at {}: {}", url, e);
            output_error(&message, format)?;
            std::process::exit(1);
        }
    }
}
