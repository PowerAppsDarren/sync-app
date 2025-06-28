//! Main sync engine that orchestrates the synchronization process

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{Result, SyncError};
use crate::scanner::{DirectoryScanner, ScanOptions, FileEntry};
use crate::comparator::{ComparisonMethod, FileComparator};
use crate::diff::{DiffEngine, SyncPlan, SyncAction};
use crate::conflict::{ConflictResolver, ConflictStrategy};
use crate::filter::{FileFilter, FilterOptions};
use crate::progress::{ProgressReporter, ProgressChannel, FileOperation};
use crate::metrics::SyncMetrics;
use crate::preservation::{AttributePreserver, PreservationOptions};

/// Options for sync operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOptions {
    /// Directory scanning options
    pub scan_options: ScanOptions,
    /// File comparison method
    pub comparison_method: ComparisonMethod,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// File filtering options
    pub filter_options: Option<FilterOptions>,
    /// Attribute preservation options
    pub preservation_options: PreservationOptions,
    /// Perform dry run (don't actually modify files)
    pub dry_run: bool,
    /// Delete files in destination that don't exist in source
    pub delete_extra: bool,
    /// Create backup directory for conflicts
    pub backup_directory: Option<PathBuf>,
    /// Maximum number of concurrent operations
    pub max_concurrency: usize,
    /// Buffer size for file operations
    pub buffer_size: usize,
    /// Continue on errors instead of stopping
    pub continue_on_error: bool,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            scan_options: ScanOptions::default(),
            comparison_method: ComparisonMethod::default(),
            conflict_strategy: ConflictStrategy::default(),
            filter_options: None,
            preservation_options: PreservationOptions::default(),
            dry_run: false,
            delete_extra: true,
            backup_directory: None,
            max_concurrency: 4,
            buffer_size: 64 * 1024, // 64KB
            continue_on_error: false,
        }
    }
}

/// Main sync engine
pub struct SyncEngine {
    options: SyncOptions,
    scanner: DirectoryScanner,
    comparator: FileComparator,
    diff_engine: DiffEngine,
    conflict_resolver: ConflictResolver,
    attribute_preserver: AttributePreserver,
    filter: Option<FileFilter>,
}

impl SyncEngine {
    /// Create a new sync engine with options
    pub fn new(options: SyncOptions) -> Self {
        let scanner = DirectoryScanner::new(options.scan_options.clone());
        let comparator = FileComparator::with_buffer_size(options.buffer_size);
        let diff_engine = DiffEngine::new();
        let mut conflict_resolver = ConflictResolver::new(options.conflict_strategy);
        
        if let Some(backup_dir) = &options.backup_directory {
            conflict_resolver.set_backup_directory(backup_dir.clone());
        }
        
        let attribute_preserver = AttributePreserver::new(options.preservation_options.clone());
        
        let filter = options.filter_options.as_ref().and_then(|opts| {
            FileFilter::new(opts.clone()).ok()
        });

        Self {
            options,
            scanner,
            comparator,
            diff_engine,
            conflict_resolver,
            attribute_preserver,
            filter,
        }
    }

    /// Perform synchronization between source and destination
    pub async fn sync<P1: AsRef<Path>, P2: AsRef<Path>>(
        &mut self,
        source: P1,
        destination: P2,
    ) -> Result<SyncMetrics> {
        let (progress_reporter, _progress_channel) = ProgressChannel::new();
        self.sync_with_progress(source, destination, Some(progress_reporter)).await
    }

    /// Perform synchronization with progress reporting
    pub async fn sync_with_progress<P1: AsRef<Path>, P2: AsRef<Path>>(
        &mut self,
        source: P1,
        destination: P2,
        progress_reporter: Option<ProgressReporter>,
    ) -> Result<SyncMetrics> {
        let source_path = source.as_ref();
        let dest_path = destination.as_ref();
        
        let mut metrics = SyncMetrics::new();
        metrics.start();

        if let Some(reporter) = &progress_reporter {
            reporter.info(format!("Starting sync from '{}' to '{}'", source_path.display(), dest_path.display()))?;
        }

        // Ensure destination directory exists
        if !dest_path.exists() {
            if self.options.dry_run {
                if let Some(reporter) = &progress_reporter {
                    reporter.info(format!("DRY RUN: Would create destination directory '{}'", dest_path.display()))?;
                }
            } else {
                fs::create_dir_all(dest_path).await.map_err(|e| {
                    SyncError::path_error(dest_path, format!("Failed to create destination directory: {}", e))
                })?;
            }
        }

        // Phase 1: Scan directories
        let (source_entries, dest_entries) = self.scan_directories(source_path, dest_path, &progress_reporter).await?;
        
        // Update metrics with scan results
        let total_bytes_scanned = source_entries.iter().map(|e| e.size).sum::<u64>() + 
                                 dest_entries.iter().map(|e| e.size).sum::<u64>();
        metrics.record_scan(source_entries.len() + dest_entries.len(), total_bytes_scanned, Duration::default());

        if let Some(reporter) = &progress_reporter {
            reporter.sync_started(source_entries.len(), source_entries.iter().map(|e| e.size).sum()).await?;
        }

        // Phase 2: Generate sync plan
        let sync_plan = self.generate_sync_plan(source_entries, dest_entries, &progress_reporter).await?;
        
        if let Some(reporter) = &progress_reporter {
            reporter.info(format!("Generated sync plan: {} actions ({} copies, {} updates, {} deletes, {} conflicts)", 
                sync_plan.summary.total_actions,
                sync_plan.summary.copies,
                sync_plan.summary.updates,
                sync_plan.summary.deletes,
                sync_plan.summary.conflicts
            ))?;
        }

        // Phase 3: Execute sync plan
        self.execute_sync_plan(sync_plan, source_path, dest_path, &progress_reporter, &mut metrics).await?;

        metrics.complete();
        
        if let Some(reporter) = &progress_reporter {
            reporter.sync_completed().await?;
            reporter.info(metrics.summary())?;
        }

        Ok(metrics)
    }

    /// Scan source and destination directories
    async fn scan_directories(
        &self,
        source_path: &Path,
        dest_path: &Path,
        progress_reporter: &Option<ProgressReporter>,
    ) -> Result<(Vec<FileEntry>, Vec<FileEntry>)> {
        if let Some(reporter) = progress_reporter {
            reporter.scan_started(source_path.to_string_lossy())?;
        }

        let start_time = Instant::now();
        let source_entries = self.scanner.scan(source_path).await?;
        let source_scan_duration = start_time.elapsed();

        if let Some(reporter) = progress_reporter {
            reporter.scan_completed(source_path.to_string_lossy(), source_entries.len(), source_scan_duration)?;
            reporter.scan_started(dest_path.to_string_lossy())?;
        }

        let start_time = Instant::now();
        let dest_entries = if dest_path.exists() {
            self.scanner.scan(dest_path).await?
        } else {
            Vec::new()
        };
        let dest_scan_duration = start_time.elapsed();

        if let Some(reporter) = progress_reporter {
            reporter.scan_completed(dest_path.to_string_lossy(), dest_entries.len(), dest_scan_duration)?;
        }

        Ok((source_entries, dest_entries))
    }

    /// Generate sync plan from file entries
    async fn generate_sync_plan(
        &self,
        source_entries: Vec<FileEntry>,
        dest_entries: Vec<FileEntry>,
        progress_reporter: &Option<ProgressReporter>,
    ) -> Result<SyncPlan> {
        if let Some(reporter) = progress_reporter {
            reporter.info("Generating sync plan...")?;
        }

        let mut plan = self.diff_engine.generate_plan(
            source_entries,
            dest_entries,
            self.options.comparison_method,
        ).await?;

        // Apply additional filtering if configured
        if let Some(filter) = &self.filter {
            plan.actions = plan.actions.into_iter()
                .filter(|action| self.should_include_action(action, filter))
                .collect();
        }

        // Sort actions for optimal execution order
        self.diff_engine.sort_actions(&mut plan);

        Ok(plan)
    }

    /// Check if an action should be included based on filters
    fn should_include_action(&self, action: &SyncAction, filter: &FileFilter) -> bool {
        match action {
            SyncAction::Copy { source, .. } |
            SyncAction::Update { source, .. } => {
                filter.should_include(source)
            }
            SyncAction::Delete { path } => {
                filter.should_include(path)
            }
            SyncAction::CreateDirectory { path } => {
                filter.should_include(path)
            }
            SyncAction::Conflict { source, .. } => {
                filter.should_include(source)
            }
            SyncAction::Skip { path, .. } => {
                filter.should_include(path)
            }
        }
    }

    /// Execute the sync plan
    async fn execute_sync_plan(
        &self,
        plan: SyncPlan,
        source_root: &Path,
        dest_root: &Path,
        progress_reporter: &Option<ProgressReporter>,
        metrics: &mut SyncMetrics,
    ) -> Result<()> {
        if let Some(reporter) = progress_reporter {
            reporter.info(format!("Executing {} actions...", plan.actions.len()))?;
        }

        for action in plan.actions {
            let start_time = Instant::now();
            let result = self.execute_action(&action, source_root, dest_root, progress_reporter).await;
            let duration = start_time.elapsed();

            match result {
                Ok(file_op) => {
                    let file_size = self.get_action_file_size(&action);
                    metrics.record_file_operation(file_op, file_size, duration);
                    
                    if let Some(reporter) = progress_reporter {
                        reporter.file_operation_completed(
                            file_op,
                            self.get_action_source_path(&action),
                            self.get_action_dest_path(&action),
                            file_size,
                            duration,
                        ).await?;
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    metrics.record_error("ActionExecution", &error_msg, !self.options.continue_on_error);
                    
                    if let Some(reporter) = progress_reporter {
                        reporter.file_operation_failed(
                            self.get_action_operation(&action),
                            self.get_action_source_path(&action),
                            self.get_action_dest_path(&action),
                            &error_msg,
                        ).await?;
                    }

                    if !self.options.continue_on_error {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a single sync action
    async fn execute_action(
        &self,
        action: &SyncAction,
        source_root: &Path,
        dest_root: &Path,
        progress_reporter: &Option<ProgressReporter>,
    ) -> Result<FileOperation> {
        match action {
            SyncAction::Copy { source, destination, .. } => {
                let source_path = source_root.join(source);
                let dest_path = dest_root.join(destination);
                
                if let Some(reporter) = progress_reporter {
                    reporter.file_operation_started(
                        FileOperation::Copy,
                        source_path.to_string_lossy(),
                        Some(dest_path.to_string_lossy().to_string()),
                        fs::metadata(&source_path).await?.len(),
                    )?;
                }

                self.copy_file(&source_path, &dest_path).await?;
                Ok(FileOperation::Copy)
            }

            SyncAction::Update { source, destination, .. } => {
                let source_path = source_root.join(source);
                let dest_path = dest_root.join(destination);
                
                if let Some(reporter) = progress_reporter {
                    reporter.file_operation_started(
                        FileOperation::Update,
                        source_path.to_string_lossy(),
                        Some(dest_path.to_string_lossy().to_string()),
                        fs::metadata(&source_path).await?.len(),
                    )?;
                }

                self.copy_file(&source_path, &dest_path).await?;
                Ok(FileOperation::Update)
            }

            SyncAction::Delete { path } => {
                let file_path = dest_root.join(path);
                
                if let Some(reporter) = progress_reporter {
                    let file_size = if file_path.exists() {
                        fs::metadata(&file_path).await.map(|m| m.len()).unwrap_or(0)
                    } else {
                        0
                    };
                    
                    reporter.file_operation_started(
                        FileOperation::Delete,
                        file_path.to_string_lossy(),
                        None,
                        file_size,
                    )?;
                }

                self.delete_file(&file_path).await?;
                Ok(FileOperation::Delete)
            }

            SyncAction::CreateDirectory { path } => {
                let dir_path = dest_root.join(path);
                
                if let Some(reporter) = progress_reporter {
                    reporter.file_operation_started(
                        FileOperation::CreateDirectory,
                        dir_path.to_string_lossy(),
                        None,
                        0,
                    )?;
                }

                self.create_directory(&dir_path).await?;
                Ok(FileOperation::CreateDirectory)
            }

            SyncAction::Conflict { source, destination, conflict_type, source_info, destination_info } => {
                let source_path = source_root.join(source);
                let dest_path = dest_root.join(destination);
                
                if let Some(reporter) = progress_reporter {
                    reporter.file_operation_started(
                        FileOperation::Conflict,
                        source_path.to_string_lossy(),
                        Some(dest_path.to_string_lossy().to_string()),
                        source_info.size,
                    )?;
                }

                let resolution = self.conflict_resolver.resolve_conflict(
                    source,
                    destination,
                    conflict_type.clone(),
                    source_info,
                    destination_info,
                )?;

                if let Some(resolved_action) = self.conflict_resolver.resolution_to_action(
                    resolution,
                    source.clone(),
                    destination.clone(),
                    source_info,
                )? {
                    return Box::pin(self.execute_action(&resolved_action, source_root, dest_root, progress_reporter)).await;
                }

                Ok(FileOperation::Conflict)
            }

            SyncAction::Skip { .. } => {
                Ok(FileOperation::Skip)
            }
        }
    }

    /// Copy a file from source to destination
    async fn copy_file(&self, source: &Path, destination: &Path) -> Result<()> {
        if self.options.dry_run {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SyncError::copy_error(source, destination, format!("Failed to create parent directory: {}", e))
            })?;
        }

        // Copy the file
        fs::copy(source, destination).await.map_err(|e| {
            SyncError::copy_error(source, destination, format!("Failed to copy file: {}", e))
        })?;

        // Preserve attributes if requested
        if self.options.preservation_options.preserve_mtime || self.options.preservation_options.preserve_permissions {
            self.attribute_preserver.copy_attributes(source, destination).await.map_err(|e| {
                // Log warning but don't fail the copy
                tracing::warn!("Failed to preserve attributes for '{}': {}", destination.display(), e);
                e
            }).ok();
        }

        Ok(())
    }

    /// Delete a file or directory
    async fn delete_file(&self, path: &Path) -> Result<()> {
        if self.options.dry_run {
            return Ok(());
        }

        if path.is_dir() {
            fs::remove_dir_all(path).await.map_err(|e| {
                SyncError::deletion_error(path, format!("Failed to delete directory: {}", e))
            })
        } else {
            fs::remove_file(path).await.map_err(|e| {
                SyncError::deletion_error(path, format!("Failed to delete file: {}", e))
            })
        }
    }

    /// Create a directory
    async fn create_directory(&self, path: &Path) -> Result<()> {
        if self.options.dry_run {
            return Ok(());
        }

        fs::create_dir_all(path).await.map_err(|e| {
            SyncError::path_error(path, format!("Failed to create directory: {}", e))
        })
    }

    /// Get file size from action
    fn get_action_file_size(&self, action: &SyncAction) -> u64 {
        match action {
            SyncAction::Copy { file_size, .. } | SyncAction::Update { file_size, .. } => *file_size,
            SyncAction::Conflict { source_info, .. } => source_info.size,
            _ => 0,
        }
    }

    /// Get source path from action
    fn get_action_source_path(&self, action: &SyncAction) -> String {
        match action {
            SyncAction::Copy { source, .. } | SyncAction::Update { source, .. } | SyncAction::Conflict { source, .. } => {
                source.to_string_lossy().to_string()
            }
            SyncAction::Delete { path } | SyncAction::CreateDirectory { path } | SyncAction::Skip { path, .. } => {
                path.to_string_lossy().to_string()
            }
        }
    }

    /// Get destination path from action
    fn get_action_dest_path(&self, action: &SyncAction) -> Option<String> {
        match action {
            SyncAction::Copy { destination, .. } | SyncAction::Update { destination, .. } | SyncAction::Conflict { destination, .. } => {
                Some(destination.to_string_lossy().to_string())
            }
            _ => None,
        }
    }

    /// Get file operation type from action
    fn get_action_operation(&self, action: &SyncAction) -> FileOperation {
        match action {
            SyncAction::Copy { .. } => FileOperation::Copy,
            SyncAction::Update { .. } => FileOperation::Update,
            SyncAction::Delete { .. } => FileOperation::Delete,
            SyncAction::CreateDirectory { .. } => FileOperation::CreateDirectory,
            SyncAction::Conflict { .. } => FileOperation::Conflict,
            SyncAction::Skip { .. } => FileOperation::Skip,
        }
    }

    /// Generate a dry-run preview of what would be synchronized
    pub async fn preview<P1: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        source: P1,
        destination: P2,
    ) -> Result<SyncPlan> {
        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        let (source_entries, dest_entries) = self.scan_directories(source_path, dest_path, &None).await?;
        self.generate_sync_plan(source_entries, dest_entries, &None).await
    }

    /// Get sync engine options
    pub fn options(&self) -> &SyncOptions {
        &self.options
    }

    /// Update sync engine options
    pub fn set_options(&mut self, options: SyncOptions) {
        self.options = options;
        // Recreate components with new options
        *self = Self::new(self.options.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_sync_engine_basic() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");

        // Create source directory with files
        fs::create_dir_all(&source_dir).await.unwrap();
        fs::write(source_dir.join("file1.txt"), b"content1").await.unwrap();
        fs::write(source_dir.join("file2.txt"), b"content2").await.unwrap();

        // Create destination directory
        fs::create_dir_all(&dest_dir).await.unwrap();

        // Create sync engine
        let mut engine = SyncEngine::new(SyncOptions::default());

        // Perform sync
        let metrics = engine.sync(&source_dir, &dest_dir).await.unwrap();

        // Verify files were copied
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file2.txt").exists());

        // Check metrics
        assert_eq!(metrics.files.copied, 2);
        assert!(metrics.is_successful());
    }

    #[tokio::test]
    async fn test_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");

        // Create source directory with files
        fs::create_dir_all(&source_dir).await.unwrap();
        fs::write(source_dir.join("file1.txt"), b"content1").await.unwrap();

        // Create destination directory
        fs::create_dir_all(&dest_dir).await.unwrap();

        // Create sync engine with dry run
        let mut options = SyncOptions::default();
        options.dry_run = true;
        let mut engine = SyncEngine::new(options);

        // Perform sync
        let metrics = engine.sync(&source_dir, &dest_dir).await.unwrap();

        // Verify files were NOT copied
        assert!(!dest_dir.join("file1.txt").exists());

        // But metrics should still be recorded
        assert_eq!(metrics.files.copied, 1);
    }

    #[tokio::test]
    async fn test_preview() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");

        // Create source directory with files
        fs::create_dir_all(&source_dir).await.unwrap();
        fs::write(source_dir.join("file1.txt"), b"content1").await.unwrap();
        fs::write(source_dir.join("file2.txt"), b"content2").await.unwrap();

        // Create destination directory with one file
        fs::create_dir_all(&dest_dir).await.unwrap();
        fs::write(dest_dir.join("file1.txt"), b"content1").await.unwrap();

        // Create sync engine
        let engine = SyncEngine::new(SyncOptions::default());

        // Generate preview
        let plan = engine.preview(&source_dir, &dest_dir).await.unwrap();

        // Should have one copy action (file2.txt) and one skip action (file1.txt)
        assert_eq!(plan.summary.copies, 1);
        assert_eq!(plan.summary.skips, 1);
    }
}
