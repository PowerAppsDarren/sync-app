use super::{auth::AuthManager, crud::CrudOperations, error::*, realtime::RealtimeClient, types::*};
use reqwest::ClientBuilder;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Main PocketBase API client
pub struct PocketBaseClient {
    /// Authentication manager for handling tokens and refresh
    pub auth: Arc<RwLock<AuthManager>>,
    
    /// CRUD operations for records and collections
    pub crud: CrudOperations,
    
    /// Realtime WebSocket client for subscriptions
    pub realtime: RealtimeClient,
    
    /// HTTP client for making requests
    http_client: reqwest::Client,
    
    /// Base URL of the PocketBase instance
    base_url: String,
}

impl PocketBaseClient {
    /// Create a new PocketBase client
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into();
        
        // Build HTTP client with reasonable defaults
        let http_client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("sync-app/0.1.0")
            .build()
            .map_err(|e| PocketBaseError::Network(e))?;
        
        let auth_manager = Arc::new(RwLock::new(AuthManager::new(http_client.clone(), base_url.clone())));
        let crud_operations = CrudOperations::new(
            http_client.clone(),
            base_url.clone(),
            auth_manager.clone(),
        );
        let realtime_client = RealtimeClient::new(base_url.clone());
        
        Ok(Self {
            auth: auth_manager,
            crud: crud_operations,
            realtime: realtime_client,
            http_client,
            base_url,
        })
    }
    
    /// Create a new client with custom HTTP client configuration
    pub fn with_client(base_url: impl Into<String>, http_client: reqwest::Client) -> Self {
        let base_url = base_url.into();
        
        let auth_manager = Arc::new(RwLock::new(AuthManager::new(http_client.clone(), base_url.clone())));
        let crud_operations = CrudOperations::new(
            http_client.clone(),
            base_url.clone(),
            auth_manager.clone(),
        );
        let realtime_client = RealtimeClient::new(base_url.clone());
        
        Self {
            auth: auth_manager,
            crud: crud_operations,
            realtime: realtime_client,
            http_client,
            base_url,
        }
    }
    
    /// Authenticate with username/email and password
    pub async fn authenticate(&mut self, identity: impl Into<String>, password: impl Into<String>) -> Result<AuthToken> {
        let auth_token = self.auth.write().await.authenticate(identity.into(), password.into()).await?;
        
        // Update realtime client with new token
        self.realtime.update_auth_token(Some(auth_token.token.clone())).await?;
        
        info!("Successfully authenticated user");
        Ok(auth_token)
    }
    
    /// Authenticate as admin
    pub async fn authenticate_admin(&mut self, email: impl Into<String>, password: impl Into<String>) -> Result<AuthToken> {
        let auth_token = self.auth.write().await.authenticate_admin(email.into(), password.into()).await?;
        
        // Update realtime client with new token
        self.realtime.update_auth_token(Some(auth_token.token.clone())).await?;
        
        info!("Successfully authenticated admin");
        Ok(auth_token)
    }
    
    /// Check if currently authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.auth.read().await.is_authenticated().await
    }
    
    /// Get current user information
    pub async fn current_user(&self) -> Option<User> {
        self.auth.read().await.get_user().await
    }
    
    /// Logout and clear authentication
    pub async fn logout(&mut self) {
        self.auth.write().await.logout().await;
        self.realtime.update_auth_token(None).await.ok();
        info!("Logged out successfully");
    }
    
    /// Connect to realtime updates
    pub async fn connect_realtime(&mut self) -> Result<()> {
        let token = self.auth.read().await.get_token().await;
        self.realtime.connect(token).await?;
        info!("Connected to realtime updates");
        Ok(())
    }
    
    /// Disconnect from realtime updates
    pub async fn disconnect_realtime(&mut self) {
        self.realtime.disconnect().await;
        info!("Disconnected from realtime updates");
    }
    
    /// Check health of the PocketBase instance
    pub async fn health_check(&self) -> Result<HealthResponse> {
        debug!("Performing health check");
        
        let response = self
            .http_client
            .get(&format!("{}/api/health", self.base_url))
            .send()
            .await?;
        
        if response.status().is_success() {
            let health: HealthResponse = response.json().await?;
            debug!("Health check successful: {}", health.message);
            Ok(health)
        } else {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            Err(PocketBaseError::Server {
                status,
                message: error_text,
            })
        }
    }
    
    /// Get server information
    pub async fn server_info(&self) -> Result<serde_json::Value> {
        debug!("Getting server information");
        
        let response = self
            .http_client
            .get(&format!("{}/api/collections", self.base_url))
            .send()
            .await?;
        
        if response.status().is_success() {
            let info: serde_json::Value = response.json().await?;
            Ok(info)
        } else {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            Err(PocketBaseError::Server {
                status,
                message: error_text,
            })
        }
    }
}

// Convenience methods for common operations
impl PocketBaseClient {
    /// Quick method to create a record
    pub async fn create_record<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        collection: &str,
        data: &T,
    ) -> Result<R> {
        self.crud.create(collection, data).await
    }
    
    /// Quick method to get a record by ID
    pub async fn get_record<T: serde::de::DeserializeOwned>(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<T> {
        self.crud.get(collection, id).await
    }
    
    /// Quick method to update a record
    pub async fn update_record<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        collection: &str,
        id: &str,
        data: &T,
    ) -> Result<R> {
        self.crud.update(collection, id, data).await
    }
    
    /// Quick method to delete a record
    pub async fn delete_record(&self, collection: &str, id: &str) -> Result<()> {
        self.crud.delete(collection, id).await
    }
    
    /// Quick method to list records
    pub async fn list_records<T: serde::de::DeserializeOwned>(
        &self,
        collection: &str,
        params: Option<ListParams>,
    ) -> Result<ListResult<T>> {
        self.crud.list(collection, params).await
    }
    
    /// Quick method to search records
    pub async fn search_records<T: serde::de::DeserializeOwned>(
        &self,
        collection: &str,
        field: &str,
        query: &str,
    ) -> Result<ListResult<T>> {
        self.crud.search(collection, field, query).await
    }
    
    /// Quick method to get all records
    pub async fn get_all_records<T: serde::de::DeserializeOwned>(
        &self,
        collection: &str,
    ) -> Result<Vec<T>> {
        self.crud.get_all(collection).await
    }
    
    /// Subscribe to realtime updates for a collection
    pub async fn subscribe_to_collection(
        &self,
        collection: &str,
        filter: Option<String>,
    ) -> Result<super::realtime::RealtimeSubscription> {
        self.realtime.subscribe(collection, filter).await
    }
    
    /// Unsubscribe from realtime updates
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<()> {
        self.realtime.unsubscribe(subscription_id).await
    }
}

// Builder pattern for client configuration
pub struct PocketBaseClientBuilder {
    base_url: String,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    user_agent: Option<String>,
    gzip: bool,
}

impl PocketBaseClientBuilder {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout: None,
            connect_timeout: None,
            user_agent: None,
            gzip: true,
        }
    }
    
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }
    
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
    
    pub fn gzip(mut self, enabled: bool) -> Self {
        self.gzip = enabled;
        self
    }
    
    pub fn build(self) -> Result<PocketBaseClient> {
        let mut client_builder = ClientBuilder::new();
        
        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }
        
        if let Some(connect_timeout) = self.connect_timeout {
            client_builder = client_builder.connect_timeout(connect_timeout);
        }
        
        if let Some(user_agent) = self.user_agent {
            client_builder = client_builder.user_agent(user_agent);
        }
        
        // Note: gzip is enabled by default in reqwest
        
        let http_client = client_builder
            .build()
            .map_err(|e| PocketBaseError::Network(e))?;
        
        Ok(PocketBaseClient::with_client(self.base_url, http_client))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_creation() {
        let client = PocketBaseClient::new("http://localhost:8090");
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_client_builder() {
        let client = PocketBaseClientBuilder::new("http://localhost:8090")
            .timeout(Duration::from_secs(60))
            .user_agent("test-client/1.0")
            .build();
        assert!(client.is_ok());
    }
}
