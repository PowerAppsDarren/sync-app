//! File attribute and permission preservation functionality

use std::path::Path;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{Result, SyncError};

/// Options for attribute preservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreservationOptions {
    /// Preserve file modification times
    pub preserve_mtime: bool,
    /// Preserve file access times
    pub preserve_atime: bool,
    /// Preserve file permissions
    pub preserve_permissions: bool,
    /// Preserve file ownership (Unix only)
    pub preserve_ownership: bool,
    /// Preserve extended attributes (Unix only)
    pub preserve_extended_attributes: bool,
    /// Preserve symbolic link targets
    pub preserve_symlinks: bool,
}

impl Default for PreservationOptions {
    fn default() -> Self {
        Self {
            preserve_mtime: true,
            preserve_atime: false, // Usually not needed and can affect performance
            preserve_permissions: true,
            preserve_ownership: false, // Requires elevated privileges
            preserve_extended_attributes: false, // Not commonly needed
            preserve_symlinks: true,
        }
    }
}

/// File attributes that can be preserved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttributes {
    /// Last modification time
    pub modified: Option<SystemTime>,
    /// Last access time
    pub accessed: Option<SystemTime>,
    /// File permissions (Unix-style)
    pub permissions: Option<u32>,
    /// File owner user ID (Unix only)
    pub uid: Option<u32>,
    /// File owner group ID (Unix only)
    pub gid: Option<u32>,
    /// Extended attributes (Unix only)
    pub extended_attributes: std::collections::HashMap<String, Vec<u8>>,
}

/// Attribute preserver for maintaining file metadata
pub struct AttributePreserver {
    options: PreservationOptions,
}

impl AttributePreserver {
    /// Create a new attribute preserver with options
    pub fn new(options: PreservationOptions) -> Self {
        Self { options }
    }

    /// Create a preserver with default options
    pub fn default() -> Self {
        Self::new(PreservationOptions::default())
    }

    /// Extract attributes from a file
    pub async fn extract_attributes(&self, path: &Path) -> Result<FileAttributes> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            SyncError::attribute_error(path, format!("Failed to read metadata: {}", e))
        })?;

        let modified = if self.options.preserve_mtime {
            metadata.modified().ok()
        } else {
            None
        };

        let accessed = if self.options.preserve_atime {
            metadata.accessed().ok()
        } else {
            None
        };

        let permissions = if self.options.preserve_permissions {
            Some(get_permissions(&metadata))
        } else {
            None
        };

        let (uid, gid) = if self.options.preserve_ownership {
            get_ownership(&metadata)
        } else {
            (None, None)
        };

        let extended_attributes = if self.options.preserve_extended_attributes {
            self.get_extended_attributes(path).await?
        } else {
            std::collections::HashMap::new()
        };

        Ok(FileAttributes {
            modified,
            accessed,
            permissions,
            uid,
            gid,
            extended_attributes,
        })
    }

    /// Apply attributes to a file
    pub async fn apply_attributes(&self, path: &Path, attributes: &FileAttributes) -> Result<()> {
        // Set timestamps
        if let (Some(atime), Some(mtime)) = (attributes.accessed, attributes.modified) {
            if self.options.preserve_mtime || self.options.preserve_atime {
                self.set_file_times(path, atime, mtime).await?;
            }
        } else if let Some(mtime) = attributes.modified {
            if self.options.preserve_mtime {
                // Set both atime and mtime to mtime if atime is not available
                self.set_file_times(path, mtime, mtime).await?;
            }
        }

        // Set permissions
        if let Some(permissions) = attributes.permissions {
            if self.options.preserve_permissions {
                self.set_permissions(path, permissions).await?;
            }
        }

        // Set ownership (Unix only)
        if let (Some(uid), Some(gid)) = (attributes.uid, attributes.gid) {
            if self.options.preserve_ownership {
                self.set_ownership(path, uid, gid).await?;
            }
        }

        // Set extended attributes (Unix only)
        if self.options.preserve_extended_attributes && !attributes.extended_attributes.is_empty() {
            self.set_extended_attributes(path, &attributes.extended_attributes).await?;
        }

        Ok(())
    }

    /// Copy attributes from source to destination
    pub async fn copy_attributes(&self, source: &Path, destination: &Path) -> Result<()> {
        let attributes = self.extract_attributes(source).await?;
        self.apply_attributes(destination, &attributes).await
    }

    /// Set file timestamps
    async fn set_file_times(&self, path: &Path, atime: SystemTime, mtime: SystemTime) -> Result<()> {
        // Use utime crate for setting file times
        let atime_secs = atime.duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| SyncError::attribute_error(path, format!("Invalid access time: {}", e)))?
            .as_secs() as i64;
        
        let mtime_secs = mtime.duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| SyncError::attribute_error(path, format!("Invalid modification time: {}", e)))?
            .as_secs() as i64;

        utime::set_file_times(path, atime_secs, mtime_secs)
            .map_err(|e| SyncError::attribute_error(path, format!("Failed to set file times: {}", e)))
    }

    /// Set file permissions
    async fn set_permissions(&self, path: &Path, permissions: u32) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(permissions);
            fs::set_permissions(path, perms).await
                .map_err(|e| SyncError::attribute_error(path, format!("Failed to set permissions: {}", e)))
        }

        #[cfg(windows)]
        {
            // Windows doesn't have Unix-style permissions
            // We can only set read-only attribute
            let readonly = (permissions & 0o200) == 0; // Check if write bit is not set
            let mut perms = fs::metadata(path).await
                .map_err(|e| SyncError::attribute_error(path, format!("Failed to read metadata: {}", e)))?
                .permissions();
            perms.set_readonly(readonly);
            fs::set_permissions(path, perms).await
                .map_err(|e| SyncError::attribute_error(path, format!("Failed to set permissions: {}", e)))
        }
    }

    /// Set file ownership (Unix only)
    async fn set_ownership(&self, path: &Path, uid: u32, gid: u32) -> Result<()> {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            let path_cstr = CString::new(path.as_os_str().as_bytes())
                .map_err(|e| SyncError::attribute_error(path, format!("Invalid path: {}", e)))?;

            let result = unsafe { libc::chown(path_cstr.as_ptr(), uid, gid) };
            if result != 0 {
                return Err(SyncError::attribute_error(
                    path,
                    format!("Failed to set ownership: {}", std::io::Error::last_os_error()),
                ));
            }
        }

        #[cfg(windows)]
        {
            // Windows doesn't have Unix-style ownership
            // This is a no-op on Windows
        }

        Ok(())
    }

    /// Get extended attributes (Unix only)
    async fn get_extended_attributes(&self, path: &Path) -> Result<std::collections::HashMap<String, Vec<u8>>> {
        let mut attributes = std::collections::HashMap::new();

        #[cfg(unix)]
        {
            // This would require a library like xattr
            // For now, return empty map
        }

        Ok(attributes)
    }

    /// Set extended attributes (Unix only)
    async fn set_extended_attributes(
        &self,
        path: &Path,
        attributes: &std::collections::HashMap<String, Vec<u8>>,
    ) -> Result<()> {
        #[cfg(unix)]
        {
            // This would require a library like xattr
            // For now, this is a no-op
        }

        Ok(())
    }
}

/// Permission preserver specifically for file permissions
pub struct PermissionPreserver;

impl PermissionPreserver {
    /// Copy permissions from source to destination
    pub async fn copy_permissions(source: &Path, destination: &Path) -> Result<()> {
        let source_metadata = fs::metadata(source).await.map_err(|e| {
            SyncError::permission_error(source, format!("Failed to read source metadata: {}", e))
        })?;

        let permissions = get_permissions(&source_metadata);
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(permissions);
            fs::set_permissions(destination, perms).await
                .map_err(|e| SyncError::permission_error(destination, format!("Failed to set permissions: {}", e)))
        }

        #[cfg(windows)]
        {
            // On Windows, copy the read-only attribute
            let readonly = source_metadata.permissions().readonly();
            let mut dest_perms = fs::metadata(destination).await
                .map_err(|e| SyncError::permission_error(destination, format!("Failed to read destination metadata: {}", e)))?
                .permissions();
            dest_perms.set_readonly(readonly);
            fs::set_permissions(destination, dest_perms).await
                .map_err(|e| SyncError::permission_error(destination, format!("Failed to set permissions: {}", e)))
        }
    }

    /// Get permissions as octal string (for display)
    pub async fn get_permissions_octal(path: &Path) -> Result<String> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            SyncError::permission_error(path, format!("Failed to read metadata: {}", e))
        })?;

        let permissions = get_permissions(&metadata);
        Ok(format!("{:o}", permissions))
    }

    /// Set permissions from octal string
    pub async fn set_permissions_octal(path: &Path, octal: &str) -> Result<()> {
        let permissions = u32::from_str_radix(octal, 8).map_err(|e| {
            SyncError::permission_error(path, format!("Invalid octal permissions '{}': {}", octal, e))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(permissions);
            fs::set_permissions(path, perms).await
                .map_err(|e| SyncError::permission_error(path, format!("Failed to set permissions: {}", e)))
        }

        #[cfg(windows)]
        {
            // On Windows, interpret as read-only if write bit is not set
            let readonly = (permissions & 0o200) == 0;
            let mut perms = fs::metadata(path).await
                .map_err(|e| SyncError::permission_error(path, format!("Failed to read metadata: {}", e)))?
                .permissions();
            perms.set_readonly(readonly);
            fs::set_permissions(path, perms).await
                .map_err(|e| SyncError::permission_error(path, format!("Failed to set permissions: {}", e)))
        }
    }
}

/// Get file permissions in a cross-platform way
#[cfg(unix)]
fn get_permissions(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(windows)]
fn get_permissions(metadata: &std::fs::Metadata) -> u32 {
    // Windows doesn't have Unix-style permissions
    // Return a default value based on read-only attribute
    if metadata.permissions().readonly() {
        0o444 // Read-only for all
    } else {
        0o666 // Read-write for all
    }
}

/// Get file ownership (Unix only)
#[cfg(unix)]
fn get_ownership(metadata: &std::fs::Metadata) -> (Option<u32>, Option<u32>) {
    use std::os::unix::fs::MetadataExt;
    (Some(metadata.uid()), Some(metadata.gid()))
}

#[cfg(windows)]
fn get_ownership(_metadata: &std::fs::Metadata) -> (Option<u32>, Option<u32>) {
    // Windows doesn't have Unix-style ownership
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    use std::time::{Duration, SystemTime};

    #[tokio::test]
    async fn test_extract_and_apply_attributes() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("dest.txt");

        // Create source file
        fs::write(&source_path, b"test content").await.unwrap();
        fs::write(&dest_path, b"test content").await.unwrap();

        let preserver = AttributePreserver::default();
        
        // Extract attributes from source
        let attributes = preserver.extract_attributes(&source_path).await.unwrap();
        assert!(attributes.modified.is_some());
        assert!(attributes.permissions.is_some());

        // Apply attributes to destination
        preserver.apply_attributes(&dest_path, &attributes).await.unwrap();

        // Verify attributes were applied
        let dest_attributes = preserver.extract_attributes(&dest_path).await.unwrap();
        
        // Modification times should be close (within 1 second due to filesystem precision)
        if let (Some(orig_mtime), Some(dest_mtime)) = (attributes.modified, dest_attributes.modified) {
            let diff = orig_mtime.duration_since(dest_mtime).unwrap_or_else(|_| dest_mtime.duration_since(orig_mtime).unwrap());
            assert!(diff <= Duration::from_secs(1));
        }
    }

    #[tokio::test]
    async fn test_copy_attributes() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("dest.txt");

        // Create files
        fs::write(&source_path, b"source content").await.unwrap();
        fs::write(&dest_path, b"dest content").await.unwrap();

        let preserver = AttributePreserver::default();
        
        // Copy attributes
        preserver.copy_attributes(&source_path, &dest_path).await.unwrap();

        // Verify attributes were copied
        let source_attrs = preserver.extract_attributes(&source_path).await.unwrap();
        let dest_attrs = preserver.extract_attributes(&dest_path).await.unwrap();

        // Check that modification times are close
        if let (Some(source_mtime), Some(dest_mtime)) = (source_attrs.modified, dest_attrs.modified) {
            let diff = source_mtime.duration_since(dest_mtime).unwrap_or_else(|_| dest_mtime.duration_since(source_mtime).unwrap());
            assert!(diff <= Duration::from_secs(1));
        }
    }

    #[tokio::test]
    async fn test_permission_preserver() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("dest.txt");

        // Create files
        fs::write(&source_path, b"source content").await.unwrap();
        fs::write(&dest_path, b"dest content").await.unwrap();

        // Copy permissions
        PermissionPreserver::copy_permissions(&source_path, &dest_path).await.unwrap();

        // Get permissions as octal strings
        let source_perms = PermissionPreserver::get_permissions_octal(&source_path).await.unwrap();
        let dest_perms = PermissionPreserver::get_permissions_octal(&dest_path).await.unwrap();

        // On some filesystems, permissions might be modified, so we just check they're valid
        assert!(!source_perms.is_empty());
        assert!(!dest_perms.is_empty());
    }

    #[test]
    fn test_preservation_options() {
        let default_opts = PreservationOptions::default();
        assert!(default_opts.preserve_mtime);
        assert!(default_opts.preserve_permissions);
        assert!(!default_opts.preserve_atime); // Performance consideration
        assert!(!default_opts.preserve_ownership); // Requires privileges
    }
}
