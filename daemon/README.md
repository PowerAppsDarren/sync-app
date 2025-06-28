# Sync Daemon

A cross-platform background service for automated file synchronization with PocketBase backend support.

## Features

### 🔍 Telemetry & Monitoring (**NEW**)

- **Structured Logging**: JSON and pretty console formats using `tracing`
- **Log Persistence**: Automatic upload to PocketBase with local file rotation  
- **Prometheus Metrics**: 16 different metrics covering operations, health, and performance
- **Real-time Monitoring**: HTTP endpoint on port 9090 for metrics collection
- **Operational Insights**: Memory usage, CPU usage, uptime, error rates, sync performance

*See [Telemetry Documentation](../docs/TELEMETRY.md) for complete details.*

- **Cross-Platform Service Support**:
  - Windows: Native Windows Service or NSSM integration
  - macOS: launchd daemon integration  
  - Linux: systemd service integration

- **Multiple Scheduling Options**:
  - Interval-based scheduling (e.g., every 5 minutes)
  - Cron expression support for complex schedules
  - Manual triggering

- **File System Monitoring**:
  - Real-time file change detection using `notify` crate
  - Configurable debouncing to prevent excessive triggering
  - Recursive directory watching

- **Concurrency Management**:
  - Configurable limits on concurrent sync operations
  - Queue-based sync request processing
  - Semaphore-based resource control

- **Configuration Management**:
  - TOML-based configuration files
  - Dynamic configuration reloading from PocketBase
  - Local configuration caching

## Installation

### As a Standalone Binary

```bash
# Build the daemon
cargo build --release -p sync-daemon

# Run directly
./target/release/sync-daemon start --pocketbase-url http://localhost:8090
```

### As a System Service

#### Windows
```cmd
# Install as Windows Service
sync-daemon install --service-name "SyncDaemon" --description "File Sync Daemon"

# Or manually with NSSM (if sc command fails)
# Download NSSM from https://nssm.cc/download
# Then run the provided commands
```

#### Linux (systemd)
```bash
# Install as systemd service
sudo sync-daemon install --service-name "sync-daemon" --description "File Sync Daemon"

# Check status
sudo systemctl status sync-daemon

# View logs
sudo journalctl -u sync-daemon -f
```

#### macOS (launchd)
```bash
# Install as launchd daemon
sudo sync-daemon install --service-name "sync-daemon" --description "File Sync Daemon"

# Check status
sudo launchctl list | grep sync-daemon

# View logs
tail -f /var/log/sync-daemon.out
```

## Configuration

The daemon uses a TOML configuration file. Generate a default configuration:

```bash
sync-daemon config generate --output sync-daemon.toml
```

### Example Configuration

```toml
[pocketbase]
url = "http://localhost:8090"
admin_email = "admin@example.com"
admin_password = "admin123456"
timeout_secs = 30
retry_attempts = 3
retry_delay_secs = 5

[daemon]
pid_file = "/var/run/sync-daemon.pid"
log_file = "/var/log/sync-daemon.log"
log_level = "info"
config_refresh_interval_secs = 300
auto_restart_on_config_change = false

[[sync_jobs]]
id = "documents"
name = "Documents Sync"
source_path = "/home/user/Documents"
destination_path = "/backup/Documents"
enabled = true
filters = ["*.tmp", "*.log", ".git/*"]

[sync_jobs.schedule]
type = "interval"
interval = "5m"

[sync_jobs.sync_options]
dry_run = false
preserve_permissions = true
preserve_timestamps = true
delete_destination_files = false
comparison_method = "checksum"
ignore_hidden_files = false

[[file_watchers]]
id = "documents-watcher"
name = "Documents File Watcher"
watch_path = "/home/user/Documents"
sync_job_id = "documents"
enabled = true
recursive = true
debounce_ms = 1000
watch_events = ["create", "write", "remove", "rename"]

[concurrency]
max_concurrent_syncs = 4
max_file_operations = 100
sync_queue_size = 1000

[cache]
cache_dir = "/var/cache/sync-daemon"
config_cache_ttl_secs = 300
file_metadata_cache_ttl_secs = 60
enable_persistent_cache = false
```

## CLI Commands

### Basic Usage

```bash
# Start daemon in foreground
sync-daemon start --foreground

# Start with custom config
sync-daemon start --config /path/to/config.toml

# Check daemon status
sync-daemon status

# Stop daemon
sync-daemon stop

# Restart daemon
sync-daemon restart
```

### Configuration Management

```bash
# Validate configuration
sync-daemon config validate

# Show current configuration
sync-daemon config show

# Generate default configuration
sync-daemon config generate --output config.toml
```

### Service Management

```bash
# Install as system service
sync-daemon install --service-name "my-sync" --description "My Sync Service"

# Uninstall service
sync-daemon uninstall --service-name "my-sync"
```

## Schedule Types

### Interval Scheduling
```toml
[sync_jobs.schedule]
type = "interval"
interval = "5m"  # Human-readable duration: 1s, 30s, 5m, 1h, etc.
```

### Cron Scheduling
```toml
[sync_jobs.schedule]
type = "cron"
expression = "0 */15 * * * *"  # Every 15 minutes
```

Common cron expressions:
- `0 0 * * * *` - Every hour
- `0 0 0 * * *` - Daily at midnight
- `0 0 2 * * MON` - Every Monday at 2 AM
- `0 */30 * * * *` - Every 30 minutes

### Manual Scheduling
```toml
[sync_jobs.schedule]
type = "manual"
```

## File Watching

The daemon can monitor directories for changes and trigger sync operations automatically:

- **Recursive watching**: Monitor subdirectories
- **Event filtering**: Choose which events trigger syncs (create, write, remove, rename, chmod)
- **Debouncing**: Prevent rapid-fire triggering from multiple file changes
- **Cross-platform**: Uses platform-optimal file watching mechanisms

## Logging

The daemon provides comprehensive logging:

- **Structured logging**: JSON-formatted logs with tracing crate
- **Log levels**: trace, debug, info, warn, error
- **Platform integration**: 
  - Windows: Event Log integration available
  - Linux: systemd journal integration
  - macOS: system log integration

## Integration with PocketBase

The daemon can:
- Load configuration from PocketBase collections
- Store sync job definitions and schedules
- Cache configuration locally for offline operation
- Automatically reload configuration changes

## Security Considerations

- Run with minimal required privileges
- Secure configuration file permissions
- Consider using dedicated service user accounts
- Regular password rotation for PocketBase admin accounts

## Troubleshooting

### Common Issues

1. **Permission Denied**: Ensure the daemon has read/write access to source and destination paths
2. **Service Won't Start**: Check logs for configuration errors
3. **High CPU Usage**: Adjust file watcher debouncing and concurrency limits
4. **PocketBase Connection**: Verify network connectivity and credentials

### Debug Mode

Run with debug logging:
```bash
sync-daemon start --foreground --log-level debug
```

### View Service Logs

**Windows (Event Viewer)**:
- Look for events from source "sync-daemon"

**Linux (systemd)**:
```bash
sudo journalctl -u sync-daemon -f
```

**macOS (Console.app)**:
- Filter for "sync-daemon" process

## Building from Source

```bash
# Clone repository
git clone <repository-url>
cd sync-app

# Build daemon
cargo build --release -p sync-daemon

# Run tests
cargo test -p sync-daemon
```

## License

This project is licensed under the AGPL-3.0 License - see the LICENSE file for details.
