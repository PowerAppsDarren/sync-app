# PocketBase API Client

A comprehensive Rust client library for PocketBase that wraps `reqwest` calls with authentication, CRUD operations, realtime subscriptions, automatic retries, and token refresh functionality.

## Features

- 🔐 **Authentication Management**: Automatic token refresh and credential management
- 📊 **CRUD Operations**: Full create, read, update, delete operations with type safety
- 🔄 **Automatic Retries**: Exponential backoff for retryable errors
- 🌐 **Realtime Subscriptions**: WebSocket-based real-time updates
- 🎯 **Type Safety**: Fully typed with `serde` support
- 🚀 **Async/Await**: Modern async Rust patterns
- 🛡️ **Error Handling**: Comprehensive error types and handling
- 📚 **Builder Pattern**: Flexible client configuration

## Quick Start

```rust
use sync_core::api::{PocketBaseClient, ListParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Todo {
    id: Option<String>,
    title: String,
    completed: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let mut client = PocketBaseClient::new("http://localhost:8090")?;
    
    // Authenticate
    let auth_token = client.authenticate("user@example.com", "password").await?;
    println!("Authenticated as: {:?}", auth_token.user);
    
    // Create a record
    let todo = Todo {
        id: None,
        title: "Learn PocketBase API".to_string(),
        completed: false,
    };
    
    let created_todo: Todo = client.create_record("todos", &todo).await?;
    println!("Created: {:?}", created_todo);
    
    // List records with pagination and filtering
    let params = ListParams::new()
        .page(1)
        .per_page(10)
        .sort("-created")
        .filter("completed = false");
    
    let todos = client.list_records::<Todo>("todos", Some(params)).await?;
    println!("Found {} todos", todos.items.len());
    
    Ok(())
}
```

## Client Configuration

The client can be configured using the builder pattern:

```rust
use std::time::Duration;
use sync_core::api::PocketBaseClientBuilder;

let client = PocketBaseClientBuilder::new("http://localhost:8090")
    .timeout(Duration::from_secs(30))
    .connect_timeout(Duration::from_secs(10))
    .user_agent("my-app/1.0")
    .build()?;
```

## Authentication

The client supports both user and admin authentication:

```rust
// User authentication
let auth_token = client.authenticate("user@example.com", "password").await?;

// Admin authentication
let admin_token = client.authenticate_admin("admin@example.com", "admin_password").await?;

// Check authentication status
if client.is_authenticated().await {
    println!("Currently authenticated");
}

// Get current user
if let Some(user) = client.current_user().await {
    println!("Current user: {}", user.email);
}

// Logout
client.logout().await;
```

### Token Refresh

The client automatically handles token refresh when tokens are about to expire (within 5 minutes of expiration). This is transparent to the user.

## CRUD Operations

### Basic Operations

```rust
// Create
let new_record: MyRecord = client.create_record("collection", &data).await?;

// Read
let record: MyRecord = client.get_record("collection", "record_id").await?;

// Update  
let updated: MyRecord = client.update_record("collection", "record_id", &data).await?;

// Delete
client.delete_record("collection", "record_id").await?;
```

### List Operations

```rust
// Simple list
let records: ListResult<MyRecord> = client.list_records("collection", None).await?;

// With parameters
let params = ListParams::new()
    .page(1)
    .per_page(20)
    .sort("-created")
    .filter("active = true")
    .expand("user")
    .fields("id,title,created");

let records = client.list_records("collection", Some(params)).await?;

// Search
let results = client.search_records("collection", "title", "search query").await?;

// Get all records (handles pagination automatically)
let all_records: Vec<MyRecord> = client.get_all_records("collection").await?;
```

### Collection Management

```rust
// Get collection info
let collection = client.crud.get_collection("my_collection").await?;
println!("Collection type: {:?}", collection.r#type);

// List all collections
let collections = client.crud.list_collections().await?;
```

## Realtime Subscriptions

The client supports WebSocket-based realtime subscriptions:

```rust
// Connect to realtime
client.connect_realtime().await?;

// Subscribe to a collection
let mut subscription = client.subscribe_to_collection("todos", None).await?;

// Listen for events
while let Some(event) = subscription.next().await {
    match event.action {
        RealtimeAction::Create => println!("New record created: {:?}", event.record),
        RealtimeAction::Update => println!("Record updated: {:?}", event.record),
        RealtimeAction::Delete => println!("Record deleted"),
    }
}

// Subscribe with filter
let subscription = client.subscribe_to_collection(
    "todos", 
    Some("completed = false".to_string())
).await?;

// Unsubscribe
client.unsubscribe(&subscription.id).await?;

// Disconnect
client.disconnect_realtime().await;
```

## Error Handling

The client provides comprehensive error handling:

```rust
use sync_core::api::PocketBaseError;

match client.get_record::<MyRecord>("collection", "id").await {
    Ok(record) => println!("Got record: {:?}", record),
    Err(PocketBaseError::NotFound) => println!("Record not found"),
    Err(PocketBaseError::Authentication(msg)) => println!("Auth error: {}", msg),
    Err(PocketBaseError::Network(err)) => println!("Network error: {}", err),
    Err(PocketBaseError::RateLimit { retry_after }) => {
        println!("Rate limited, retry after: {:?} seconds", retry_after);
    }
    Err(e) => println!("Other error: {}", e),
}
```

### Retryable Errors

The client automatically retries certain types of errors:
- Network errors
- Server errors (5xx status codes)
- Rate limit errors

Retries use exponential backoff with configurable limits.

## Type Definitions

The client provides comprehensive type definitions for PocketBase responses:

```rust
use sync_core::api::types::*;

// Records
struct MyRecord {
    id: String,
    created: DateTime<Utc>,
    updated: DateTime<Utc>,
    // ... custom fields
}

// List results
struct ListResult<T> {
    page: u32,
    per_page: u32,
    total_items: u32,
    total_pages: u32,
    items: Vec<T>,
}

// Authentication
struct AuthToken {
    token: String,
    user: Option<User>,
    metadata: Option<Value>,
}

// Collections
struct Collection {
    id: String,
    name: String,
    r#type: CollectionType,
    schema: Vec<SchemaField>,
    // ...
}
```

## Advanced Usage

### Custom HTTP Client

You can provide your own configured `reqwest::Client`:

```rust
let http_client = reqwest::ClientBuilder::new()
    .timeout(Duration::from_secs(60))
    .build()?;

let client = PocketBaseClient::with_client("http://localhost:8090", http_client);
```

### Batch Operations

```rust
// Batch multiple operations (if supported by PocketBase)
let requests = vec![/* ... */];
let results = client.crud.batch(&requests).await?;
```

### Health Monitoring

```rust
// Check server health
let health = client.health_check().await?;
println!("Server status: {}", health.message);

// Get server info
let info = client.server_info().await?;
```

## Integration with CLI, Daemon, and TUI

This API client is designed to be used across different application components:

### CLI Usage

```rust
// In CLI commands
use sync_core::api::PocketBaseClient;

async fn sync_command() -> anyhow::Result<()> {
    let client = PocketBaseClient::new(&config.pocketbase_url)?;
    client.authenticate(&config.username, &config.password).await?;
    
    let records = client.get_all_records("sync_items").await?;
    // Process records...
    
    Ok(())
}
```

### Daemon Usage

```rust
// In background daemon
async fn sync_daemon() {
    let mut client = PocketBaseClient::new(&config.pocketbase_url).unwrap();
    client.authenticate(&config.username, &config.password).await.unwrap();
    client.connect_realtime().await.unwrap();
    
    let mut subscription = client.subscribe_to_collection("sync_items", None).await.unwrap();
    
    while let Some(event) = subscription.next().await {
        // Handle realtime updates
        process_sync_event(event).await;
    }
}
```

### TUI Usage

```rust
// In TUI application
struct App {
    client: PocketBaseClient,
    items: Vec<SyncItem>,
}

impl App {
    async fn refresh_items(&mut self) -> Result<()> {
        self.items = self.client.get_all_records("items").await?;
        Ok(())
    }
    
    async fn create_item(&mut self, item: &SyncItem) -> Result<()> {
        let created = self.client.create_record("items", item).await?;
        self.items.push(created);
        Ok(())
    }
}
```

## Testing

The client includes comprehensive tests:

```bash
# Run tests
cargo test -p sync-core

# Run example
cargo run --example api_usage
```

## Dependencies

The client uses the following main dependencies:
- `reqwest` - HTTP client
- `tokio-tungstenite` - WebSocket client
- `serde` - Serialization
- `backoff` - Retry logic
- `chrono` - Date/time handling
- `uuid` - UUID generation
- `tracing` - Logging

## License

This project is licensed under the AGPL-3.0 license.
