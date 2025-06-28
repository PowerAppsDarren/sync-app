//! Comprehensive unit tests for the diff engine module

use super::*;
use crate::scanner::{FileEntry, HashAlgorithm};
use crate::comparator::ComparisonMethod;
use std::collections::HashSet;
use std::time::{SystemTime, Duration};
use tempfile::TempDir;
use proptest::prelude::*;
use rstest::*;
use test_case::test_case;

/// Create a test file entry with specified parameters
fn create_test_file_entry(
    relative_path: &str,
    size: u64,
    is_dir: bool,
    modified_offset_secs: i64,
) -> FileEntry {
    let temp_dir = TempDir::new().unwrap();
    let full_path = temp_dir.path().join(relative_path);

    let base_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let modified = if modified_offset_secs >= 0 {
        base_time + Duration::from_secs(modified_offset_secs as u64)
    } else {
        base_time - Duration::from_secs((-modified_offset_secs) as u64)
    };

    FileEntry {
        path: full_path,
        relative_path: PathBuf::from(relative_path),
        size,
        modified,
        created: Some(modified),
        is_dir,
        is_symlink: false,
        hash: None,
        permissions: 0o644,
    }
}

/// Create a file entry with a specific hash
fn create_file_entry_with_hash(
    relative_path: &str,
    size: u64,
    hash: Option<String>,
    modified_offset_secs: i64,
) -> FileEntry {
    let mut entry = create_test_file_entry(relative_path, size, false, modified_offset_secs);
    entry.hash = hash;
    entry
}

mod diff_engine_tests {
    use super::*;

    #[tokio::test]
    async fn test_new_file_copy() {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![
            create_test_file_entry("new_file.txt", 100, false, 0),
        ];
        let dest_entries = vec![];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Copy { source, file_size, .. } => {
                assert_eq!(source, &PathBuf::from("new_file.txt"));
                assert_eq!(*file_size, 100);
            }
            action => panic!("Expected Copy action, got {:?}", action),
        }

        assert_eq!(plan.summary.copies, 1);
        assert_eq!(plan.summary.total_bytes_to_transfer, 100);
    }

    #[tokio::test]
    async fn test_delete_removed_file() {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![];
        let dest_entries = vec![
            create_test_file_entry("old_file.txt", 100, false, 0),
        ];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Delete { path } => {
                assert_eq!(path, &PathBuf::from("old_file.txt"));
            }
            action => panic!("Expected Delete action, got {:?}", action),
        }

        assert_eq!(plan.summary.deletes, 1);
    }

    #[tokio::test]
    async fn test_skip_identical_files() {
        let diff_engine = DiffEngine::new();
        
        let source_entry = create_test_file_entry("same_file.txt", 100, false, 0);
        let dest_entry = create_test_file_entry("same_file.txt", 100, false, 0);

        let source_entries = vec![source_entry];
        let dest_entries = vec![dest_entry];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::Size
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Skip { path, .. } => {
                assert_eq!(path, &PathBuf::from("same_file.txt"));
            }
            action => panic!("Expected Skip action, got {:?}", action),
        }

        assert_eq!(plan.summary.skips, 1);
    }

    #[tokio::test]
    async fn test_update_newer_file() {
        let diff_engine = DiffEngine::new();
        
        let source_entry = create_test_file_entry("file.txt", 100, false, 100); // newer
        let dest_entry = create_test_file_entry("file.txt", 100, false, 0);     // older

        let source_entries = vec![source_entry];
        let dest_entries = vec![dest_entry];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Update { source, destination, file_size } => {
                assert_eq!(source, &PathBuf::from("file.txt"));
                assert_eq!(destination, &PathBuf::from("file.txt"));
                assert_eq!(*file_size, 100);
            }
            action => panic!("Expected Update action, got {:?}", action),
        }

        assert_eq!(plan.summary.updates, 1);
        assert_eq!(plan.summary.total_bytes_to_transfer, 100);
    }

    #[tokio::test]
    async fn test_conflict_both_modified() {
        let diff_engine = DiffEngine::new();
        
        let source_entry = create_test_file_entry("file.txt", 100, false, 0);     // older
        let dest_entry = create_test_file_entry("file.txt", 200, false, 100);    // newer and different

        let source_entries = vec![source_entry];
        let dest_entries = vec![dest_entry];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Conflict { conflict_type, .. } => {
                assert_eq!(*conflict_type, ConflictType::BothModified);
            }
            action => panic!("Expected Conflict action, got {:?}", action),
        }

        assert_eq!(plan.summary.conflicts, 1);
    }

    #[tokio::test]
    async fn test_file_directory_conflict() {
        let diff_engine = DiffEngine::new();
        
        let source_entry = create_test_file_entry("path", 100, false, 0);  // file
        let dest_entry = create_test_file_entry("path", 0, true, 0);       // directory

        let source_entries = vec![source_entry];
        let dest_entries = vec![dest_entry];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Conflict { conflict_type, .. } => {
                assert_eq!(*conflict_type, ConflictType::FileDirectoryConflict);
            }
            action => panic!("Expected Conflict action, got {:?}", action),
        }
    }

    #[tokio::test]
    async fn test_create_directory() {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![
            create_test_file_entry("new_dir", 0, true, 0),
        ];
        let dest_entries = vec![];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::CreateDirectory { path } => {
                assert_eq!(path, &PathBuf::from("new_dir"));
            }
            action => panic!("Expected CreateDirectory action, got {:?}", action),
        }

        assert_eq!(plan.summary.directory_creates, 1);
    }

    #[tokio::test]
    async fn test_size_mismatch_conflict() {
        let diff_engine = DiffEngine::new();
        
        // Same timestamp but different sizes
        let source_entry = create_test_file_entry("file.txt", 100, false, 0);
        let dest_entry = create_test_file_entry("file.txt", 200, false, 0);

        let source_entries = vec![source_entry];
        let dest_entries = vec![dest_entry];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Conflict { conflict_type, .. } => {
                assert_eq!(*conflict_type, ConflictType::SizeMismatch);
            }
            action => panic!("Expected Conflict action, got {:?}", action),
        }
    }

    #[tokio::test]
    async fn test_multiple_files_mixed_actions() {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![
            create_test_file_entry("new_file.txt", 100, false, 0),           // copy
            create_test_file_entry("updated_file.txt", 200, false, 100),     // update (newer)
            create_test_file_entry("same_file.txt", 300, false, 0),          // skip (identical)
            create_test_file_entry("conflict_file.txt", 400, false, 0),      // conflict (dest newer)
        ];
        
        let dest_entries = vec![
            create_test_file_entry("updated_file.txt", 200, false, 0),       // older
            create_test_file_entry("same_file.txt", 300, false, 0),          // same
            create_test_file_entry("conflict_file.txt", 500, false, 100),    // newer and different
            create_test_file_entry("deleted_file.txt", 600, false, 0),       // delete
        ];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::SizeAndTimestamp
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 5);
        
        // Check summary counts
        assert_eq!(plan.summary.copies, 1);
        assert_eq!(plan.summary.updates, 1);
        assert_eq!(plan.summary.skips, 1);
        assert_eq!(plan.summary.conflicts, 1);
        assert_eq!(plan.summary.deletes, 1);
    }

    #[test_case(ComparisonMethod::Size)]
    #[test_case(ComparisonMethod::Timestamp)]
    #[test_case(ComparisonMethod::SizeAndTimestamp)]
    #[tokio::test]
    async fn test_different_comparison_methods(method: ComparisonMethod) {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![
            create_test_file_entry("file.txt", 100, false, 100),
        ];
        let dest_entries = vec![
            create_test_file_entry("file.txt", 100, false, 0),
        ];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            method
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        
        match method {
            ComparisonMethod::Size => {
                // Same size should result in skip
                assert!(matches!(plan.actions[0], SyncAction::Skip { .. }));
            }
            ComparisonMethod::Timestamp | ComparisonMethod::SizeAndTimestamp => {
                // Source is newer, should update
                assert!(matches!(plan.actions[0], SyncAction::Update { .. }));
            }
            _ => {} // Other methods not tested here
        }
    }

    #[tokio::test]
    async fn test_hash_based_comparison() {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![
            create_file_entry_with_hash("file.txt", 100, Some("hash1".to_string()), 0),
        ];
        let dest_entries = vec![
            create_file_entry_with_hash("file.txt", 100, Some("hash2".to_string()), 0),
        ];

        let plan = diff_engine.generate_plan(
            source_entries, 
            dest_entries, 
            ComparisonMethod::Sha256
        ).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        // Different hashes should result in update
        assert!(matches!(plan.actions[0], SyncAction::Update { .. }));
    }
}

mod action_filter_tests {
    use super::*;

    #[test]
    fn test_action_filter_modifications_only() {
        let diff_engine = DiffEngine::new();
        
        let actions = vec![
            SyncAction::Copy { 
                source: PathBuf::from("file1"), 
                destination: PathBuf::from("file1"), 
                file_size: 100 
            },
            SyncAction::Update { 
                source: PathBuf::from("file2"), 
                destination: PathBuf::from("file2"), 
                file_size: 200 
            },
            SyncAction::Delete { path: PathBuf::from("file3") },
            SyncAction::Skip { 
                path: PathBuf::from("file4"), 
                reason: "identical".to_string() 
            },
            SyncAction::Conflict {
                source: PathBuf::from("file5"),
                destination: PathBuf::from("file5"),
                conflict_type: ConflictType::BothModified,
                source_info: FileInfo {
                    size: 100,
                    modified: SystemTime::now(),
                    is_dir: false,
                    is_symlink: false,
                    permissions: 0o644,
                    hash: None,
                },
                destination_info: FileInfo {
                    size: 200,
                    modified: SystemTime::now(),
                    is_dir: false,
                    is_symlink: false,
                    permissions: 0o644,
                    hash: None,
                },
            },
        ];

        let plan = SyncPlan {
            summary: diff_engine.generate_summary(&actions),
            actions,
        };

        let filter = ActionFilter::modifications_only();
        let filtered_plan = diff_engine.filter_actions(&plan, filter);

        assert_eq!(filtered_plan.actions.len(), 3); // Copy, Update, Delete (no Skip, no Conflict)
        assert_eq!(filtered_plan.summary.skips, 0);
        assert_eq!(filtered_plan.summary.conflicts, 0);
    }

    #[test]
    fn test_action_filter_conflicts_only() {
        let diff_engine = DiffEngine::new();
        
        let actions = vec![
            SyncAction::Copy { 
                source: PathBuf::from("file1"), 
                destination: PathBuf::from("file1"), 
                file_size: 100 
            },
            SyncAction::Conflict {
                source: PathBuf::from("file2"),
                destination: PathBuf::from("file2"),
                conflict_type: ConflictType::BothModified,
                source_info: FileInfo {
                    size: 100,
                    modified: SystemTime::now(),
                    is_dir: false,
                    is_symlink: false,
                    permissions: 0o644,
                    hash: None,
                },
                destination_info: FileInfo {
                    size: 200,
                    modified: SystemTime::now(),
                    is_dir: false,
                    is_symlink: false,
                    permissions: 0o644,
                    hash: None,
                },
            },
        ];

        let plan = SyncPlan {
            summary: diff_engine.generate_summary(&actions),
            actions,
        };

        let filter = ActionFilter::conflicts_only();
        let filtered_plan = diff_engine.filter_actions(&plan, filter);

        assert_eq!(filtered_plan.actions.len(), 1); // Only conflict
        assert_eq!(filtered_plan.summary.conflicts, 1);
        assert_eq!(filtered_plan.summary.copies, 0);
    }

    #[test]
    fn test_action_filter_all() {
        let diff_engine = DiffEngine::new();
        
        let actions = vec![
            SyncAction::Copy { 
                source: PathBuf::from("file1"), 
                destination: PathBuf::from("file1"), 
                file_size: 100 
            },
            SyncAction::Skip { 
                path: PathBuf::from("file2"), 
                reason: "identical".to_string() 
            },
        ];

        let plan = SyncPlan {
            summary: diff_engine.generate_summary(&actions),
            actions,
        };

        let filter = ActionFilter::all();
        let filtered_plan = diff_engine.filter_actions(&plan, filter);

        assert_eq!(filtered_plan.actions.len(), 2); // All actions included
    }
}

mod sync_plan_sorting_tests {
    use super::*;

    #[test]
    fn test_sort_actions_directories_first() {
        let diff_engine = DiffEngine::new();
        
        let actions = vec![
            SyncAction::Copy { 
                source: PathBuf::from("file1.txt"), 
                destination: PathBuf::from("file1.txt"), 
                file_size: 100 
            },
            SyncAction::CreateDirectory { path: PathBuf::from("dir1") },
            SyncAction::Update { 
                source: PathBuf::from("file2.txt"), 
                destination: PathBuf::from("file2.txt"), 
                file_size: 200 
            },
            SyncAction::CreateDirectory { path: PathBuf::from("dir2") },
        ];

        let mut plan = SyncPlan {
            summary: diff_engine.generate_summary(&actions),
            actions,
        };

        diff_engine.sort_actions(&mut plan);

        // Directories should come first
        assert!(matches!(plan.actions[0], SyncAction::CreateDirectory { .. }));
        assert!(matches!(plan.actions[1], SyncAction::CreateDirectory { .. }));
        
        // Then files, larger first
        match (&plan.actions[2], &plan.actions[3]) {
            (SyncAction::Update { file_size: size1, .. }, SyncAction::Copy { file_size: size2, .. }) => {
                assert!(size1 >= size2, "Larger files should come first");
            }
            _ => panic!("Expected Update and Copy actions in that order"),
        }
    }
}

// Property-based tests using proptest
proptest! {
    #[test]
    fn test_plan_summary_consistency(
        num_copies in 0usize..20,
        num_updates in 0usize..20,
        num_deletes in 0usize..20,
        num_conflicts in 0usize..20,
        num_skips in 0usize..20,
        file_sizes in prop::collection::vec(1u64..10000, 0..100)
    ) {
        let diff_engine = DiffEngine::new();
        let mut actions = Vec::new();
        let mut expected_bytes = 0u64;

        // Generate copy actions
        for i in 0..num_copies {
            let file_size = file_sizes.get(i % file_sizes.len()).unwrap_or(&100);
            expected_bytes += file_size;
            actions.push(SyncAction::Copy {
                source: PathBuf::from(format!("copy_{}", i)),
                destination: PathBuf::from(format!("copy_{}", i)),
                file_size: *file_size,
            });
        }

        // Generate update actions
        for i in 0..num_updates {
            let file_size = file_sizes.get((i + num_copies) % file_sizes.len()).unwrap_or(&200);
            expected_bytes += file_size;
            actions.push(SyncAction::Update {
                source: PathBuf::from(format!("update_{}", i)),
                destination: PathBuf::from(format!("update_{}", i)),
                file_size: *file_size,
            });
        }

        // Generate delete actions
        for i in 0..num_deletes {
            actions.push(SyncAction::Delete {
                path: PathBuf::from(format!("delete_{}", i)),
            });
        }

        // Generate conflict actions
        for i in 0..num_conflicts {
            actions.push(SyncAction::Conflict {
                source: PathBuf::from(format!("conflict_{}", i)),
                destination: PathBuf::from(format!("conflict_{}", i)),
                conflict_type: ConflictType::BothModified,
                source_info: FileInfo {
                    size: 100,
                    modified: SystemTime::now(),
                    is_dir: false,
                    is_symlink: false,
                    permissions: 0o644,
                    hash: None,
                },
                destination_info: FileInfo {
                    size: 200,
                    modified: SystemTime::now(),
                    is_dir: false,
                    is_symlink: false,
                    permissions: 0o644,
                    hash: None,
                },
            });
        }

        // Generate skip actions
        for i in 0..num_skips {
            actions.push(SyncAction::Skip {
                path: PathBuf::from(format!("skip_{}", i)),
                reason: "test".to_string(),
            });
        }

        let summary = diff_engine.generate_summary(&actions);

        // Verify summary consistency
        prop_assert_eq!(summary.total_actions, actions.len());
        prop_assert_eq!(summary.copies, num_copies);
        prop_assert_eq!(summary.updates, num_updates);
        prop_assert_eq!(summary.deletes, num_deletes);
        prop_assert_eq!(summary.conflicts, num_conflicts);
        prop_assert_eq!(summary.skips, num_skips);
        prop_assert_eq!(summary.total_bytes_to_transfer, expected_bytes);
    }

    #[test]
    fn test_path_handling_consistency(
        paths in prop::collection::vec("[a-zA-Z0-9_./\\-]{1,50}", 1..20)
    ) {
        // Test that relative paths are handled consistently
        let unique_paths: HashSet<_> = paths.iter().collect();
        
        for path in unique_paths {
            let path_buf = PathBuf::from(path);
            
            // Test that paths can be converted and used consistently
            prop_assert!(path_buf.as_os_str().len() > 0);
            
            // Test relative path operations
            if !path.starts_with('/') && !path.contains("\\\\") {
                let relative = PathBuf::from(path);
                prop_assert!(!relative.is_absolute() || cfg!(windows));
            }
        }
    }

    #[test]
    fn test_file_entry_creation_properties(
        relative_path in "[a-zA-Z0-9_./\\-]{1,50}",
        size in 0u64..1_000_000,
        is_dir in any::<bool>(),
        modified_offset in -86400i64..86400i64  // ±1 day in seconds
    ) {
        let entry = create_test_file_entry(&relative_path, size, is_dir, modified_offset);
        
        prop_assert_eq!(entry.relative_path, PathBuf::from(&relative_path));
        prop_assert_eq!(entry.size, size);
        prop_assert_eq!(entry.is_dir, is_dir);
        prop_assert!(!entry.is_symlink);
        prop_assert_eq!(entry.permissions, 0o644);
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use criterion::{black_box, Criterion};

    pub fn benchmark_diff_engine_performance(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("diff_engine_1000_files", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let diff_engine = DiffEngine::new();
                    
                    // Create 1000 source and destination files
                    let source_entries: Vec<_> = (0..1000)
                        .map(|i| create_test_file_entry(&format!("file_{}.txt", i), 1000, false, 0))
                        .collect();
                    
                    let dest_entries: Vec<_> = (0..500)
                        .map(|i| create_test_file_entry(&format!("file_{}.txt", i), 1000, false, 0))
                        .collect();
                    
                    let plan = diff_engine.generate_plan(
                        source_entries,
                        dest_entries,
                        ComparisonMethod::SizeAndTimestamp
                    ).await.unwrap();
                    
                    black_box(plan);
                });
            });
        });
    }
}
