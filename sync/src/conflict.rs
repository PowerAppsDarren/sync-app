//! Conflict resolution strategies for sync operations

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncError};
use crate::diff::{SyncAction, ConflictType, FileInfo};

/// Strategies for resolving conflicts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ConflictStrategy {
    /// Always use the source file
    PreferSource,
    /// Always use the destination file
    PreferDestination,
    /// Use the newer file (by modification time)
    PreferNewer,
    /// Use the older file (by modification time)
    PreferOlder,
    /// Use the larger file (by size)
    PreferLarger,
    /// Use the smaller file (by size)
    PreferSmaller,
    /// Skip the conflicted file
    Skip,
    /// Create a backup of the destination and use source
    BackupAndUseSource,
    /// Create a backup of the source and keep destination
    BackupAndKeepDestination,
    /// Prompt user for manual resolution
    Manual,
    /// Fail on any conflict
    Fail,
}

impl Default for ConflictStrategy {
    fn default() -> Self {
        Self::Manual
    }
}

/// Result of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use source file
    UseSource,
    /// Use destination file
    UseDestination,
    /// Skip this conflict
    Skip,
    /// Create backup and use source
    BackupAndUseSource { backup_path: PathBuf },
    /// Create backup and keep destination
    BackupAndKeepDestination { backup_path: PathBuf },
    /// Manual resolution required
    ManualRequired {
        source_info: FileInfo,
        destination_info: FileInfo,
        suggested_action: String,
    },
    /// Resolution failed
    Failed { reason: String },
}

/// Conflict resolver with configurable strategies
pub struct ConflictResolver {
    /// Default strategy for all conflict types
    default_strategy: ConflictStrategy,
    /// Specific strategies for different conflict types
    type_strategies: std::collections::HashMap<ConflictType, ConflictStrategy>,
    /// Backup directory for conflict resolution
    backup_directory: Option<PathBuf>,
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(ConflictStrategy::default())
    }
}

impl ConflictResolver {
    /// Create a new conflict resolver with a default strategy
    pub fn new(default_strategy: ConflictStrategy) -> Self {
        Self {
            default_strategy,
            type_strategies: std::collections::HashMap::new(),
            backup_directory: None,
        }
    }

    /// Set a specific strategy for a conflict type
    pub fn set_strategy_for_type(&mut self, conflict_type: ConflictType, strategy: ConflictStrategy) {
        self.type_strategies.insert(conflict_type, strategy);
    }

    /// Set the backup directory for backup strategies
    pub fn set_backup_directory(&mut self, path: PathBuf) {
        self.backup_directory = Some(path);
    }

    /// Resolve a conflict using the configured strategies
    pub fn resolve_conflict(
        &self,
        source: &PathBuf,
        destination: &PathBuf,
        conflict_type: ConflictType,
        source_info: &FileInfo,
        destination_info: &FileInfo,
    ) -> Result<ConflictResolution> {
        let strategy = self.type_strategies
            .get(&conflict_type)
            .copied()
            .unwrap_or(self.default_strategy);

        self.apply_strategy(
            strategy,
            source,
            destination,
            conflict_type,
            source_info,
            destination_info,
        )
    }

    /// Apply a specific resolution strategy
    fn apply_strategy(
        &self,
        strategy: ConflictStrategy,
        source: &PathBuf,
        destination: &PathBuf,
        conflict_type: ConflictType,
        source_info: &FileInfo,
        destination_info: &FileInfo,
    ) -> Result<ConflictResolution> {
        match strategy {
            ConflictStrategy::PreferSource => Ok(ConflictResolution::UseSource),
            
            ConflictStrategy::PreferDestination => Ok(ConflictResolution::UseDestination),
            
            ConflictStrategy::PreferNewer => {
                if source_info.modified > destination_info.modified {
                    Ok(ConflictResolution::UseSource)
                } else if destination_info.modified > source_info.modified {
                    Ok(ConflictResolution::UseDestination)
                } else {
                    // Same timestamp, fall back to preferring source
                    Ok(ConflictResolution::UseSource)
                }
            }
            
            ConflictStrategy::PreferOlder => {
                if source_info.modified < destination_info.modified {
                    Ok(ConflictResolution::UseSource)
                } else if destination_info.modified < source_info.modified {
                    Ok(ConflictResolution::UseDestination)
                } else {
                    // Same timestamp, fall back to preferring source
                    Ok(ConflictResolution::UseSource)
                }
            }
            
            ConflictStrategy::PreferLarger => {
                if source_info.size > destination_info.size {
                    Ok(ConflictResolution::UseSource)
                } else if destination_info.size > source_info.size {
                    Ok(ConflictResolution::UseDestination)
                } else {
                    // Same size, fall back to preferring newer
                    self.apply_strategy(
                        ConflictStrategy::PreferNewer,
                        source,
                        destination,
                        conflict_type,
                        source_info,
                        destination_info,
                    )
                }
            }
            
            ConflictStrategy::PreferSmaller => {
                if source_info.size < destination_info.size {
                    Ok(ConflictResolution::UseSource)
                } else if destination_info.size < source_info.size {
                    Ok(ConflictResolution::UseDestination)
                } else {
                    // Same size, fall back to preferring newer
                    self.apply_strategy(
                        ConflictStrategy::PreferNewer,
                        source,
                        destination,
                        conflict_type,
                        source_info,
                        destination_info,
                    )
                }
            }
            
            ConflictStrategy::Skip => Ok(ConflictResolution::Skip),
            
            ConflictStrategy::BackupAndUseSource => {
                let backup_path = self.generate_backup_path(destination, "dest")?;
                Ok(ConflictResolution::BackupAndUseSource { backup_path })
            }
            
            ConflictStrategy::BackupAndKeepDestination => {
                let backup_path = self.generate_backup_path(source, "src")?;
                Ok(ConflictResolution::BackupAndKeepDestination { backup_path })
            }
            
            ConflictStrategy::Manual => {
                let suggested_action = self.suggest_resolution(source_info, destination_info, conflict_type);
                Ok(ConflictResolution::ManualRequired {
                    source_info: source_info.clone(),
                    destination_info: destination_info.clone(),
                    suggested_action,
                })
            }
            
            ConflictStrategy::Fail => Ok(ConflictResolution::Failed {
                reason: format!("Conflict resolution strategy is set to fail on conflict type: {:?}", conflict_type),
            }),
        }
    }

    /// Generate a backup path for a file
    fn generate_backup_path(&self, original_path: &PathBuf, suffix: &str) -> Result<PathBuf> {
        let backup_dir = self.backup_directory.as_ref().ok_or_else(|| {
            SyncError::ConflictResolution("No backup directory configured".to_string())
        })?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let file_name = original_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                SyncError::ConflictResolution(format!("Invalid file name: {:?}", original_path))
            })?;

        let backup_name = format!("{}_{}.{}", file_name, suffix, timestamp);
        Ok(backup_dir.join(backup_name))
    }

    /// Suggest a resolution based on file properties
    fn suggest_resolution(&self, source_info: &FileInfo, destination_info: &FileInfo, conflict_type: ConflictType) -> String {
        match conflict_type {
            ConflictType::BothModified => {
                if source_info.modified > destination_info.modified {
                    "Source file is newer, consider using source".to_string()
                } else if destination_info.modified > source_info.modified {
                    "Destination file is newer, consider keeping destination".to_string()
                } else {
                    "Files have same modification time, consider comparing content".to_string()
                }
            }
            ConflictType::FileDirectoryConflict => {
                "File/directory conflict: consider renaming one of them".to_string()
            }
            ConflictType::TypeMismatch => {
                "File type mismatch: check if both files are needed".to_string()
            }
            ConflictType::PermissionConflict => {
                "Permission conflict: verify which permissions are correct".to_string()
            }
            ConflictType::SizeMismatch => {
                if source_info.size > destination_info.size {
                    "Source file is larger, may contain more data".to_string()
                } else {
                    "Destination file is larger, may contain more data".to_string()
                }
            }
        }
    }

    /// Convert a conflict resolution to a sync action
    pub fn resolution_to_action(
        &self,
        resolution: ConflictResolution,
        source: PathBuf,
        destination: PathBuf,
        source_info: &FileInfo,
    ) -> Result<Option<SyncAction>> {
        match resolution {
            ConflictResolution::UseSource => Ok(Some(SyncAction::Update {
                source,
                destination,
                file_size: source_info.size,
            })),
            
            ConflictResolution::UseDestination => Ok(Some(SyncAction::Skip {
                path: destination,
                reason: "Keeping destination file due to conflict resolution".to_string(),
            })),
            
            ConflictResolution::Skip => Ok(Some(SyncAction::Skip {
                path: destination,
                reason: "Skipped due to conflict".to_string(),
            })),
            
            ConflictResolution::BackupAndUseSource { backup_path } => {
                // This would need to be handled by the sync engine
                // Return the main action and let the engine handle the backup
                Ok(Some(SyncAction::Update {
                    source,
                    destination,
                    file_size: source_info.size,
                }))
            }
            
            ConflictResolution::BackupAndKeepDestination { .. } => Ok(Some(SyncAction::Skip {
                path: destination,
                reason: "Keeping destination file with backup of source".to_string(),
            })),
            
            ConflictResolution::ManualRequired { .. } => {
                // Return a conflict action that requires manual intervention
                Ok(None) // Indicates manual resolution needed
            }
            
            ConflictResolution::Failed { reason } => Err(SyncError::ConflictResolution(reason)),
        }
    }

    /// Create a conflict resolver with common presets
    pub fn with_preset(preset: ConflictPreset) -> Self {
        let mut resolver = match preset {
            ConflictPreset::SafeSync => Self::new(ConflictStrategy::Manual),
            ConflictPreset::ForceSource => Self::new(ConflictStrategy::PreferSource),
            ConflictPreset::ForceDestination => Self::new(ConflictStrategy::PreferDestination),
            ConflictPreset::PreferNewer => Self::new(ConflictStrategy::PreferNewer),
            ConflictPreset::SkipConflicts => Self::new(ConflictStrategy::Skip),
        };

        // Set specific strategies for different conflict types if needed
        match preset {
            ConflictPreset::SafeSync => {
                // For safe sync, be more conservative with destructive conflicts
                resolver.set_strategy_for_type(ConflictType::FileDirectoryConflict, ConflictStrategy::Fail);
                resolver.set_strategy_for_type(ConflictType::TypeMismatch, ConflictStrategy::Manual);
            }
            _ => {}
        }

        resolver
    }
}

/// Common conflict resolution presets
#[derive(Debug, Clone, Copy)]
pub enum ConflictPreset {
    /// Safe sync - manual resolution for all conflicts
    SafeSync,
    /// Always prefer source files
    ForceSource,
    /// Always prefer destination files
    ForceDestination,
    /// Prefer newer files
    PreferNewer,
    /// Skip all conflicts
    SkipConflicts,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, Duration};

    fn create_file_info(size: u64, modified_offset_secs: i64) -> FileInfo {
        let base_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let modified = if modified_offset_secs >= 0 {
            base_time + Duration::from_secs(modified_offset_secs as u64)
        } else {
            base_time - Duration::from_secs((-modified_offset_secs) as u64)
        };

        FileInfo {
            size,
            modified,
            is_dir: false,
            is_symlink: false,
            permissions: 0o644,
            hash: None,
        }
    }

    #[test]
    fn test_prefer_newer_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferNewer);
        
        let source_info = create_file_info(100, 100); // newer
        let dest_info = create_file_info(100, 0);     // older
        
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
    fn test_prefer_larger_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::PreferLarger);
        
        let source_info = create_file_info(200, 0);   // larger
        let dest_info = create_file_info(100, 0);     // smaller
        
        let resolution = resolver.resolve_conflict(
            &PathBuf::from("source"),
            &PathBuf::from("dest"),
            ConflictType::SizeMismatch,
            &source_info,
            &dest_info,
        ).unwrap();

        match resolution {
            ConflictResolution::UseSource => {}
            _ => panic!("Expected UseSource resolution"),
        }
    }

    #[test]
    fn test_skip_strategy() {
        let resolver = ConflictResolver::new(ConflictStrategy::Skip);
        
        let source_info = create_file_info(100, 0);
        let dest_info = create_file_info(100, 0);
        
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
        
        let source_info = create_file_info(100, 100);
        let dest_info = create_file_info(200, 0);
        
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
    fn test_type_specific_strategy() {
        let mut resolver = ConflictResolver::new(ConflictStrategy::PreferSource);
        resolver.set_strategy_for_type(ConflictType::FileDirectoryConflict, ConflictStrategy::Fail);
        
        let source_info = create_file_info(100, 0);
        let dest_info = create_file_info(100, 0);
        
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
            _ => panic!("Expected UseSource resolution"),
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
            _ => panic!("Expected Failed resolution"),
        }
    }

    #[test]
    fn test_conflict_presets() {
        let safe_resolver = ConflictResolver::with_preset(ConflictPreset::SafeSync);
        assert_eq!(safe_resolver.default_strategy, ConflictStrategy::Manual);

        let force_source_resolver = ConflictResolver::with_preset(ConflictPreset::ForceSource);
        assert_eq!(force_source_resolver.default_strategy, ConflictStrategy::PreferSource);
    }
}
