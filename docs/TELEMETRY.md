# Telemetry, Logging, and Metrics

This document describes the comprehensive telemetry system integrated into the sync daemon, which provides structured logging, metrics collection, and operational insights.

## Overview

The telemetry system consists of three main components:

1. **Structured Logging**: Using `tracing` with multiple output formats (JSON, pretty console)
2. **Log Persistence**: Automatic log upload to PocketBase with local file rotation
3. **Prometheus Metrics**: Comprehensive metrics endpoint for operational monitoring

## Features

### Structured Logging

- **Multiple Formats**: JSON for machine parsing, pretty format for human consumption
- **Contextual Information**: Each log entry includes daemon ID, session ID, and structured fields
- **Span Tracking**: Distributed tracing for understanding operation flow
- **Log Levels**: Configurable verbosity from trace to error

### Log Persistence

#### Local File Logging
- **Rotation**: Daily/hourly rotation with configurable retention
- **Compression**: Automatic compression of old log files
- **Size Limits**: Maximum file size before rotation

#### PocketBase Integration
- **Batch Upload**: Efficient batching of log entries
- **Retry Logic**: Automatic retry with exponential backoff
- **Structured Storage**: Rich metadata and searchable fields

### Prometheus Metrics

The daemon exposes comprehensive metrics on port 9090 (configurable):

#### Sync Operation Metrics
- `sync_operations_total`: Total sync operations by job ID, status, and trigger source
- `sync_operations_duration_seconds`: Histogram of sync operation durations
- `sync_files_processed_total`: Files processed by operation type
- `sync_bytes_transferred_total`: Data transfer volume
- `sync_errors_total`: Error counts by type and job

#### Daemon Health Metrics
- `daemon_uptime_seconds`: Daemon uptime
- `daemon_memory_usage_bytes`: Current memory usage
- `daemon_cpu_usage_percent`: CPU utilization
- `active_sync_jobs`: Currently running sync operations
- `file_watchers_active`: Number of active file watchers

#### PocketBase Metrics
- `pocketbase_requests_total`: API request counts by method and status
- `pocketbase_request_duration_seconds`: Request timing histogram
- `pocketbase_connection_errors_total`: Connection failure count

#### Log Metrics
- `log_entries_total`: Log entries by level and target
- `log_upload_errors_total`: Failed log uploads
- `log_buffer_size`: Current log buffer size

## Configuration

### Basic Configuration

```toml
[telemetry]
log_level = "info"
json_logging = true
console_logging = true
log_file_path = "logs/daemon.log"
```

### Log Rotation

```toml
[telemetry.log_rotation]
enabled = true
frequency = "daily"  # or "hourly"
keep_files = 7
max_size_mb = 100
```

### PocketBase Logging

```toml
[telemetry.pocketbase_logging]
enabled = true
collection = "daemon_logs"
batch_size = 100
flush_interval_secs = 30
max_retries = 3
```

### Metrics Configuration

```toml
[telemetry.metrics]
enabled = true
bind_address = "127.0.0.1"
port = 9090
collection_interval_secs = 15
```

## Usage Examples

### Accessing Metrics

```bash
# Get all metrics
curl http://localhost:9090/metrics

# Monitor sync operations
curl -s http://localhost:9090/metrics | grep sync_operations

# Check daemon health
curl -s http://localhost:9090/metrics | grep daemon_uptime
```

### Log Analysis

#### Structured Logs
Logs are structured with consistent fields:

```json
{
  "timestamp": "2024-01-01T12:00:00Z",
  "level": "INFO",
  "target": "sync_daemon::daemon",
  "message": "Sync job completed successfully",
  "fields": {
    "job_name": "documents_sync",
    "duration_ms": 1250,
    "files_processed": 42,
    "bytes_transferred": 1048576,
    "success_rate": 100.0
  },
  "daemon_id": "550e8400-e29b-41d4-a716-446655440000",
  "session_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
}
```

#### Query Examples
```bash
# Find all sync operations
jq 'select(.fields.job_name)' logs/daemon.log

# Get error summary
jq 'select(.level == "ERROR")' logs/daemon.log | jq '.fields.error_type' | sort | uniq -c

# Performance analysis
jq 'select(.fields.duration_ms) | .fields.duration_ms' logs/daemon.log | jq -s 'add/length'
```

### Monitoring Setup

#### Prometheus Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'sync-daemon'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
    metrics_path: /metrics
```

#### Grafana Dashboard

Key metrics to monitor:
- Sync success rate over time
- Operation duration percentiles
- Error rate by type
- Resource utilization trends
- File throughput rates

## PocketBase Schema

### Log Collection

```javascript
// pb_migrations/sync_daemon_logs.js
migrate((db) => {
  const collection = new Collection({
    name: "daemon_logs",
    type: "base",
    schema: [
      {
        name: "timestamp",
        type: "date",
        required: true
      },
      {
        name: "level",
        type: "select",
        options: ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"],
        required: true
      },
      {
        name: "message",
        type: "text",
        required: true
      },
      {
        name: "target",
        type: "text"
      },
      {
        name: "module_path",
        type: "text"
      },
      {
        name: "file",
        type: "text"
      },
      {
        name: "line",
        type: "number"
      },
      {
        name: "fields",
        type: "json"
      },
      {
        name: "daemon_id",
        type: "text",
        required: true
      },
      {
        name: "session_id",
        type: "text",
        required: true
      }
    ],
    indexes: [
      "CREATE INDEX idx_daemon_logs_timestamp ON daemon_logs (timestamp)",
      "CREATE INDEX idx_daemon_logs_level ON daemon_logs (level)",
      "CREATE INDEX idx_daemon_logs_daemon_id ON daemon_logs (daemon_id)",
      "CREATE INDEX idx_daemon_logs_session_id ON daemon_logs (session_id)"
    ]
  });
  
  return Dao(db).saveCollection(collection);
});
```

## Error Handling

### Log Upload Failures
- Automatic retry with exponential backoff
- Circuit breaker to prevent overwhelming PocketBase
- Local buffering with overflow protection
- Graceful degradation to file-only logging

### Metrics Collection
- Non-blocking metrics collection
- Default values for missing metrics
- Automatic recovery from collection failures

## Performance Considerations

### Log Volume Management
- Configurable log levels to control verbosity
- Efficient batching for PocketBase uploads
- Local log rotation to prevent disk overflow
- Asynchronous logging to avoid blocking operations

### Metrics Overhead
- Minimal performance impact (< 1% CPU overhead)
- In-memory counters with periodic collection
- Efficient Prometheus format encoding

## Security

### Log Sanitization
- Automatic removal of sensitive data (passwords, tokens)
- Configurable field filtering
- Path normalization to prevent information leakage

### Metrics Access
- Metrics endpoint on localhost by default
- Optional authentication integration
- Rate limiting for metrics requests

## Troubleshooting

### Common Issues

#### High Log Volume
```toml
# Reduce log level
log_level = "warn"

# Increase batch size
[telemetry.pocketbase_logging]
batch_size = 500
flush_interval_secs = 60
```

#### PocketBase Connection Issues
```toml
# Disable PocketBase logging temporarily
[telemetry.pocketbase_logging]
enabled = false

# Increase retry settings
max_retries = 10
```

#### Metrics Performance
```toml
# Reduce collection frequency
[telemetry.metrics]
collection_interval_secs = 60
```

### Debug Mode

Enable debug logging for telemetry troubleshooting:

```bash
RUST_LOG=sync_daemon::telemetry=debug sync-daemon start
```

## Integration Examples

### Log Aggregation with ELK Stack

```yaml
# filebeat.yml
filebeat.inputs:
- type: log
  paths:
    - /var/log/sync-daemon/*.log
  json.keys_under_root: true
  json.add_error_key: true

output.elasticsearch:
  hosts: ["localhost:9200"]
  index: "sync-daemon-logs-%{+yyyy.MM.dd}"
```

### Alerting with Prometheus

```yaml
# alerts.yml
groups:
- name: sync-daemon
  rules:
  - alert: SyncJobFailure
    expr: rate(sync_errors_total[5m]) > 0.1
    for: 2m
    annotations:
      summary: "High error rate in sync operations"
      
  - alert: DaemonDown
    expr: up{job="sync-daemon"} == 0
    for: 1m
    annotations:
      summary: "Sync daemon is down"
```

This comprehensive telemetry system provides deep visibility into sync daemon operations, enabling effective monitoring, debugging, and operational insights.
