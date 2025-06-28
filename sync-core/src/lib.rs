//! Core synchronization library
//!
//! This crate provides the core functionality for the sync application,
//! including data structures, traits, and utilities for synchronization.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration for the sync application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub pocketbase_url: String,
    pub admin_email: String,
    pub admin_password: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            pocketbase_url: "http://localhost:8090".to_string(),
            admin_email: "admin@example.com".to_string(),
            admin_password: "admin123456".to_string(),
        }
    }
}

/// Sync client for interacting with PocketBase
pub struct SyncClient {
    config: SyncConfig,
    client: reqwest::Client,
}

impl SyncClient {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/api/health", self.config.pocketbase_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.pocketbase_url, "http://localhost:8090");
    }
}
