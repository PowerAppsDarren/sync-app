# Sync Dashboard TUI

A terminal user interface (TUI) for monitoring and controlling synchronization jobs with real-time updates from PocketBase.

## Features

### 🏠 Home View
- **Job List**: Display all synchronization jobs with their current status
- **Progress Bars**: Visual progress indicators for running jobs
- **Job Information**: Source/target paths, file counts, and data transfer progress
- **Status Colors**: Color-coded job statuses (Running: Green, Failed: Red, etc.)
- **Priority Sorting**: Jobs sorted by priority (Critical → High → Normal → Low)

### 📋 Job Detail View
- **Detailed Information**: Complete job metadata and statistics
- **Real-time Action Log**: Live log entries with timestamps and severity levels
- **Progress Tracking**: Files processed, bytes transferred, and completion percentage
- **Error Display**: Any errors encountered during synchronization

### ⚡ Conflict Resolution View
- **Conflict List**: All file conflicts that require manual resolution
- **Conflict Details**: File paths, modification times, and conflict types
- **Resolution Options**: Keep Source, Keep Target, Merge, or Skip
- **Progress Tracking**: Shows resolved vs pending conflicts

### 🔧 Settings View
- **Connection Status**: WebSocket connection state to PocketBase
- **System Information**: Job counts, last update time, and configuration
- **Live Statistics**: Real-time dashboard metrics

### ❓ Help View
- **Key Bindings**: Complete list of keyboard shortcuts
- **Navigation Help**: How to move between views and select items
- **Feature Guide**: Instructions for job control and conflict resolution

## Key Bindings

### Global Navigation
| Key | Action |
|-----|--------|
| `h` or `?` | Show help |
| `q` | Quit application (from Home view) |
| `Ctrl+C` | Force quit |
| `1` | Switch to Home view |
| `2` | Switch to Settings view |
| `Esc` | Go back/Return to previous view |

### Home View
| Key | Action |
|-----|--------|
| `↑`/`k` | Move selection up |
| `↓`/`j` | Move selection down |
| `Enter` | Enter job detail view |
| `s` | Start selected job |
| `p` | Pause selected job |
| `t` | Stop selected job |
| `r` | Retry failed job |
| `c` | View conflicts (if any) |
| `Page Up` | Move up 5 items |
| `Page Down` | Move down 5 items |
| `Home` | Go to first job |
| `End` | Go to last job |

### Job Detail View
| Key | Action |
|-----|--------|
| `↑`/`k` | Scroll log up |
| `↓`/`j` | Scroll log down |
| `s` | Start job |
| `p` | Pause job |
| `t` | Stop job |
| `r` | Retry job |
| `c` | View conflicts |
| `Page Up`/`Page Down` | Scroll log by 5 entries |

### Conflict Resolution View
| Key | Action |
|-----|--------|
| `↑`/`k` | Select previous conflict |
| `↓`/`j` | Select next conflict |
| `1` | Resolve: Keep Source |
| `2` | Resolve: Keep Target |
| `3` | Resolve: Merge |
| `4` | Resolve: Skip |
| `Enter` | Apply default resolution (Keep Source) |

## Job Status Colors

- 🟢 **Green**: Running - Job is actively synchronizing
- 🟡 **Yellow**: Paused - Job temporarily stopped by user
- 🔴 **Red**: Failed - Job encountered errors and stopped
- 🔵 **Blue**: Completed - Job finished successfully
- ⚪ **Gray**: Cancelled - Job was cancelled by user
- 🔷 **Cyan**: Pending - Job waiting to start

## Real-time Updates

The dashboard connects to PocketBase via WebSocket for real-time updates:

- **Job Progress**: Live progress bars and statistics
- **Status Changes**: Immediate updates when jobs start, pause, or complete
- **Action Logs**: Real-time log entries as they occur
- **Conflict Notifications**: Instant alerts when conflicts need resolution
- **Error Reporting**: Live error messages and diagnostics

## Connection States

### 🟢 Connected
- WebSocket connection to PocketBase is active
- Real-time updates are working
- Job control commands can be sent

### 🔴 Disconnected
- No connection to PocketBase
- Dashboard runs in offline mode
- Last known state is displayed
- Automatic reconnection attempts in progress

## Usage Examples

### Starting the Dashboard
```bash
# Run the TUI dashboard
cargo run --bin ui

# Or with specific log level
RUST_LOG=debug cargo run --bin ui
```

### Basic Workflow
1. Launch the dashboard
2. Browse jobs in the Home view
3. Select a job and press `Enter` for details
4. Use `s`/`p`/`t` keys to control job execution
5. Press `c` to resolve any conflicts
6. Use `h` for help at any time

### Conflict Resolution
1. Navigate to a job with conflicts
2. Press `c` to enter conflict resolution view
3. Use `↑`/`↓` to select conflicts
4. Press `1`-`4` to choose resolution:
   - `1`: Keep source file
   - `2`: Keep target file
   - `3`: Attempt merge
   - `4`: Skip this file
5. Press `Esc` to return to job details

## Configuration

The dashboard uses the default PocketBase configuration:
- **URL**: `http://localhost:8090`
- **WebSocket**: `ws://localhost:8090/ws`
- **Auto-reconnect**: Enabled with exponential backoff
- **Update Frequency**: 250ms refresh rate

## Development

### Demo Mode
The dashboard includes demo jobs for testing:
- Documents Backup (Running, 65% complete)
- Photos Sync (Paused with conflicts)
- Code Repository Backup (Failed)
- Music Library (Completed)

### Building
```bash
# Build the UI crate
cargo build -p ui

# Run tests
cargo test -p ui

# Build release version
cargo build -p ui --release
```

### Dependencies
- **ratatui**: Terminal UI framework
- **crossterm**: Cross-platform terminal manipulation
- **tokio-tungstenite**: WebSocket client
- **serde**: JSON serialization
- **chrono**: Date/time handling
- **uuid**: Unique identifiers
- **tracing**: Logging and diagnostics

## Troubleshooting

### WebSocket Connection Issues
- Ensure PocketBase is running on `localhost:8090`
- Check firewall settings
- Verify WebSocket endpoint is accessible
- Dashboard continues to work offline if connection fails

### Performance
- Terminal refresh rate: 250ms
- Log entries limited to 1000 per job
- Efficient rendering with minimal redraws
- Responsive input handling

### Logging
Set log level for debugging:
```bash
RUST_LOG=debug cargo run --bin ui
```

## Architecture

```
ui/
├── src/
│   ├── main.rs          # Application entry point and loop
│   ├── types.rs         # Data structures and state management
│   ├── ui.rs           # TUI rendering and layout
│   ├── events.rs       # Keyboard input handling
│   └── websocket.rs    # Real-time communication
├── Cargo.toml          # Dependencies and configuration
└── README.md           # This documentation
```

The dashboard follows a clean architecture with separation of concerns:
- **Types**: Pure data structures and business logic
- **UI**: Rendering and visual presentation
- **Events**: Input handling and user interaction
- **WebSocket**: Real-time communication and state updates
- **Main**: Application lifecycle and coordination
