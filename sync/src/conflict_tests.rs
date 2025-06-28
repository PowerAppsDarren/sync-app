//! Comprehensive unit tests for the conflict resolver module

use super::*;
use crate::diff::{FileInfo, ConflictType, SyncAction};
use std::time::{SystemTime, Duration};
use tempfile::TempDir;
use proptest::prelude::*;
use rstest::*;
use test_case::test_case;

/// Create a test file info with specified parameters
fn create_test_file_info(
    size: u64,
    modified_offset_secs: i64,
    is_dir: bool,
    is_symlink: bool,
    permissions: u32,
) -> FileInfo {
    let base_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let modified = if modified_offset_secs >= 0 {
        base_time + Duration::from_secs(modified_offset_secs as u64)
    } else {
        base_time - Duration::from_secs((-modified_offset_secs) as u64)
    };

    FileInfo {
        size,
        modified,
        is_dir,
        is_symlink,
        permissions,
        hash: None,
    }
}

/// Create file info with hash
fn create_file_info_with_hash(
    size: u64,
    modified_offset_secs: i64,
    hash: Option<String>,
) -> FileInfo {
    let mut info = create_test_file_info(size, modified_offset_secs, false, false, 0o644);
    info.hash = hash;
    info
}

mod conflict_strategy_tests {
    use super::*;

    #[test]
    fn test_prefer_source_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferSource);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 100, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution"),
        }
    }

    #[test]
    fn test_prefer_destination_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferDestination);
        
        let source_info = create_test_file_info(100, 100, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseDestination => {}
            _ => panic!("Expected UseDestination resolution"),
        }
    }

    #[test]
    fn test_prefer_newer_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferNewer);
        
        // Source is newer
        let source_info = create_test_file_info(100, 100, false, false, 0o644);
        let dest_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution for newer source"),
        }

        // Destination is newer
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(100, 100, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseDestination => {}
            _ => panic!("Expected UseDestination resolution for newer destination"),
        }
    }

    #[test]
    fn test_prefer_older_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferOlder);
        
        // Source is older
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(100, 100, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution for older source"),
        }
    }

    #[test]
    fn test_prefer_larger_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferLarger);
        
        // Source is larger
        let source_info = create_test_file_info(200, 0, false, false, 0o644);
        let dest_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::SizeMismatch,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution for larger source"),
        }

        // Destination is larger
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::SizeMismatch,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseDestination => {}
            _ => panic!("Expected UseDestination resolution for larger destination"),
        }
    }

    #[test]
    fn test_prefer_smaller_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferSmaller);
        
        // Source is smaller
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::SizeMismatch,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution for smaller source"),
        }
    }

    #[test]
    fn test_skip_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::Skip);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::Skip => {}
            _ => panic!("Expected Skip resolution"),
        }
    }

    #[test]
    fn test_manual_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::Manual);
        
        let source_info = create_test_file_info(100, 100, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::ManualRequired { suggested_action, .. } => {
                assert!(suggested_action.contains("newer"));
            }
            _ => panic!("Expected ManualRequired resolution"),
        }
    }

    #[test]
    fn test_fail_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::Fail);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::Failed { reason } => {
                assert!(reason.contains("BothModified"));
            }
            _ => panic!("Expected Failed resolution"),
        }
    }
}

mod backup_tests {
    use super::*;

    #[test]
    fn test_backup_and_use_source_strategy() {
        let temp_dir = TempDir::new().unwrap();
        let mut resolver = ConflictResolver::new(ConflictStrategy::BackupAndUseSource);
        resolver.set_backup_directory(temp_dir.path().to_path_buf());
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source.txt"),
            &PathBuf::from("dest.txt"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::BackupAndUseSource { backup_path } => {
                assert!(backup_path.parent().unwrap() == temp_dir.path());
                assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("dest.txt"));
                assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("dest"));
            }
            _ => panic!("Expected BackupAndUseSource resolution"),
        }
    }

    #[test]
    fn test_backup_and_keep_destination_strategy() {
        let temp_dir = TempDir::new().unwrap();
        let mut resolver = ConflictResolver::new(ConflictStrategy::BackupAndKeepDestination);
        resolver.set_backup_directory(temp_dir.path().to_path_buf());
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source.txt"),
            &PathBuf::from("dest.txt"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::BackupAndKeepDestination { backup_path } => {
                assert!(backup_path.parent().unwrap() == temp_dir.path());
                assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("source.txt"));
                assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("src"));
            }
            _ => panic!("Expected BackupAndKeepDestination resolution"),
        }
    }

    #[test]
    fn test_backup_without_directory_configured() {
        let resolver = ConflictResolver::new(ConflictStrategy::BackupAndUseSource);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let result = resolver.resolve_conflict(
            &PathBuf::from("source.txt"),
            &PathBuf::from("dest.txt"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No backup directory configured"));
    }
}

mod type_specific_strategies_tests {
    use super::*;

    #[test]
    fn test_type_specific_strategy_override() {
        let mut resolver = ConflictResolver::new(ConflictStrategy::PreferSource);
        resolver.set_strategy_for_type(ConflictType::FileDirectoryConflict, ConflictStrategy::Fail);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(100, 0, false, false, 0o644);
        
        // Should use default strategy for this type
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution for default strategy"),
        }

        // Should use specific strategy for file/directory conflict
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::FileDirectoryConflict,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::Failed { .. } => {}
            _ => panic!("Expected Failed resolution for specific strategy"),
        }
    }

    #[test_case(ConflictType::BothModified)]
    #[test_case(ConflictType::FileDirectoryConflict)]
    #[test_case(ConflictType::TypeMismatch)]
    #[test_case(ConflictType::PermissionConflict)]
    #[test_case(ConflictType::SizeMismatch)]
    fn test_all_conflict_types_with_manual_strategy(conflict_type: ConflictType) {
        let resolver = ConflictResolver::new(ConflictStrategy::Manual);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 100, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            conflict_type,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::ManualRequired { suggested_action, .. } => {
                assert!(!suggested_action.is_empty());
            }
            _ => panic!("Expected ManualRequired resolution for {:?}", conflict_type),
        }
    }
}

mod suggestion_tests {
    use super::*;

    #[test]
    fn test_suggestions_for_both_modified() {
        let resolver = ConflictResolver::new(ConflictStrategy::Manual);
        
        // Source newer
        let source_info = create_test_file_info(100, 100, false, false, 0o644);
        let dest_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        if let ConflictResolution::ManualRequired { suggested_action, .. } = resolution {
            assert!(suggested_action.contains("newer"));
            assert!(suggested_action.contains("source"));
        } else {
            panic!("Expected ManualRequired resolution");
        }

        // Destination newer
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(100, 100, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        if let ConflictResolution::ManualRequired { suggested_action, .. } = resolution {
            assert!(suggested_action.contains("newer"));
            assert!(suggested_action.contains("destination"));
        } else {
            panic!("Expected ManualRequired resolution");
        }
    }

    #[test]
    fn test_suggestions_for_size_mismatch() {
        let resolver = ConflictResolver::new(ConflictStrategy::Manual);
        
        // Source larger
        let source_info = create_test_file_info(200, 0, false, false, 0o644);
        let dest_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::SizeMismatch,
            &source_info,
            &dest_info,
        ).unwrap();

        if let ConflictResolution::ManualRequired { suggested_action, .. } = resolution {
            assert!(suggested_action.contains("larger"));
            assert!(suggested_action.contains("Source"));
        } else {
            panic!("Expected ManualRequired resolution");
        }
    }

    #[test]
    fn test_suggestions_for_file_directory_conflict() {
        let resolver = ConflictResolver::new(ConflictStrategy::Manual);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(0, 0, true, false, 0o755);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::FileDirectoryConflict,
            &source_info,
            &dest_info,
        ).unwrap();

        if let ConflictResolution::ManualRequired { suggested_action, .. } = resolution {
            assert!(suggested_action.contains("File/directory"));
            assert!(suggested_action.contains("renaming"));
        } else {
            panic!("Expected ManualRequired resolution");
        }
    }
}

mod conflict_presets_tests {
    use super::*;

    #[test]
    fn test_safe_sync_preset() {
        let resolver = ConflictResolver::with_preset(ConflictPreset::SafeSync);
        assert_eq!(resolver.default_strategy, ConflictStrategy::Manual);
        
        // Test that file/directory conflicts are set to fail
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(0, 0, true, false, 0o755);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::FileDirectoryConflict,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::Failed { .. } => {}
            _ => panic!("Expected Failed resolution for safe sync preset"),
        }
    }

    #[test]
    fn test_force_source_preset() {
        let resolver = ConflictResolver::with_preset(ConflictPreset::ForceSource);
        assert_eq!(resolver.default_strategy, ConflictStrategy::PreferSource);
        
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 100, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution"),
        }
    }

    #[test]
    fn test_force_destination_preset() {
        let resolver = ConflictResolver::with_preset(ConflictPreset::ForceDestination);
        assert_eq!(resolver.default_strategy, ConflictStrategy::PreferDestination);
    }

    #[test]
    fn test_prefer_newer_preset() {
        let resolver = ConflictResolver::with_preset(ConflictPreset::PreferNewer);
        assert_eq!(resolver.default_strategy, ConflictStrategy::PreferNewer);
    }

    #[test]
    fn test_skip_conflicts_preset() {
        let resolver = ConflictResolver::with_preset(ConflictPreset::SkipConflicts);
        assert_eq!(resolver.default_strategy, ConflictStrategy::Skip);
    }
}

mod resolution_to_action_tests {
    use super::*;

    #[test]
    fn test_use_source_to_action() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferSource);
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let action = resolver.resolution_to_action(
            ConflictResolution::UseSource,
            PathBuf::from("source.txt"),
            PathBuf::from("dest.txt"),
            &source_info,
        ).unwrap();

        match action {
            Some(SyncAction::Update { source, destination, file_size }) => {
                assert_eq!(source, PathBuf::from("source.txt"));
                assert_eq!(destination, PathBuf::from("dest.txt"));
                assert_eq!(file_size, 100);
            }
            _ => panic!("Expected Update action"),
        }
    }

    #[test]
    fn test_use_destination_to_action() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferDestination);
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let action = resolver.resolution_to_action(
            ConflictResolution::UseDestination,
            PathBuf::from("source.txt"),
            PathBuf::from("dest.txt"),
            &source_info,
        ).unwrap();

        match action {
            Some(SyncAction::Skip { path, reason }) => {
                assert_eq!(path, PathBuf::from("dest.txt"));
                assert!(reason.contains("destination"));
            }
            _ => panic!("Expected Skip action"),
        }
    }

    #[test]
    fn test_skip_to_action() {
        let resolver = ConflictResolver::new(ConflictStrategy::Skip);
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let action = resolver.resolution_to_action(
            ConflictResolution::Skip,
            PathBuf::from("source.txt"),
            PathBuf::from("dest.txt"),
            &source_info,
        ).unwrap();

        match action {
            Some(SyncAction::Skip { path, reason }) => {
                assert_eq!(path, PathBuf::from("dest.txt"));
                assert!(reason.contains("conflict"));
            }
            _ => panic!("Expected Skip action"),
        }
    }

    #[test]
    fn test_backup_and_use_source_to_action() {
        let resolver = ConflictResolver::new(ConflictStrategy::BackupAndUseSource);
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let action = resolver.resolution_to_action(
            ConflictResolution::BackupAndUseSource { 
                backup_path: PathBuf::from("backup.txt") 
            },
            PathBuf::from("source.txt"),
            PathBuf::from("dest.txt"),
            &source_info,
        ).unwrap();

        match action {
            Some(SyncAction::Update { source, destination, file_size }) => {
                assert_eq!(source, PathBuf::from("source.txt"));
                assert_eq!(destination, PathBuf::from("dest.txt"));
                assert_eq!(file_size, 100);
            }
            _ => panic!("Expected Update action"),
        }
    }

    #[test]
    fn test_manual_required_to_action() {
        let resolver = ConflictResolver::new(ConflictStrategy::Manual);
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let action = resolver.resolution_to_action(
            ConflictResolution::ManualRequired {
                source_info: source_info.clone(),
                destination_info: create_test_file_info(200, 0, false, false, 0o644),
                suggested_action: "test suggestion".to_string(),
            },
            PathBuf::from("source.txt"),
            PathBuf::from("dest.txt"),
            &source_info,
        ).unwrap();

        assert!(action.is_none()); // Manual resolution returns None
    }

    #[test]
    fn test_failed_to_action() {
        let resolver = ConflictResolver::new(ConflictStrategy::Fail);
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        
        let result = resolver.resolution_to_action(
            ConflictResolution::Failed { reason: "test failure".to_string() },
            PathBuf::from("source.txt"),
            PathBuf::from("dest.txt"),
            &source_info,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("test failure"));
    }
}

// Property-based tests using proptest
proptest! {
    #[test]
    fn test_strategy_consistency(
        strategy in prop::sample::select(&[
            ConflictStrategy::PreferSource,
            ConflictStrategy::PreferDestination,
            ConflictStrategy::PreferNewer,
            ConflictStrategy::PreferOlder,
            ConflictStrategy::PreferLarger,
            ConflictStrategy::PreferSmaller,
            ConflictStrategy::Skip,
            ConflictStrategy::Manual,
            ConflictStrategy::Fail,
        ]),
        source_size in 1u64..1_000_000,
        dest_size in 1u64..1_000_000,
        source_time_offset in -86400i64..86400i64,
        dest_time_offset in -86400i64..86400i64,
    ) {
        let resolver = ConflictResolver::new(strategy);
        
        let source_info = create_test_file_info(source_size, source_time_offset, false, false, 0o644);
        let dest_info = create_test_file_info(dest_size, dest_time_offset, false, false, 0o644);
        
        let result = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        );
        
        // All strategies should succeed for basic conflict types
        prop_assert!(result.is_ok());
        
        let resolution = result.unwrap();
        
        // Verify that the resolution matches the expected strategy behavior
        match strategy {
            ConflictStrategy::PreferSource => {
                prop_assert!(matches!(resolution, ConflictResolution::UseSource));
            }
            ConflictStrategy::PreferDestination => {
                prop_assert!(matches!(resolution, ConflictResolution::UseDestination));
            }
            ConflictStrategy::Skip => {
                prop_assert!(matches!(resolution, ConflictResolution::Skip));
            }
            ConflictStrategy::Manual => {
                prop_assert!(matches!(resolution, ConflictResolution::ManualRequired { .. }));
            }
            ConflictStrategy::Fail => {
                prop_assert!(matches!(resolution, ConflictResolution::Failed { .. }));
            }
            ConflictStrategy::PreferNewer => {
                // Should choose based on modification time
                let should_use_source = source_info.modified >= dest_info.modified;
                if should_use_source {
                    prop_assert!(matches!(resolution, ConflictResolution::UseSource));
                } else {
                    prop_assert!(matches!(resolution, ConflictResolution::UseDestination));
                }
            }
            ConflictStrategy::PreferOlder => {
                // Should choose based on modification time (opposite of newer)
                let should_use_source = source_info.modified <= dest_info.modified;
                if should_use_source {
                    prop_assert!(matches!(resolution, ConflictResolution::UseSource));
                } else {
                    prop_assert!(matches!(resolution, ConflictResolution::UseDestination));
                }
            }
            ConflictStrategy::PreferLarger => {
                // Should choose based on file size
                if source_info.size > dest_info.size {
                    prop_assert!(matches!(resolution, ConflictResolution::UseSource));
                } else if dest_info.size > source_info.size {
                    prop_assert!(matches!(resolution, ConflictResolution::UseDestination));
                } else {
                    // Equal sizes fall back to newer
                    prop_assert!(matches!(resolution, ConflictResolution::UseSource | ConflictResolution::UseDestination));
                }
            }
            ConflictStrategy::PreferSmaller => {
                // Should choose based on file size (opposite of larger)
                if source_info.size < dest_info.size {
                    prop_assert!(matches!(resolution, ConflictResolution::UseSource));
                } else if dest_info.size < source_info.size {
                    prop_assert!(matches!(resolution, ConflictResolution::UseDestination));
                } else {
                    // Equal sizes fall back to newer
                    prop_assert!(matches!(resolution, ConflictResolution::UseSource | ConflictResolution::UseDestination));
                }
            }
            _ => {}
        }
    }

    #[test]
    fn test_backup_path_generation(
        file_name in "[a-zA-Z0-9_-]{1,50}\\.(txt|md|rs|json)",
        suffix in "[a-zA-Z0-9_-]{1,10}",
    ) {
        let temp_dir = TempDir::new().unwrap();
        let mut resolver = ConflictResolver::new(ConflictStrategy::BackupAndUseSource);
        resolver.set_backup_directory(temp_dir.path().to_path_buf());
        
        let original_path = PathBuf::from(&file_name);
        let backup_result = resolver.generate_backup_path(&original_path, &suffix);
        
        prop_assert!(backup_result.is_ok());
        
        let backup_path = backup_result.unwrap();
        let backup_name = backup_path.file_name().unwrap().to_str().unwrap();
        
        // Backup name should contain original filename and suffix
        prop_assert!(backup_name.contains(&file_name));
        prop_assert!(backup_name.contains(&suffix));
        
        // Backup should be in the configured directory
        prop_assert_eq!(backup_path.parent().unwrap(), temp_dir.path());
    }

    #[test]
    fn test_conflict_type_handling(
        conflict_type in prop::sample::select(&[
            ConflictType::BothModified,
            ConflictType::FileDirectoryConflict,
            ConflictType::TypeMismatch,
            ConflictType::PermissionConflict,
            ConflictType::SizeMismatch,
        ]),
        size in 1u64..1000,
        time_offset in -3600i64..3600i64,
    ) {
        let resolver = ConflictResolver::new(ConflictStrategy::Manual);
        
        let source_info = create_test_file_info(size, time_offset, false, false, 0o644);
        let dest_info = create_test_file_info(size + 1, time_offset + 1, false, false, 0o644);
        
        let result = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            conflict_type,
            &source_info,
            &dest_info,
        );
        
        prop_assert!(result.is_ok());
        
        if let ConflictResolution::ManualRequired { suggested_action, .. } = result.unwrap() {
            // Suggestion should not be empty
            prop_assert!(!suggested_action.is_empty());
            
            // Suggestion should mention the conflict type somehow
            match conflict_type {
                ConflictType::BothModified => {
                    prop_assert!(suggested_action.contains("newer") || suggested_action.contains("modified"));
                }
                ConflictType::FileDirectoryConflict => {
                    prop_assert!(suggested_action.contains("File/directory") || suggested_action.contains("renaming"));
                }
                ConflictType::TypeMismatch => {
                    prop_assert!(suggested_action.contains("type") || suggested_action.contains("mismatch"));
                }
                ConflictType::PermissionConflict => {
                    prop_assert!(suggested_action.contains("permission") || suggested_action.contains("Permission"));
                }
                ConflictType::SizeMismatch => {
                    prop_assert!(suggested_action.contains("larger") || suggested_action.contains("size") || suggested_action.contains("data"));
                }
            }
        } else {
            panic!("Expected ManualRequired resolution for manual strategy");
        }
    }
}

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[test]
    fn test_same_timestamps_prefer_newer() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferNewer);
        
        // Identical timestamps should fall back to source
        let source_info = create_test_file_info(100, 0, false, false, 0o644);
        let dest_info = create_test_file_info(200, 0, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::BothModified,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution for same timestamps"),
        }
    }

    #[test]
    fn test_same_sizes_prefer_larger() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferLarger);
        
        // Same sizes should fall back to newer
        let source_info = create_test_file_info(100, 100, false, false, 0o644); // newer
        let dest_info = create_test_file_info(100, 0, false, false, 0o644);    // older
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::SizeMismatch,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution for same sizes (fallback to newer)"),
        }
    }

    #[test]
    fn test_zero_size_files() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferLarger);
        
        let source_info = create_test_file_info(0, 0, false, false, 0o644);
        let dest_info = create_test_file_info(0, 100, false, false, 0o644);
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::SizeMismatch,
            &source_info,
            &dest_info,
        ).unwrap();

        // Same size (both zero), should fall back to newer (dest is newer)
        match resolution {
            ConflictResolution::UseDestination => {}
            _ => panic!("Expected UseDestination resolution for zero-size files with dest newer"),
        }
    }

    #[test]
    fn test_invalid_file_name_for_backup() {
        let temp_dir = TempDir::new().unwrap();
        let mut resolver = ConflictResolver::new(ConflictStrategy::BackupAndUseSource);
        resolver.set_backup_directory(temp_dir.path().to_path_buf());
        
        // Test with empty path (should fail)
        let empty_path = PathBuf::new();
        let result = resolver.generate_backup_path(&empty_path, "test");
        assert!(result.is_err());
    }
}
