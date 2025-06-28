//! Sync Engine Library
//!
//! A comprehensive async file synchronization library providing:
//! - Directory scanning with filtering
//! - File comparison and diffing
//! - Conflict resolution strategies
//! - Progress reporting and metrics
//! - Dry-run capabilities
//! - Attribute and permission preservation

pub mod scanner;
pub mod comparator;
pub mod diff;
pub mod conflict;
pub mod filter;
pub mod sync_engine;
pub mod progress;
pub mod metrics;
pub mod preservation;
pub mod error;

// Re-export main types and functions
pub use scanner::{DirectoryScanner, ScanOptions, FileEntry};
pub use comparator::{FileComparator, ComparisonMethod, ComparisonResult};
pub use diff::{DiffEngine, SyncAction, SyncPlan};
pub use conflict::{ConflictResolver, ConflictStrategy, ConflictResolution};
pub use filter::{FileFilter, FilterOptions};
pub use sync_engine::{SyncEngine, SyncOptions};
pub use progress::{ProgressReporter, ProgressEvent, ProgressChannel};
pub use metrics::{SyncMetrics, FileStats};
pub use preservation::{AttributePreserver, PermissionPreserver, PreservationOptions};
pub use error::{SyncError, Result};

/// The main synchronization function that orchestrates the entire sync process
pub async fn sync_directories(
    source: impl AsRef<std::path::Path>,
    destination: impl AsRef<std::path::Path>,
    options: SyncOptions,
) -> Result<SyncMetrics> {
    let mut engine = SyncEngine::new(options);
    engine.sync(source, destination).await
}

/// Scan a directory and return file entries
pub async fn scan_directory(
    path: impl AsRef<std::path::Path>,
    options: ScanOptions,
) -> Result<Vec<FileEntry>> {
    let scanner = DirectoryScanner::new(options);
    scanner.scan(path).await
}

/// Compare two files using the specified method
pub async fn compare_files(
    file1: impl AsRef<std::path::Path>,
    file2: impl AsRef<std::path::Path>,
    method: ComparisonMethod,
) -> Result<ComparisonResult> {
    let comparator = FileComparator::new();
    comparator.compare(file1, file2, method).await
}

/// Generate a diff between source and destination directories
pub async fn generate_diff(
    source_entries: Vec<FileEntry>,
    dest_entries: Vec<FileEntry>,
    comparison_method: ComparisonMethod,
) -> Result<SyncPlan> {
    let diff_engine = DiffEngine::new();
    diff_engine.generate_plan(source_entries, dest_entries, comparison_method).await
}

// Test modules
#[cfg(test)]
mod diff_tests;
#[cfg(test)]
mod conflict_tests;
#[cfg(test)]
mod filter_tests;
#[cfg(test)]
mod path_property_tests;
#[cfg(test)]
pub mod integration_tests;
