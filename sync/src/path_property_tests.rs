//! Property tests for path handling using proptest

use std::path::{Path, PathBuf};
use proptest::prelude::*;
use crate::error::{Result, SyncError};

/// Strategy for generating valid file names
pub fn valid_file_name() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_\\-\\.]{1,50}"
}

/// Strategy for generating valid directory names
pub fn valid_dir_name() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_\\-]{1,30}"
}

/// Strategy for generating file extensions
pub fn file_extension() -> impl Strategy<Value = String> {
    "[a-z]{1,5}"
}

/// Strategy for generating relative paths
pub fn relative_path() -> impl Strategy<Value = PathBuf> {
    prop::collection::vec(valid_dir_name(), 0..5)
        .prop_flat_map(|dirs| {
            (Just(dirs), prop::option::of(valid_file_name()))
        })
        .prop_map(|(dirs, file_name)| {
            let mut path = PathBuf::new();
            for dir in dirs {
                path.push(dir);
            }
            if let Some(name) = file_name {
                path.push(name);
            }
            path
        })
}

/// Strategy for generating absolute paths (Unix-style)
pub fn absolute_unix_path() -> impl Strategy<Value = PathBuf> {
    relative_path().prop_map(|rel| {
        let mut abs = PathBuf::from("/");
        abs.push(rel);
        abs
    })
}

/// Strategy for generating absolute paths (Windows-style)
pub fn absolute_windows_path() -> impl Strategy<Value = PathBuf> {
    relative_path().prop_map(|rel| {
        let mut abs = PathBuf::from("C:\\");
        abs.push(rel);
        abs
    })
}

/// Strategy for generating paths with various characteristics
pub fn path_with_properties() -> impl Strategy<Value = (PathBuf, bool, bool, bool)> {
    (
        relative_path(),
        any::<bool>(), // is_hidden
        any::<bool>(), // is_absolute
        any::<bool>(), // has_extension
    ).prop_map(|(mut path, is_hidden, is_absolute, has_extension)| {
        // Make hidden if requested
        if is_hidden && !path.as_os_str().is_empty() {
            let file_name = path.file_name().unwrap_or_default().to_str().unwrap_or("file");
            let hidden_name = format!(".{}", file_name);
            path.set_file_name(hidden_name);
        }

        // Make absolute if requested
        if is_absolute {
            if cfg!(windows) {
                let mut abs = PathBuf::from("C:\\");
                abs.push(&path);
                path = abs;
            } else {
                let mut abs = PathBuf::from("/");
                abs.push(&path);
                path = abs;
            }
        }

        // Add extension if requested and it's a file
        if has_extension && path.file_name().is_some() {
            let current_name = path.file_name().unwrap().to_str().unwrap();
            if !current_name.contains('.') {
                let new_name = format!("{}.txt", current_name);
                path.set_file_name(new_name);
            }
        }

        (path, is_hidden, is_absolute, has_extension)
    })
}

/// Utility functions for path operations
pub mod path_utils {
    use super::*;

    /// Check if a path is hidden (starts with dot)
    pub fn is_hidden_path(path: &Path) -> bool {
        path.components().any(|component| {
            component.as_os_str()
                .to_str()
                .map(|s| s.starts_with('.') && s != "." && s != "..")
                .unwrap_or(false)
        })
    }

    /// Get the depth of a path (number of components)
    pub fn path_depth(path: &Path) -> usize {
        path.components().count()
    }

    /// Check if a path is safe (no .. components, no absolute paths in relative context)
    pub fn is_safe_relative_path(path: &Path) -> bool {
        !path.is_absolute() && 
        !path.components().any(|comp| matches!(comp, std::path::Component::ParentDir))
    }

    /// Normalize a path by removing redundant components
    pub fn normalize_path(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();
        
        for component in path.components() {
            match component {
                std::path::Component::CurDir => {
                    // Skip current directory references
                }
                std::path::Component::ParentDir => {
                    // Handle parent directory references
                    if normalized.components().count() > 0 && 
                       !normalized.ends_with("..") {
                        normalized.pop();
                    } else {
                        normalized.push("..");
                    }
                }
                _ => {
                    normalized.push(component);
                }
            }
        }
        
        if normalized.as_os_str().is_empty() {
            normalized.push(".");
        }
        
        normalized
    }

    /// Convert a path to a safe relative path
    pub fn to_safe_relative(path: &Path) -> Result<PathBuf> {
        let normalized = normalize_path(path);
        
        if normalized.is_absolute() {
            return Err(SyncError::path_error(
                path,
                "Cannot convert absolute path to relative"
            ));
        }
        
        if !is_safe_relative_path(&normalized) {
            return Err(SyncError::path_error(
                path,
                "Path contains unsafe components"
            ));
        }
        
        Ok(normalized)
    }

    /// Join two paths safely
    pub fn safe_join(base: &Path, relative: &Path) -> Result<PathBuf> {
        if relative.is_absolute() {
            return Err(SyncError::path_error(
                relative,
                "Cannot join with absolute path"
            ));
        }
        
        let safe_relative = to_safe_relative(relative)?;
        Ok(base.join(safe_relative))
    }
}

proptest! {
    #[test]
    fn test_path_normalization_properties(
        path in relative_path()
    ) {
        let normalized = path_utils::normalize_path(&path);
        
        // Normalized path should not contain current directory references
        prop_assert!(!normalized.components().any(|c| matches!(c, std::path::Component::CurDir)));
        
        // If input doesn't contain parent dirs, output shouldn't either (unless it started with them)
        if !path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            prop_assert!(!normalized.components().any(|c| matches!(c, std::path::Component::ParentDir)));
        }
        
        // Normalized path should not be empty (should be "." at minimum)
        prop_assert!(!normalized.as_os_str().is_empty());
    }

    #[test]
    fn test_hidden_path_detection(
        (path, is_hidden, _, _) in path_with_properties()
    ) {
        let detected_hidden = path_utils::is_hidden_path(&path);
        
        // If we explicitly made it hidden, it should be detected as hidden
        if is_hidden && !path.as_os_str().is_empty() {
            prop_assert!(detected_hidden);
        }
        
        // The detection should be consistent
        prop_assert_eq!(detected_hidden, path_utils::is_hidden_path(&path));
    }

    #[test]
    fn test_path_depth_properties(
        path in relative_path()
    ) {
        let depth = path_utils::path_depth(&path);
        
        // Depth should be non-negative
        prop_assert!(depth >= 0);
        
        // Empty path should have depth 0 or 1 (depending on implementation)
        if path.as_os_str().is_empty() {
            prop_assert!(depth <= 1);
        }
        
        // Adding a component should increase depth
        let extended = path.join("extra");
        prop_assert!(path_utils::path_depth(&extended) > depth);
    }

    #[test]
    fn test_safe_relative_path_properties(
        path in relative_path()
    ) {
        let is_safe = path_utils::is_safe_relative_path(&path);
        
        // Non-absolute paths without parent dir components should be safe
        if !path.is_absolute() && 
           !path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            prop_assert!(is_safe);
        }
        
        // Absolute paths should not be safe in relative context
        if path.is_absolute() {
            prop_assert!(!is_safe);
        }
    }

    #[test]
    fn test_safe_join_properties(
        base in relative_path(),
        relative in relative_path()
    ) {
        let join_result = path_utils::safe_join(&base, &relative);
        
        // Join should succeed for safe relative paths
        if path_utils::is_safe_relative_path(&relative) {
            prop_assert!(join_result.is_ok());
            
            if let Ok(joined) = join_result {
                // Joined path should contain both components
                let joined_str = joined.to_string_lossy();
                if !base.as_os_str().is_empty() && !relative.as_os_str().is_empty() {
                    // Should contain elements from both paths
                    prop_assert!(joined.components().count() >= 
                               std::cmp::max(base.components().count(), relative.components().count()));
                }
            }
        }
    }

    #[test]
    fn test_path_conversion_consistency(
        path_str in "[a-zA-Z0-9_/\\\\.-]{1,100}"
    ) {
        let path = PathBuf::from(&path_str);
        
        // Converting to string and back should be consistent for valid paths
        if let Some(converted_str) = path.to_str() {
            let reconverted = PathBuf::from(converted_str);
            prop_assert_eq!(path, reconverted);
        }
        
        // OS string conversion should be consistent
        let os_str = path.as_os_str();
        let from_os_str = PathBuf::from(os_str);
        prop_assert_eq!(path, from_os_str);
    }

    #[test]
    fn test_path_extension_handling(
        name in valid_file_name(),
        ext in file_extension()
    ) {
        let file_with_ext = format!("{}.{}", name, ext);
        let path = PathBuf::from(&file_with_ext);
        
        // Extension should be detectable
        if let Some(detected_ext) = path.extension() {
            prop_assert_eq!(detected_ext, ext.as_str());
        }
        
        // Stem should be the name part
        if let Some(stem) = path.file_stem() {
            prop_assert_eq!(stem, name.as_str());
        }
        
        // File name should be the full name
        if let Some(file_name) = path.file_name() {
            prop_assert_eq!(file_name, file_with_ext.as_str());
        }
    }

    #[test]
    fn test_cross_platform_path_handling(
        components in prop::collection::vec(valid_dir_name(), 1..5)
    ) {
        // Test that path operations work consistently across platforms
        let mut path = PathBuf::new();
        for component in &components {
            path.push(component);
        }
        
        // Components should be preserved
        let path_components: Vec<_> = path.components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        
        // Should contain all our components (though order might differ due to normalization)
        for component in &components {
            prop_assert!(path_components.iter().any(|&c| c == component));
        }
        
        // Path should be valid for the current platform
        prop_assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn test_path_parent_child_relationships(
        parent_components in prop::collection::vec(valid_dir_name(), 1..3),
        child_name in valid_file_name()
    ) {
        let mut parent = PathBuf::new();
        for component in parent_components {
            parent.push(component);
        }
        
        let child = parent.join(&child_name);
        
        // Child should have parent as its parent
        prop_assert_eq!(child.parent(), Some(parent.as_path()));
        
        // Parent should be an ancestor of child
        prop_assert!(child.starts_with(&parent));
        
        // Child should have more components than parent
        prop_assert!(child.components().count() > parent.components().count());
        
        // Child's file name should be what we set
        prop_assert_eq!(child.file_name().and_then(|n| n.to_str()), Some(child_name.as_str()));
    }

    #[test]
    fn test_path_strip_prefix_properties(
        base_components in prop::collection::vec(valid_dir_name(), 1..3),
        additional_components in prop::collection::vec(valid_dir_name(), 1..3)
    ) {
        let mut base = PathBuf::new();
        for component in &base_components {
            base.push(component);
        }
        
        let mut full = base.clone();
        for component in &additional_components {
            full.push(component);
        }
        
        // Should be able to strip the base prefix
        if let Ok(relative) = full.strip_prefix(&base) {
            // Relative path should contain the additional components
            let relative_components: Vec<_> = relative.components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();
            
            for component in &additional_components {
                prop_assert!(relative_components.contains(&component.as_str()));
            }
            
            // Rejoining should give us back the full path
            let rejoined = base.join(relative);
            prop_assert_eq!(rejoined, full);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_path_operations_with_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Test creating nested directory structure
        let nested_path = base_path.join("level1").join("level2").join("level3");
        fs::create_dir_all(&nested_path).unwrap();

        // Test that the path operations work with real filesystem
        assert!(nested_path.exists());
        assert!(nested_path.is_dir());
        
        // Test relative path operations
        let relative = nested_path.strip_prefix(base_path).unwrap();
        assert_eq!(relative, Path::new("level1/level2/level3"));
        
        // Test safe join
        let rejoined = path_utils::safe_join(base_path, relative).unwrap();
        assert_eq!(rejoined, nested_path);
    }

    #[test]
    fn test_hidden_file_detection_with_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create normal and hidden files
        let normal_file = base_path.join("normal.txt");
        let hidden_file = base_path.join(".hidden.txt");
        let nested_hidden = base_path.join(".hidden_dir").join("file.txt");

        fs::write(&normal_file, b"content").unwrap();
        fs::write(&hidden_file, b"content").unwrap();
        fs::create_dir_all(nested_hidden.parent().unwrap()).unwrap();
        fs::write(&nested_hidden, b"content").unwrap();

        // Test detection
        assert!(!path_utils::is_hidden_path(&normal_file));
        assert!(path_utils::is_hidden_path(&hidden_file));
        assert!(path_utils::is_hidden_path(&nested_hidden));
    }

    #[test]
    fn test_path_normalization_with_complex_paths() {
        // Test various complex path scenarios
        let test_cases = vec![
            ("./a/b/../c", "a/c"),
            ("a/./b/./c", "a/b/c"),
            ("a/b/../../c", "c"),
            ("./.", "."),
            ("a/b/c/../../..", "."),
        ];

        for (input, expected) in test_cases {
            let input_path = PathBuf::from(input);
            let normalized = path_utils::normalize_path(&input_path);
            let expected_path = PathBuf::from(expected);
            
            assert_eq!(normalized, expected_path, 
                "Failed for input '{}': expected '{}', got '{}'", 
                input, expected, normalized.display());
        }
    }

    #[test]
    fn test_safe_relative_path_validation() {
        let test_cases = vec![
            ("normal/path", true),
            ("../outside", false),
            ("/absolute/path", false),
            ("./current/path", true),
            ("path/with/../parent", false),
            ("", true), // Empty path is considered safe
        ];

        for (input, expected_safe) in test_cases {
            let path = PathBuf::from(input);
            let is_safe = path_utils::is_safe_relative_path(&path);
            
            assert_eq!(is_safe, expected_safe,
                "Safety check failed for '{}': expected {}, got {}",
                input, expected_safe, is_safe);
        }
    }
}
