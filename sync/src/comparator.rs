//! File comparison functionality with multiple comparison methods

use std::path::Path;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::{AsyncReadExt, BufReader};
use sha2::{Sha256, Digest};

use crate::error::{Result, SyncError};
use crate::scanner::FileEntry;

/// Methods for comparing files
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ComparisonMethod {
    /// Compare by file size only
    Size,
    /// Compare by last modified timestamp
    Timestamp,
    /// Compare by size and timestamp
    SizeAndTimestamp,
    /// Compare by SHA-256 hash
    Sha256,
    /// Compare by Blake3 hash (faster)
    Blake3,
    /// Compare byte-by-byte (most thorough but slowest)
    ByteByByte,
    /// Combination of size, timestamp, and hash
    Comprehensive,
}

impl Default for ComparisonMethod {
    fn default() -> Self {
        Self::SizeAndTimestamp
    }
}

/// Result of file comparison
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComparisonResult {
    /// Files are identical
    Identical,
    /// Files are different
    Different,
    /// Source file is newer than destination
    SourceNewer,
    /// Destination file is newer than source
    DestinationNewer,
    /// Files have different sizes
    DifferentSize,
    /// Files have different content (determined by hash or byte comparison)
    DifferentContent,
    /// Source file exists but destination doesn't
    SourceOnly,
    /// Destination file exists but source doesn't
    DestinationOnly,
    /// Comparison failed due to error
    Error(String),
}

/// File comparator with configurable methods
pub struct FileComparator {
    /// Buffer size for byte-by-byte comparison
    buffer_size: usize,
}

impl Default for FileComparator {
    fn default() -> Self {
        Self::new()
    }
}

impl FileComparator {
    /// Create a new file comparator
    pub fn new() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB buffer
        }
    }

    /// Create a new file comparator with custom buffer size
    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self { buffer_size }
    }

    /// Compare two files using the specified method
    pub async fn compare<P1: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        source: P1,
        destination: P2,
        method: ComparisonMethod,
    ) -> Result<ComparisonResult> {
        let source_path = source.as_ref();
        let dest_path = destination.as_ref();

        // Check if files exist
        let source_exists = source_path.exists();
        let dest_exists = dest_path.exists();

        match (source_exists, dest_exists) {
            (true, false) => return Ok(ComparisonResult::SourceOnly),
            (false, true) => return Ok(ComparisonResult::DestinationOnly),
            (false, false) => return Ok(ComparisonResult::Error("Neither file exists".to_string())),
            (true, true) => {}
        }

        // Get metadata for both files
        let source_metadata = fs::metadata(source_path).await.map_err(|e| {
            SyncError::comparison_error(source_path, dest_path, format!("Failed to read source metadata: {}", e))
        })?;

        let dest_metadata = fs::metadata(dest_path).await.map_err(|e| {
            SyncError::comparison_error(source_path, dest_path, format!("Failed to read destination metadata: {}", e))
        })?;

        // Both must be files (not directories)
        if source_metadata.is_dir() || dest_metadata.is_dir() {
            return Err(SyncError::comparison_error(
                source_path,
                dest_path,
                "Cannot compare directories",
            ));
        }

        match method {
            ComparisonMethod::Size => self.compare_by_size(&source_metadata, &dest_metadata),
            ComparisonMethod::Timestamp => self.compare_by_timestamp(&source_metadata, &dest_metadata),
            ComparisonMethod::SizeAndTimestamp => {
                self.compare_by_size_and_timestamp(&source_metadata, &dest_metadata)
            }
            ComparisonMethod::Sha256 => self.compare_by_hash(source_path, dest_path, HashType::Sha256).await,
            ComparisonMethod::Blake3 => self.compare_by_hash(source_path, dest_path, HashType::Blake3).await,
            ComparisonMethod::ByteByByte => self.compare_byte_by_byte(source_path, dest_path).await,
            ComparisonMethod::Comprehensive => {
                self.compare_comprehensive(source_path, dest_path, &source_metadata, &dest_metadata).await
            }
        }
    }

    /// Compare two FileEntry objects
    pub async fn compare_entries(
        &self,
        source: &FileEntry,
        destination: &FileEntry,
        method: ComparisonMethod,
    ) -> Result<ComparisonResult> {
        // If both have hashes and we're using a hash method, compare hashes directly
        if let (Some(source_hash), Some(dest_hash)) = (&source.hash, &destination.hash) {
            match method {
                ComparisonMethod::Sha256 | ComparisonMethod::Blake3 | ComparisonMethod::Comprehensive => {
                    return Ok(if source_hash == dest_hash {
                        ComparisonResult::Identical
                    } else {
                        ComparisonResult::DifferentContent
                    });
                }
                _ => {}
            }
        }

        // Fall back to file-based comparison
        self.compare(&source.path, &destination.path, method).await
    }

    /// Compare files by size only
    fn compare_by_size(
        &self,
        source_metadata: &std::fs::Metadata,
        dest_metadata: &std::fs::Metadata,
    ) -> Result<ComparisonResult> {
        if source_metadata.len() == dest_metadata.len() {
            Ok(ComparisonResult::Identical)
        } else {
            Ok(ComparisonResult::DifferentSize)
        }
    }

    /// Compare files by timestamp only
    fn compare_by_timestamp(
        &self,
        source_metadata: &std::fs::Metadata,
        dest_metadata: &std::fs::Metadata,
    ) -> Result<ComparisonResult> {
        let source_modified = source_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let dest_modified = dest_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        match source_modified.cmp(&dest_modified) {
            std::cmp::Ordering::Greater => Ok(ComparisonResult::SourceNewer),
            std::cmp::Ordering::Less => Ok(ComparisonResult::DestinationNewer),
            std::cmp::Ordering::Equal => Ok(ComparisonResult::Identical),
        }
    }

    /// Compare files by both size and timestamp
    fn compare_by_size_and_timestamp(
        &self,
        source_metadata: &std::fs::Metadata,
        dest_metadata: &std::fs::Metadata,
    ) -> Result<ComparisonResult> {
        // First check size
        if source_metadata.len() != dest_metadata.len() {
            return Ok(ComparisonResult::DifferentSize);
        }

        // Then check timestamp
        self.compare_by_timestamp(source_metadata, dest_metadata)
    }

    /// Compare files by hash
    async fn compare_by_hash(
        &self,
        source_path: &Path,
        dest_path: &Path,
        hash_type: HashType,
    ) -> Result<ComparisonResult> {
        let source_hash = self.compute_hash(source_path, hash_type).await?;
        let dest_hash = self.compute_hash(dest_path, hash_type).await?;

        if source_hash == dest_hash {
            Ok(ComparisonResult::Identical)
        } else {
            Ok(ComparisonResult::DifferentContent)
        }
    }

    /// Compare files byte by byte
    async fn compare_byte_by_byte(&self, source_path: &Path, dest_path: &Path) -> Result<ComparisonResult> {
        let source_file = fs::File::open(source_path).await.map_err(|e| {
            SyncError::comparison_error(source_path, dest_path, format!("Failed to open source file: {}", e))
        })?;

        let dest_file = fs::File::open(dest_path).await.map_err(|e| {
            SyncError::comparison_error(source_path, dest_path, format!("Failed to open destination file: {}", e))
        })?;

        let mut source_reader = BufReader::new(source_file);
        let mut dest_reader = BufReader::new(dest_file);

        let mut source_buffer = vec![0u8; self.buffer_size];
        let mut dest_buffer = vec![0u8; self.buffer_size];

        loop {
            let source_bytes = source_reader.read(&mut source_buffer).await.map_err(|e| {
                SyncError::comparison_error(source_path, dest_path, format!("Failed to read source file: {}", e))
            })?;

            let dest_bytes = dest_reader.read(&mut dest_buffer).await.map_err(|e| {
                SyncError::comparison_error(source_path, dest_path, format!("Failed to read destination file: {}", e))
            })?;

            // Check if we've reached the end of both files
            if source_bytes == 0 && dest_bytes == 0 {
                return Ok(ComparisonResult::Identical);
            }

            // Check if one file is longer than the other
            if source_bytes != dest_bytes {
                return Ok(ComparisonResult::DifferentContent);
            }

            // Compare the buffers
            if source_buffer[..source_bytes] != dest_buffer[..dest_bytes] {
                return Ok(ComparisonResult::DifferentContent);
            }
        }
    }

    /// Comprehensive comparison using size, timestamp, and hash
    async fn compare_comprehensive(
        &self,
        source_path: &Path,
        dest_path: &Path,
        source_metadata: &std::fs::Metadata,
        dest_metadata: &std::fs::Metadata,
    ) -> Result<ComparisonResult> {
        // First check size - if different, no need to continue
        if source_metadata.len() != dest_metadata.len() {
            return Ok(ComparisonResult::DifferentSize);
        }

        // Check timestamp
        let source_modified = source_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let dest_modified = dest_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        // If timestamps are identical, files are likely identical
        if source_modified == dest_modified {
            return Ok(ComparisonResult::Identical);
        }

        // Use hash to determine if content is actually different
        let result = self.compare_by_hash(source_path, dest_path, HashType::Blake3).await?;
        
        match result {
            ComparisonResult::Identical => Ok(ComparisonResult::Identical),
            ComparisonResult::DifferentContent => {
                // Content is different, determine which is newer
                if source_modified > dest_modified {
                    Ok(ComparisonResult::SourceNewer)
                } else {
                    Ok(ComparisonResult::DestinationNewer)
                }
            }
            other => Ok(other),
        }
    }

    /// Compute file hash
    async fn compute_hash(&self, path: &Path, hash_type: HashType) -> Result<String> {
        let mut file = fs::File::open(path).await.map_err(|e| {
            SyncError::hash_error(path, format!("Failed to open file: {}", e))
        })?;

        let mut buffer = vec![0u8; self.buffer_size];

        match hash_type {
            HashType::Sha256 => {
                let mut hasher = Sha256::new();
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
            HashType::Blake3 => {
                let mut hasher = blake3::Hasher::new();
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

    /// Quick comparison that only checks metadata without reading file content
    pub fn quick_compare(
        source: &FileEntry,
        destination: &FileEntry,
    ) -> ComparisonResult {
        // Compare sizes first
        if source.size != destination.size {
            return ComparisonResult::DifferentSize;
        }

        // Compare timestamps
        match source.modified.cmp(&destination.modified) {
            std::cmp::Ordering::Greater => ComparisonResult::SourceNewer,
            std::cmp::Ordering::Less => ComparisonResult::DestinationNewer,
            std::cmp::Ordering::Equal => ComparisonResult::Identical,
        }
    }
}

/// Hash types for file comparison
#[derive(Debug, Clone, Copy)]
enum HashType {
    Sha256,
    Blake3,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    use std::time::Duration;

    #[tokio::test]
    async fn test_size_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let path1 = temp_dir.path().join("file1.txt");
        let path2 = temp_dir.path().join("file2.txt");

        fs::write(&path1, b"hello").await.unwrap();
        fs::write(&path2, b"hello").await.unwrap();

        let comparator = FileComparator::new();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::Size).await.unwrap();
        assert_eq!(result, ComparisonResult::Identical);

        fs::write(&path2, b"hello world").await.unwrap();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::Size).await.unwrap();
        assert_eq!(result, ComparisonResult::DifferentSize);
    }

    #[tokio::test]
    async fn test_hash_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let path1 = temp_dir.path().join("file1.txt");
        let path2 = temp_dir.path().join("file2.txt");

        fs::write(&path1, b"hello world").await.unwrap();
        fs::write(&path2, b"hello world").await.unwrap();

        let comparator = FileComparator::new();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::Sha256).await.unwrap();
        assert_eq!(result, ComparisonResult::Identical);

        fs::write(&path2, b"hello rust").await.unwrap();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::Sha256).await.unwrap();
        assert_eq!(result, ComparisonResult::DifferentContent);
    }

    #[tokio::test]
    async fn test_byte_by_byte_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let path1 = temp_dir.path().join("file1.txt");
        let path2 = temp_dir.path().join("file2.txt");

        fs::write(&path1, b"hello world").await.unwrap();
        fs::write(&path2, b"hello world").await.unwrap();

        let comparator = FileComparator::new();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::ByteByByte).await.unwrap();
        assert_eq!(result, ComparisonResult::Identical);

        fs::write(&path2, b"hello rust").await.unwrap();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::ByteByByte).await.unwrap();
        assert_eq!(result, ComparisonResult::DifferentContent);
    }

    #[tokio::test]
    async fn test_nonexistent_files() {
        let temp_dir = TempDir::new().unwrap();
        let path1 = temp_dir.path().join("file1.txt");
        let path2 = temp_dir.path().join("file2.txt");

        fs::write(&path1, b"hello").await.unwrap();

        let comparator = FileComparator::new();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::Size).await.unwrap();
        assert_eq!(result, ComparisonResult::SourceOnly);

        let result = comparator.compare(&path2, &path1, ComparisonMethod::Size).await.unwrap();
        assert_eq!(result, ComparisonResult::DestinationOnly);
    }

    #[tokio::test]
    async fn test_timestamp_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let path1 = temp_dir.path().join("file1.txt");
        let path2 = temp_dir.path().join("file2.txt");

        fs::write(&path1, b"hello").await.unwrap();
        
        // Add a small delay to ensure different timestamps
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        fs::write(&path2, b"hello").await.unwrap();

        let comparator = FileComparator::new();
        let result = comparator.compare(&path1, &path2, ComparisonMethod::Timestamp).await.unwrap();
        assert_eq!(result, ComparisonResult::DestinationNewer);
    }
}
