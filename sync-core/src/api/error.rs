use thiserror::Error;

#[derive(Error, Debug)]
pub enum PocketBaseError {
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    #[error("Authorization failed: {0}")]
    Authorization(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("Token expired or invalid")]
    TokenExpired,
    
    #[error("Rate limited: {retry_after:?}")]
    RateLimit { retry_after: Option<u64> },
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Record not found")]
    NotFound,
    
    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl PocketBaseError {
    pub fn is_retryable(&self) -> bool {
        match self {
            PocketBaseError::Network(_) => true,
            PocketBaseError::Server { status, .. } if *status >= 500 => true,
            PocketBaseError::RateLimit { .. } => true,
            _ => false,
        }
    }
    
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            PocketBaseError::RateLimit { retry_after } => *retry_after,
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, PocketBaseError>;
