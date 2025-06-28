//! Directory scanning functionality using walkdir, ignore, and tokio::fs

use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use tokio::fs;
use walkdir::WalkDir;
use ignore::WalkBuilder;

use crate::error::{Result, SyncError};
use crate::filter::{FileFilter, FilterOptions};

/// Options for directory scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    /// Follow symbolic links
    pub follow_links: bool,
    /// Maximum recursion depth (None for unlimited)
    pub max_depth: Option<usize>,
    /// Include hidden files and directories
    pub include_hidden: bool,
    /// Use .gitignore and similar files for filtering
    pub respect_ignore_files: bool,
    /// File filter options
    pub filter_options: Option<FilterOptions>,
    /// Collect file hashes during scan
    pub collect_hashes: bool,
    /// Hash algorithm to use when collect_hashes is true
    pub hash_algorithm: HashAlgorithm,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            follow_links: false,
            max_depth: None,
            include_hidden: false,
            respect_ignore_files: true,
            filter_options: None,
            collect_hashes: false,
            hash_algorithm: HashAlgorithm::Blake3,
        }
    }
}

/// Hash algorithms supported for file scanning
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// SHA-256 hash
    Sha256,
    /// Blake3 hash (faster)
    Blake3,
}

/// File entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Relative path from the scan root
    pub relative_path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Last modified time
    pub modified: SystemTime,
    /// Created time (if available)
    pub created: Option<SystemTime>,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a symbolic link
    pub is_symlink: bool,
    /// File hash (if collected)
    pub hash: Option<String>,
    /// File permissions (Unix-style)
    pub permissions: u32,
}

/// Directory scanner using walkdir and ignore crates
pub struct DirectoryScanner {
    options: ScanOptions,
    filter: Option<FileFilter>,
}

impl DirectoryScanner {
    /// Create a new directory scanner with options
    pub fn new(options: ScanOptions) -> Self {
        let filter = options.filter_options.as_ref().map(|opts| {
            FileFilter::new(opts.clone()).unwrap_or_else(|_| FileFilter::default())
        });

        Self { options, filter }
    }

    /// Scan a directory and return file entries
    pub async fn scan<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<FileEntry>> {
        let root_path = root_path.as_ref();
        
        if !root_path.exists() {
            return Err(SyncError::path_error(
                root_path,
                "Directory does not exist",
            ));
        }

        if !root_path.is_dir() {
            return Err(SyncError::path_error(
                root_path,
                "Path is not a directory",
            ));
        }

        let entries = if self.options.respect_ignore_files {
            self.scan_with_ignore(root_path).await?
        } else {
            self.scan_with_walkdir(root_path).await?
        };

        // Apply filters if configured
        if let Some(filter) = &self.filter {
            Ok(entries.into_iter()
                .filter(|entry| filter.should_include(&entry.relative_path))
                .collect())
        } else {
            Ok(entries)
        }
    }

    /// Scan using the ignore crate (respects .gitignore, etc.)
    async fn scan_with_ignore(&self, root_path: &Path) -> Result<Vec<FileEntry>> {
        let mut builder = WalkBuilder::new(root_path);
        
        builder
            .follow_links(self.options.follow_links)
            .hidden(!self.options.include_hidden);

        if let Some(max_depth) = self.options.max_depth {
            builder.max_depth(Some(max_depth));
        }

        let walk = builder.build();
        let mut entries = Vec::new();

        for result in walk {
            let entry = result.map_err(|e| {
                SyncError::scan_error(root_path, format!("Walk error: {}", e))
            })?;

            let file_entry = self.create_file_entry(entry.path(), root_path).await?;
            entries.push(file_entry);
        }

        Ok(entries)
    }

    /// Scan using walkdir crate (does not respect ignore files)
    async fn scan_with_walkdir(&self, root_path: &Path) -> Result<Vec<FileEntry>> {
        let mut builder = WalkDir::new(root_path);
        
        builder = builder.follow_links(self.options.follow_links);

        if let Some(max_depth) = self.options.max_depth {
            builder = builder.max_depth(max_depth);
        }

        let mut entries = Vec::new();

        for entry in builder {
            let entry = entry.map_err(|e| {
                SyncError::scan_error(root_path, format!("Walk error: {}", e))
            })?;

            let path = entry.path();
            
            // Skip hidden files if not included
            if !self.options.include_hidden && is_hidden(path) {
                continue;
            }

            let file_entry = self.create_file_entry(path, root_path).await?;
            entries.push(file_entry);
        }

        Ok(entries)
    }

    /// Create a FileEntry from a path
    async fn create_file_entry(&self, path: &Path, root_path: &Path) -> Result<FileEntry> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            SyncError::path_error(path, format!("Failed to read metadata: {}", e))
        })?;

        let relative_path = path.strip_prefix(root_path)
            .map_err(|e| SyncError::path_error(path, format!("Failed to create relative path: {}", e)))?
            .to_path_buf();

        let hash = if self.options.collect_hashes && !metadata.is_dir() {
            Some(self.compute_file_hash(path).await?)
        } else {
            None
        };

        Ok(FileEntry {
            path: path.to_path_buf(),
            relative_path,
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            created: metadata.created().ok(),
            is_dir: metadata.is_dir(),
            is_symlink: metadata.file_type().is_symlink(),
            hash,
            permissions: get_permissions(&metadata),
        })
    }

    /// Compute file hash using the configured algorithm
    async fn compute_file_hash(&self, path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
        use tokio::io::AsyncReadExt;

        let mut file = fs::File::open(path).await.map_err(|e| {
            SyncError::hash_error(path, format!("Failed to open file: {}", e))
        })?;

        match self.options.hash_algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                let mut buffer = vec![0; 8192];
                
                loop {
                    let bytes_read = file.read(&mut buffer).await.map_err(|e| {
                        SyncError::hash_error(path, format!("Failed to read file: {}", e))
                    })?;
                    
                    if bytes_read == 0 {
                        break;
                    }
                    
                    hasher.update(&buffer[..bytes_read]);
                }
                
                Ok(format!("{:x}", hasher.finalize()))
            }
            HashAlgorithm::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                let mut buffer = vec![0; 8192];
                
                loop {
                    let bytes_read = file.read(&mut buffer).await.map_err(|e| {
                        SyncError::hash_error(path, format!("Failed to read file: {}", e))
                    })?;
                    
                    if bytes_read == 0 {
                        break;
                    }
                    
                    hasher.update(&buffer[..bytes_read]);
                }
                
                Ok(hasher.finalize().to_hex().to_string())
            }
        }
    }
}

/// Check if a path represents a hidden file or directory
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
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
    // Return a default value or compute from attributes
    if metadata.permissions().readonly() {
        0o444 // Read-only
    } else {
        0o666 // Read-write
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_basic_scan() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::write(root.join("file1.txt"), b"content1").await.unwrap();
        fs::write(root.join("file2.txt"), b"content2").await.unwrap();
        fs::create_dir(root.join("subdir")).await.unwrap();
        fs::write(root.join("subdir").join("file3.txt"), b"content3").await.unwrap();

        let scanner = DirectoryScanner::new(ScanOptions::default());
        let entries = scanner.scan(root).await.unwrap();

        assert!(entries.len() >= 4); // At least root, file1, file2, subdir, file3
        
        // Check that we have both files and directories
        let has_files = entries.iter().any(|e| !e.is_dir);
        let has_dirs = entries.iter().any(|e| e.is_dir);
        assert!(has_files && has_dirs);
    }

    #[tokio::test]
    async fn test_scan_with_hashes() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("test.txt"), b"test content").await.unwrap();

        let options = ScanOptions {
            collect_hashes: true,
            hash_algorithm: HashAlgorithm::Blake3,
            ..Default::default()
        };

        let scanner = DirectoryScanner::new(options);
        let entries = scanner.scan(root).await.unwrap();

        let file_entry = entries.iter().find(|e| e.path.file_name().unwrap() == "test.txt");
        assert!(file_entry.is_some());
        assert!(file_entry.unwrap().hash.is_some());
    }

    #[tokio::test]
    async fn test_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested structure
        fs::create_dir(root.join("level1")).await.unwrap();
        fs::create_dir(root.join("level1").join("level2")).await.unwrap();
        fs::write(root.join("level1").join("level2").join("deep.txt"), b"deep").await.unwrap();

        let options = ScanOptions {
            max_depth: Some(2),
            ..Default::default()
        };

        let scanner = DirectoryScanner::new(options);
        let entries = scanner.scan(root).await.unwrap();

        // Should not include the deep.txt file
        let deep_file = entries.iter().find(|e| e.path.file_name().unwrap() == "deep.txt");
        assert!(deep_file.is_none());
    }
}
