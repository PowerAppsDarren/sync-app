//! Diff algorithm for generating sync plans and actions

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncError};
use crate::scanner::FileEntry;
use crate::comparator::{ComparisonMethod, ComparisonResult, FileComparator};

/// Actions that can be performed during synchronization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncAction {
    /// Copy file from source to destination
    Copy {
        source: PathBuf,
        destination: PathBuf,
        file_size: u64,
    },
    /// Update file at destination with source version
    Update {
        source: PathBuf,
        destination: PathBuf,
        file_size: u64,
    },
    /// Delete file at destination
    Delete {
        path: PathBuf,
    },
    /// Create directory at destination
    CreateDirectory {
        path: PathBuf,
    },
    /// File conflict requiring resolution
    Conflict {
        source: PathBuf,
        destination: PathBuf,
        conflict_type: ConflictType,
        source_info: FileInfo,
        destination_info: FileInfo,
    },
    /// Skip file (no action needed)
    Skip {
        path: PathBuf,
        reason: String,
    },
}

/// Types of conflicts that can occur
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConflictType {
    /// Both files modified since last sync
    BothModified,
    /// File vs directory conflict
    FileDirectoryConflict,
    /// Different file types (e.g., regular file vs symlink)
    TypeMismatch,
    /// Permission conflicts
    PermissionConflict,
    /// Size mismatch with same timestamp
    SizeMismatch,
}

/// File information for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub permissions: u32,
    pub hash: Option<String>,
}

impl From<&FileEntry> for FileInfo {
    fn from(entry: &FileEntry) -> Self {
        Self {
            size: entry.size,
            modified: entry.modified,
            is_dir: entry.is_dir,
            is_symlink: entry.is_symlink,
            permissions: entry.permissions,
            hash: entry.hash.clone(),
        }
    }
}

/// A complete sync plan with all actions to be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    /// List of actions to perform
    pub actions: Vec<SyncAction>,
    /// Summary statistics
    pub summary: PlanSummary,
}

/// Summary of a sync plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub total_actions: usize,
    pub copies: usize,
    pub updates: usize,
    pub deletes: usize,
    pub directory_creates: usize,
    pub conflicts: usize,
    pub skips: usize,
    pub total_bytes_to_transfer: u64,
}

impl Default for PlanSummary {
    fn default() -> Self {
        Self {
            total_actions: 0,
            copies: 0,
            updates: 0,
            deletes: 0,
            directory_creates: 0,
            conflicts: 0,
            skips: 0,
            total_bytes_to_transfer: 0,
        }
    }
}

/// Diff engine for generating sync plans
pub struct DiffEngine {
    comparator: FileComparator,
}

impl Default for DiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffEngine {
    /// Create a new diff engine
    pub fn new() -> Self {
        Self {
            comparator: FileComparator::new(),
        }
    }

    /// Generate a sync plan by comparing source and destination file lists
    pub async fn generate_plan(
        &self,
        source_entries: Vec<FileEntry>,
        dest_entries: Vec<FileEntry>,
        comparison_method: ComparisonMethod,
    ) -> Result<SyncPlan> {
        // Create maps for efficient lookup
        let source_map: HashMap<PathBuf, &FileEntry> = source_entries
            .iter()
            .map(|entry| (entry.relative_path.clone(), entry))
            .collect();

        let dest_map: HashMap<PathBuf, &FileEntry> = dest_entries
            .iter()
            .map(|entry| (entry.relative_path.clone(), entry))
            .collect();

        let mut actions = Vec::new();
        let mut processed_paths = HashSet::new();

        // Process all source files
        for source_entry in &source_entries {
            let relative_path = &source_entry.relative_path;
            processed_paths.insert(relative_path.clone());

            let action = if let Some(dest_entry) = dest_map.get(relative_path) {
                // File exists in both source and destination
                self.compare_and_decide(source_entry, dest_entry, comparison_method).await?
            } else {
                // File only exists in source - copy it
                if source_entry.is_dir {
                    SyncAction::CreateDirectory {
                        path: source_entry.relative_path.clone(),
                    }
                } else {
                    SyncAction::Copy {
                        source: source_entry.relative_path.clone(),
                        destination: source_entry.relative_path.clone(),
                        file_size: source_entry.size,
                    }
                }
            };

            actions.push(action);
        }

        // Process destination files that don't exist in source
        for dest_entry in &dest_entries {
            let relative_path = &dest_entry.relative_path;
            
            if !processed_paths.contains(relative_path) {
                // File only exists in destination - delete it
                let action = SyncAction::Delete {
                    path: dest_entry.relative_path.clone(),
                };
                actions.push(action);
            }
        }

        // Generate summary
        let summary = self.generate_summary(&actions);

        Ok(SyncPlan { actions, summary })
    }

    /// Compare two file entries and decide what action to take
    async fn compare_and_decide(
        &self,
        source: &FileEntry,
        destination: &FileEntry,
        comparison_method: ComparisonMethod,
    ) -> Result<SyncAction> {
        // Check for type mismatches
        if source.is_dir != destination.is_dir {
            return Ok(SyncAction::Conflict {
                source: source.relative_path.clone(),
                destination: destination.relative_path.clone(),
                conflict_type: ConflictType::FileDirectoryConflict,
                source_info: source.into(),
                destination_info: destination.into(),
            });
        }

        if source.is_symlink != destination.is_symlink {
            return Ok(SyncAction::Conflict {
                source: source.relative_path.clone(),
                destination: destination.relative_path.clone(),
                conflict_type: ConflictType::TypeMismatch,
                source_info: source.into(),
                destination_info: destination.into(),
            });
        }

        // For directories, no action needed if they both exist
        if source.is_dir && destination.is_dir {
            return Ok(SyncAction::Skip {
                path: source.relative_path.clone(),
                reason: "Directory already exists".to_string(),
            });
        }

        // Compare files
        let comparison_result = self.comparator.compare_entries(source, destination, comparison_method).await?;

        match comparison_result {
            ComparisonResult::Identical => Ok(SyncAction::Skip {
                path: source.relative_path.clone(),
                reason: "Files are identical".to_string(),
            }),
            ComparisonResult::SourceNewer => Ok(SyncAction::Update {
                source: source.relative_path.clone(),
                destination: destination.relative_path.clone(),
                file_size: source.size,
            }),
            ComparisonResult::DestinationNewer => {
                // Destination is newer - this could be a conflict or we might skip
                Ok(SyncAction::Conflict {
                    source: source.relative_path.clone(),
                    destination: destination.relative_path.clone(),
                    conflict_type: ConflictType::BothModified,
                    source_info: source.into(),
                    destination_info: destination.into(),
                })
            }
            ComparisonResult::DifferentSize => {
                if source.modified == destination.modified {
                    // Same timestamp but different size - possible corruption
                    Ok(SyncAction::Conflict {
                        source: source.relative_path.clone(),
                        destination: destination.relative_path.clone(),
                        conflict_type: ConflictType::SizeMismatch,
                        source_info: source.into(),
                        destination_info: destination.into(),
                    })
                } else if source.modified > destination.modified {
                    Ok(SyncAction::Update {
                        source: source.relative_path.clone(),
                        destination: destination.relative_path.clone(),
                        file_size: source.size,
                    })
                } else {
                    Ok(SyncAction::Conflict {
                        source: source.relative_path.clone(),
                        destination: destination.relative_path.clone(),
                        conflict_type: ConflictType::BothModified,
                        source_info: source.into(),
                        destination_info: destination.into(),
                    })
                }
            }
            ComparisonResult::DifferentContent => {
                if source.modified > destination.modified {
                    Ok(SyncAction::Update {
                        source: source.relative_path.clone(),
                        destination: destination.relative_path.clone(),
                        file_size: source.size,
                    })
                } else {
                    Ok(SyncAction::Conflict {
                        source: source.relative_path.clone(),
                        destination: destination.relative_path.clone(),
                        conflict_type: ConflictType::BothModified,
                        source_info: source.into(),
                        destination_info: destination.into(),
                    })
                }
            }
            ComparisonResult::Different => Ok(SyncAction::Update {
                source: source.relative_path.clone(),
                destination: destination.relative_path.clone(),
                file_size: source.size,
            }),
            ComparisonResult::SourceOnly => Ok(SyncAction::Copy {
                source: source.relative_path.clone(),
                destination: destination.relative_path.clone(),
                file_size: source.size,
            }),
            ComparisonResult::DestinationOnly => Ok(SyncAction::Delete {
                path: destination.relative_path.clone(),
            }),
            ComparisonResult::Error(msg) => Err(SyncError::comparison_error(
                &source.path,
                &destination.path,
                msg,
            )),
        }
    }

    /// Generate summary statistics for a list of actions
    fn generate_summary(&self, actions: &[SyncAction]) -> PlanSummary {
        let mut summary = PlanSummary::default();
        summary.total_actions = actions.len();

        for action in actions {
            match action {
                SyncAction::Copy { file_size, .. } => {
                    summary.copies += 1;
                    summary.total_bytes_to_transfer += file_size;
                }
                SyncAction::Update { file_size, .. } => {
                    summary.updates += 1;
                    summary.total_bytes_to_transfer += file_size;
                }
                SyncAction::Delete { .. } => {
                    summary.deletes += 1;
                }
                SyncAction::CreateDirectory { .. } => {
                    summary.directory_creates += 1;
                }
                SyncAction::Conflict { .. } => {
                    summary.conflicts += 1;
                }
                SyncAction::Skip { .. } => {
                    summary.skips += 1;
                }
            }
        }

        summary
    }

    /// Filter actions based on criteria
    pub fn filter_actions(&self, plan: &SyncPlan, filter: ActionFilter) -> SyncPlan {
        let filtered_actions: Vec<SyncAction> = plan.actions
            .iter()
            .filter(|action| self.matches_filter(action, &filter))
            .cloned()
            .collect();

        let summary = self.generate_summary(&filtered_actions);

        SyncPlan {
            actions: filtered_actions,
            summary,
        }
    }

    /// Check if an action matches the filter criteria
    fn matches_filter(&self, action: &SyncAction, filter: &ActionFilter) -> bool {
        match action {
            SyncAction::Copy { .. } => filter.include_copies,
            SyncAction::Update { .. } => filter.include_updates,
            SyncAction::Delete { .. } => filter.include_deletes,
            SyncAction::CreateDirectory { .. } => filter.include_directory_creates,
            SyncAction::Conflict { .. } => filter.include_conflicts,
            SyncAction::Skip { .. } => filter.include_skips,
        }
    }

    /// Sort actions by priority (directories first, then files by size)
    pub fn sort_actions(&self, plan: &mut SyncPlan) {
        plan.actions.sort_by(|a, b| {
            use std::cmp::Ordering;

            // Directories first
            let a_is_dir = matches!(a, SyncAction::CreateDirectory { .. });
            let b_is_dir = matches!(b, SyncAction::CreateDirectory { .. });

            match (a_is_dir, b_is_dir) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => {
                    // For non-directories, sort by file size (larger files first)
                    let a_size = self.get_action_file_size(a);
                    let b_size = self.get_action_file_size(b);
                    b_size.cmp(&a_size)
                }
            }
        });
    }

    /// Get file size from an action
    fn get_action_file_size(&self, action: &SyncAction) -> u64 {
        match action {
            SyncAction::Copy { file_size, .. } => *file_size,
            SyncAction::Update { file_size, .. } => *file_size,
            _ => 0,
        }
    }
}

/// Filter for selecting which actions to include
#[derive(Debug, Clone)]
pub struct ActionFilter {
    pub include_copies: bool,
    pub include_updates: bool,
    pub include_deletes: bool,
    pub include_directory_creates: bool,
    pub include_conflicts: bool,
    pub include_skips: bool,
}

impl Default for ActionFilter {
    fn default() -> Self {
        Self {
            include_copies: true,
            include_updates: true,
            include_deletes: true,
            include_directory_creates: true,
            include_conflicts: true,
            include_skips: false, // Usually don't want to see skips
        }
    }
}

impl ActionFilter {
    /// Create a filter that includes all action types
    pub fn all() -> Self {
        Self {
            include_copies: true,
            include_updates: true,
            include_deletes: true,
            include_directory_creates: true,
            include_conflicts: true,
            include_skips: true,
        }
    }

    /// Create a filter that only includes actions that modify files
    pub fn modifications_only() -> Self {
        Self {
            include_copies: true,
            include_updates: true,
            include_deletes: true,
            include_directory_creates: true,
            include_conflicts: false,
            include_skips: false,
        }
    }

    /// Create a filter that only includes conflicts
    pub fn conflicts_only() -> Self {
        Self {
            include_copies: false,
            include_updates: false,
            include_deletes: false,
            include_directory_creates: false,
            include_conflicts: true,
            include_skips: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn create_test_file_entry(relative_path: &str, size: u64, is_dir: bool) -> FileEntry {
        let temp_dir = TempDir::new().unwrap();
        let full_path = temp_dir.path().join(relative_path);

        FileEntry {
            path: full_path,
            relative_path: PathBuf::from(relative_path),
            size,
            modified: SystemTime::now(),
            created: Some(SystemTime::now()),
            is_dir,
            is_symlink: false,
            hash: None,
            permissions: 0o644,
        }
    }

    #[tokio::test]
    async fn test_copy_new_file() {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![
            create_test_file_entry("new_file.txt", 100, false),
        ];
        let dest_entries = vec![];

        let plan = diff_engine.generate_plan(source_entries, dest_entries, ComparisonMethod::SizeAndTimestamp).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Copy { source, file_size, .. } => {
                assert_eq!(source, &PathBuf::from("new_file.txt"));
                assert_eq!(*file_size, 100);
            }
            _ => panic!("Expected Copy action"),
        }

        assert_eq!(plan.summary.copies, 1);
        assert_eq!(plan.summary.total_bytes_to_transfer, 100);
    }

    #[tokio::test]
    async fn test_delete_removed_file() {
        let diff_engine = DiffEngine::new();
        
        let source_entries = vec![];
        let dest_entries = vec![
            create_test_file_entry("old_file.txt", 100, false),
        ];

        let plan = diff_engine.generate_plan(source_entries, dest_entries, ComparisonMethod::SizeAndTimestamp).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Delete { path } => {
                assert_eq!(path, &PathBuf::from("old_file.txt"));
            }
            _ => panic!("Expected Delete action"),
        }

        assert_eq!(plan.summary.deletes, 1);
    }

    #[tokio::test]
    async fn test_skip_identical_files() {
        let diff_engine = DiffEngine::new();
        
        let source_entry = create_test_file_entry("same_file.txt", 100, false);
        let dest_entry = create_test_file_entry("same_file.txt", 100, false);

        let source_entries = vec![source_entry];
        let dest_entries = vec![dest_entry];

        let plan = diff_engine.generate_plan(source_entries, dest_entries, ComparisonMethod::Size).await.unwrap();

        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            SyncAction::Skip { path, .. } => {
                assert_eq!(path, &PathBuf::from("same_file.txt"));
            }
            _ => panic!("Expected Skip action"),
        }

        assert_eq!(plan.summary.skips, 1);
    }

    #[test]
    fn test_action_filter() {
        let diff_engine = DiffEngine::new();
        
        let actions = vec![
            SyncAction::Copy { source: PathBuf::from("file1"), destination: PathBuf::from("file1"), file_size: 100 },
            SyncAction::Update { source: PathBuf::from("file2"), destination: PathBuf::from("file2"), file_size: 200 },
            SyncAction::Delete { path: PathBuf::from("file3") },
            SyncAction::Skip { path: PathBuf::from("file4"), reason: "identical".to_string() },
        ];

        let plan = SyncPlan {
            summary: diff_engine.generate_summary(&actions),
            actions,
        };

        let filter = ActionFilter::modifications_only();
        let filtered_plan = diff_engine.filter_actions(&plan, filter);

        assert_eq!(filtered_plan.actions.len(), 3); // Copy, Update, Delete (no Skip)
        assert_eq!(filtered_plan.summary.skips, 0);
    }
}
