# Bidirectional Sync with Cron Example

This example demonstrates setting up bidirectional synchronization with cron scheduling for keeping two directories in sync across different locations.

## Use Case

- **Location A**: `/home/user/shared-docs` (primary workspace)
- **Location B**: `/mnt/remote/shared-docs` (remote/network share)
- **Behavior**: Both locations stay synchronized, conflicts resolved intelligently
- **Schedule**: Sync every 2 hours during business hours, daily full sync at midnight
- **Conflict Resolution**: Newer files win, with detailed logging

## Configuration

### Daemon Configuration (daemon-bidirectional.toml)

```toml
[pocketbase]
url = "http://localhost:8090"
admin_email = "admin@example.com"
admin_password = "admin123456"
timeout_secs = 45
retry_attempts = 5
retry_delay_secs = 10

[daemon]
log_level = "info"
log_file = "logs/bidirectional-daemon.log"
config_refresh_interval_secs = 600
auto_restart_on_config_change = true

[telemetry]
console_logging = true
json_logging = true
log_file_path = "logs/bidirectional-daemon.log"

[telemetry.log_rotation]
enabled = true
frequency = "daily"
keep_files = 14
max_size_mb = 100

[telemetry.pocketbase_logging]
enabled = true
collection = "bidirectional_logs"
batch_size = 50
flush_interval_secs = 60
max_retries = 5

[telemetry.metrics]
enabled = true
bind_address = "127.0.0.1"
port = 9091  # Different port to avoid conflicts

# Business hours sync (every 2 hours, 8 AM to 6 PM)
[[sync_jobs]]
id = "shared_docs_business_hours"
name = "Shared Documents - Business Hours"
source_path = "/home/user/shared-docs"
destination_path = "/mnt/remote/shared-docs"
enabled = true
priority = 1

# Exclude files that change frequently or are user-specific
filters = [
    "*.tmp", "*.temp", "*.swp", "*.swo", "*.bak",
    ".DS_Store", "Thumbs.db", "desktop.ini",
    "~$*", ".~lock.*", "*.lnk",
    ".directory", ".fuse_hidden*"
]

exclude_patterns = [
    ".git", ".svn", ".hg", ".bzr",
    "node_modules", "target", "dist", "build", "out",
    "__pycache__", "*.pyc", "*.pyo", ".cache",
    ".idea", ".vscode", ".vs",
    "*.log", "logs/", "tmp/"
]

# Business hours schedule (every 2 hours from 8 AM to 6 PM, Monday to Friday)
[sync_jobs.schedule]
type = "cron"
expression = "0 8-18/2 * * 1-5"  # At minute 0 past every 2nd hour from 8 through 18 on Monday through Friday
timezone = "America/New_York"
max_run_duration_secs = 3600
skip_if_running = true

# Bidirectional sync options
[sync_jobs.sync_options]
dry_run = false
preserve_permissions = true
preserve_timestamps = true
preserve_ownership = true
delete_destination_files = false  # Conservative - don't delete files
comparison_method = "sha256"
ignore_hidden_files = true
continue_on_error = true
bidirectional = true               # Enable bidirectional sync
conflict_resolution = "newer"      # Newer file wins conflicts
max_file_size_mb = 500
follow_symlinks = false
verify_checksums = true

# Full sync job (daily at midnight)
[[sync_jobs]]
id = "shared_docs_daily_full"
name = "Shared Documents - Daily Full Sync"
source_path = "/home/user/shared-docs"
destination_path = "/mnt/remote/shared-docs"
enabled = true
priority = 2

# Same filters as business hours sync
filters = [
    "*.tmp", "*.temp", "*.swp", "*.swo", "*.bak",
    ".DS_Store", "Thumbs.db", "desktop.ini",
    "~$*", ".~lock.*", "*.lnk"
]

exclude_patterns = [
    ".git", ".svn", ".hg",
    "node_modules", "target", "dist", "build",
    "__pycache__", "*.pyc", ".cache",
    ".idea", ".vscode"
]

# Daily full sync at midnight
[sync_jobs.schedule]
type = "cron"
expression = "0 0 * * *"  # Daily at midnight
timezone = "America/New_York"
max_run_duration_secs = 7200  # 2 hours max
skip_if_running = false  # Force run even if business hours sync is running

# More thorough sync options for daily run
[sync_jobs.sync_options]
dry_run = false
preserve_permissions = true
preserve_timestamps = true
preserve_ownership = true
delete_destination_files = true   # Clean up orphaned files in daily sync
comparison_method = "sha256"
ignore_hidden_files = false       # Include hidden files in full sync
continue_on_error = false          # Fail fast in full sync
bidirectional = true
conflict_resolution = "newer"
max_file_size_mb = 2000           # Allow larger files in full sync
follow_symlinks = false
verify_checksums = true

# Optional file watcher for immediate sync of critical changes
[[file_watchers]]
id = "critical_docs_watcher"
name = "Critical Documents Watcher"
watch_path = "/home/user/shared-docs/critical"
sync_job_id = "shared_docs_business_hours"
enabled = true
recursive = true
debounce_ms = 5000  # 5 second delay to batch changes

# Only watch for important events
watch_events = ["create", "write", "remove"]

# Ignore temporary files in watcher
ignore_patterns = [
    "*.tmp", "*.temp", "*.swp", "*.swo",
    ".DS_Store", "Thumbs.db", "~$*",
    "*.autosave", "*.backup"
]
max_events_per_second = 50

[concurrency]
max_concurrent_syncs = 2  # Allow business and full sync to overlap if needed
max_file_operations = 200
sync_queue_size = 1000
thread_pool_size = 6

[cache]
cache_dir = "/home/user/.cache/sync-bidirectional"
enable_persistent_cache = true
file_metadata_cache_ttl_secs = 600  # 10 minutes cache
max_cache_size_mb = 500
cache_cleanup_interval_secs = 3600
```

## Setup Script

Create `setup-bidirectional.sh`:

```bash
#!/bin/bash
# setup-bidirectional.sh - Set up bidirectional sync with cron scheduling

set -e

# Configuration
SOURCE_DIR="/home/user/shared-docs"
DEST_DIR="/mnt/remote/shared-docs"
CRITICAL_DIR="$SOURCE_DIR/critical"
CONFIG_FILE="daemon-bidirectional.toml"
LOG_DIR="logs"

echo "Setting up bidirectional sync with cron scheduling..."

# Create directories
mkdir -p "$SOURCE_DIR"
mkdir -p "$DEST_DIR"
mkdir -p "$CRITICAL_DIR"
mkdir -p "$LOG_DIR"

# Create test directory structure
mkdir -p "$SOURCE_DIR/projects/project-alpha"
mkdir -p "$SOURCE_DIR/projects/project-beta"
mkdir -p "$SOURCE_DIR/documents/contracts"
mkdir -p "$SOURCE_DIR/documents/proposals"
mkdir -p "$SOURCE_DIR/shared/templates"
mkdir -p "$CRITICAL_DIR/urgent"

# Create test files
cat > "$SOURCE_DIR/README.md" << 'EOF'
# Shared Documents

This directory is synchronized bidirectionally with the remote location.

## Sync Schedule
- Business hours: Every 2 hours (8 AM - 6 PM, Mon-Fri)
- Full sync: Daily at midnight
- Critical docs: Real-time sync with 5-second delay

## Conflict Resolution
- Newer files automatically win conflicts
- Detailed logging available in logs/
- Full sync performs cleanup of orphaned files

## Directory Structure
- `/projects/` - Active project files
- `/documents/` - Formal documents and contracts
- `/shared/` - Shared resources and templates
- `/critical/` - Files that sync immediately (watched)
EOF

echo "Project Alpha documentation" > "$SOURCE_DIR/projects/project-alpha/README.md"
echo "Project Beta specifications" > "$SOURCE_DIR/projects/project-beta/specs.txt"
echo "Contract template v1.0" > "$SOURCE_DIR/documents/contracts/template.docx"
echo "Proposal template" > "$SOURCE_DIR/documents/proposals/template.pptx"
echo "Company letterhead template" > "$SOURCE_DIR/shared/templates/letterhead.docx"

# Critical files that need immediate sync
echo "URGENT: Server maintenance scheduled" > "$CRITICAL_DIR/urgent/maintenance-notice.txt"
echo "Emergency contact information" > "$CRITICAL_DIR/emergency-contacts.txt"

# Create sample conflict scenario files
echo "This file was modified on location A at $(date)" > "$SOURCE_DIR/test-conflict.txt"
if [ -d "$DEST_DIR" ]; then
    echo "This file was modified on location B at $(date -d '1 hour ago')" > "$DEST_DIR/test-conflict.txt"
fi

# Set permissions
chmod -R 755 "$SOURCE_DIR"
chmod -R 755 "$DEST_DIR" 2>/dev/null || true

echo "✅ Directory structure and test files created"

# Create cron schedule reference
cat > cron-schedule.txt << 'EOF'
# Sync App Bidirectional Cron Schedule

The daemon manages these cron-based sync jobs:

1. Business Hours Sync (shared_docs_business_hours)
   - Schedule: 0 8-18/2 * * 1-5
   - Description: Every 2 hours from 8 AM to 6 PM, Monday to Friday
   - Times: 8:00 AM, 10:00 AM, 12:00 PM, 2:00 PM, 4:00 PM, 6:00 PM
   - Conservative sync (no file deletions)

2. Daily Full Sync (shared_docs_daily_full)
   - Schedule: 0 0 * * *
   - Description: Every day at midnight
   - Full cleanup and comprehensive sync

3. Critical Files Watcher (critical_docs_watcher)
   - Real-time sync for /critical/ directory
   - 5-second debounce delay
   - Immediate sync for urgent changes

All times are in America/New_York timezone.
EOF

echo "✅ Cron schedule reference created: cron-schedule.txt"

# Validate configuration
if command -v daemon >/dev/null 2>&1; then
    echo "🔍 Validating configuration..."
    daemon config validate --config "$CONFIG_FILE"
    echo "✅ Configuration is valid"
else
    echo "⚠️  Daemon binary not found. Please build first:"
    echo "   cargo build --release"
fi

echo "
🚀 Bidirectional sync setup complete!

📁 Directory Structure:
   Source: $SOURCE_DIR
   Destination: $DEST_DIR
   Critical: $CRITICAL_DIR

⏰ Sync Schedule:
   Business hours: Every 2 hours (8 AM - 6 PM, Mon-Fri)
   Daily full sync: Midnight
   Critical files: Real-time (5s delay)

📋 Next Steps:
1. Start PocketBase: ./pocketbase/setup.sh
2. Start daemon: daemon start --config $CONFIG_FILE
3. Monitor: tail -f $LOG_DIR/bidirectional-daemon.log
4. Check metrics: curl http://localhost:9091/metrics
5. Review schedule: cat cron-schedule.txt

🔄 Test Scenarios:
- Add files to either location
- Modify same file in both locations (test conflict resolution)
- Add files to critical/ directory (test immediate sync)
- Wait for scheduled sync times
"
```

## Management Script

Create `manage-bidirectional.sh`:

```bash
#!/bin/bash
# manage-bidirectional.sh - Manage bidirectional sync daemon

CONFIG_FILE="daemon-bidirectional.toml"
PID_FILE="bidirectional-daemon.pid"
LOG_FILE="logs/bidirectional-daemon.log"
METRICS_PORT="9091"

show_next_sync_times() {
    echo "📅 Next Scheduled Sync Times:"
    echo "Business Hours (Every 2 hours, 8-18, Mon-Fri):"
    
    # Calculate next business hour sync
    current_hour=$(date +%H)
    current_dow=$(date +%u)  # 1=Monday, 7=Sunday
    
    if [ $current_dow -le 5 ]; then  # Monday to Friday
        if [ $current_hour -lt 8 ]; then
            echo "  Next: Today at 8:00 AM"
        elif [ $current_hour -lt 10 ]; then
            echo "  Next: Today at 10:00 AM"
        elif [ $current_hour -lt 12 ]; then
            echo "  Next: Today at 12:00 PM"
        elif [ $current_hour -lt 14 ]; then
            echo "  Next: Today at 2:00 PM"
        elif [ $current_hour -lt 16 ]; then
            echo "  Next: Today at 4:00 PM"
        elif [ $current_hour -lt 18 ]; then
            echo "  Next: Today at 6:00 PM"
        else
            echo "  Next: Tomorrow at 8:00 AM (if weekday)"
        fi
    else
        echo "  Next: Monday at 8:00 AM"
    fi
    
    echo "Daily Full Sync: Every day at 12:00 AM"
    echo "Critical Files: Real-time (5s delay)"
}

show_conflict_report() {
    echo "🔍 Recent Conflict Resolution:"
    if [ -f "$LOG_FILE" ]; then
        grep -i "conflict\|resolution\|newer.*chosen" "$LOG_FILE" | tail -10
    else
        echo "No log file found"
    fi
}

show_sync_stats() {
    echo "📊 Sync Statistics:"
    if command -v curl >/dev/null 2>&1; then
        echo "Checking metrics on port $METRICS_PORT..."
        
        # Total operations
        total_ops=$(curl -s "http://localhost:$METRICS_PORT/metrics" | grep 'sync_operations_total' | head -1 | awk '{print $2}' || echo "0")
        echo "Total sync operations: $total_ops"
        
        # File counts
        files_processed=$(curl -s "http://localhost:$METRICS_PORT/metrics" | grep 'sync_files_processed_total' | head -1 | awk '{print $2}' || echo "0")
        echo "Files processed: $files_processed"
        
        # Error count
        errors=$(curl -s "http://localhost:$METRICS_PORT/metrics" | grep 'sync_errors_total' | head -1 | awk '{print $2}' || echo "0")
        echo "Sync errors: $errors"
        
        # Active jobs
        active_jobs=$(curl -s "http://localhost:$METRICS_PORT/metrics" | grep 'active_sync_jobs' | head -1 | awk '{print $2}' || echo "0")
        echo "Active sync jobs: $active_jobs"
    else
        echo "curl not available for metrics"
    fi
}

case "$1" in
    start)
        echo "Starting bidirectional sync daemon..."
        daemon start --config "$CONFIG_FILE" --pid-file "$PID_FILE" &
        echo "Daemon started (PID: $!)"
        echo "Logs: tail -f $LOG_FILE"
        echo "Metrics: curl http://localhost:$METRICS_PORT/metrics"
        show_next_sync_times
        ;;
    
    stop)
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            echo "Stopping bidirectional sync daemon (PID: $PID)..."
            kill "$PID"
            rm -f "$PID_FILE"
            echo "Daemon stopped"
        else
            echo "No PID file found"
        fi
        ;;
    
    restart)
        $0 stop
        sleep 3
        $0 start
        ;;
    
    status)
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if kill -0 "$PID" 2>/dev/null; then
                echo "✅ Bidirectional sync daemon is running (PID: $PID)"
                show_next_sync_times
                echo ""
                show_sync_stats
                echo ""
                echo "Recent sync activity:"
                grep -i "sync.*complete\|sync.*failed\|conflict" "$LOG_FILE" | tail -5
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
    
    conflicts)
        show_conflict_report
        ;;
    
    schedule)
        show_next_sync_times
        echo ""
        cat cron-schedule.txt 2>/dev/null || echo "Schedule file not found"
        ;;
    
    test-conflict)
        SOURCE_DIR="/home/user/shared-docs"
        DEST_DIR="/mnt/remote/shared-docs"
        
        echo "Creating conflict test scenario..."
        
        # Create conflicting files
        TEST_FILE="conflict-test-$(date +%s).txt"
        echo "Source version modified at $(date)" > "$SOURCE_DIR/$TEST_FILE"
        
        if [ -d "$DEST_DIR" ]; then
            echo "Destination version modified at $(date -d '30 minutes ago')" > "$DEST_DIR/$TEST_FILE"
            echo "Created conflicting files:"
            echo "  Source: $SOURCE_DIR/$TEST_FILE"
            echo "  Dest:   $DEST_DIR/$TEST_FILE"
            echo ""
            echo "Wait for next sync and check logs for conflict resolution:"
            echo "  $0 conflicts"
        else
            echo "Destination directory not accessible: $DEST_DIR"
        fi
        ;;
    
    test-critical)
        CRITICAL_DIR="/home/user/shared-docs/critical"
        if [ -d "$CRITICAL_DIR" ]; then
            TEST_FILE="urgent-$(date +%s).txt"
            echo "URGENT: Test critical file created at $(date)" > "$CRITICAL_DIR/$TEST_FILE"
            echo "Created critical file: $CRITICAL_DIR/$TEST_FILE"
            echo "Should sync within 5 seconds via file watcher"
            echo "Monitor with: $0 logs"
        else
            echo "Critical directory not found: $CRITICAL_DIR"
        fi
        ;;
    
    metrics)
        if command -v curl >/dev/null 2>&1; then
            echo "📊 Full Metrics Report:"
            curl -s "http://localhost:$METRICS_PORT/metrics" | grep -E "sync_|daemon_|pocketbase_" | sort
        else
            echo "curl not available for metrics"
        fi
        ;;
    
    force-sync)
        echo "Manual sync trigger not yet implemented"
        echo "Alternative: restart daemon to trigger immediate sync"
        echo "$0 restart"
        ;;
    
    *)
        echo "Usage: $0 {start|stop|restart|status|logs|conflicts|schedule|test-conflict|test-critical|metrics|force-sync}"
        echo ""
        echo "Commands:"
        echo "  start         - Start the bidirectional sync daemon"
        echo "  stop          - Stop the daemon"
        echo "  restart       - Restart the daemon"
        echo "  status        - Show daemon status and next sync times"
        echo "  logs          - Follow daemon logs"
        echo "  conflicts     - Show recent conflict resolutions"
        echo "  schedule      - Show sync schedule information"
        echo "  test-conflict - Create test files to trigger conflict resolution"
        echo "  test-critical - Create test file in critical directory"
        echo "  metrics       - Show detailed metrics"
        echo "  force-sync    - Trigger manual sync (placeholder)"
        exit 1
        ;;
esac
```

## Usage Examples

### 1. Initial Setup and Testing

```bash
# Setup
chmod +x setup-bidirectional.sh manage-bidirectional.sh
./setup-bidirectional.sh

# Start services
./pocketbase/setup.sh
./manage-bidirectional.sh start

# Check status and schedule
./manage-bidirectional.sh status
./manage-bidirectional.sh schedule
```

### 2. Conflict Resolution Testing

```bash
# Create conflict scenario
./manage-bidirectional.sh test-conflict

# Wait for next sync and check resolution
./manage-bidirectional.sh conflicts

# Monitor real-time
./manage-bidirectional.sh logs
```

### 3. Critical File Testing

```bash
# Test immediate sync
./manage-bidirectional.sh test-critical

# Verify quick sync in logs
./manage-bidirectional.sh logs | grep -i critical
```

### 4. Monitoring and Maintenance

```bash
# Check comprehensive status
./manage-bidirectional.sh status

# View all metrics
./manage-bidirectional.sh metrics

# Monitor conflict resolution
watch "./manage-bidirectional.sh conflicts"

# Schedule information
./manage-bidirectional.sh schedule
```

## Conflict Resolution Strategy

### How Conflicts Are Resolved

1. **Newer File Wins**: File with more recent modification time is kept
2. **Detailed Logging**: All conflicts logged with timestamps and resolution
3. **Backup Creation**: Original conflicting file may be backed up (configurable)
4. **User Notification**: Conflicts logged for review

### Example Conflict Scenario

```
Source:      /home/user/shared-docs/document.txt (modified 2024-01-15 14:30)
Destination: /mnt/remote/shared-docs/document.txt (modified 2024-01-15 14:25)
Resolution:  Source file wins (newer by 5 minutes)
Action:      Destination file overwritten
Log Entry:   "Conflict resolved: newer source file chosen for document.txt"
```

## Schedule Overview

### Business Hours Sync (Every 2 hours)
- **Times**: 8:00 AM, 10:00 AM, 12:00 PM, 2:00 PM, 4:00 PM, 6:00 PM
- **Days**: Monday through Friday
- **Behavior**: Conservative sync, no file deletions
- **Duration**: Up to 1 hour max

### Daily Full Sync (Midnight)
- **Time**: 12:00 AM (midnight) daily
- **Behavior**: Comprehensive sync with cleanup
- **File Deletions**: Enabled to remove orphaned files
- **Duration**: Up to 2 hours max

### Critical File Watcher
- **Directory**: `/critical/` subdirectory
- **Delay**: 5 seconds after file changes
- **Events**: Create, modify, delete
- **Behavior**: Immediate bidirectional sync

## Monitoring Dashboard

### Key Metrics
```bash
# Sync success rate over time
curl -s http://localhost:9091/metrics | grep sync_operations_total

# Conflict resolution frequency
grep -c "conflict" logs/bidirectional-daemon.log

# Average sync duration
curl -s http://localhost:9091/metrics | grep sync_operations_duration_seconds

# File throughput
curl -s http://localhost:9091/metrics | grep sync_files_processed_total
```

This bidirectional sync configuration provides robust, scheduled synchronization with intelligent conflict resolution, making it ideal for collaborative environments where multiple users need access to shared files across different locations.
