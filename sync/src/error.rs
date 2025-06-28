//! Error types for the sync engine library

use std::path::PathBuf;

/// Result type alias for sync operations
pub type Result<T> = std::result::Result<T, SyncError>;

/// Comprehensive error type for sync operations
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Path-related errors
    #[error("Path error at '{path}': {message}")]
    Path { path: PathBuf, message: String },

    /// Permission errors
    #[error("Permission error at '{path}': {message}")]
    Permission { path: PathBuf, message: String },

    /// File comparison errors
    #[error("Comparison error: {message}")]
    Comparison {
        message: String,
    },

    /// Filter pattern errors
    #[error("Filter pattern error: {0}")]
    FilterPattern(String),

    /// Conflict resolution errors
    #[error("Conflict resolution error: {0}")]
    ConflictResolution(String),

    /// Hash computation errors
    #[error("Hash computation error for '{path}': {message}")]
    Hash { path: PathBuf, message: String },

    /// Sync operation errors
    #[error("Sync operation failed: {0}")]
    SyncOperation(String),

    /// Progress reporting errors
    #[error("Progress reporting error: {0}")]
    Progress(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic errors with context
    #[error("Error: {0}")]
    Generic(#[from] anyhow::Error),

    /// File attribute preservation errors
    #[error("Attribute preservation error for '{path}': {message}")]
    AttributePreservation { path: PathBuf, message: String },

    /// Directory scanning errors
    #[error("Directory scan error at '{path}': {message}")]
    DirectoryScan { path: PathBuf, message: String },

    /// File copying errors
    #[error("File copy error: {message}")]
    FileCopy {
        message: String,
    },

    /// File deletion errors
    #[error("File deletion error at '{path}': {message}")]
    FileDeletion { path: PathBuf, message: String },

    /// Cancellation error
    #[error("Operation was cancelled")]
    Cancelled,
}

impl SyncError {
    /// Create a new path error
    pub fn path_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Path {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a new permission error
    pub fn permission_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Permission {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a new comparison error
    pub fn comparison_error(
        source: impl AsRef<std::path::Path>,
        dest: impl AsRef<std::path::Path>,
        message: impl Into<String>,
    ) -> Self {
        let full_message = format!(
            "Comparison error between '{}' and '{}': {}",
            source.as_ref().display(),
            dest.as_ref().display(),
            message.into()
        );
        Self::Comparison {
            message: full_message,
        }
    }

    /// Create a new hash error
    pub fn hash_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Hash {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a new attribute preservation error
    pub fn attribute_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::AttributePreservation {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a new directory scan error
    pub fn scan_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::DirectoryScan {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a new file copy error
    pub fn copy_error(
        source: impl AsRef<std::path::Path>,
        dest: impl AsRef<std::path::Path>,
        message: impl Into<String>,
    ) -> Self {
        let full_message = format!(
            "File copy error from '{}' to '{}': {}",
            source.as_ref().display(),
            dest.as_ref().display(),
            message.into()
        );
        Self::FileCopy {
            message: full_message,
        }
    }

    /// Create a new file deletion error
    pub fn deletion_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::FileDeletion {
            path: path.into(),
            message: message.into(),
        }
    }
}
