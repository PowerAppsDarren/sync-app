# Sync CLI Tool Usage

The sync CLI tool provides a comprehensive command-line interface for managing synchronization configurations and operations with PocketBase backend.

## Installation

Build the CLI from source:
```bash
cargo build --release --bin sync
```

The binary will be available at `target/release/sync` (or `sync.exe` on Windows).

## Basic Usage

```bash
sync [OPTIONS] <COMMAND>
```

### Global Options

- `-v, --verbose`: Enable verbose output with debug information
- `-q, --quiet`: Suppress non-essential messages (conflicts with verbose)
- `--format <FORMAT>`: Output format - `human` (default) or `json`
- `--config <CONFIG>`: Custom config file path (default: system config directory)

## Commands

### Configuration Management

#### Add a new configuration
```bash
sync add --name "my-config" --source "./source" --dest "./backup"
```

Options:
- `--name <NAME>`: Configuration name (required)
- `--source <PATH>`: Source directory path (required)
- `--dest <PATH>`: Destination directory path (required)
- `--pocketbase-url <URL>`: PocketBase URL (default: http://localhost:8090)
- `--admin-email <EMAIL>`: Admin email for PocketBase
- `--admin-password <PASSWORD>`: Admin password for PocketBase
- `--remote`: Save to PocketBase instead of local config

#### List configurations
```bash
sync list [--detailed]
```

Options:
- `--detailed`: Show detailed information including paths, timestamps

#### Edit a configuration
```bash
sync edit <CONFIG_ID> [OPTIONS]
```

Options:
- `--name <NAME>`: Update configuration name
- `--source <PATH>`: Update source directory path
- `--dest <PATH>`: Update destination directory path
- `--pocketbase-url <URL>`: Update PocketBase URL
- `--remote`: Edit in PocketBase instead of local config

#### Remove a configuration
```bash
sync remove <CONFIG_ID> [--yes] [--remote]
```

Options:
- `--yes`: Skip confirmation prompt
- `--remote`: Remove from PocketBase instead of local config

### Synchronization Operations

#### Run synchronization
```bash
sync run <CONFIG_ID> [OPTIONS]
```

Options:
- `--dry-run <true|false>`: Override dry-run setting
- `--force`: Force sync even if conflicts exist

#### Perform a dry run
```bash
sync dry-run <CONFIG_ID> [--detailed]
```

Shows planned actions without executing them.

Options:
- `--detailed`: Show file-level details

### Import/Export

#### Import configurations
```bash
sync import <FILE> [--remote] [--overwrite]
```

Options:
- `--remote`: Import to PocketBase instead of local storage
- `--overwrite`: Overwrite existing configurations

Supports both JSON and YAML formats (detected by file extension).

#### Export configurations
```bash
sync export <FILE> [OPTIONS]
```

Options:
- `--export-format <FORMAT>`: Export format - `json` (default) or `yaml`
- `--remote`: Export from PocketBase instead of local storage
- `--config-id <ID>`: Export specific config (exports all if not specified)

### Health Check

#### Check service health
```bash
sync health [--url <URL>]
```

Options:
- `--url <URL>`: PocketBase URL to check (default: http://localhost:8090)

## Examples

### Basic workflow
```bash
# Add a new configuration
sync add --name "documents" --source "~/Documents" --dest "~/backup/docs"

# List all configurations
sync list

# Run a dry run to see what would be synced
sync dry-run <config-id> --detailed

# Run the actual sync
sync run <config-id>

# Export configurations for backup
sync export ./my-configs.json

# Import configurations on another machine
sync import ./my-configs.json
```

### JSON output mode
```bash
# Get configuration list in JSON format
sync --format json list

# Check health with JSON output
sync --format json health

# Get detailed dry-run results in JSON
sync --format json dry-run <config-id> --detailed
```

### Verbose debugging
```bash
# Run with verbose output for debugging
sync --verbose run <config-id>

# Quiet mode for scripts
sync --quiet run <config-id>
```

## Configuration File Format

The CLI stores configurations in JSON format. Example structure:

```json
{
  "configs": {
    "config-id": {
      "id": "uuid",
      "name": "config-name",
      "source_path": "/path/to/source",
      "destination_path": "/path/to/dest",
      "pocketbase_url": "http://localhost:8090",
      "admin_email": null,
      "admin_password": null,
      "filters": [],
      "exclude_patterns": [".git", "node_modules"],
      "dry_run": false,
      "preserve_permissions": true,
      "created_at": "2025-06-28T00:00:00Z",
      "updated_at": "2025-06-28T00:00:00Z"
    }
  },
  "default_config": null
}
```

## Exit Codes

- `0`: Success
- `1`: General error (health check failure, configuration not found, etc.)
- `101`: Panic or critical error

## Notes

- Configuration IDs are UUIDs generated automatically
- Local configurations are stored in the system config directory by default
- Remote operations with PocketBase are not yet fully implemented
- The actual sync engine integration is pending implementation
