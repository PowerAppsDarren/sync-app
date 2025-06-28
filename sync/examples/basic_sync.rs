//! Basic sync example demonstrating the sync engine library

use std::error::Error;
use sync::{
    SyncEngine, SyncOptions, ScanOptions, ComparisonMethod, 
    ConflictStrategy, PreservationOptions, FileFilter,
    ProgressChannel
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Basic Sync Engine Example");
    println!("========================");

    // Create temporary directories for testing
    let temp_dir = tempfile::TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("destination");

    // Setup source directory with some test files
    tokio::fs::create_dir_all(&source_dir).await?;
    tokio::fs::write(source_dir.join("file1.txt"), b"This is file 1 content").await?;
    tokio::fs::write(source_dir.join("file2.txt"), b"This is file 2 content").await?;
    
    // Create a subdirectory
    tokio::fs::create_dir(source_dir.join("subdir")).await?;
    tokio::fs::write(source_dir.join("subdir").join("file3.txt"), b"This is file 3 content").await?;

    // Setup destination directory
    tokio::fs::create_dir_all(&dest_dir).await?;

    println!("Source directory: {}", source_dir.display());
    println!("Destination directory: {}", dest_dir.display());
    println!();

    // Configure sync options
    let mut sync_options = SyncOptions {
        scan_options: ScanOptions {
            follow_links: false,
            max_depth: None,
            include_hidden: false,
            respect_ignore_files: false,
            filter_options: None,
            collect_hashes: false,
            hash_algorithm: sync::scanner::HashAlgorithm::Blake3,
        },
        comparison_method: ComparisonMethod::SizeAndTimestamp,
        conflict_strategy: ConflictStrategy::PreferSource,
        filter_options: None,
        preservation_options: PreservationOptions::default(),
        dry_run: false,
        delete_extra: false,
        backup_directory: None,
        max_concurrency: 4,
        buffer_size: 64 * 1024,
        continue_on_error: false,
    };

    // Example 1: Basic sync
    println!("Example 1: Basic Sync");
    println!("--------------------");
    
    let mut engine = SyncEngine::new(sync_options.clone());
    let metrics = engine.sync(&source_dir, &dest_dir).await?;
    
    println!("Sync completed!");
    println!("Files copied: {}", metrics.files.copied);
    println!("Directories created: {}", metrics.files.directories_created);
    println!("Total bytes transferred: {}", metrics.transfer.bytes_transferred);
    println!("Duration: {:.2}s", metrics.duration.as_secs_f64());
    println!();

    // Verify files were copied
    assert!(dest_dir.join("file1.txt").exists());
    assert!(dest_dir.join("file2.txt").exists());
    assert!(dest_dir.join("subdir").exists());
    assert!(dest_dir.join("subdir").join("file3.txt").exists());

    // Example 2: Dry run mode
    println!("Example 2: Dry Run Mode");
    println!("-----------------------");
    
    // Clear destination for clean test
    tokio::fs::remove_dir_all(&dest_dir).await?;
    tokio::fs::create_dir_all(&dest_dir).await?;
    
    sync_options.dry_run = true;
    let mut engine = SyncEngine::new(sync_options.clone());
    let metrics = engine.sync(&source_dir, &dest_dir).await?;
    
    println!("Dry run completed!");
    println!("Would copy {} files", metrics.files.copied);
    println!("Would transfer {} bytes", metrics.transfer.bytes_transferred);
    
    // Verify no files were actually copied in dry run
    assert!(!dest_dir.join("file1.txt").exists());
    println!();

    // Example 3: Sync with progress reporting
    println!("Example 3: Sync with Progress Reporting");
    println!("---------------------------------------");
    
    sync_options.dry_run = false;
    let mut engine = SyncEngine::new(sync_options.clone());
    let (progress_reporter, mut progress_channel) = ProgressChannel::new();
    
    // Start sync in a task
    let source_dir_clone = source_dir.clone();
    let dest_dir_clone = dest_dir.clone();
    let sync_task = tokio::spawn(async move {
        engine.sync_with_progress(&source_dir_clone, &dest_dir_clone, Some(progress_reporter)).await
    });
    
    // Monitor progress
    let mut files_processed = 0;
    let mut total_files = 0;
    
    while let Some(event) = progress_channel.recv().await {
        match event {
            sync::ProgressEvent::SyncStarted { total_files: total, .. } => {
                total_files = total;
                println!("Sync started - {} files to process", total);
            }
            sync::ProgressEvent::FileOperationCompleted { operation, source_path, .. } => {
                files_processed += 1;
                println!("  {:?}: {} ({}/{})", operation, source_path, files_processed, total_files);
            }
            sync::ProgressEvent::SyncCompleted { duration, .. } => {
                println!("Sync completed in {:.2}s", duration.as_secs_f64());
                break;
            }
            sync::ProgressEvent::Info { message } => {
                println!("Info: {}", message);
            }
            _ => {}
        }
    }
    
    let metrics = sync_task.await??;
    println!("Final metrics: {} files processed", metrics.files.processed);
    println!();

    // Example 4: File filtering
    println!("Example 4: File Filtering");
    println!("-------------------------");
    
    // Clear destination
    tokio::fs::remove_dir_all(&dest_dir).await?;
    tokio::fs::create_dir_all(&dest_dir).await?;
    
    // Create a filter that only includes .txt files
    let filter = FileFilter::by_extensions(&["txt"], false)?;
    sync_options.filter_options = Some(filter.options().clone());
    
    let mut engine = SyncEngine::new(sync_options.clone());
    let metrics = engine.sync(&source_dir, &dest_dir).await?;
    
    println!("Filtered sync completed!");
    println!("Files copied: {}", metrics.files.copied);
    println!();

    // Example 5: Preview mode (dry run with detailed plan)
    println!("Example 5: Sync Preview");
    println!("----------------------");
    
    // Modify a file to create an update scenario
    tokio::fs::write(dest_dir.join("file1.txt"), b"Modified content").await?;
    
    let engine = SyncEngine::new(sync_options.clone());
    let plan = engine.preview(&source_dir, &dest_dir).await?;
    
    println!("Sync plan preview:");
    println!("  Total actions: {}", plan.summary.total_actions);
    println!("  Copies: {}", plan.summary.copies);
    println!("  Updates: {}", plan.summary.updates);
    println!("  Deletes: {}", plan.summary.deletes);
    println!("  Skips: {}", plan.summary.skips);
    println!("  Bytes to transfer: {}", plan.summary.total_bytes_to_transfer);
    
    for (i, action) in plan.actions.iter().enumerate().take(5) {
        println!("  Action {}: {:?}", i + 1, action);
    }
    
    if plan.actions.len() > 5 {
        println!("  ... and {} more actions", plan.actions.len() - 5);
    }

    println!();
    println!("All examples completed successfully!");

    Ok(())
}
