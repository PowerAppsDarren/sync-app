//! File filtering functionality using globset

use std::path::Path;
use serde::{Deserialize, Serialize};
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::error::{Result, SyncError};

/// File filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOptions {
    /// Patterns to include (if empty, include all)
    pub include_patterns: Vec<String>,
    /// Patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Case sensitive matching
    pub case_sensitive: bool,
    /// Include hidden files (starting with .)
    pub include_hidden: bool,
    /// Maximum file size in bytes (None for no limit)
    pub max_file_size: Option<u64>,
    /// Minimum file size in bytes
    pub min_file_size: Option<u64>,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            case_sensitive: false,
            include_hidden: true,
            max_file_size: None,
            min_file_size: None,
        }
    }
}

/// File filter using globset patterns
pub struct FileFilter {
    include_set: Option<GlobSet>,
    exclude_set: Option<GlobSet>,
    options: FilterOptions,
}

impl Default for FileFilter {
    fn default() -> Self {
        Self::new(FilterOptions::default()).unwrap()
    }
}

impl FileFilter {
    /// Create a new file filter with the given options
    pub fn new(options: FilterOptions) -> Result<Self> {
        let include_set = if options.include_patterns.is_empty() {
            None
        } else {
            Some(Self::build_globset(&options.include_patterns, options.case_sensitive)?)
        };

        let exclude_set = if options.exclude_patterns.is_empty() {
            None
        } else {
            Some(Self::build_globset(&options.exclude_patterns, options.case_sensitive)?)
        };

        Ok(Self {
            include_set,
            exclude_set,
            options,
        })
    }

    /// Check if a path should be included based on the filter rules
    pub fn should_include(&self, path: &Path) -> bool {
        // Check hidden file filter
        if !self.options.include_hidden && self.is_hidden(path) {
            return false;
        }

        // Check include patterns (if any)
        if let Some(include_set) = &self.include_set {
            if !include_set.is_match(path) {
                return false;
            }
        }

        // Check exclude patterns
        if let Some(exclude_set) = &self.exclude_set {
            if exclude_set.is_match(path) {
                return false;
            }
        }

        true
    }

    /// Check if a file should be included based on size constraints
    pub fn should_include_size(&self, file_size: u64) -> bool {
        if let Some(max_size) = self.options.max_file_size {
            if file_size > max_size {
                return false;
            }
        }

        if let Some(min_size) = self.options.min_file_size {
            if file_size < min_size {
                return false;
            }
        }

        true
    }

    /// Comprehensive check including both path and size filters
    pub fn should_include_file(&self, path: &Path, file_size: u64) -> bool {
        self.should_include(path) && self.should_include_size(file_size)
    }

    /// Build a globset from patterns
    fn build_globset(patterns: &[String], case_sensitive: bool) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();

        for pattern in patterns {
            let mut glob = globset::GlobBuilder::new(pattern);
            
            if !case_sensitive {
                glob.case_insensitive(true);
            }

            let compiled_glob = glob.build().map_err(|e| {
                SyncError::FilterPattern(format!("Failed to compile glob '{}': {}", pattern, e))
            })?;
            
            builder.add(compiled_glob);
        }

        builder.build().map_err(|e| {
            SyncError::FilterPattern(format!("Failed to build globset: {}", e))
        })
    }

    /// Check if a path represents a hidden file or directory
    fn is_hidden(&self, path: &Path) -> bool {
        path.components().any(|component| {
            component.as_os_str()
                .to_str()
                .map(|s| s.starts_with('.') && s != "." && s != "..")
                .unwrap_or(false)
        })
    }

    /// Get the filter options
    pub fn options(&self) -> &FilterOptions {
        &self.options
    }

    /// Create a filter that includes only specific file extensions
    pub fn by_extensions(extensions: &[&str], case_sensitive: bool) -> Result<Self> {
        let patterns = extensions.iter()
            .map(|ext| {
                if ext.starts_with('.') {
                    format!("**/*{}", ext)
                } else {
                    format!("**/*.{}", ext)
                }
            })
            .collect();

        let options = FilterOptions {
            include_patterns: patterns,
            case_sensitive,
            ..Default::default()
        };

        Self::new(options)
    }

    /// Create a filter that excludes specific patterns (commonly ignored files)
    pub fn exclude_common_ignore_patterns() -> Result<Self> {
        let exclude_patterns = vec![
            "**/.git/**".to_string(),
            "**/.svn/**".to_string(),
            "**/.hg/**".to_string(),
            "**/node_modules/**".to_string(),
            "**/target/**".to_string(),
            "**/.DS_Store".to_string(),
            "**/Thumbs.db".to_string(),
            "**/.vs/**".to_string(),
            "**/.vscode/**".to_string(),
            "**/bin/**".to_string(),
            "**/obj/**".to_string(),
            "**/*.tmp".to_string(),
            "**/*.temp".to_string(),
            "**/*.log".to_string(),
        ];

        let options = FilterOptions {
            exclude_patterns,
            ..Default::default()
        };

        Self::new(options)
    }

    /// Create a filter that includes only text files
    pub fn text_files_only() -> Result<Self> {
        let include_patterns = vec![
            "**/*.txt".to_string(),
            "**/*.md".to_string(),
            "**/*.rst".to_string(),
            "**/*.json".to_string(),
            "**/*.xml".to_string(),
            "**/*.yaml".to_string(),
            "**/*.yml".to_string(),
            "**/*.toml".to_string(),
            "**/*.ini".to_string(),
            "**/*.cfg".to_string(),
            "**/*.conf".to_string(),
        ];

        let options = FilterOptions {
            include_patterns,
            ..Default::default()
        };

        Self::new(options)
    }

    /// Create a filter with size constraints
    pub fn with_size_limits(min_size: Option<u64>, max_size: Option<u64>) -> Self {
        let options = FilterOptions {
            min_file_size: min_size,
            max_file_size: max_size,
            ..Default::default()
        };

        Self::new(options).unwrap()
    }

    /// Combine this filter with another filter using AND logic
    pub fn and(&self, other: &FileFilter) -> Result<FileFilter> {
        let mut combined_include = self.options.include_patterns.clone();
        combined_include.extend(other.options.include_patterns.clone());

        let mut combined_exclude = self.options.exclude_patterns.clone();
        combined_exclude.extend(other.options.exclude_patterns.clone());

        let options = FilterOptions {
            include_patterns: combined_include,
            exclude_patterns: combined_exclude,
            case_sensitive: self.options.case_sensitive || other.options.case_sensitive,
            include_hidden: self.options.include_hidden && other.options.include_hidden,
            max_file_size: match (self.options.max_file_size, other.options.max_file_size) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
            min_file_size: match (self.options.min_file_size, other.options.min_file_size) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
        };

        Self::new(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_basic_include_filter() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.txt".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("dir/test.txt")));
        assert!(!filter.should_include(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_exclude_filter() {
        let options = FilterOptions {
            exclude_patterns: vec!["**/*.tmp".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(!filter.should_include(&PathBuf::from("test.tmp")));
        assert!(!filter.should_include(&PathBuf::from("dir/test.tmp")));
    }

    #[test]
    fn test_hidden_files() {
        let options = FilterOptions {
            include_hidden: false,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(!filter.should_include(&PathBuf::from(".hidden.txt")));
        assert!(!filter.should_include(&PathBuf::from("dir/.hidden.txt")));
        assert!(!filter.should_include(&PathBuf::from(".hidden/test.txt")));
    }

    #[test]
    fn test_size_filter() {
        let filter = FileFilter::with_size_limits(Some(100), Some(1000));

        assert!(!filter.should_include_size(50));   // Too small
        assert!(filter.should_include_size(500));   // Just right
        assert!(!filter.should_include_size(2000)); // Too large
    }

    #[test]
    fn test_extension_filter() {
        let filter = FileFilter::by_extensions(&["txt", "md"], false).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("README.md")));
        assert!(!filter.should_include(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_case_sensitivity() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.TXT".to_string()],
            case_sensitive: false,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("test.TXT")));

        let options = FilterOptions {
            include_patterns: vec!["**/*.TXT".to_string()],
            case_sensitive: true,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(!filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("test.TXT")));
    }

    #[test]
    fn test_combined_filter() {
        let filter1 = FileFilter::by_extensions(&["txt"], false).unwrap();
        let filter2 = FileFilter::with_size_limits(Some(100), Some(1000));

        let combined = filter1.and(&filter2).unwrap();

        assert!(combined.should_include_file(&PathBuf::from("test.txt"), 500));
        assert!(!combined.should_include_file(&PathBuf::from("test.txt"), 50));
        assert!(!combined.should_include_file(&PathBuf::from("test.rs"), 500));
    }
}
