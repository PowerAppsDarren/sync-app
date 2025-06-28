//! Comprehensive unit tests for the file filter module

use super::*;
use std::path::PathBuf;
use tempfile::TempDir;
use proptest::prelude::*;
use rstest::*;
use test_case::test_case;

mod basic_filter_tests {
    use super::*;

    #[test]
    fn test_default_filter_options() {
        let options = FilterOptions::default();
        
        assert!(options.include_patterns.is_empty());
        assert!(options.exclude_patterns.is_empty());
        assert!(!options.case_sensitive);
        assert!(options.include_hidden);
        assert!(options.max_file_size.is_none());
        assert!(options.min_file_size.is_none());
    }

    #[test]
    fn test_basic_include_filter() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.txt".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("dir/test.txt")));
        assert!(filter.should_include(&PathBuf::from("nested/dir/test.txt")));
        assert!(!filter.should_include(&PathBuf::from("test.rs")));
        assert!(!filter.should_include(&PathBuf::from("test.md")));
    }

    #[test]
    fn test_basic_exclude_filter() {
        let options = FilterOptions {
            exclude_patterns: vec!["**/*.tmp".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("test.rs")));
        assert!(!filter.should_include(&PathBuf::from("test.tmp")));
        assert!(!filter.should_include(&PathBuf::from("dir/test.tmp")));
        assert!(!filter.should_include(&PathBuf::from("nested/dir/test.tmp")));
    }

    #[test]
    fn test_combined_include_exclude_filter() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.txt".to_string()],
            exclude_patterns: vec!["**/temp_*.txt".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("dir/test.txt")));
        assert!(!filter.should_include(&PathBuf::from("temp_file.txt")));
        assert!(!filter.should_include(&PathBuf::from("dir/temp_data.txt")));
        assert!(!filter.should_include(&PathBuf::from("test.rs"))); // Not included by include pattern
    }

    #[test]
    fn test_empty_patterns() {
        let options = FilterOptions::default(); // No patterns
        let filter = FileFilter::new(options).unwrap();

        // Should include everything when no patterns are specified
        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("test.rs")));
        assert!(filter.should_include(&PathBuf::from("any_file.xyz")));
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let options = FilterOptions {
            include_patterns: vec!["[".to_string()], // Invalid glob pattern
            ..Default::default()
        };

        let result = FileFilter::new(options);
        assert!(result.is_err());
    }
}

mod hidden_files_tests {
    use super::*;

    #[test]
    fn test_include_hidden_files() {
        let options = FilterOptions {
            include_hidden: true,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from(".hidden.txt")));
        assert!(filter.should_include(&PathBuf::from("dir/.hidden.txt")));
        assert!(filter.should_include(&PathBuf::from(".hidden/test.txt")));
        assert!(filter.should_include(&PathBuf::from(".")));
        assert!(filter.should_include(&PathBuf::from("..")));
    }

    #[test]
    fn test_exclude_hidden_files() {
        let options = FilterOptions {
            include_hidden: false,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("dir/test.txt")));
        assert!(!filter.should_include(&PathBuf::from(".hidden.txt")));
        assert!(!filter.should_include(&PathBuf::from("dir/.hidden.txt")));
        assert!(!filter.should_include(&PathBuf::from(".hidden/test.txt")));
        
        // . and .. should still be allowed
        assert!(filter.should_include(&PathBuf::from(".")));
        assert!(filter.should_include(&PathBuf::from("..")));
    }

    #[test]
    fn test_nested_hidden_directories() {
        let options = FilterOptions {
            include_hidden: false,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(!filter.should_include(&PathBuf::from(".git/config")));
        assert!(!filter.should_include(&PathBuf::from("project/.vscode/settings.json")));
        assert!(!filter.should_include(&PathBuf::from("deeply/.nested/.hidden/file.txt")));
        assert!(filter.should_include(&PathBuf::from("project/src/main.rs")));
    }
}

mod case_sensitivity_tests {
    use super::*;

    #[test]
    fn test_case_insensitive_matching() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.TXT".to_string()],
            case_sensitive: false,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("test.TXT")));
        assert!(filter.should_include(&PathBuf::from("test.Txt")));
        assert!(!filter.should_include(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_case_sensitive_matching() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.TXT".to_string()],
            case_sensitive: true,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(!filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("test.TXT")));
        assert!(!filter.should_include(&PathBuf::from("test.Txt")));
        assert!(!filter.should_include(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_case_sensitivity_exclude_patterns() {
        let options = FilterOptions {
            exclude_patterns: vec!["**/TEMP_*".to_string()],
            case_sensitive: false,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(!filter.should_include(&PathBuf::from("TEMP_file.txt")));
        assert!(!filter.should_include(&PathBuf::from("temp_file.txt")));
        assert!(!filter.should_include(&PathBuf::from("Temp_File.txt")));
    }
}

mod size_filter_tests {
    use super::*;

    #[test]
    fn test_min_file_size_filter() {
        let filter = FileFilter::with_size_limits(Some(100), None);

        assert!(!filter.should_include_size(50));   // Too small
        assert!(filter.should_include_size(100));   // Exactly min
        assert!(filter.should_include_size(500));   // Above min
        assert!(filter.should_include_size(1000));  // Well above min
    }

    #[test]
    fn test_max_file_size_filter() {
        let filter = FileFilter::with_size_limits(None, Some(1000));

        assert!(filter.should_include_size(50));    // Below max
        assert!(filter.should_include_size(500));   // Below max
        assert!(filter.should_include_size(1000));  // Exactly max
        assert!(!filter.should_include_size(1001)); // Above max
        assert!(!filter.should_include_size(2000)); // Well above max
    }

    #[test]
    fn test_min_max_file_size_filter() {
        let filter = FileFilter::with_size_limits(Some(100), Some(1000));

        assert!(!filter.should_include_size(50));   // Too small
        assert!(filter.should_include_size(100));   // Exactly min
        assert!(filter.should_include_size(500));   // In range
        assert!(filter.should_include_size(1000));  // Exactly max
        assert!(!filter.should_include_size(1001)); // Too large
        assert!(!filter.should_include_size(2000)); // Too large
    }

    #[test]
    fn test_no_size_limits() {
        let filter = FileFilter::with_size_limits(None, None);

        assert!(filter.should_include_size(0));
        assert!(filter.should_include_size(100));
        assert!(filter.should_include_size(u64::MAX));
    }

    #[test]
    fn test_zero_size_files() {
        let filter = FileFilter::with_size_limits(Some(0), None);

        assert!(filter.should_include_size(0));
        assert!(filter.should_include_size(1));
        assert!(filter.should_include_size(100));
    }

    #[test]
    fn test_combined_path_and_size_filter() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.txt".to_string()],
            min_file_size: Some(100),
            max_file_size: Some(1000),
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        // Path matches, size in range
        assert!(filter.should_include_file(&PathBuf::from("test.txt"), 500));
        
        // Path matches, size too small
        assert!(!filter.should_include_file(&PathBuf::from("test.txt"), 50));
        
        // Path matches, size too large
        assert!(!filter.should_include_file(&PathBuf::from("test.txt"), 2000));
        
        // Path doesn't match, size in range
        assert!(!filter.should_include_file(&PathBuf::from("test.rs"), 500));
    }
}

mod pattern_complexity_tests {
    use super::*;

    #[test]
    fn test_wildcard_patterns() {
        let options = FilterOptions {
            include_patterns: vec!["test_*.txt".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test_1.txt")));
        assert!(filter.should_include(&PathBuf::from("test_data.txt")));
        assert!(filter.should_include(&PathBuf::from("test_.txt")));
        assert!(!filter.should_include(&PathBuf::from("test.txt")));
        assert!(!filter.should_include(&PathBuf::from("other_test.txt")));
    }

    #[test]
    fn test_character_class_patterns() {
        let options = FilterOptions {
            include_patterns: vec!["test_[0-9].txt".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test_1.txt")));
        assert!(filter.should_include(&PathBuf::from("test_9.txt")));
        assert!(!filter.should_include(&PathBuf::from("test_a.txt")));
        assert!(!filter.should_include(&PathBuf::from("test_10.txt")));
    }

    #[test]
    fn test_question_mark_patterns() {
        let options = FilterOptions {
            include_patterns: vec!["test?.txt".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("test1.txt")));
        assert!(filter.should_include(&PathBuf::from("testa.txt")));
        assert!(!filter.should_include(&PathBuf::from("test.txt")));
        assert!(!filter.should_include(&PathBuf::from("test12.txt")));
    }

    #[test]
    fn test_recursive_patterns() {
        let options = FilterOptions {
            include_patterns: vec!["src/**/*.rs".to_string()],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("src/main.rs")));
        assert!(filter.should_include(&PathBuf::from("src/lib/mod.rs")));
        assert!(filter.should_include(&PathBuf::from("src/deep/nested/file.rs")));
        assert!(!filter.should_include(&PathBuf::from("main.rs")));
        assert!(!filter.should_include(&PathBuf::from("test/main.rs")));
    }

    #[test]
    fn test_multiple_patterns() {
        let options = FilterOptions {
            include_patterns: vec![
                "**/*.rs".to_string(),
                "**/*.toml".to_string(),
                "**/*.md".to_string(),
            ],
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();

        assert!(filter.should_include(&PathBuf::from("src/main.rs")));
        assert!(filter.should_include(&PathBuf::from("Cargo.toml")));
        assert!(filter.should_include(&PathBuf::from("README.md")));
        assert!(!filter.should_include(&PathBuf::from("test.txt")));
        assert!(!filter.should_include(&PathBuf::from("config.json")));
    }
}

mod preset_filters_tests {
    use super::*;

    #[test]
    fn test_by_extensions_filter() {
        let filter = FileFilter::by_extensions(&["txt", "md"], false).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("README.md")));
        assert!(filter.should_include(&PathBuf::from("dir/file.txt")));
        assert!(!filter.should_include(&PathBuf::from("test.rs")));
        assert!(!filter.should_include(&PathBuf::from("config.json")));
    }

    #[test]
    fn test_by_extensions_with_dots() {
        let filter = FileFilter::by_extensions(&[".txt", ".md"], false).unwrap();

        assert!(filter.should_include(&PathBuf::from("test.txt")));
        assert!(filter.should_include(&PathBuf::from("README.md")));
        assert!(!filter.should_include(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_exclude_common_ignore_patterns() {
        let filter = FileFilter::exclude_common_ignore_patterns().unwrap();

        // Should include normal files
        assert!(filter.should_include(&PathBuf::from("src/main.rs")));
        assert!(filter.should_include(&PathBuf::from("README.md")));
        assert!(filter.should_include(&PathBuf::from("Cargo.toml")));

        // Should exclude common patterns
        assert!(!filter.should_include(&PathBuf::from(".git/config")));
        assert!(!filter.should_include(&PathBuf::from("node_modules/package.json")));
        assert!(!filter.should_include(&PathBuf::from("target/debug/main")));
        assert!(!filter.should_include(&PathBuf::from(".DS_Store")));
        assert!(!filter.should_include(&PathBuf::from("Thumbs.db")));
        assert!(!filter.should_include(&PathBuf::from("temp.log")));
        assert!(!filter.should_include(&PathBuf::from("cache.tmp")));
    }

    #[test]
    fn test_text_files_only() {
        let filter = FileFilter::text_files_only().unwrap();

        assert!(filter.should_include(&PathBuf::from("README.txt")));
        assert!(filter.should_include(&PathBuf::from("doc.md")));
        assert!(filter.should_include(&PathBuf::from("config.json")));
        assert!(filter.should_include(&PathBuf::from("data.xml")));
        assert!(filter.should_include(&PathBuf::from("settings.yaml")));
        assert!(filter.should_include(&PathBuf::from("package.toml")));
        assert!(filter.should_include(&PathBuf::from("app.ini")));

        assert!(!filter.should_include(&PathBuf::from("image.png")));
        assert!(!filter.should_include(&PathBuf::from("video.mp4")));
        assert!(!filter.should_include(&PathBuf::from("binary.exe")));
    }
}

mod filter_combination_tests {
    use super::*;

    #[test]
    fn test_and_combination() {
        let filter1 = FileFilter::by_extensions(&["txt"], false).unwrap();
        let filter2 = FileFilter::with_size_limits(Some(100), Some(1000));

        let combined = filter1.and(&filter2).unwrap();

        // Should include .txt files in size range
        assert!(combined.should_include_file(&PathBuf::from("test.txt"), 500));
        
        // Should exclude .txt files outside size range
        assert!(!combined.should_include_file(&PathBuf::from("test.txt"), 50));
        assert!(!combined.should_include_file(&PathBuf::from("test.txt"), 2000));
        
        // Should exclude non-.txt files even in size range
        assert!(!combined.should_include_file(&PathBuf::from("test.rs"), 500));
    }

    #[test]
    fn test_and_combination_patterns() {
        let filter1 = FileFilter::new(FilterOptions {
            include_patterns: vec!["**/*.txt".to_string()],
            ..Default::default()
        }).unwrap();

        let filter2 = FileFilter::new(FilterOptions {
            exclude_patterns: vec!["**/temp_*".to_string()],
            ..Default::default()
        }).unwrap();

        let combined = filter1.and(&filter2).unwrap();

        assert!(combined.should_include(&PathBuf::from("test.txt")));
        assert!(!combined.should_include(&PathBuf::from("temp_file.txt")));
        assert!(!combined.should_include(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_and_combination_hidden_files() {
        let filter1 = FileFilter::new(FilterOptions {
            include_hidden: true,
            ..Default::default()
        }).unwrap();

        let filter2 = FileFilter::new(FilterOptions {
            include_hidden: false,
            ..Default::default()
        }).unwrap();

        let combined = filter1.and(&filter2).unwrap();

        // Combined should be more restrictive (exclude hidden)
        assert!(combined.should_include(&PathBuf::from("test.txt")));
        assert!(!combined.should_include(&PathBuf::from(".hidden.txt")));
    }

    #[test]
    fn test_and_combination_size_limits() {
        let filter1 = FileFilter::with_size_limits(Some(100), Some(2000));
        let filter2 = FileFilter::with_size_limits(Some(200), Some(1000));

        let combined = filter1.and(&filter2).unwrap();

        // Should use the most restrictive limits
        assert!(!combined.should_include_size(150)); // Below 200 (higher min)
        assert!(combined.should_include_size(500));  // In range
        assert!(!combined.should_include_size(1500)); // Above 1000 (lower max)
    }

    #[test]
    fn test_and_combination_case_sensitivity() {
        let filter1 = FileFilter::new(FilterOptions {
            include_patterns: vec!["**/*.TXT".to_string()],
            case_sensitive: false,
            ..Default::default()
        }).unwrap();

        let filter2 = FileFilter::new(FilterOptions {
            include_patterns: vec!["test_*".to_string()],
            case_sensitive: true,
            ..Default::default()
        }).unwrap();

        let combined = filter1.and(&filter2).unwrap();

        // Should be case sensitive (more restrictive)
        assert!(combined.should_include(&PathBuf::from("test_file.TXT")));
        assert!(!combined.should_include(&PathBuf::from("Test_file.TXT"))); // Wrong case for second pattern
        assert!(!combined.should_include(&PathBuf::from("test_file.txt"))); // Wrong case for first pattern
    }
}

// Property-based tests using proptest
proptest! {
    #[test]
    fn test_pattern_compilation_robustness(
        patterns in prop::collection::vec("[a-zA-Z0-9*?\\[\\]_./\\-]{1,50}", 1..10)
    ) {
        let options = FilterOptions {
            include_patterns: patterns.clone(),
            ..Default::default()
        };

        // Pattern compilation should either succeed or fail gracefully
        let result = FileFilter::new(options);
        
        if result.is_ok() {
            let filter = result.unwrap();
            
            // If compilation succeeded, the filter should work with basic paths
            let test_paths = vec![
                PathBuf::from("test.txt"),
                PathBuf::from("file.rs"),
                PathBuf::from("dir/subfile.md"),
            ];
            
            for path in test_paths {
                // Should not panic when checking paths
                let _ = filter.should_include(&path);
            }
        }
        // If compilation failed, that's acceptable for invalid patterns
    }

    #[test]
    fn test_size_filter_consistency(
        min_size in prop::option::of(0u64..1_000_000),
        max_size in prop::option::of(0u64..1_000_000),
        test_size in 0u64..2_000_000,
    ) {
        // Ensure min <= max if both are specified
        let (min, max) = match (min_size, max_size) {
            (Some(min), Some(max)) if min > max => (Some(max), Some(min)),
            _ => (min_size, max_size),
        };

        let filter = FileFilter::with_size_limits(min, max);
        let result = filter.should_include_size(test_size);

        // Check that the result is consistent with the limits
        let expected = match (min, max) {
            (Some(min_val), Some(max_val)) => test_size >= min_val && test_size <= max_val,
            (Some(min_val), None) => test_size >= min_val,
            (None, Some(max_val)) => test_size <= max_val,
            (None, None) => true,
        };

        prop_assert_eq!(result, expected);
    }

    #[test]
    fn test_path_consistency(
        path_components in prop::collection::vec("[a-zA-Z0-9_\\-]{1,20}", 1..5),
        extension in prop::option::of("[a-z]{1,5}"),
        include_hidden in any::<bool>(),
    ) {
        let mut path = PathBuf::new();
        
        for (i, component) in path_components.iter().enumerate() {
            let comp = if i == 0 && !include_hidden && component.starts_with('.') {
                // Make first component not hidden if we're testing hidden exclusion
                format!("_{}", component)
            } else {
                component.clone()
            };
            path.push(comp);
        }

        if let Some(ext) = extension {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            let new_name = format!("{}.{}", file_name, ext);
            path.set_file_name(new_name);
        }

        let options = FilterOptions {
            include_hidden,
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();
        let result = filter.should_include(&path);

        // If include_hidden is false and path contains hidden components, should be excluded
        if !include_hidden {
            let has_hidden = path.components().any(|comp| {
                comp.as_os_str()
                    .to_str()
                    .map(|s| s.starts_with('.') && s != "." && s != "..")
                    .unwrap_or(false)
            });
            
            if has_hidden {
                prop_assert!(!result);
            } else {
                prop_assert!(result);
            }
        } else {
            // If including hidden files, should always include (no other filters)
            prop_assert!(result);
        }
    }

    #[test]
    fn test_extension_filter_consistency(
        extensions in prop::collection::vec("[a-z]{1,5}", 1..5),
        file_name in "[a-zA-Z0-9_\\-]{1,20}",
        file_ext in "[a-z]{1,5}",
        case_sensitive in any::<bool>(),
    ) {
        let filter = FileFilter::by_extensions(&extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>(), case_sensitive).unwrap();
        
        let test_path = PathBuf::from(format!("{}.{}", file_name, file_ext));
        let result = filter.should_include(&test_path);

        let should_match = if case_sensitive {
            extensions.contains(&file_ext)
        } else {
            extensions.iter().any(|ext| ext.to_lowercase() == file_ext.to_lowercase())
        };

        prop_assert_eq!(result, should_match);
    }

    #[test]
    fn test_combined_filter_consistency(
        include_patterns in prop::collection::vec("[a-zA-Z0-9*._\\-/]{1,20}", 0..3),
        exclude_patterns in prop::collection::vec("[a-zA-Z0-9*._\\-/]{1,20}", 0..3),
        min_size in prop::option::of(0u64..1000),
        max_size in prop::option::of(1000u64..2000),
        test_path in "[a-zA-Z0-9._\\-/]{1,30}",
        test_size in 0u64..3000,
        include_hidden in any::<bool>(),
    ) {
        let options = FilterOptions {
            include_patterns,
            exclude_patterns,
            min_file_size: min_size,
            max_file_size: max_size,
            include_hidden,
            case_sensitive: false,
        };

        // Only test if filter creation succeeds (some patterns might be invalid)
        if let Ok(filter) = FileFilter::new(options) {
            let path = PathBuf::from(&test_path);
            
            let path_result = filter.should_include(&path);
            let size_result = filter.should_include_size(test_size);
            let combined_result = filter.should_include_file(&path, test_size);

            // Combined result should be AND of individual results
            prop_assert_eq!(combined_result, path_result && size_result);
        }
    }
}

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_path() {
        let filter = FileFilter::default();
        
        let empty_path = PathBuf::new();
        assert!(filter.should_include(&empty_path));
    }

    #[test]
    fn test_root_path() {
        let filter = FileFilter::default();
        
        assert!(filter.should_include(&PathBuf::from("/")));
        assert!(filter.should_include(&PathBuf::from("C:\\")));
    }

    #[test]
    fn test_very_long_path() {
        let filter = FileFilter::default();
        
        let long_component = "a".repeat(255);
        let long_path = PathBuf::from(&long_component);
        
        assert!(filter.should_include(&long_path));
    }

    #[test]
    fn test_special_characters_in_path() {
        let filter = FileFilter::default();
        
        // Test various special characters
        assert!(filter.should_include(&PathBuf::from("file with spaces.txt")));
        assert!(filter.should_include(&PathBuf::from("file-with-dashes.txt")));
        assert!(filter.should_include(&PathBuf::from("file_with_underscores.txt")));
        assert!(filter.should_include(&PathBuf::from("file.with.dots.txt")));
    }

    #[test]
    fn test_unicode_in_path() {
        let filter = FileFilter::default();
        
        assert!(filter.should_include(&PathBuf::from("файл.txt"))); // Cyrillic
        assert!(filter.should_include(&PathBuf::from("文件.txt"))); // Chinese
        assert!(filter.should_include(&PathBuf::from("ファイル.txt"))); // Japanese
        assert!(filter.should_include(&PathBuf::from("📁folder/📄file.txt"))); // Emoji
    }

    #[test]
    fn test_max_size_zero() {
        let filter = FileFilter::with_size_limits(None, Some(0));
        
        assert!(filter.should_include_size(0));
        assert!(!filter.should_include_size(1));
    }

    #[test]
    fn test_min_size_max_value() {
        let filter = FileFilter::with_size_limits(Some(u64::MAX), None);
        
        assert!(!filter.should_include_size(u64::MAX - 1));
        assert!(filter.should_include_size(u64::MAX));
    }

    #[test]
    fn test_overlapping_patterns() {
        let options = FilterOptions {
            include_patterns: vec!["**/*.txt".to_string()],
            exclude_patterns: vec!["**/*.txt".to_string()], // Same pattern
            ..Default::default()
        };

        let filter = FileFilter::new(options).unwrap();
        
        // Exclude should take precedence
        assert!(!filter.should_include(&PathBuf::from("test.txt")));
    }

    #[test]
    fn test_pattern_order_independence() {
        let options1 = FilterOptions {
            include_patterns: vec!["**/*.txt".to_string(), "**/*.md".to_string()],
            ..Default::default()
        };

        let options2 = FilterOptions {
            include_patterns: vec!["**/*.md".to_string(), "**/*.txt".to_string()],
            ..Default::default()
        };

        let filter1 = FileFilter::new(options1).unwrap();
        let filter2 = FileFilter::new(options2).unwrap();

        let test_paths = vec![
            PathBuf::from("test.txt"),
            PathBuf::from("doc.md"),
            PathBuf::from("file.rs"),
        ];

        for path in test_paths {
            assert_eq!(filter1.should_include(&path), filter2.should_include(&path));
        }
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use criterion::{black_box, Criterion};

    pub fn benchmark_filter_performance(c: &mut Criterion) {
        c.bench_function("filter_1000_paths", |b| {
            let filter = FileFilter::exclude_common_ignore_patterns().unwrap();
            
            let paths: Vec<_> = (0..1000)
                .map(|i| PathBuf::from(format!("src/file_{}.rs", i)))
                .collect();
            
            b.iter(|| {
                for path in &paths {
                    black_box(filter.should_include(path));
                }
            });
        });

        c.bench_function("complex_filter_performance", |b| {
            let options = FilterOptions {
                include_patterns: vec![
                    "**/*.rs".to_string(),
                    "**/*.toml".to_string(),
                    "**/*.md".to_string(),
                ],
                exclude_patterns: vec![
                    "**/target/**".to_string(),
                    "**/.git/**".to_string(),
                    "**/node_modules/**".to_string(),
                ],
                min_file_size: Some(1),
                max_file_size: Some(1_000_000),
                include_hidden: false,
                case_sensitive: false,
            };

            let filter = FileFilter::new(options).unwrap();
            
            let test_cases: Vec<_> = vec![
                ("src/main.rs", 1000),
                ("target/debug/main", 50000),
                (".git/config", 500),
                ("Cargo.toml", 2000),
                ("README.md", 5000),
                ("node_modules/package.json", 1500),
                ("tests/integration.rs", 3000),
                (".hidden/file.txt", 100),
            ];
            
            b.iter(|| {
                for (path, size) in &test_cases {
                    black_box(filter.should_include_file(&PathBuf::from(path), *size));
                }
            });
        });
    }
}
