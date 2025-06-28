# Configuration Reference

This document provides a comprehensive reference for all configuration options in Sync App, including daemon, CLI, and PocketBase configurations.

## Configuration File Format

Sync App uses TOML format for configuration files. The main configuration file is typically named `daemon.toml` and contains multiple sections for different components.

## Daemon Configuration

### Complete Example

```toml
# daemon.toml - Complete configuration example

[pocketbase]
url = "http://localhost:8090"
admin_email = "admin@example.com"
admin_password = "admin123456"
timeout_secs = 30
retry_attempts = 3
retry_delay_secs = 5
connection_pool_size = 10
max_idle_connections = 5

[daemon]
pid_file = "/var/run/sync-daemon.pid"
log_file = "/var/log/sync-daemon.log"
log_level = "info"
config_refresh_interval_secs = 300
auto_restart_on_config_change = false
shutdown_timeout_secs = 30
health_check_interval_secs = 60

[telemetry]
log_level = "info"
json_logging = true
console_logging = true
log_file_path = "logs/daemon.log"

[telemetry.log_rotation]
enabled = true
frequency = "daily"
keep_files = 7
max_size_mb = 100
compress_rotated = true

[telemetry.pocketbase_logging]
enabled = true
collection = "daemon_logs"
batch_size = 100
flush_interval_secs = 30
max_retries = 3
buffer_size = 1000

[telemetry.metrics]
enabled = true
bind_address = "127.0.0.1"
port = 9090
collection_interval_secs = 15
endpoint_path = "/metrics"

[concurrency]
max_concurrent_syncs = 4
max_file_operations = 100
sync_queue_size = 1000
thread_pool_size = 8
io_thread_pool_size = 4

[cache]
cache_dir = "~/.cache/sync-daemon"
config_cache_ttl_secs = 300
file_metadata_cache_ttl_secs = 60
enable_persistent_cache = true
max_cache_size_mb = 500
cache_cleanup_interval_secs = 3600

[[sync_jobs]]
id = "documents_sync"
name = "Documents Synchronization"
source_path = "~/Documents"
destination_path = "/backup/documents"
enabled = true
filters = ["*.tmp", "*.log", ".DS_Store"]
exclude_patterns = [".git", "node_modules", "target"]

[sync_jobs.schedule]
type = "interval"
interval = "5m"

[sync_jobs.sync_options]
dry_run = false
preserve_permissions = true
preserve_timestamps = true
delete_destination_files = false
comparison_method = "sha256"
ignore_hidden_files = true
continue_on_error = true
bidirectional = false
conflict_resolution = "newer"
max_file_size_mb = 1000
follow_symlinks = false

[[file_watchers]]
id = "documents_watcher"
name = "Documents File Watcher"
watch_path = "~/Documents"
sync_job_id = "documents_sync"
enabled = true
recursive = true
debounce_ms = 1000
watch_events = ["create", "write", "remove"]
ignore_patterns = [".tmp", ".lock"]
```

## Configuration Sections

### [pocketbase]

PocketBase backend connection settings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | string | `"http://localhost:8090"` | PocketBase server URL |
| `admin_email` | string | *required* | Admin email for authentication |
| `admin_password` | string | *required* | Admin password for authentication |
| `timeout_secs` | integer | `30` | Request timeout in seconds |
| `retry_attempts` | integer | `3` | Number of retry attempts on failure |
| `retry_delay_secs` | integer | `5` | Delay between retry attempts |
| `connection_pool_size` | integer | `10` | HTTP connection pool size |
| `max_idle_connections` | integer | `5` | Maximum idle connections |
| `use_tls` | boolean | `false` | Enable TLS/SSL |
| `verify_certificates` | boolean | `true` | Verify SSL certificates |
| `api_timeout_secs` | integer | `60` | API operation timeout |

### [daemon]

Daemon process configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pid_file` | string | `"/var/run/sync-daemon.pid"` | PID file location |
| `log_file` | string | `"/var/log/sync-daemon.log"` | Log file location |
| `log_level` | string | `"info"` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `config_refresh_interval_secs` | integer | `300` | Configuration reload interval |
| `auto_restart_on_config_change` | boolean | `false` | Auto-restart on config changes |
| `shutdown_timeout_secs` | integer | `30` | Graceful shutdown timeout |
| `health_check_interval_secs` | integer | `60` | Internal health check interval |
| `run_as_user` | string | *optional* | User to run daemon as |
| `run_as_group` | string | *optional* | Group to run daemon as |
| `working_directory` | string | *current dir* | Daemon working directory |

### [telemetry]

Telemetry and logging configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `log_level` | string | `"info"` | Telemetry log level |
| `json_logging` | boolean | `false` | Enable JSON log format |
| `console_logging` | boolean | `true` | Enable console output |
| `log_file_path` | string | *optional* | Custom log file path |
| `enable_tracing` | boolean | `false` | Enable distributed tracing |
| `trace_sample_rate` | float | `0.1` | Tracing sample rate (0.0-1.0) |

#### [telemetry.log_rotation]

Log rotation settings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable log rotation |
| `frequency` | string | `"daily"` | Rotation frequency (`hourly`, `daily`, `weekly`) |
| `keep_files` | integer | `7` | Number of files to keep |
| `max_size_mb` | integer | `100` | Max file size before rotation |
| `compress_rotated` | boolean | `true` | Compress rotated files |
| `rotation_time` | string | `"00:00"` | Time of day for rotation (HH:MM) |

#### [telemetry.pocketbase_logging]

PocketBase log upload configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable PocketBase logging |
| `collection` | string | `"daemon_logs"` | PocketBase collection name |
| `batch_size` | integer | `100` | Log batch size |
| `flush_interval_secs` | integer | `30` | Flush interval |
| `max_retries` | integer | `3` | Retry attempts |
| `buffer_size` | integer | `1000` | Log buffer size |
| `include_sensitive_data` | boolean | `false` | Include sensitive fields |

#### [telemetry.metrics]

Prometheus metrics configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable metrics endpoint |
| `bind_address` | string | `"127.0.0.1"` | Metrics server bind address |
| `port` | integer | `9090` | Metrics server port |
| `collection_interval_secs` | integer | `15` | Metrics collection interval |
| `endpoint_path` | string | `"/metrics"` | Metrics endpoint path |
| `enable_process_metrics` | boolean | `true` | Include process metrics |
| `histogram_buckets` | array | *default buckets* | Custom histogram buckets |

### [concurrency]

Concurrency and performance settings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_concurrent_syncs` | integer | `4` | Maximum concurrent sync operations |
| `max_file_operations` | integer | `100` | Max concurrent file operations |
| `sync_queue_size` | integer | `1000` | Sync operation queue size |
| `thread_pool_size` | integer | *CPU cores* | General thread pool size |
| `io_thread_pool_size` | integer | *CPU cores* | I/O thread pool size |
| `network_thread_pool_size` | integer | `4` | Network operation thread pool |
| `channel_buffer_size` | integer | `1000` | Internal channel buffer size |

### [cache]

Caching configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cache_dir` | string | `"~/.cache/sync-daemon"` | Cache directory |
| `config_cache_ttl_secs` | integer | `300` | Configuration cache TTL |
| `file_metadata_cache_ttl_secs` | integer | `60` | File metadata cache TTL |
| `enable_persistent_cache` | boolean | `false` | Persist cache across restarts |
| `max_cache_size_mb` | integer | `500` | Maximum cache size |
| `cache_cleanup_interval_secs` | integer | `3600` | Cache cleanup interval |
| `compression_enabled` | boolean | `true` | Enable cache compression |

### [[sync_jobs]]

Sync job definitions (array of tables).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | string | *required* | Unique job identifier |
| `name` | string | *required* | Human-readable job name |
| `source_path` | string | *required* | Source directory path |
| `destination_path` | string | *required* | Destination directory path |
| `enabled` | boolean | `true` | Enable/disable job |
| `filters` | array | `[]` | File exclusion patterns |
| `exclude_patterns` | array | `[]` | Additional exclusion patterns |
| `include_patterns` | array | `[]` | File inclusion patterns |
| `priority` | integer | `0` | Job priority (higher = more priority) |
| `tags` | array | `[]` | Job tags for organization |

#### [sync_jobs.schedule]

Job scheduling configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `type` | string | *required* | Schedule type (`interval`, `cron`, `manual`) |
| `interval` | string | *conditional* | Interval (e.g., "5m", "1h", "30s") |
| `expression` | string | *conditional* | Cron expression (if type=cron) |
| `timezone` | string | `"UTC"` | Timezone for cron scheduling |
| `max_run_duration_secs` | integer | `3600` | Maximum run duration |
| `skip_if_running` | boolean | `true` | Skip if previous run still active |

#### [sync_jobs.sync_options]

Sync operation options.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `dry_run` | boolean | `false` | Perform dry run (no changes) |
| `preserve_permissions` | boolean | `true` | Preserve file permissions |
| `preserve_timestamps` | boolean | `true` | Preserve file timestamps |
| `preserve_ownership` | boolean | `false` | Preserve file ownership |
| `delete_destination_files` | boolean | `false` | Delete extra files in destination |
| `comparison_method` | string | `"sha256"` | File comparison method |
| `ignore_hidden_files` | boolean | `true` | Ignore hidden files |
| `continue_on_error` | boolean | `true` | Continue sync on individual errors |
| `bidirectional` | boolean | `false` | Enable bidirectional sync |
| `conflict_resolution` | string | `"newer"` | Conflict resolution strategy |
| `max_file_size_mb` | integer | `1000` | Maximum file size to sync |
| `follow_symlinks` | boolean | `false` | Follow symbolic links |
| `verify_checksums` | boolean | `true` | Verify file checksums |
| `compression_enabled` | boolean | `false` | Enable compression during transfer |

**Comparison Methods:**
- `"size"` - Compare by file size only
- `"mtime"` - Compare by modification time
- `"md5"` - Compare by MD5 hash
- `"sha256"` - Compare by SHA256 hash (recommended)
- `"xxhash"` - Compare by xxHash (fastest)

**Conflict Resolution Strategies:**
- `"newer"` - Use file with newer timestamp
- `"larger"` - Use larger file
- `"source"` - Always prefer source file
- `"destination"` - Always prefer destination file
- `"manual"` - Require manual resolution
- `"skip"` - Skip conflicting files

### [[file_watchers]]

File watcher configurations (array of tables).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | string | *required* | Unique watcher identifier |
| `name` | string | *required* | Human-readable name |
| `watch_path` | string | *required* | Path to watch |
| `sync_job_id` | string | *required* | Associated sync job ID |
| `enabled` | boolean | `true` | Enable/disable watcher |
| `recursive` | boolean | `true` | Watch subdirectories |
| `debounce_ms` | integer | `1000` | Event debounce time |
| `watch_events` | array | `["create", "write", "remove"]` | Events to watch |
| `ignore_patterns` | array | `[]` | Patterns to ignore |
| `max_events_per_second` | integer | `100` | Rate limiting |

**Watch Events:**
- `"create"` - File/directory creation
- `"write"` - File modification
- `"remove"` - File/directory deletion
- `"rename"` - File/directory rename
- `"chmod"` - Permission changes
- `"all"` - All event types

## CLI Configuration

### Configuration File Location

The CLI tool uses a separate configuration file, typically located at:

- Linux/macOS: `~/.config/sync-app/config.json`
- Windows: `%APPDATA%\sync-app\config.json`

### CLI Configuration Format

```json
{
  "default_pocketbase_url": "http://localhost:8090",
  "default_admin_email": "admin@example.com",
  "output_format": "human",
  "verbose": false,
  "configs": {
    "config-uuid": {
      "id": "config-uuid",
      "name": "My Sync Config",
      "source_path": "/path/to/source",
      "destination_path": "/path/to/dest",
      "pocketbase_url": "http://localhost:8090",
      "admin_email": null,
      "admin_password": null,
      "filters": [],
      "exclude_patterns": [".git", "node_modules"],
      "dry_run": false,
      "preserve_permissions": true,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  },
  "default_config": null
}
```

### CLI Configuration Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_pocketbase_url` | string | `"http://localhost:8090"` | Default PocketBase URL |
| `default_admin_email` | string | `null` | Default admin email |
| `output_format` | string | `"human"` | Output format (`human`, `json`) |
| `verbose` | boolean | `false` | Enable verbose output |
| `default_config` | string | `null` | Default configuration ID |

## Environment Variables

Override configuration values using environment variables:

### Daemon Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `SYNC_DAEMON_CONFIG` | Configuration file path | `/etc/sync-daemon.toml` |
| `SYNC_PB_URL` | PocketBase URL | `http://localhost:8090` |
| `SYNC_PB_ADMIN_EMAIL` | PocketBase admin email | `admin@example.com` |
| `SYNC_PB_ADMIN_PASSWORD` | PocketBase admin password | `password` |
| `SYNC_LOG_LEVEL` | Log level override | `debug` |
| `SYNC_METRICS_PORT` | Metrics port override | `9091` |
| `RUST_LOG` | Rust logging configuration | `sync_daemon=debug` |

### CLI Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `SYNC_CLI_CONFIG` | CLI config file path | `~/.sync/config.json` |
| `SYNC_DEFAULT_URL` | Default PocketBase URL | `http://localhost:8090` |
| `SYNC_OUTPUT_FORMAT` | Default output format | `json` |

## Validation Rules

### Path Validation

- All paths must be absolute or properly expandable (e.g., `~` for home directory)
- Source and destination paths cannot be the same
- Paths must be accessible with appropriate permissions

### Network Configuration

- URLs must be valid HTTP/HTTPS endpoints
- Ports must be in range 1-65535
- Timeout values must be positive integers

### Schedule Validation

- Interval format: `{number}{unit}` where unit is `s`, `m`, `h`, `d`
- Cron expressions must be valid (5 or 6 field format)
- Timezones must be valid IANA timezone identifiers

### Size Limits

- File sizes specified in MB (minimum 1MB)
- Cache sizes specified in MB (minimum 10MB)
- Queue sizes must be positive integers

## Configuration Examples

### Development Configuration

```toml
# Minimal development setup
[pocketbase]
url = "http://localhost:8090"
admin_email = "admin@example.com"
admin_password = "admin123456"

[daemon]
log_level = "debug"

[telemetry]
console_logging = true
json_logging = false

[[sync_jobs]]
id = "dev_sync"
name = "Development Sync"
source_path = "./source"
destination_path = "./backup"
enabled = true

[sync_jobs.schedule]
type = "interval"
interval = "30s"

[sync_jobs.sync_options]
dry_run = true
```

### Production Configuration

```toml
# Production-ready configuration
[pocketbase]
url = "https://api.yourdomain.com"
admin_email = "admin@yourdomain.com"
admin_password = "secure-password"
timeout_secs = 60
retry_attempts = 5

[daemon]
pid_file = "/opt/sync-app/daemon.pid"
log_file = "/opt/sync-app/logs/daemon.log"
log_level = "info"
auto_restart_on_config_change = true

[telemetry]
json_logging = true
console_logging = false

[telemetry.log_rotation]
enabled = true
frequency = "daily"
keep_files = 30
max_size_mb = 50

[telemetry.metrics]
enabled = true
bind_address = "0.0.0.0"
port = 9090

[concurrency]
max_concurrent_syncs = 8
max_file_operations = 500

[[sync_jobs]]
id = "production_backup"
name = "Production Backup"
source_path = "/data/important"
destination_path = "/backup/important"
enabled = true

[sync_jobs.schedule]
type = "cron"
expression = "0 2 * * *"  # Daily at 2 AM

[sync_jobs.sync_options]
preserve_permissions = true
comparison_method = "sha256"
continue_on_error = false
```

### High-Performance Configuration

```toml
# Optimized for high throughput
[concurrency]
max_concurrent_syncs = 16
max_file_operations = 1000
sync_queue_size = 5000
thread_pool_size = 16

[cache]
max_cache_size_mb = 2000
file_metadata_cache_ttl_secs = 30
enable_persistent_cache = true

[sync_jobs.sync_options]
comparison_method = "xxhash"
verify_checksums = false
compression_enabled = true
```

## Configuration Migration

When upgrading Sync App versions, configuration files may need migration. The daemon automatically detects older configuration formats and provides migration assistance.

### Migration Commands

```bash
# Check configuration compatibility
sync-daemon config validate --config daemon.toml

# Migrate configuration to latest format
sync-daemon config migrate --config daemon.toml --output daemon-new.toml

# Backup current configuration
cp daemon.toml daemon-backup-$(date +%Y%m%d).toml
```

This comprehensive configuration reference covers all available options for customizing Sync App behavior. Choose appropriate settings based on your deployment environment and performance requirements.
