# Comprehensive Testing Guide

This document describes the comprehensive testing setup for the sync application, including unit tests, property tests, integration tests, and cross-platform builds.

## Test Structure

### Unit Tests

#### Diff Engine Tests (`src/diff_tests.rs`)
- **Basic Operations**: Copy new files, delete removed files, skip identical files
- **Update Logic**: Handle newer files, conflict detection
- **Complex Scenarios**: Multiple file types, mixed actions
- **Filter Operations**: Action filtering with various criteria
- **Plan Sorting**: Directory-first ordering, size-based sorting
- **Hash Comparisons**: Different hash algorithms and content validation
- **Property Tests**: Plan summary consistency, path handling edge cases

#### Conflict Resolver Tests (`src/conflict_tests.rs`)
- **Strategy Testing**: All conflict resolution strategies (PreferSource, PreferNewer, etc.)
- **Backup Operations**: Backup path generation and validation
- **Type-Specific Strategies**: Different strategies for different conflict types
- **Suggestion Generation**: Contextual suggestions for manual resolution
- **Preset Configurations**: SafeSync, ForceSource, PreferNewer presets
- **Resolution to Action**: Converting resolutions to sync actions
- **Property Tests**: Strategy consistency, backup path generation, conflict type handling
- **Edge Cases**: Same timestamps, zero-size files, invalid paths

#### Filter Parser Tests (`src/filter_tests.rs`)
- **Basic Filtering**: Include/exclude patterns, pattern compilation
- **Hidden Files**: Detection and filtering of hidden files and directories
- **Case Sensitivity**: Case-sensitive and case-insensitive matching
- **Size Filters**: Min/max file size constraints
- **Complex Patterns**: Wildcards, character classes, recursive patterns
- **Preset Filters**: Extension filters, common ignore patterns, text files
- **Filter Combinations**: AND logic for combining multiple filters
- **Property Tests**: Pattern compilation robustness, size filter consistency
- **Edge Cases**: Unicode paths, special characters, overlapping patterns

#### Path Property Tests (`src/path_property_tests.rs`)
- **Path Normalization**: Removing redundant components (`.`, `..`)
- **Hidden Path Detection**: Cross-platform hidden file detection
- **Path Safety**: Validation of relative paths, preventing directory traversal
- **Path Operations**: Join, strip prefix, parent-child relationships
- **Cross-Platform Handling**: Windows vs Unix path separators and conventions
- **Conversion Consistency**: String to Path and back conversions
- **Extension Handling**: File name, stem, and extension parsing

### Integration Tests

#### PocketBase Integration (`src/integration_tests.rs`)
- **Temporary Instance Management**: Automatic PocketBase download and setup
- **Schema Creation**: Test collections and indexes
- **Authentication**: Admin user creation and token management
- **Data Operations**: CRUD operations on sync entries
- **Cleanup**: Automatic cleanup of test data and processes
- **Cross-Platform Support**: Platform-specific PocketBase executables
- **Test Utilities**: Directory structure creation, timeout handling

#### Features
- Automatic PocketBase executable discovery and download
- Isolated test instances with random ports
- Complete lifecycle management (start, test, cleanup)
- Real HTTP API testing with authentication
- Test data creation and verification

### Property-Based Testing

Uses `proptest` to generate thousands of test cases with random inputs:

#### Path Properties
- Valid file and directory name generation
- Path normalization invariants
- Cross-platform path handling
- Safe relative path validation
- Join operation properties

#### Filter Properties
- Pattern compilation robustness
- Size filter consistency
- Combined filter logic
- Extension matching consistency

#### Diff Engine Properties
- Plan summary consistency
- File size calculations
- Action count verification

## Test Categories

### Unit Tests
```bash
# Run all unit tests
cargo test

# Run specific test module
cargo test diff_tests
cargo test conflict_tests
cargo test filter_tests
```

### Property Tests
```bash
# Run property tests specifically
cargo test proptest

# Run with more test cases
PROPTEST_CASES=10000 cargo test proptest
```

### Integration Tests
```bash
# Run integration tests (requires PocketBase)
cargo test integration_tests

# Run with PocketBase auto-download
cargo test integration_tests -- --ignored
```

### Benchmark Tests
```bash
# Run performance benchmarks
cargo test benchmark_tests

# Run with criterion (if configured)
cargo bench
```

## Cross-Platform CI Matrix

### Platforms Tested
- **Ubuntu Latest**
  - x86_64-unknown-linux-gnu (stable, beta)
  - aarch64-unknown-linux-gnu (stable)
- **Windows Latest**
  - x86_64-pc-windows-msvc (stable)
  - i686-pc-windows-msvc (stable)
- **macOS Latest**
  - x86_64-apple-darwin (stable)
  - aarch64-apple-darwin (stable)

### CI Pipeline Stages

1. **Formatting Check**: `cargo fmt --check`
2. **Linting**: `cargo clippy` with warnings as errors
3. **Unit Tests**: Platform-specific test execution
4. **Property Tests**: Extended proptest runs
5. **Cross-Compilation**: Multi-target builds
6. **Artifact Upload**: Build artifacts for each platform
7. **Coverage**: Code coverage with tarpaulin (Linux only)

## Test Utilities

### Integration Test Harness
```rust
// Start temporary PocketBase instance
let mut harness = IntegrationTestHarness::new();
harness.with_pocketbase(|pb| async move {
    // Test with real PocketBase instance
    Ok(())
}).await?;
```

### Test Directory Creation
```rust
// Create structured test directory
let temp_dir = integration_tests::utils::create_test_directory_structure()?;

// Create test file with specific size
integration_tests::utils::create_test_file("test.txt", 1024)?;
```

### Property Test Strategies
```rust
// Generate valid file paths
let strategy = path_property_tests::relative_path();

// Generate paths with specific properties
let strategy = path_property_tests::path_with_properties();
```

## Running Tests Locally

### Prerequisites
```bash
# Install testing dependencies
cargo install cargo-tarpaulin  # For coverage
cargo install criterion        # For benchmarks
```

### Full Test Suite
```bash
# Run complete test suite
cargo test --all-features --workspace

# Run with coverage
cargo tarpaulin --all-features --workspace --out html

# Run property tests with more cases
PROPTEST_CASES=10000 cargo test proptest
```

### Platform-Specific Testing
```bash
# Test cross-compilation (Linux to Windows)
cargo test --target x86_64-pc-windows-gnu

# Test specific features
cargo test --features "integration-tests"
```

## Test Configuration

### Proptest Configuration
- Default: 256 test cases per property
- CI: Extended to 1000+ cases
- Timeout: 30 seconds per test
- Shrinking: Enabled for minimal failing cases

### Integration Test Settings
- PocketBase startup timeout: 30 seconds
- Test data cleanup: Automatic
- Port allocation: Random to avoid conflicts
- Isolation: Each test gets fresh instance

### CI Test Matrix
- **Fast Tests**: Unit and property tests on native platforms
- **Cross-Compilation**: Build verification for all targets
- **Coverage**: Detailed coverage reporting on Linux
- **Artifacts**: Build artifacts for release preparation

## Debugging Tests

### Verbose Output
```bash
# Show test output
cargo test -- --nocapture

# Show property test cases
PROPTEST_VERBOSE=1 cargo test proptest

# Debug integration tests
RUST_LOG=debug cargo test integration_tests
```

### Failed Test Investigation
```bash
# Run specific failing test
cargo test test_name -- --exact

# Save property test failures
PROPTEST_PERSIST_DIR=./proptest-regressions cargo test proptest
```

## Test Maintenance

### Adding New Tests
1. Follow existing patterns in test modules
2. Use property tests for algorithmic verification
3. Add integration tests for end-to-end scenarios
4. Update CI matrix for new platforms if needed

### Test Performance
- Keep unit tests fast (< 1s each)
- Use `#[ignore]` for slow integration tests
- Property tests should complete in reasonable time
- Benchmark critical performance paths

### Coverage Goals
- Unit tests: > 90% line coverage
- Integration tests: > 80% feature coverage
- Property tests: Edge case validation
- Cross-platform: All major platforms supported
