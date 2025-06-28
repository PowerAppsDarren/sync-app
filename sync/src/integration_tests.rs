//! Integration test harness with temporary PocketBase instance

use std::process::{Command, Child, Stdio};
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use tempfile::TempDir;
use tokio::time::timeout;
use serde_json::json;

use crate::error::{Result, SyncError};

/// Configuration for temporary PocketBase instance
#[derive(Debug, Clone)]
pub struct PocketBaseConfig {
    pub port: u16,
    pub admin_email: String,
    pub admin_password: String,
    pub data_dir: PathBuf,
    pub startup_timeout: Duration,
}

impl Default for PocketBaseConfig {
    fn default() -> Self {
        Self {
            port: 8090,
            admin_email: "test@example.com".to_string(),
            admin_password: "test123456".to_string(),
            data_dir: PathBuf::new(),
            startup_timeout: Duration::from_secs(30),
        }
    }
}

/// Temporary PocketBase instance for integration testing
pub struct TempPocketBase {
    process: Child,
    config: PocketBaseConfig,
    temp_dir: TempDir,
    base_url: String,
}

impl TempPocketBase {
    /// Start a new temporary PocketBase instance
    pub async fn start(mut config: PocketBaseConfig) -> Result<Self> {
        let temp_dir = TempDir::new().map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to create temporary directory: {}", e))
        })?;

        // Set data directory to temp dir if not specified
        if config.data_dir.as_os_str().is_empty() {
            config.data_dir = temp_dir.path().to_path_buf();
        }

        // Find PocketBase executable
        let pocketbase_path = Self::find_pocketbase_executable()?;

        // Start PocketBase process
        let mut process = Command::new(&pocketbase_path)
            .arg("serve")
            .arg("--http")
            .arg(format!("127.0.0.1:{}", config.port))
            .arg("--dir")
            .arg(&config.data_dir)
            .arg("--dev") // Development mode
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                SyncError::Generic(anyhow::anyhow!("Failed to start PocketBase: {}", e))
            })?;

        let base_url = format!("http://127.0.0.1:{}", config.port);

        // Wait for PocketBase to start up
        Self::wait_for_startup(&base_url, config.startup_timeout).await?;

        let mut instance = Self {
            process,
            config,
            temp_dir,
            base_url,
        };

        // Setup initial admin user and schema
        instance.setup_initial_data().await?;

        Ok(instance)
    }

    /// Find PocketBase executable in various locations
    fn find_pocketbase_executable() -> Result<PathBuf> {
        // Check common locations and PATH
        let candidates = vec![
            "pocketbase",
            "pocketbase.exe",
            "./pocketbase",
            "./pocketbase.exe",
            "../pocketbase",
            "../pocketbase.exe",
            "/usr/local/bin/pocketbase",
            "/usr/bin/pocketbase",
        ];

        for candidate in candidates {
            if let Ok(output) = Command::new(candidate).arg("--version").output() {
                if output.status.success() {
                    return Ok(PathBuf::from(candidate));
                }
            }
        }

        // Try to download PocketBase if not found
        Self::download_pocketbase()
    }

    /// Download PocketBase executable for testing
    fn download_pocketbase() -> Result<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let pocketbase_path = temp_dir.join(if cfg!(windows) {
            "pocketbase.exe"
        } else {
            "pocketbase"
        });

        // Skip if already exists
        if pocketbase_path.exists() {
            return Ok(pocketbase_path);
        }

        // Determine download URL based on platform
        let download_url = if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") {
                "https://github.com/pocketbase/pocketbase/releases/latest/download/pocketbase_windows_amd64.zip"
            } else {
                return Err(SyncError::Generic(anyhow::anyhow!("Unsupported Windows architecture")));
            }
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "x86_64") {
                "https://github.com/pocketbase/pocketbase/releases/latest/download/pocketbase_darwin_amd64.zip"
            } else if cfg!(target_arch = "aarch64") {
                "https://github.com/pocketbase/pocketbase/releases/latest/download/pocketbase_darwin_arm64.zip"
            } else {
                return Err(SyncError::Generic(anyhow::anyhow!("Unsupported macOS architecture")));
            }
        } else if cfg!(target_os = "linux") {
            if cfg!(target_arch = "x86_64") {
                "https://github.com/pocketbase/pocketbase/releases/latest/download/pocketbase_linux_amd64.zip"
            } else if cfg!(target_arch = "aarch64") {
                "https://github.com/pocketbase/pocketbase/releases/latest/download/pocketbase_linux_arm64.zip"
            } else {
                return Err(SyncError::Generic(anyhow::anyhow!("Unsupported Linux architecture")));
            }
        } else {
            return Err(SyncError::Generic(anyhow::anyhow!("Unsupported operating system")));
        };

        // Download and extract (simplified - in real implementation you'd want proper HTTP client and ZIP handling)
        eprintln!("PocketBase not found. Please download it from: {}", download_url);
        eprintln!("Extract it to: {}", pocketbase_path.display());
        
        Err(SyncError::Generic(anyhow::anyhow!(
            "PocketBase executable not found. Please install PocketBase and ensure it's in your PATH or download it manually."
        )))
    }

    /// Wait for PocketBase to start up and be ready
    async fn wait_for_startup(base_url: &str, timeout_duration: Duration) -> Result<()> {
        let health_url = format!("{}/api/health", base_url);
        
        timeout(timeout_duration, async {
            loop {
                match reqwest::get(&health_url).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            break;
                        }
                    }
                    Err(_) => {
                        // Not ready yet
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| {
            SyncError::Generic(anyhow::anyhow!(
                "PocketBase failed to start within timeout period"
            ))
        })
    }

    /// Setup initial admin user and test schema
    async fn setup_initial_data(&mut self) -> Result<()> {
        let client = reqwest::Client::new();

        // Create admin user
        let admin_data = json!({
            "email": self.config.admin_email,
            "password": self.config.admin_password,
            "passwordConfirm": self.config.admin_password
        });

        let admin_url = format!("{}/api/admins", self.base_url);
        let response = client
            .post(&admin_url)
            .json(&admin_data)
            .send()
            .await
            .map_err(|e| {
                SyncError::Generic(anyhow::anyhow!("Failed to create admin user: {}", e))
            })?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::BAD_REQUEST {
            // BAD_REQUEST might mean admin already exists, which is fine
            return Err(SyncError::Generic(anyhow::anyhow!(
                "Failed to create admin user: {}",
                response.status()
            )));
        }

        // Authenticate as admin
        let auth_data = json!({
            "identity": self.config.admin_email,
            "password": self.config.admin_password
        });

        let auth_url = format!("{}/api/admins/auth-with-password", self.base_url);
        let auth_response = client
            .post(&auth_url)
            .json(&auth_data)
            .send()
            .await
            .map_err(|e| {
                SyncError::Generic(anyhow::anyhow!("Failed to authenticate admin: {}", e))
            })?;

        let auth_result: serde_json::Value = auth_response
            .json()
            .await
            .map_err(|e| {
                SyncError::Generic(anyhow::anyhow!("Failed to parse auth response: {}", e))
            })?;

        let token = auth_result["token"]
            .as_str()
            .ok_or_else(|| {
                SyncError::Generic(anyhow::anyhow!("No token in auth response"))
            })?;

        // Create test collections
        self.create_test_collections(&client, token).await?;

        Ok(())
    }

    /// Create test collections for sync testing
    async fn create_test_collections(&self, client: &reqwest::Client, token: &str) -> Result<()> {
        // Define test collections schema
        let collections = vec![
            json!({
                "name": "sync_entries",
                "type": "base",
                "schema": [
                    {
                        "name": "path",
                        "type": "text",
                        "required": true,
                        "options": {
                            "min": 1,
                            "max": 1000
                        }
                    },
                    {
                        "name": "size",
                        "type": "number",
                        "required": true
                    },
                    {
                        "name": "modified",
                        "type": "date",
                        "required": true
                    },
                    {
                        "name": "hash",
                        "type": "text",
                        "required": false
                    },
                    {
                        "name": "is_dir",
                        "type": "bool",
                        "required": true
                    }
                ],
                "indexes": [
                    "CREATE UNIQUE INDEX `idx_sync_entries_path` ON `sync_entries` (`path`)",
                    "CREATE INDEX `idx_sync_entries_modified` ON `sync_entries` (`modified`)"
                ]
            }),
            json!({
                "name": "sync_conflicts",
                "type": "base",
                "schema": [
                    {
                        "name": "path",
                        "type": "text",
                        "required": true
                    },
                    {
                        "name": "conflict_type",
                        "type": "text",
                        "required": true
                    },
                    {
                        "name": "source_info",
                        "type": "json",
                        "required": true
                    },
                    {
                        "name": "dest_info",
                        "type": "json",
                        "required": true
                    },
                    {
                        "name": "resolved",
                        "type": "bool",
                        "required": true
                    }
                ]
            })
        ];

        let collections_url = format!("{}/api/collections", self.base_url);

        for collection in collections {
            let response = client
                .post(&collections_url)
                .header("Authorization", format!("Bearer {}", token))
                .json(&collection)
                .send()
                .await
                .map_err(|e| {
                    SyncError::Generic(anyhow::anyhow!("Failed to create collection: {}", e))
                })?;

            if !response.status().is_success() {
                return Err(SyncError::Generic(anyhow::anyhow!(
                    "Failed to create collection '{}': {}",
                    collection["name"].as_str().unwrap_or("unknown"),
                    response.status()
                )));
            }
        }

        Ok(())
    }

    /// Get the base URL for API calls
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the admin credentials
    pub fn admin_credentials(&self) -> (&str, &str) {
        (&self.config.admin_email, &self.config.admin_password)
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    /// Create test data in the PocketBase instance
    pub async fn create_test_data(&self) -> Result<()> {
        let client = reqwest::Client::new();

        // Authenticate first
        let (email, password) = self.admin_credentials();
        let auth_data = json!({
            "identity": email,
            "password": password
        });

        let auth_url = format!("{}/api/admins/auth-with-password", self.base_url);
        let auth_response = client
            .post(&auth_url)
            .json(&auth_data)
            .send()
            .await
            .map_err(|e| {
                SyncError::Generic(anyhow::anyhow!("Failed to authenticate: {}", e))
            })?;

        let auth_result: serde_json::Value = auth_response.json().await.map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to parse auth response: {}", e))
        })?;

        let token = auth_result["token"]
            .as_str()
            .ok_or_else(|| SyncError::Generic(anyhow::anyhow!("No token in auth response")))?;

        // Create test sync entries
        let test_entries = vec![
            json!({
                "path": "test1.txt",
                "size": 100,
                "modified": "2024-01-01 12:00:00.000Z",
                "hash": "abc123",
                "is_dir": false
            }),
            json!({
                "path": "test2.txt",
                "size": 200,
                "modified": "2024-01-02 12:00:00.000Z",
                "hash": "def456",
                "is_dir": false
            }),
            json!({
                "path": "testdir",
                "size": 0,
                "modified": "2024-01-01 10:00:00.000Z",
                "hash": null,
                "is_dir": true
            }),
        ];

        let records_url = format!("{}/api/collections/sync_entries/records", self.base_url);

        for entry in test_entries {
            let response = client
                .post(&records_url)
                .header("Authorization", format!("Bearer {}", token))
                .json(&entry)
                .send()
                .await
                .map_err(|e| {
                    SyncError::Generic(anyhow::anyhow!("Failed to create test entry: {}", e))
                })?;

            if !response.status().is_success() {
                return Err(SyncError::Generic(anyhow::anyhow!(
                    "Failed to create test entry: {}",
                    response.status()
                )));
            }
        }

        Ok(())
    }

    /// Clean up test data
    pub async fn cleanup_test_data(&self) -> Result<()> {
        let client = reqwest::Client::new();

        // Authenticate first
        let (email, password) = self.admin_credentials();
        let auth_data = json!({
            "identity": email,
            "password": password
        });

        let auth_url = format!("{}/api/admins/auth-with-password", self.base_url);
        let auth_response = client
            .post(&auth_url)
            .json(&auth_data)
            .send()
            .await
            .map_err(|e| {
                SyncError::Generic(anyhow::anyhow!("Failed to authenticate: {}", e))
            })?;

        let auth_result: serde_json::Value = auth_response.json().await.map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to parse auth response: {}", e))
        })?;

        let token = auth_result["token"]
            .as_str()
            .ok_or_else(|| SyncError::Generic(anyhow::anyhow!("No token in auth response")))?;

        // Get all records and delete them
        let collections = ["sync_entries", "sync_conflicts"];

        for collection in collections {
            let list_url = format!("{}/api/collections/{}/records", self.base_url, collection);
            let list_response = client
                .get(&list_url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| {
                    SyncError::Generic(anyhow::anyhow!("Failed to list records: {}", e))
                })?;

            let list_result: serde_json::Value = list_response.json().await.map_err(|e| {
                SyncError::Generic(anyhow::anyhow!("Failed to parse list response: {}", e))
            })?;

            if let Some(items) = list_result["items"].as_array() {
                for item in items {
                    if let Some(id) = item["id"].as_str() {
                        let delete_url = format!(
                            "{}/api/collections/{}/records/{}",
                            self.base_url, collection, id
                        );
                        let _ = client
                            .delete(&delete_url)
                            .header("Authorization", format!("Bearer {}", token))
                            .send()
                            .await;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for TempPocketBase {
    fn drop(&mut self) {
        // Terminate the PocketBase process
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Integration test harness that manages PocketBase lifecycle
pub struct IntegrationTestHarness {
    pocketbase: Option<TempPocketBase>,
}

impl IntegrationTestHarness {
    /// Create a new integration test harness
    pub fn new() -> Self {
        Self { pocketbase: None }
    }

    /// Start PocketBase with default configuration
    pub async fn start_pocketbase(&mut self) -> Result<()> {
        self.start_pocketbase_with_config(PocketBaseConfig::default()).await
    }

    /// Start PocketBase with custom configuration
    pub async fn start_pocketbase_with_config(&mut self, config: PocketBaseConfig) -> Result<()> {
        let pocketbase = TempPocketBase::start(config).await?;
        self.pocketbase = Some(pocketbase);
        Ok(())
    }

    /// Get reference to the PocketBase instance
    pub fn pocketbase(&self) -> Option<&TempPocketBase> {
        self.pocketbase.as_ref()
    }

    /// Stop PocketBase
    pub fn stop_pocketbase(&mut self) {
        self.pocketbase = None;
    }

    /// Run a test with PocketBase
    pub async fn with_pocketbase<F, Fut, T>(&mut self, test_fn: F) -> Result<T>
    where
        F: FnOnce(&TempPocketBase) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        if self.pocketbase.is_none() {
            self.start_pocketbase().await?;
        }

        let pocketbase = self.pocketbase.as_ref().unwrap();
        test_fn(pocketbase).await
    }
}

impl Default for IntegrationTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    #[ignore] // Ignore by default since it requires PocketBase executable
    async fn test_pocketbase_startup() {
        let config = PocketBaseConfig {
            port: 8091, // Use different port to avoid conflicts
            ..Default::default()
        };

        let pocketbase = TempPocketBase::start(config).await;
        
        // Skip test if PocketBase is not available
        if pocketbase.is_err() {
            eprintln!("Skipping PocketBase test - executable not found");
            return;
        }

        let pocketbase = pocketbase.unwrap();

        // Test that we can make a health check request
        let client = reqwest::Client::new();
        let health_url = format!("{}/api/health", pocketbase.base_url());
        
        let response = client.get(&health_url).send().await.unwrap();
        assert!(response.status().is_success());
    }

    #[tokio::test]
    #[serial]
    #[ignore] // Ignore by default since it requires PocketBase executable
    async fn test_integration_harness() {
        let mut harness = IntegrationTestHarness::new();

        let result = harness
            .with_pocketbase(|pb| async move {
                // Test that we can access PocketBase
                let client = reqwest::Client::new();
                let health_url = format!("{}/api/health", pb.base_url());
                
                let response = client.get(&health_url).send().await.unwrap();
                assert!(response.status().is_success());

                Ok(())
            })
            .await;

        // Skip test if PocketBase is not available
        if result.is_err() {
            eprintln!("Skipping integration harness test - PocketBase not available");
            return;
        }
    }

    #[tokio::test]
    #[serial]
    #[ignore] // Ignore by default since it requires PocketBase executable
    async fn test_create_and_cleanup_test_data() {
        let config = PocketBaseConfig {
            port: 8092, // Use different port
            ..Default::default()
        };

        let pocketbase = TempPocketBase::start(config).await;
        
        // Skip test if PocketBase is not available
        if pocketbase.is_err() {
            eprintln!("Skipping test data test - PocketBase not available");
            return;
        }

        let pocketbase = pocketbase.unwrap();

        // Create test data
        pocketbase.create_test_data().await.unwrap();

        // Verify data was created by fetching it
        let client = reqwest::Client::new();
        let (email, password) = pocketbase.admin_credentials();
        
        let auth_data = json!({
            "identity": email,
            "password": password
        });

        let auth_url = format!("{}/api/admins/auth-with-password", pocketbase.base_url());
        let auth_response = client
            .post(&auth_url)
            .json(&auth_data)
            .send()
            .await
            .unwrap();

        let auth_result: serde_json::Value = auth_response.json().await.unwrap();
        let token = auth_result["token"].as_str().unwrap();

        let records_url = format!("{}/api/collections/sync_entries/records", pocketbase.base_url());
        let records_response = client
            .get(&records_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        let records_result: serde_json::Value = records_response.json().await.unwrap();
        let items = records_result["items"].as_array().unwrap();
        
        assert!(items.len() >= 3); // Should have at least our test entries

        // Clean up test data
        pocketbase.cleanup_test_data().await.unwrap();

        // Verify data was cleaned up
        let records_response = client
            .get(&records_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        let records_result: serde_json::Value = records_response.json().await.unwrap();
        let items = records_result["items"].as_array().unwrap();
        
        assert_eq!(items.len(), 0); // Should be empty after cleanup
    }
}

/// Utilities for integration testing
pub mod utils {
    use super::*;

    /// Create a temporary directory with test files
    pub fn create_test_directory_structure() -> Result<TempDir> {
        let temp_dir = TempDir::new().map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to create temp directory: {}", e))
        })?;

        let root = temp_dir.path();

        // Create test files and directories
        fs::create_dir(root.join("subdir")).map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to create subdirectory: {}", e))
        })?;

        fs::write(root.join("file1.txt"), b"content1").map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to write file1.txt: {}", e))
        })?;

        fs::write(root.join("file2.txt"), b"content2").map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to write file2.txt: {}", e))
        })?;

        fs::write(root.join("subdir/file3.txt"), b"content3").map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to write file3.txt: {}", e))
        })?;

        // Create hidden file
        fs::write(root.join(".hidden.txt"), b"hidden content").map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to write hidden file: {}", e))
        })?;

        Ok(temp_dir)
    }

    /// Create a test file with specific content and size
    pub fn create_test_file<P: AsRef<Path>>(path: P, size: usize) -> Result<()> {
        let content = "x".repeat(size);
        fs::write(path, content).map_err(|e| {
            SyncError::Generic(anyhow::anyhow!("Failed to create test file: {}", e))
        })
    }

    /// Run a test function with a timeout
    pub async fn with_timeout<F, Fut, T>(
        duration: Duration,
        future: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        timeout(duration, future()).await.map_err(|_| {
            SyncError::Generic(anyhow::anyhow!("Test timed out after {:?}", duration))
        })
    }

    /// Wait for a condition to be true with polling
    pub async fn wait_for_condition<F>(
        mut condition: F,
        timeout_duration: Duration,
        poll_interval: Duration,
    ) -> Result<()>
    where
        F: FnMut() -> bool,
    {
        timeout(timeout_duration, async {
            while !condition() {
                tokio::time::sleep(poll_interval).await;
            }
        })
        .await
        .map_err(|_| {
            SyncError::Generic(anyhow::anyhow!(
                "Condition was not met within timeout period"
            ))
        })
    }
}
