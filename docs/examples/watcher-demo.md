# File Watcher Demo Example

This example demonstrates real-time file synchronization using file watchers for immediate response to file system changes.

## Use Case

- **Source**: `/home/user/watched-files` (actively monitored)
- **Destination**: `/backup/watched-files` (immediate backup)
- **Behavior**: Near-instantaneous sync on file changes
- **Features**: Multiple watchers, different debounce settings, event filtering
- **Performance**: Optimized for high-frequency file operations

## Configuration

### Daemon Configuration (daemon-watcher.toml)

```toml
[pocketbase]
url = "http://localhost:8090"
admin_email = "admin@example.com"
admin_password = "admin123456"
timeout_secs = 30
retry_attempts = 3

[daemon]
log_level = "debug"  # Verbose logging for watcher events
log_file = "logs/watcher-daemon.log"
config_refresh_interval_secs = 120

[telemetry]
console_logging = true
json_logging = true
log_file_path = "logs/watcher-daemon.log"

[telemetry.log_rotation]
enabled = true
frequency = "hourly"  # More frequent rotation for high activity
keep_files = 24
max_size_mb = 25

[telemetry.metrics]
enabled = true
bind_address = "127.0.0.1"
port = 9092  # Unique port for watcher demo

# Fast sync job for watcher triggers
[[sync_jobs]]
id = "watcher_fast_sync"
name = "Fast Watcher-Triggered Sync"
source_path = "/home/user/watched-files"
destination_path = "/backup/watched-files"
enabled = true
priority = 1

# Minimal filters for fast sync
filters = ["*.tmp", "*.swp", ".DS_Store"]

# Manual trigger only (triggered by watchers)
[sync_jobs.schedule]
type = "manual"

# Optimized for speed
[sync_jobs.sync_options]
dry_run = false
preserve_permissions = true
preserve_timestamps = true
delete_destination_files = false
comparison_method = "mtime"  # Faster than hash comparison
ignore_hidden_files = true
continue_on_error = true
bidirectional = false
verify_checksums = false  # Skip for speed
max_file_size_mb = 100

# Comprehensive watcher for documents
[[file_watchers]]
id = "documents_watcher"
name = "Documents Real-time Watcher"
watch_path = "/home/user/watched-files/documents"
sync_job_id = "watcher_fast_sync"
enabled = true
recursive = true
debounce_ms = 500  # Very fast response

# Watch all change events
watch_events = ["create", "write", "remove", "rename", "chmod"]

# Ignore temporary files
ignore_patterns = [
    "*.tmp", "*.temp", "*.swp", "*.swo", "*.bak",
    ".DS_Store", "Thumbs.db", "desktop.ini",
    "~$*", ".~lock.*", "*.autosave"
]
max_events_per_second = 200

# Medium-speed watcher for projects (batch changes)
[[file_watchers]]
id = "projects_watcher"
name = "Projects Batch Watcher"
watch_path = "/home/user/watched-files/projects"
sync_job_id = "watcher_fast_sync"
enabled = true
recursive = true
debounce_ms = 2000  # 2-second batching

# Focus on important events
watch_events = ["create", "write", "remove"]

ignore_patterns = [
    "*.tmp", "*.log", "*.lock",
    ".git", "node_modules", "target", "build",
    "__pycache__", "*.pyc", ".cache"
]
max_events_per_second = 100

# Slow watcher for media (large files)
[[file_watchers]]
id = "media_watcher"
name = "Media Files Slow Watcher"
watch_path = "/home/user/watched-files/media"
sync_job_id = "watcher_fast_sync"
enabled = true
recursive = true
debounce_ms = 10000  # 10-second delay for large files

# Only final write events
watch_events = ["create", "write"]

ignore_patterns = [
    "*.tmp", "*.part", "*.download",
    ".thumbnails", ".cache"
]
max_events_per_second = 20

# Critical files watcher (immediate sync)
[[file_watchers]]
id = "critical_watcher"
name = "Critical Files Instant Watcher"
watch_path = "/home/user/watched-files/critical"
sync_job_id = "watcher_fast_sync"
enabled = true
recursive = true
debounce_ms = 100  # Near-instant response

watch_events = ["create", "write", "remove", "rename"]
ignore_patterns = []  # Watch everything in critical
max_events_per_second = 500

# Configuration files watcher
[[file_watchers]]
id = "config_watcher"
name = "Configuration Files Watcher"
watch_path = "/home/user/watched-files/config"
sync_job_id = "watcher_fast_sync"
enabled = true
recursive = false  # Only watch root config directory
debounce_ms = 1000

watch_events = ["write", "rename"]  # Only modifications
ignore_patterns = ["*.bak", "*.old", "*~"]
max_events_per_second = 50

[concurrency]
max_concurrent_syncs = 1  # Single sync to avoid conflicts
max_file_operations = 50  # Conservative for real-time
sync_queue_size = 200
thread_pool_size = 4

[cache]
cache_dir = "/home/user/.cache/sync-watcher"
enable_persistent_cache = true
file_metadata_cache_ttl_secs = 30  # Short cache for real-time
max_cache_size_mb = 100
```

## Setup Script

Create `setup-watcher.sh`:

```bash
#!/bin/bash
# setup-watcher.sh - Set up file watcher demo

set -e

# Configuration
WATCH_ROOT="/home/user/watched-files"
BACKUP_ROOT="/backup/watched-files"
CONFIG_FILE="daemon-watcher.toml"
LOG_DIR="logs"

echo "Setting up file watcher demo..."

# Create watch directory structure
mkdir -p "$WATCH_ROOT"/{documents,projects,media,critical,config}
mkdir -p "$BACKUP_ROOT"
mkdir -p "$LOG_DIR"

# Create subdirectories
mkdir -p "$WATCH_ROOT/documents"/{reports,presentations,spreadsheets}
mkdir -p "$WATCH_ROOT/projects"/{web-app,mobile-app,api}
mkdir -p "$WATCH_ROOT/media"/{images,videos,audio}
mkdir -p "$WATCH_ROOT/critical"/{passwords,keys,certificates}
mkdir -p "$WATCH_ROOT/config"

echo "✅ Directory structure created"

# Create test files for each watcher type
cat > "$WATCH_ROOT/README.md" << 'EOF'
# File Watcher Demo

This directory demonstrates real-time file synchronization using multiple file watchers with different configurations:

## Watcher Types

1. **Documents** (`/documents/`) - 500ms debounce, all events
   - Reports, presentations, spreadsheets
   - Very fast sync for document changes

2. **Projects** (`/projects/`) - 2s debounce, batch changes
   - Source code and project files
   - Batches multiple changes for efficiency

3. **Media** (`/media/`) - 10s debounce, large files
   - Images, videos, audio files
   - Slower sync to handle large file operations

4. **Critical** (`/critical/`) - 100ms debounce, instant sync
   - Passwords, keys, certificates
   - Near-instantaneous sync for security

5. **Config** (`/config/`) - 1s debounce, modifications only
   - Configuration files
   - Watches only file modifications and renames

## Testing

- Add, modify, or delete files in any subdirectory
- Watch logs to see real-time sync activity
- Different directories have different sync speeds
EOF

# Documents - fast sync
echo "Quarterly report Q1 2024" > "$WATCH_ROOT/documents/reports/q1-2024.txt"
echo "Project presentation slides" > "$WATCH_ROOT/documents/presentations/project-demo.pptx"
echo "Budget spreadsheet data" > "$WATCH_ROOT/documents/spreadsheets/budget-2024.xlsx"

# Projects - batched sync
echo "Web application source code" > "$WATCH_ROOT/projects/web-app/app.js"
echo "Mobile app configuration" > "$WATCH_ROOT/projects/mobile-app/config.json"
echo "API documentation" > "$WATCH_ROOT/projects/api/README.md"

# Media - slow sync
echo "Sample image file" > "$WATCH_ROOT/media/images/sample.jpg"
echo "Sample video file" > "$WATCH_ROOT/media/videos/demo.mp4"
echo "Sample audio file" > "$WATCH_ROOT/media/audio/music.mp3"

# Critical - instant sync
echo "Database password: secret123" > "$WATCH_ROOT/critical/passwords/db.txt"
echo "API key: abc123xyz" > "$WATCH_ROOT/critical/keys/api.key"
echo "SSL certificate data" > "$WATCH_ROOT/critical/certificates/ssl.pem"

# Config - modification-only sync
cat > "$WATCH_ROOT/config/app.conf" << 'EOF'
# Application Configuration
debug=true
port=8080
database_url=localhost:5432
EOF

echo "✅ Test files created"

# Set permissions
chmod -R 755 "$WATCH_ROOT"
chmod -R 755 "$BACKUP_ROOT"

# Validate configuration
if command -v daemon >/dev/null 2>&1; then
    echo "🔍 Validating watcher configuration..."
    daemon config validate --config "$CONFIG_FILE"
    echo "✅ Configuration is valid"
else
    echo "⚠️  Daemon binary not found. Please build first:"
    echo "   cargo build --release"
fi

# Create watcher test script
cat > test-watchers.sh << 'EOF'
#!/bin/bash
# test-watchers.sh - Test different watcher behaviors

WATCH_ROOT="/home/user/watched-files"

echo "🧪 Testing File Watchers"
echo "Monitor logs with: tail -f logs/watcher-daemon.log"
echo ""

test_documents() {
    echo "📄 Testing Documents Watcher (500ms debounce)..."
    echo "Document update $(date)" >> "$WATCH_ROOT/documents/reports/test.txt"
    echo "  → Should sync within ~500ms"
}

test_projects() {
    echo "💻 Testing Projects Watcher (2s debounce)..."
    echo "Code update $(date)" >> "$WATCH_ROOT/projects/web-app/update.js"
    echo "More code $(date)" >> "$WATCH_ROOT/projects/web-app/feature.js"
    echo "  → Should batch changes and sync in ~2s"
}

test_media() {
    echo "🎬 Testing Media Watcher (10s debounce)..."
    echo "Large media file $(date)" > "$WATCH_ROOT/media/images/large-$(date +%s).jpg"
    echo "  → Should sync in ~10s (simulating large file handling)"
}

test_critical() {
    echo "🔒 Testing Critical Watcher (100ms debounce)..."
    echo "URGENT: Password changed $(date)" > "$WATCH_ROOT/critical/passwords/urgent-$(date +%s).txt"
    echo "  → Should sync almost instantly (~100ms)"
}

test_config() {
    echo "⚙️  Testing Config Watcher (1s debounce, modifications only)..."
    sed -i "s/debug=.*/debug=$(date +%s)/" "$WATCH_ROOT/config/app.conf"
    echo "  → Should sync config modification in ~1s"
}

case "$1" in
    documents) test_documents ;;
    projects) test_projects ;;
    media) test_media ;;
    critical) test_critical ;;
    config) test_config ;;
    all)
        test_documents
        sleep 1
        test_projects
        sleep 3
        test_media
        sleep 1
        test_critical
        sleep 2
        test_config
        ;;
    *)
        echo "Usage: $0 {documents|projects|media|critical|config|all}"
        echo ""
        echo "Test individual watchers or run 'all' to test everything"
        exit 1
        ;;
esac

echo ""
echo "Watch the logs to see sync activity:"
echo "  tail -f logs/watcher-daemon.log | grep -E '(watcher|sync)'"
EOF

chmod +x test-watchers.sh

echo "✅ Test script created: test-watchers.sh"

echo "
🚀 File Watcher Demo Setup Complete!

📁 Directory Structure:
   Watch Root: $WATCH_ROOT
   Backup Root: $BACKUP_ROOT

👁️  Watchers Configured:
   Documents:  500ms debounce (fast)
   Projects:   2s debounce (batched)
   Media:      10s debounce (slow)
   Critical:   100ms debounce (instant)
   Config:     1s debounce (modifications only)

🧪 Testing:
   ./test-watchers.sh all          # Test all watchers
   ./test-watchers.sh documents    # Test specific watcher
   
📊 Monitoring:
   tail -f $LOG_DIR/watcher-daemon.log
   curl http://localhost:9092/metrics

🚀 Next Steps:
1. Start PocketBase: ./pocketbase/setup.sh
2. Start daemon: daemon start --config $CONFIG_FILE
3. Run tests: ./test-watchers.sh all
4. Monitor activity: tail -f $LOG_DIR/watcher-daemon.log
"
```

## Management Script

Create `manage-watcher.sh`:

```bash
#!/bin/bash
# manage-watcher.sh - Manage file watcher daemon

CONFIG_FILE="daemon-watcher.toml"
PID_FILE="watcher-daemon.pid"
LOG_FILE="logs/watcher-daemon.log"
METRICS_PORT="9092"
WATCH_ROOT="/home/user/watched-files"

show_watcher_status() {
    echo "👁️  File Watcher Status:"
    if command -v curl >/dev/null 2>&1; then
        # Get watcher metrics
        curl -s "http://localhost:$METRICS_PORT/metrics" | grep -E "file_watchers|watcher_events" | while read line; do
            echo "  $line"
        done
    else
        echo "  Metrics not available (curl not found)"
    fi
}

show_recent_events() {
    echo "📝 Recent Watcher Events (last 20):"
    if [ -f "$LOG_FILE" ]; then
        grep -i "watcher\|file.*event\|sync.*triggered" "$LOG_FILE" | tail -20 | while read line; do
            echo "  $line"
        done
    else
        echo "  No log file found"
    fi
}

show_sync_performance() {
    echo "⚡ Sync Performance:"
    if [ -f "$LOG_FILE" ]; then
        echo "  Watcher-triggered syncs in last hour:"
        since_time=$(date -d '1 hour ago' '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -v-1H '+%Y-%m-%d %H:%M:%S')
        grep -c "watcher.*triggered.*sync" "$LOG_FILE" || echo "  0"
        
        echo "  Average sync duration (seconds):"
        grep "sync.*completed.*duration" "$LOG_FILE" | tail -10 | awk '{print $NF}' | awk '{sum+=$1; count++} END {if(count>0) print "  " sum/count; else print "  No data"}'
    else
        echo "  No performance data available"
    fi
}

test_watcher_by_type() {
    local watcher_type="$1"
    local test_file
    local debounce_time
    
    case "$watcher_type" in
        documents)
            test_file="$WATCH_ROOT/documents/watcher-test-$(date +%s).txt"
            debounce_time="500ms"
            ;;
        projects)
            test_file="$WATCH_ROOT/projects/watcher-test-$(date +%s).js"
            debounce_time="2s"
            ;;
        media)
            test_file="$WATCH_ROOT/media/watcher-test-$(date +%s).jpg"
            debounce_time="10s"
            ;;
        critical)
            test_file="$WATCH_ROOT/critical/watcher-test-$(date +%s).key"
            debounce_time="100ms"
            ;;
        config)
            test_file="$WATCH_ROOT/config/test-$(date +%s).conf"
            debounce_time="1s"
            ;;
        *)
            echo "Unknown watcher type: $watcher_type"
            return 1
            ;;
    esac
    
    echo "Testing $watcher_type watcher (debounce: $debounce_time)..."
    echo "Test file created at $(date)" > "$test_file"
    echo "Created: $test_file"
    echo "Expected sync delay: $debounce_time"
    echo "Monitor with: $0 events"
}

case "$1" in
    start)
        echo "Starting file watcher daemon..."
        daemon start --config "$CONFIG_FILE" --pid-file "$PID_FILE" &
        echo "Daemon started (PID: $!)"
        echo "Logs: tail -f $LOG_FILE"
        echo "Metrics: curl http://localhost:$METRICS_PORT/metrics"
        ;;
    
    stop)
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            echo "Stopping file watcher daemon (PID: $PID)..."
            kill "$PID"
            rm -f "$PID_FILE"
            echo "Daemon stopped"
        else
            echo "No PID file found"
        fi
        ;;
    
    restart)
        $0 stop
        sleep 2
        $0 start
        ;;
    
    status)
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if kill -0 "$PID" 2>/dev/null; then
                echo "✅ File watcher daemon is running (PID: $PID)"
                echo ""
                show_watcher_status
                echo ""
                show_sync_performance
            else
                echo "❌ PID file exists but daemon not running"
                rm -f "$PID_FILE"
            fi
        else
            echo "❌ Daemon is not running"
        fi
        ;;
    
    logs)
        if [ -f "$LOG_FILE" ]; then
            tail -f "$LOG_FILE"
        else
            echo "Log file not found: $LOG_FILE"
        fi
        ;;
    
    events)
        show_recent_events
        ;;
    
    watch-events)
        echo "👀 Watching for file events (Ctrl+C to stop)..."
        tail -f "$LOG_FILE" | grep --line-buffered -i "watcher\|file.*event\|sync.*triggered"
        ;;
    
    test)
        watcher_type="$2"
        if [ -z "$watcher_type" ]; then
            echo "Usage: $0 test {documents|projects|media|critical|config}"
            exit 1
        fi
        test_watcher_by_type "$watcher_type"
        ;;
    
    test-all)
        echo "🧪 Testing all watchers sequentially..."
        ./test-watchers.sh all
        echo ""
        echo "Monitor results with: $0 watch-events"
        ;;
    
    performance)
        show_sync_performance
        ;;
    
    metrics)
        if command -v curl >/dev/null 2>&1; then
            echo "📊 Watcher Metrics:"
            curl -s "http://localhost:$METRICS_PORT/metrics" | grep -E "watcher|sync|file" | sort
        else
            echo "curl not available for metrics"
        fi
        ;;
    
    stress-test)
        echo "🏋️  Running stress test..."
        stress_dir="$WATCH_ROOT/documents/stress-test"
        mkdir -p "$stress_dir"
        
        echo "Creating 50 files rapidly..."
        for i in {1..50}; do
            echo "Stress test file $i created at $(date)" > "$stress_dir/stress-$i.txt"
            [ $((i % 10)) -eq 0 ] && echo "Created $i files..."
        done
        
        echo "Files created. Monitor sync activity:"
        echo "  $0 watch-events"
        echo "  $0 performance"
        ;;
    
    cleanup)
        echo "🧹 Cleaning up test files..."
        find "$WATCH_ROOT" -name "*test*" -type f -delete
        find "$WATCH_ROOT" -name "stress-*" -type f -delete
        rm -rf "$WATCH_ROOT/documents/stress-test"
        echo "Test files cleaned up"
        ;;
    
    *)
        echo "Usage: $0 {start|stop|restart|status|logs|events|watch-events|test|test-all|performance|metrics|stress-test|cleanup}"
        echo ""
        echo "Commands:"
        echo "  start        - Start the watcher daemon"
        echo "  stop         - Stop the daemon"
        echo "  restart      - Restart the daemon"
        echo "  status       - Show daemon and watcher status"
        echo "  logs         - Follow all daemon logs"
        echo "  events       - Show recent watcher events"
        echo "  watch-events - Watch watcher events in real-time"
        echo "  test TYPE    - Test specific watcher (documents|projects|media|critical|config)"
        echo "  test-all     - Test all watchers"
        echo "  performance  - Show sync performance metrics"
        echo "  metrics      - Show detailed metrics"
        echo "  stress-test  - Create many files rapidly for testing"
        echo "  cleanup      - Remove test files"
        exit 1
        ;;
esac
```

## Demo Scenarios

### 1. Basic Setup and Testing

```bash
# Setup
chmod +x setup-watcher.sh manage-watcher.sh test-watchers.sh
./setup-watcher.sh

# Start services
./pocketbase/setup.sh
./manage-watcher.sh start

# Test all watchers
./manage-watcher.sh test-all
```

### 2. Individual Watcher Testing

```bash
# Test documents watcher (fast)
./manage-watcher.sh test documents

# Test critical watcher (instant)
./manage-watcher.sh test critical

# Test media watcher (slow)
./manage-watcher.sh test media
```

### 3. Real-time Monitoring

```bash
# Watch events as they happen
./manage-watcher.sh watch-events

# Monitor performance
watch "./manage-watcher.sh performance"

# View all metrics
./manage-watcher.sh metrics
```

### 4. Stress Testing

```bash
# Create many files rapidly
./manage-watcher.sh stress-test

# Monitor how system handles load
./manage-watcher.sh watch-events
```

## Watcher Characteristics

### Documents Watcher
- **Debounce**: 500ms (very responsive)
- **Events**: All (create, write, remove, rename, chmod)
- **Use Case**: Text documents, office files
- **Behavior**: Fast sync for document changes

### Projects Watcher  
- **Debounce**: 2s (batches changes)
- **Events**: Create, write, remove
- **Use Case**: Source code, project files
- **Behavior**: Batches multiple rapid changes

### Media Watcher
- **Debounce**: 10s (handles large files)
- **Events**: Create, write
- **Use Case**: Images, videos, audio
- **Behavior**: Waits for large file operations to complete

### Critical Watcher
- **Debounce**: 100ms (near-instant)
- **Events**: All
- **Use Case**: Passwords, keys, certificates
- **Behavior**: Immediate sync for security-critical files

### Config Watcher
- **Debounce**: 1s
- **Events**: Write, rename only
- **Use Case**: Configuration files
- **Behavior**: Only syncs actual modifications

## Performance Monitoring

### Key Metrics

```bash
# Watcher event frequency
curl -s http://localhost:9092/metrics | grep watcher_events_total

# Sync trigger rate
curl -s http://localhost:9092/metrics | grep sync_operations_total

# File throughput
curl -s http://localhost:9092/metrics | grep sync_files_processed_total

# Average response time
grep "sync.*duration" logs/watcher-daemon.log | tail -10
```

### Event Analysis

```bash
# Count events by watcher
grep "watcher.*event" logs/watcher-daemon.log | awk '{print $3}' | sort | uniq -c

# Average debounce effectiveness
grep "debounce.*triggered" logs/watcher-daemon.log | wc -l

# Peak event rate
grep "watcher.*event" logs/watcher-daemon.log | awk '{print $1" "$2}' | uniq -c | sort -nr | head -5
```

## Optimization Tips

### For High-Frequency Changes
- Increase debounce time to batch changes
- Reduce max_events_per_second
- Use `mtime` comparison instead of hash

### For Large Files
- Set higher debounce times (10s+)
- Watch only final events (write, not create)
- Disable checksum verification

### For Critical Files
- Minimize debounce time (100ms or less)
- Watch all events
- Enable immediate conflict resolution

This file watcher configuration provides real-time synchronization with fine-tuned performance characteristics for different types of files and usage patterns.
