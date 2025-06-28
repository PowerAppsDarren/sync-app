# Sync Engine Library

A comprehensive async file synchronization library built in Rust providing powerful directory synchronization capabilities with advanced features.

## Features

### Core Functionality
- **Async Directory Scanning** using `walkdir`, `ignore`, and `tokio::fs`
- **Multiple File Comparison Methods**: timestamp, size, SHA-256, Blake3, or byte-by-byte
- **Intelligent Diff Algorithm** producing sync actions (copy, update, delete, conflict)
- **Configurable Conflict Resolution** strategies
- **Advanced File Filtering** with globset patterns
- **Attribute & Permission Preservation** using `fs_extra` and `utime`

### Advanced Features
- **Dry-run Mode** for preview without modifications
- **Progress Reporting** with detailed channels
- **Comprehensive Metrics** and statistics
- **Cross-platform Support** (Windows, macOS, Linux)
- **Configurable Concurrency** and buffering
- **Error Recovery** and continuation options

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
sync = { path = "../sync" }  # or use from crates.io when published
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use sync::{SyncEngine, SyncOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = SyncEngine::new(SyncOptions::default());
    let metrics = engine.sync("source/", "destination/").await?;
    
    println!("Copied {} files", metrics.files.copied);
    println!("Transferred {} bytes", metrics.transfer.bytes_transferred);
    
    Ok(())
}
```

### Advanced Usage with Progress Reporting

```rust
use sync::{SyncEngine, SyncOptions, ProgressChannel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = SyncEngine::new(SyncOptions::default());
    let (progress_reporter, mut progress_channel) = ProgressChannel::new();
    
    // Start sync in background
    let sync_task = tokio::spawn(async move {
        engine.sync_with_progress("source/", "destination/", Some(progress_reporter)).await
    });
    
    // Monitor progress
    while let Some(event) = progress_channel.recv().await {
        match event {
            sync::ProgressEvent::FileOperationCompleted { operation, source_path, .. } => {
                println!("{:?}: {}", operation, source_path);
            }
            sync::ProgressEvent::SyncCompleted { duration, .. } => {
                println!("Sync completed in {:.2}s", duration.as_secs_f64());
                break;
            }
            _ => {}
        }
    }
    
    let metrics = sync_task.await??;
    Ok(())
}
```

## API Overview

### Core Types

- **`SyncEngine`** - Main orchestration engine
- **`SyncOptions`** - Configuration for sync operations
- **`SyncMetrics`** - Comprehensive statistics and metrics
- **`ProgressChannel`** - Real-time progress reporting
- **`FileFilter`** - Advanced file filtering with glob patterns

### Comparison Methods

```rust
use sync::ComparisonMethod;

let method = ComparisonMethod::Comprehensive; // Size + timestamp + hash
let method = ComparisonMethod::Blake3;        // Fast Blake3 hashing
let method = ComparisonMethod::Sha256;        // SHA-256 hashing
let method = ComparisonMethod::ByteByByte;    // Thorough byte comparison
let method = ComparisonMethod::SizeAndTimestamp; // Quick metadata check
```

### Conflict Resolution

```rust
use sync::{ConflictStrategy, ConflictResolver};

let strategy = ConflictStrategy::PreferNewer;   // Use newer file
let strategy = ConflictStrategy::PreferSource;  // Always use source
let strategy = ConflictStrategy::Manual;        // Require manual resolution
let strategy = ConflictStrategy::BackupAndUseSource; // Backup + use source
```

### File Filtering

```rust
use sync::FileFilter;

// Filter by extensions
let filter = FileFilter::by_extensions(&["txt", "md"], false)?;

// Exclude common patterns
let filter = FileFilter::exclude_common_ignore_patterns()?;

// Size constraints
let filter = FileFilter::with_size_limits(Some(1024), Some(1024*1024)); // 1KB-1MB
```

## Configuration Options

### Sync Options

```rust
use sync::{SyncOptions, ScanOptions, ComparisonMethod, ConflictStrategy};

let options = SyncOptions {
    scan_options: ScanOptions {
        follow_links: false,
        max_depth: Some(10),
        include_hidden: false,
        respect_ignore_files: true,
        collect_hashes: true,
        ..Default::default()
    },
    comparison_method: ComparisonMethod::Blake3,
    conflict_strategy: ConflictStrategy::PreferNewer,
    dry_run: false,
    delete_extra: true,
    continue_on_error: false,
    max_concurrency: 4,
    buffer_size: 64 * 1024,
    ..Default::default()
};
```

### Preservation Options

```rust
use sync::PreservationOptions;

let preservation = PreservationOptions {
    preserve_mtime: true,
    preserve_atime: false,
    preserve_permissions: true,
    preserve_ownership: false,
    preserve_extended_attributes: false,
    preserve_symlinks: true,
};
```

## Examples

Run the included example to see the library in action:

```bash
cargo run --example basic_sync
```

This demonstrates:
1. Basic synchronization
2. Dry-run mode
3. Progress reporting
4. File filtering
5. Sync preview

## Performance Features

- **Async I/O** - Non-blocking file operations
- **Configurable Concurrency** - Control parallel operations
- **Streaming Processing** - Memory-efficient for large directories
- **Fast Hashing** - Blake3 for quick content verification
- **Efficient Scanning** - Respects .gitignore and similar files
- **Smart Comparison** - Multiple strategies for different use cases

## Platform Support

- **Windows** - Full support with NTFS attributes
- **macOS** - Full support with extended attributes
- **Linux** - Full support with extended attributes
- **Cross-platform** - Graceful fallbacks for unsupported features

## Error Handling

The library provides comprehensive error handling with detailed context:

```rust
use sync::SyncError;

match error {
    SyncError::Io(io_err) => println!("IO error: {}", io_err),
    SyncError::Permission { path, message } => {
        println!("Permission error at {}: {}", path.display(), message);
    }
    SyncError::ConflictResolution(msg) => println!("Conflict: {}", msg),
    _ => println!("Other error: {}", error),
}
```

## Contributing

This library is part of a larger sync application project. Contributions are welcome!

## License

Licensed under AGPL-3.0. See LICENSE file for details.
