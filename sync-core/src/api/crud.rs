use super::{auth::AuthManager, error::*, types::*};
use backoff::{future::retry, ExponentialBackoff};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, warn};

pub struct CrudOperations {
    client: reqwest::Client,
    base_url: String,
    auth_manager: std::sync::Arc<tokio::sync::RwLock<AuthManager>>,
}

impl CrudOperations {
    pub fn new(
        client: reqwest::Client,
        base_url: String,
        auth_manager: std::sync::Arc<tokio::sync::RwLock<AuthManager>>,
    ) -> Self {
        Self {
            client,
            base_url,
            auth_manager,
        }
    }

    /// List records from a collection
    pub async fn list<T: DeserializeOwned>(
        &self,
        collection: &str,
        params: Option<ListParams>,
    ) -> Result<ListResult<T>> {
        let operation = || async {
            let mut url = format!("{}/api/collections/{}/records", self.base_url, collection);
            
            if let Some(params) = &params {
                url.push_str("?");
                url.push_str(&serde_urlencoded::to_string(params).unwrap_or_default());
            }

            let mut request = self.client.get(&url);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.handle_response(response).await
        };

        self.retry_operation(operation).await
    }

    /// Get a single record by ID
    pub async fn get<T: DeserializeOwned>(&self, collection: &str, id: &str) -> Result<T> {
        let operation = || async {
            let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
            
            let mut request = self.client.get(&url);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.handle_response(response).await
        };

        self.retry_operation(operation).await
    }

    /// Create a new record
    pub async fn create<T: Serialize, R: DeserializeOwned>(
        &self,
        collection: &str,
        data: &T,
    ) -> Result<R> {
        let operation = || async {
            let url = format!("{}/api/collections/{}/records", self.base_url, collection);
            
            let mut request = self.client.post(&url).json(data);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.handle_response(response).await
        };

        self.retry_operation(operation).await
    }

    /// Update an existing record
    pub async fn update<T: Serialize, R: DeserializeOwned>(
        &self,
        collection: &str,
        id: &str,
        data: &T,
    ) -> Result<R> {
        let operation = || async {
            let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
            
            let mut request = self.client.patch(&url).json(data);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.handle_response(response).await
        };

        self.retry_operation(operation).await
    }

    /// Delete a record
    pub async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let operation = || async {
            let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
            
            let mut request = self.client.delete(&url);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            
            if response.status().is_success() {
                Ok(())
            } else {
                let error = self.parse_error_response(response).await?;
                Err(error)
            }
        };

        self.retry_operation(operation).await
    }

    /// Get collection information
    pub async fn get_collection(&self, name_or_id: &str) -> Result<Collection> {
        let operation = || async {
            let url = format!("{}/api/collections/{}", self.base_url, name_or_id);
            
            let mut request = self.client.get(&url);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.handle_response(response).await
        };

        self.retry_operation(operation).await
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<ListResult<Collection>> {
        let operation = || async {
            let url = format!("{}/api/collections", self.base_url);
            
            let mut request = self.client.get(&url);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.handle_response(response).await
        };

        self.retry_operation(operation).await
    }

    /// Perform a batch operation (transaction-like)
    pub async fn batch<T: Serialize, R: DeserializeOwned>(
        &self,
        requests: &[T],
    ) -> Result<Vec<R>> {
        let operation = || async {
            let url = format!("{}/api/batch", self.base_url);
            
            let mut request = self.client.post(&url).json(requests);

            if let Some(token) = self.auth_manager.read().await.get_valid_token().await? {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            self.handle_response(response).await
        };

        self.retry_operation(operation).await
    }

    // Helper methods

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();
        
        if status.is_success() {
            let content: T = response.json().await?;
            Ok(content)
        } else {
            let error = self.parse_error_response(response).await?;
            Err(error)
        }
    }

    async fn parse_error_response(&self, response: reqwest::Response) -> Result<PocketBaseError> {
        let status = response.status();
        
        match status {
            StatusCode::UNAUTHORIZED => {
                Ok(PocketBaseError::Authentication("Unauthorized".to_string()))
            }
            StatusCode::FORBIDDEN => {
                Ok(PocketBaseError::Authorization("Forbidden".to_string()))
            }
            StatusCode::NOT_FOUND => Ok(PocketBaseError::NotFound),
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                
                Ok(PocketBaseError::RateLimit { retry_after })
            }
            status if status.is_client_error() => {
                let error_text = response.text().await.unwrap_or_default();
                
                if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                    Ok(PocketBaseError::Validation(error_response.message))
                } else {
                    Ok(PocketBaseError::Server {
                        status: status.as_u16(),
                        message: error_text,
                    })
                }
            }
            status if status.is_server_error() => {
                let error_text = response.text().await.unwrap_or_default();
                Ok(PocketBaseError::Server {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
            _ => {
                let error_text = response.text().await.unwrap_or_default();
                Ok(PocketBaseError::Unknown(error_text))
            }
        }
    }

    async fn retry_operation<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let backoff = ExponentialBackoff {
            initial_interval: Duration::from_millis(100),
            max_interval: Duration::from_secs(10),
            max_elapsed_time: Some(Duration::from_secs(60)),
            ..Default::default()
        };

        retry(backoff, || async {
            match operation().await {
                Ok(result) => Ok(result),
                Err(error) => {
                    if error.is_retryable() {
                        warn!("Retryable error occurred: {}", error);
                        Err(backoff::Error::transient(error))
                    } else {
                        debug!("Non-retryable error: {}", error);
                        Err(backoff::Error::permanent(error))
                    }
                }
            }
        })
        .await
        .map_err(|_e| PocketBaseError::Unknown("Retry attempts exhausted".to_string()))
    }
}

// Convenience functions for common record operations
impl CrudOperations {
    /// List records with a simple filter
    pub async fn list_filtered<T: DeserializeOwned>(
        &self,
        collection: &str,
        filter: &str,
    ) -> Result<ListResult<T>> {
        let params = ListParams::new().filter(filter);
        self.list(collection, Some(params)).await
    }

    /// Get records with pagination
    pub async fn list_paginated<T: DeserializeOwned>(
        &self,
        collection: &str,
        page: u32,
        per_page: u32,
    ) -> Result<ListResult<T>> {
        let params = ListParams::new().page(page).per_page(per_page);
        self.list(collection, Some(params)).await
    }

    /// Search records by a text field
    pub async fn search<T: DeserializeOwned>(
        &self,
        collection: &str,
        field: &str,
        query: &str,
    ) -> Result<ListResult<T>> {
        let filter = format!("{} ~ '{}'", field, query.replace("'", "\\'"));
        self.list_filtered(collection, &filter).await
    }

    /// Get all records (with automatic pagination)
    pub async fn get_all<T: DeserializeOwned>(&self, collection: &str) -> Result<Vec<T>> {
        let mut all_items = Vec::new();
        let mut page = 1;
        const PER_PAGE: u32 = 100;

        loop {
            let result: ListResult<T> = self.list_paginated(collection, page, PER_PAGE).await?;
            
            all_items.extend(result.items);
            
            if page >= result.total_pages {
                break;
            }
            
            page += 1;
        }

        Ok(all_items)
    }
}
