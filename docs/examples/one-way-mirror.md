# One-Way Mirror Sync Example

This example demonstrates setting up a one-way synchronization that mirrors a source directory to a destination, similar to `rsync --mirror`.

## Use Case

- **Source**: `/home/user/important-docs` (master copy)
- **Destination**: `/backup/important-docs` (mirror copy)
- **Behavior**: Destination exactly mirrors source (including deletions)
- **Schedule**: Sync every 30 minutes
- **Triggers**: Also sync on file changes

## Configuration

### Daemon Configuration (daemon-mirror.toml)

```toml
[pocketbase]
url = "http://localhost:8090"
admin_email = "admin@example.com"
admin_password = "admin123456"
timeout_secs = 30

[daemon]
log_level = "info"
log_file = "logs/mirror-daemon.log"

[telemetry]
console_logging = true
json_logging = true
log_file_path = "logs/mirror-daemon.log"

[telemetry.log_rotation]
enabled = true
frequency = "daily"
keep_files = 7
max_size_mb = 50

[telemetry.metrics]
enabled = true
bind_address = "127.0.0.1"
port = 9090

# One-way mirror sync job
[[sync_jobs]]
id = "important_docs_mirror"
name = "Important Documents Mirror"
source_path = "/home/user/important-docs"
destination_path = "/backup/important-docs"
enabled = true

# Exclude temporary and system files
filters = [
    "*.tmp", "*.temp", "*.swp", "*.swo", 
    ".DS_Store", "Thumbs.db", "desktop.ini",
    "~$*", ".~lock.*"
]

# Exclude version control and build directories
exclude_patterns = [
    ".git", ".svn", ".hg",
    "node_modules", "target", "dist", "build",
    "__pycache__", "*.pyc", ".cache"
]

# Scheduled sync every 30 minutes
[sync_jobs.schedule]
type = "interval"
interval = "30m"
max_run_duration_secs = 1800
skip_if_running = true

# One-way mirror options
[sync_jobs.sync_options]
dry_run = false
preserve_permissions = true
preserve_timestamps = true
preserve_ownership = false
delete_destination_files = true  # Mirror behavior - delete files not in source
comparison_method = "sha256"
ignore_hidden_files = false
continue_on_error = true
bidirectional = false
conflict_resolution = "source"   # Always prefer source
max_file_size_mb = 2000
follow_symlinks = false
verify_checksums = true

# File watcher for real-time sync
[[file_watchers]]
id = "important_docs_watcher"
name = "Important Documents Watcher"
watch_path = "/home/user/important-docs"
sync_job_id = "important_docs_mirror"
enabled = true
recursive = true
debounce_ms = 2000

# Watch for all relevant events
watch_events = ["create", "write", "remove", "rename"]

# Ignore temporary files in watcher
ignore_patterns = [
    "*.tmp", "*.temp", "*.swp", "*.swo",
    ".DS_Store", "Thumbs.db", "~$*"
]

[concurrency]
max_concurrent_syncs = 2
max_file_operations = 100
sync_queue_size = 500

[cache]
cache_dir = "/home/user/.cache/sync-mirror"
enable_persistent_cache = true
file_metadata_cache_ttl_secs = 300
max_cache_size_mb = 200
```

## Setup Script

Create `setup-mirror.sh`:

```bash
#!/bin/bash
# setup-mirror.sh - Set up one-way mirror sync

set -e

# Configuration
SOURCE_DIR="/home/user/important-docs"
DEST_DIR="/backup/important-docs"
CONFIG_FILE="daemon-mirror.toml"
LOG_DIR="logs"

echo "Setting up one-way mirror sync..."

# Create directories if they don't exist
mkdir -p "$SOURCE_DIR"
mkdir -p "$DEST_DIR"
mkdir -p "$LOG_DIR"

# Create some test files in source
if [ ! -f "$SOURCE_DIR/README.txt" ]; then
    cat > "$SOURCE_DIR/README.txt" << 'EOF'
# Important Documents

This directory contains important documents that are automatically
mirrored to the backup location.

Files added, modified, or deleted here will be reflected in the backup
within 2 seconds (via file watcher) or 30 minutes (via scheduled sync).
EOF
fi

# Create sample subdirectory and files
mkdir -p "$SOURCE_DIR/projects/project-a"
mkdir -p "$SOURCE_DIR/documents/contracts"

echo "Project A documentation" > "$SOURCE_DIR/projects/project-a/notes.txt"
echo "Contract template" > "$SOURCE_DIR/documents/contracts/template.docx"
echo "Meeting notes from $(date)" > "$SOURCE_DIR/meeting-notes-$(date +%Y%m%d).txt"

# Set appropriate permissions
chmod -R 755 "$SOURCE_DIR"
chmod -R 755 "$DEST_DIR"

echo "✅ Directories and test files created"

# Verify configuration
if command -v daemon >/dev/null 2>&1; then
    echo "🔍 Validating configuration..."
    daemon config validate --config "$CONFIG_FILE"
    echo "✅ Configuration is valid"
else
    echo "⚠️  Daemon binary not found. Please build the project first:"
    echo "   cargo build --release"
    echo "   export PATH=\"\$PATH:\$(pwd)/target/release\""
fi

echo "
🚀 Setup complete! Next steps:

1. Start PocketBase:
   ./pocketbase/setup.sh

2. Start the mirror daemon:
   daemon start --config $CONFIG_FILE

3. Monitor the sync:
   tail -f $LOG_DIR/mirror-daemon.log

4. Check metrics:
   curl http://localhost:9090/metrics

5. Test the mirror by:
   - Adding files to $SOURCE_DIR
   - Modifying existing files
   - Deleting files
   - Observe changes appear in $DEST_DIR

The mirror will:
✅ Copy new/modified files from source to destination
✅ Delete files from destination that don't exist in source
✅ Preserve timestamps and permissions
✅ Sync automatically on file changes (2s delay)
✅ Sync on schedule every 30 minutes
"
```

## Management Script

Create `manage-mirror.sh`:

```bash
#!/bin/bash
# manage-mirror.sh - Manage the mirror sync daemon

CONFIG_FILE="daemon-mirror.toml"
PID_FILE="mirror-daemon.pid"
LOG_FILE="logs/mirror-daemon.log"

case "$1" in
    start)
        echo "Starting mirror sync daemon..."
        daemon start --config "$CONFIG_FILE" --pid-file "$PID_FILE" &
        echo "Mirror daemon started (PID: $!)"
        echo "Logs: tail -f $LOG_FILE"
        echo "Metrics: curl http://localhost:9090/metrics"
        ;;
    
    stop)
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            echo "Stopping mirror daemon (PID: $PID)..."
            kill "$PID"
            rm -f "$PID_FILE"
            echo "Mirror daemon stopped"
        else
            echo "No PID file found. Daemon may not be running."
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
                echo "Mirror daemon is running (PID: $PID)"
                
                # Show recent sync activity
                echo "Recent sync activity:"
                grep -i "sync.*complete\|sync.*failed" "$LOG_FILE" | tail -5
                
                # Show metrics
                echo "Current metrics:"
                curl -s http://localhost:9090/metrics | grep -E "(sync_operations|sync_files)" | head -5
            else
                echo "PID file exists but daemon is not running"
                rm -f "$PID_FILE"
            fi
        else
            echo "Mirror daemon is not running"
        fi
        ;;
    
    logs)
        if [ -f "$LOG_FILE" ]; then
            tail -f "$LOG_FILE"
        else
            echo "Log file not found: $LOG_FILE"
        fi
        ;;
    
    test)
        SOURCE_DIR="/home/user/important-docs"
        echo "Testing mirror sync..."
        
        # Create test file
        TEST_FILE="$SOURCE_DIR/test-$(date +%s).txt"
        echo "Test file created at $(date)" > "$TEST_FILE"
        echo "Created test file: $TEST_FILE"
        
        # Wait for sync
        echo "Waiting 5 seconds for file watcher to trigger sync..."
        sleep 5
        
        # Check if file appears in destination
        DEST_FILE="/backup/important-docs/test-$(basename "$TEST_FILE" | cut -d'-' -f2)"
        if [ -f "/backup$TEST_FILE" ]; then
            echo "✅ Mirror sync working! File found in destination."
        else
            echo "❌ Mirror sync issue. File not found in destination."
            echo "Check logs: $0 logs"
        fi
        ;;
    
    force-sync)
        echo "Triggering manual sync via API..."
        # This would require implementing an API endpoint in the daemon
        echo "Manual sync not yet implemented. Restart daemon to force sync:"
        echo "$0 restart"
        ;;
    
    *)
        echo "Usage: $0 {start|stop|restart|status|logs|test|force-sync}"
        echo ""
        echo "Commands:"
        echo "  start      - Start the mirror daemon"
        echo "  stop       - Stop the mirror daemon"
        echo "  restart    - Restart the mirror daemon"
        echo "  status     - Show daemon status and recent activity"
        echo "  logs       - Follow daemon logs"
        echo "  test       - Test mirror functionality"
        echo "  force-sync - Trigger manual sync"
        exit 1
        ;;
esac
```

## Usage Examples

### 1. Initial Setup

```bash
# Make scripts executable
chmod +x setup-mirror.sh manage-mirror.sh

# Run setup
./setup-mirror.sh

# Start PocketBase
./pocketbase/setup.sh

# Start mirror daemon
./manage-mirror.sh start
```

### 2. Testing the Mirror

```bash
# Test automatic mirroring
./manage-mirror.sh test

# Add some files manually
echo "New document" > /home/user/important-docs/new-file.txt
mkdir /home/user/important-docs/new-folder
echo "Folder content" > /home/user/important-docs/new-folder/content.txt

# Wait a few seconds and check destination
ls -la /backup/important-docs/
```

### 3. Monitoring

```bash
# Check daemon status
./manage-mirror.sh status

# Follow logs
./manage-mirror.sh logs

# View metrics
curl -s http://localhost:9090/metrics | grep sync

# Check specific metrics
curl -s http://localhost:9090/metrics | grep sync_operations_total
curl -s http://localhost:9090/metrics | grep sync_files_processed_total
```

### 4. Maintenance

```bash
# Restart daemon (e.g., after config changes)
./manage-mirror.sh restart

# Stop daemon
./manage-mirror.sh stop

# View configuration validation
daemon config validate --config daemon-mirror.toml
```

## Expected Behavior

### ✅ What Gets Mirrored

- All files and directories from source
- File permissions and timestamps
- Directory structure
- File modifications and creations

### ✅ What Gets Deleted

- Files in destination that don't exist in source
- Empty directories after file deletions
- Files matching exclusion patterns

### ✅ What Gets Excluded

- Temporary files (*.tmp, *.temp, etc.)
- System files (.DS_Store, Thumbs.db)
- Version control directories (.git, .svn)
- Build artifacts (node_modules, target)

### ⚡ Sync Triggers

- **File watcher**: 2-second delay after file changes
- **Scheduled**: Every 30 minutes
- **Manual**: Daemon restart or API call

## Monitoring and Alerts

### Key Metrics to Monitor

```bash
# Success rate
curl -s http://localhost:9090/metrics | grep sync_operations_total

# File throughput
curl -s http://localhost:9090/metrics | grep sync_files_processed_total

# Error rate
curl -s http://localhost:9090/metrics | grep sync_errors_total

# Sync duration
curl -s http://localhost:9090/metrics | grep sync_operations_duration_seconds
```

### Log Patterns

```bash
# Successful syncs
grep "Sync job completed successfully" logs/mirror-daemon.log

# Failed syncs
grep "Sync job failed" logs/mirror-daemon.log

# File operations
grep -E "(copied|deleted|updated)" logs/mirror-daemon.log

# Watcher events
grep "File watcher triggered" logs/mirror-daemon.log
```

This one-way mirror configuration provides robust, reliable file synchronization with real-time change detection and comprehensive monitoring. The setup is ideal for backup scenarios where you need an exact replica of your source directory.
