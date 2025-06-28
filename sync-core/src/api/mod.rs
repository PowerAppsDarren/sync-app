//! PocketBase API client module
//! 
//! This module provides a comprehensive wrapper around PocketBase's REST API
//! with support for authentication, CRUD operations, realtime subscriptions,
//! automatic retries, and token refresh.

pub mod auth;
pub mod client;
pub mod crud;
pub mod error;
pub mod realtime;
pub mod types;

// Re-export main types for convenience
pub use client::{PocketBaseClient, PocketBaseClientBuilder};
pub use error::{PocketBaseError, Result};
pub use types::*;

// Re-export commonly used modules
pub use auth::{AuthManager, AuthState};
pub use crud::CrudOperations;
pub use realtime::{RealtimeClient, RealtimeEvent, RealtimeSubscription};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_client_creation() {
        let client = PocketBaseClient::new("http://localhost:8090");
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_client_builder() {
        let client = PocketBaseClientBuilder::new("http://localhost:8090")
            .timeout(Duration::from_secs(30))
            .user_agent("test/1.0")
            .build();
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_list_params_builder() {
        let params = ListParams::new()
            .page(1)
            .per_page(50)
            .sort("-created")
            .filter("active = true");
        
        assert_eq!(params.page, Some(1));
        assert_eq!(params.per_page, Some(50));
        assert_eq!(params.sort, Some("-created".to_string()));
        assert_eq!(params.filter, Some("active = true".to_string()));
    }
    
    #[test]
    fn test_error_retryability() {
        // Test rate limit error
        let rate_limit_error = PocketBaseError::RateLimit { retry_after: Some(60) };
        assert!(rate_limit_error.is_retryable());
        
        let auth_error = PocketBaseError::Authentication("invalid credentials".to_string());
        assert!(!auth_error.is_retryable());
        
        let server_error = PocketBaseError::Server {
            status: 500,
            message: "internal server error".to_string(),
        };
        assert!(server_error.is_retryable());
        
        let client_error = PocketBaseError::Server {
            status: 400,
            message: "bad request".to_string(),
        };
        assert!(!client_error.is_retryable());
    }
}
