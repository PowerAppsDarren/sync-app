use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    #[serde(rename = "record")]
    pub user: Option<User>,
    #[serde(rename = "meta")]
    pub metadata: Option<serde_json::Value>,
}

/// User record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: Option<String>,
    pub email: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub verified: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Generic record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

/// Collection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub r#type: CollectionType,
    pub schema: Vec<SchemaField>,
    pub system: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionType {
    Base,
    Auth,
    View,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub id: String,
    pub name: String,
    pub r#type: FieldType,
    pub system: bool,
    pub required: bool,
    pub unique: bool,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Bool,
    Email,
    Url,
    Date,
    Select,
    Json,
    File,
    Relation,
    User,
}

/// List response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult<T> {
    pub page: u32,
    #[serde(rename = "perPage")]
    pub per_page: u32,
    #[serde(rename = "totalItems")]
    pub total_items: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
    pub items: Vec<T>,
}

/// Query parameters for list requests
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListParams {
    pub page: Option<u32>,
    #[serde(rename = "perPage")]
    pub per_page: Option<u32>,
    pub sort: Option<String>,
    pub filter: Option<String>,
    pub expand: Option<String>,
    pub fields: Option<String>,
}

impl ListParams {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }
    
    pub fn sort(mut self, sort: impl Into<String>) -> Self {
        self.sort = Some(sort.into());
        self
    }
    
    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }
    
    pub fn expand(mut self, expand: impl Into<String>) -> Self {
        self.expand = Some(expand.into());
        self
    }
    
    pub fn fields(mut self, fields: impl Into<String>) -> Self {
        self.fields = Some(fields.into());
        self
    }
}

/// Authentication request
#[derive(Debug, Serialize)]
pub struct AuthRequest {
    pub identity: String,
    pub password: String,
}

/// Realtime subscription message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMessage {
    pub id: String,
    pub action: RealtimeAction,
    pub record: Option<Record>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RealtimeAction {
    Create,
    Update,
    Delete,
}

/// Subscription request for realtime
#[derive(Debug, Serialize)]
pub struct SubscribeRequest {
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub subscriptions: Vec<String>,
}

/// Error response from PocketBase
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
    pub data: Option<HashMap<String, serde_json::Value>>,
}

/// Health check response
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub code: u16,
    pub message: String,
}
