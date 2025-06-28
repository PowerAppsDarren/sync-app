use super::{error::Result, types::*};
use base64::prelude::*;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct AuthState {
    pub token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub user: Option<User>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            token: None,
            expires_at: None,
            user: None,
        }
    }
    
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }
    
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => Utc::now() >= expires_at,
            None => false,
        }
    }
    
    pub fn needs_refresh(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => {
                // Refresh if token expires within 5 minutes
                let refresh_threshold = expires_at - Duration::minutes(5);
                Utc::now() >= refresh_threshold
            }
            None => false,
        }
    }
    
    pub fn set_token(&mut self, auth_token: AuthToken) {
        self.token = Some(auth_token.token.clone());
        self.user = auth_token.user;
        
        // Parse JWT to get expiration time
        if let Some(exp) = self.parse_token_expiration(&auth_token.token) {
            self.expires_at = Some(exp);
        }
    }
    
    pub fn clear(&mut self) {
        self.token = None;
        self.expires_at = None;
        self.user = None;
    }
    
    fn parse_token_expiration(&self, token: &str) -> Option<DateTime<Utc>> {
        // Simple JWT parsing - in production, use a proper JWT library
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        
        // Decode the payload (second part)
        let payload = parts[1];
        let decoded = base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(payload).ok()?;
        let payload_json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
        
        let exp = payload_json.get("exp")?.as_u64()?;
        DateTime::from_timestamp(exp as i64, 0)
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AuthManager {
    client: reqwest::Client,
    base_url: String,
    state: Arc<RwLock<AuthState>>,
    credentials: Option<(String, String)>, // For auto-refresh
}

impl AuthManager {
    pub fn new(client: reqwest::Client, base_url: String) -> Self {
        Self {
            client,
            base_url,
            state: Arc::new(RwLock::new(AuthState::new())),
            credentials: None,
        }
    }
    
    pub async fn authenticate(&mut self, identity: String, password: String) -> Result<AuthToken> {
        debug!("Authenticating user: {}", identity);
        
        let auth_request = AuthRequest {
            identity: identity.clone(),
            password: password.clone(),
        };
        
        let response = self
            .client
            .post(&format!("{}/api/collections/users/auth-with-password", self.base_url))
            .json(&auth_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(super::error::PocketBaseError::Authentication(error_text));
        }
        
        let auth_token: AuthToken = response.json().await?;
        
        // Store credentials for auto-refresh
        self.credentials = Some((identity, password));
        
        // Update auth state
        {
            let mut state = self.state.write().await;
            state.set_token(auth_token.clone());
        }
        
        debug!("Authentication successful");
        Ok(auth_token)
    }
    
    pub async fn authenticate_admin(&mut self, email: String, password: String) -> Result<AuthToken> {
        debug!("Authenticating admin: {}", email);
        
        let auth_request = json!({
            "identity": email,
            "password": password
        });
        
        let response = self
            .client
            .post(&format!("{}/api/admins/auth-with-password", self.base_url))
            .json(&auth_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(super::error::PocketBaseError::Authentication(error_text));
        }
        
        let auth_token: AuthToken = response.json().await?;
        
        // Store credentials for auto-refresh
        self.credentials = Some((email, password));
        
        // Update auth state
        {
            let mut state = self.state.write().await;
            state.set_token(auth_token.clone());
        }
        
        debug!("Admin authentication successful");
        Ok(auth_token)
    }
    
    pub async fn refresh_token(&self) -> Result<Option<AuthToken>> {
        let credentials = match &self.credentials {
            Some(creds) => creds.clone(),
            None => {
                warn!("No credentials available for token refresh");
                return Ok(None);
            }
        };
        
        debug!("Refreshing authentication token");
        
        // For PocketBase, we need to re-authenticate
        // In a real implementation, you might want to use refresh tokens if available
        let auth_request = AuthRequest {
            identity: credentials.0,
            password: credentials.1,
        };
        
        let response = self
            .client
            .post(&format!("{}/api/collections/users/auth-with-password", self.base_url))
            .json(&auth_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!("Token refresh failed: {}", error_text);
            return Err(super::error::PocketBaseError::Authentication(error_text));
        }
        
        let auth_token: AuthToken = response.json().await?;
        
        // Update auth state
        {
            let mut state = self.state.write().await;
            state.set_token(auth_token.clone());
        }
        
        debug!("Token refresh successful");
        Ok(Some(auth_token))
    }
    
    pub async fn get_token(&self) -> Option<String> {
        let state = self.state.read().await;
        if state.is_expired() {
            None
        } else {
            state.token.clone()
        }
    }
    
    pub async fn get_valid_token(&self) -> Result<Option<String>> {
        {
            let state = self.state.read().await;
            if state.is_authenticated() && !state.is_expired() && !state.needs_refresh() {
                return Ok(state.token.clone());
            }
        }
        
        // Try to refresh if needed and possible
        if let Some(_) = self.refresh_token().await? {
            let state = self.state.read().await;
            Ok(state.token.clone())
        } else {
            Ok(None)
        }
    }
    
    pub async fn is_authenticated(&self) -> bool {
        let state = self.state.read().await;
        state.is_authenticated() && !state.is_expired()
    }
    
    pub async fn get_user(&self) -> Option<User> {
        let state = self.state.read().await;
        state.user.clone()
    }
    
    pub async fn logout(&mut self) {
        debug!("Logging out");
        let mut state = self.state.write().await;
        state.clear();
        self.credentials = None;
    }
    
    pub fn auth_state(&self) -> Arc<RwLock<AuthState>> {
        self.state.clone()
    }
}
