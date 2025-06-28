# Quick Start Guide

Get Sync App up and running in minutes! This guide covers local development setup and your first synchronization.

## Prerequisites

- **Rust**: Version 1.70 or later
- **Git**: For cloning the repository
- **Operating System**: Windows 10+, macOS 10.15+, or Linux (Ubuntu 20.04+)

## Installation

### Option 1: From Source (Recommended for Development)

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/sync-app.git
   cd sync-app
   ```

2. **Build the project**
   ```bash
   # Build in release mode for best performance
   cargo build --release
   ```

3. **Verify installation**
   ```bash
   ./target/release/sync --version
   ./target/release/daemon --version
   ```

### Option 2: Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/yourusername/sync-app/releases).

## Setting Up PocketBase Backend

Sync App uses PocketBase as its backend database and API server.

### Automatic Setup (Recommended)

1. **Run the setup script**
   ```bash
   # Windows
   .\pocketbase\setup.ps1
   
   # Linux/macOS
   ./pocketbase/setup.sh
   ```

2. **Verify PocketBase is running**
   ```bash
   curl http://localhost:8090/api/health
   ```
   You should see: `{"code":200,"message":"API is healthy","data":{}}`

### Manual Setup

1. **Download PocketBase** (if not using setup script)
   ```bash
   # Linux/macOS
   cd pocketbase
   wget https://github.com/pocketbase/pocketbase/releases/download/v0.22.0/pocketbase_0.22.0_linux_amd64.zip
   unzip pocketbase_0.22.0_linux_amd64.zip
   
   # Windows (PowerShell)
   cd pocketbase
   Invoke-WebRequest -Uri "https://github.com/pocketbase/pocketbase/releases/download/v0.22.0/pocketbase_0.22.0_windows_amd64.zip" -OutFile "pocketbase.zip"
   Expand-Archive pocketbase.zip
   ```

2. **Start PocketBase**
   ```bash
   # Linux/macOS
   ./pocketbase/pocketbase serve --http=0.0.0.0:8090
   
   # Windows
   .\pocketbase\pocketbase.exe serve --http=0.0.0.0:8090
   ```

## Your First Sync

### Step 1: Create Test Directories

```bash
# Create source and destination directories
mkdir test-source test-backup

# Add some test files
echo "Hello, World!" > test-source/hello.txt
echo "This is a test file" > test-source/test.md
mkdir test-source/subfolder
echo "Nested file" > test-source/subfolder/nested.txt
```

### Step 2: Add a Sync Configuration

```bash
# Add a new sync configuration
./target/release/sync add \
  --name "test-sync" \
  --source "./test-source" \
  --dest "./test-backup"
```

The command will output a configuration ID that you'll use in the next steps.

### Step 3: Preview the Sync (Dry Run)

```bash
# Replace <config-id> with the ID from the previous step
./target/release/sync dry-run <config-id> --detailed
```

This shows you what files would be copied without actually performing the sync.

### Step 4: Run the Sync

```bash
# Perform the actual sync
./target/release/sync run <config-id>
```

### Step 5: Verify the Results

```bash
# Check that files were copied
ls -la test-backup/
cat test-backup/hello.txt
```

## Using the Web UI

Launch the web interface to manage your sync configurations visually:

```bash
# Start the UI server
./target/release/ui

# Open your browser to http://localhost:3000
```

The web UI provides:
- Visual configuration management
- Real-time sync monitoring
- Log viewing
- Performance metrics

## Setting Up the Daemon

The daemon allows for automated, scheduled synchronization and file watching.

### Step 1: Create a Configuration File

```bash
# Copy the example configuration
cp daemon/examples/daemon-config.toml my-daemon-config.toml
```

### Step 2: Edit the Configuration

Edit `my-daemon-config.toml` to match your setup:

```toml
[pocketbase]
url = "http://localhost:8090"
admin_email = "admin@example.com"
admin_password = "admin123456"

[daemon]
log_level = "info"

[[sync_jobs]]
id = "my_first_job"
name = "My First Sync Job"
source_path = "./test-source"
destination_path = "./test-backup"
enabled = true

[sync_jobs.schedule]
type = "interval"
interval = "5m"  # Sync every 5 minutes
```

### Step 3: Start the Daemon

```bash
# Start the daemon with your configuration
./target/release/daemon start --config my-daemon-config.toml
```

### Step 4: Monitor the Daemon

```bash
# Check daemon status
./target/release/daemon status

# View logs
tail -f logs/daemon.log

# Check metrics
curl http://localhost:9090/metrics
```

## Example Scenarios

### One-Way Mirror

Create a one-way sync that mirrors source to destination:

```bash
./target/release/sync add \
  --name "backup-documents" \
  --source "~/Documents" \
  --dest "~/backup/documents"

./target/release/sync run <config-id>
```

### Bidirectional Sync with Conflict Resolution

For bidirectional syncing, use the daemon with a more advanced configuration:

```toml
[[sync_jobs]]
id = "bidirectional_sync"
name = "Bidirectional Documents Sync"
source_path = "~/Documents"
destination_path = "~/Dropbox/Documents"
enabled = true

[sync_jobs.sync_options]
bidirectional = true
conflict_resolution = "newer"  # Use newer file in conflicts
preserve_timestamps = true
```

### File Watcher Demo

Set up real-time file watching:

```toml
[[file_watchers]]
id = "docs_watcher"
name = "Documents Watcher"
watch_path = "~/Documents"
sync_job_id = "bidirectional_sync"
enabled = true
recursive = true
debounce_ms = 1000
watch_events = ["create", "write", "remove"]
```

## Next Steps

- **[Production Deployment](deployment.md)**: Learn how to deploy in production
- **[Configuration Reference](configuration.md)**: Explore all configuration options
- **[CLI Usage](../CLI_USAGE.md)**: Master the command-line interface
- **[Troubleshooting](troubleshooting.md)**: Solutions for common issues

## Getting Help

- **Documentation**: This guide and the [full documentation](https://yourusername.github.io/sync-app/)
- **Examples**: Check out more examples in the [examples directory](examples/)
- **Issues**: Report bugs or request features on [GitHub Issues](https://github.com/yourusername/sync-app/issues)
- **Discussions**: Join the community on [GitHub Discussions](https://github.com/yourusername/sync-app/discussions)

## Common Quick Commands

```bash
# List all configurations
./target/release/sync list --detailed

# Health check
./target/release/sync health

# Export configurations for backup
./target/release/sync export my-configs.json

# Import configurations
./target/release/sync import my-configs.json

# View daemon logs
tail -f logs/daemon.log

# Stop daemon
./target/release/daemon stop
```

Congratulations! You now have Sync App running locally. Explore the other documentation sections to learn about advanced features and production deployment.
