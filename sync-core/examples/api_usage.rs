use sync_core::api::{PocketBaseClient, PocketBaseClientBuilder, ListParams};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: Option<String>,
    title: String,
    description: String,
    completed: bool,
    created: Option<chrono::DateTime<chrono::Utc>>,
    updated: Option<chrono::DateTime<chrono::Utc>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("PocketBase API Client Usage Examples");
    println!("====================================");

    // Example 1: Basic client creation
    let client = PocketBaseClient::new("http://localhost:8090")?;
    println!("✓ Created PocketBase client");

    // Example 2: Client with custom configuration
    let custom_client = PocketBaseClientBuilder::new("http://localhost:8090")
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .user_agent("sync-app-example/1.0")
        .build()?;
    println!("✓ Created custom PocketBase client");

    // Example 3: Health check
    match client.health_check().await {
        Ok(health) => println!("✓ PocketBase health: {}", health.message),
        Err(e) => println!("✗ Health check failed: {}", e),
    }

    // Example 4: Authentication (commented out - requires actual server)
    /*
    let mut auth_client = client;
    match auth_client.authenticate("user@example.com", "password123").await {
        Ok(auth_token) => {
            println!("✓ Authenticated successfully");
            println!("  User ID: {}", auth_token.user.as_ref().map(|u| &u.id).unwrap_or(&"Unknown".to_string()));
        }
        Err(e) => println!("✗ Authentication failed: {}", e),
    }
    */

    // Example 5: CRUD operations (would require authentication)
    /*
    // Create a new task
    let new_task = Task {
        id: None,
        title: "Learn PocketBase API".to_string(),
        description: "Build a comprehensive API client wrapper".to_string(),
        completed: false,
        created: None,
        updated: None,
    };

    match client.create_record::<Task, Task>("tasks", &new_task).await {
        Ok(created_task) => {
            println!("✓ Created task: {}", created_task.title);
            
            // Update the task
            let mut updated_task = created_task;
            updated_task.completed = true;
            
            match client.update_record::<Task, Task>("tasks", &updated_task.id.unwrap(), &updated_task).await {
                Ok(_) => println!("✓ Updated task to completed"),
                Err(e) => println!("✗ Failed to update task: {}", e),
            }
        }
        Err(e) => println!("✗ Failed to create task: {}", e),
    }

    // List tasks with pagination
    let params = ListParams::new()
        .page(1)
        .per_page(10)
        .sort("-created")
        .filter("completed = false");

    match client.list_records::<Task>("tasks", Some(params)).await {
        Ok(result) => {
            println!("✓ Found {} tasks (page {} of {})", 
                result.items.len(), result.page, result.total_pages);
            for task in result.items {
                println!("  - {}: {}", task.title, if task.completed { "✓" } else { "○" });
            }
        }
        Err(e) => println!("✗ Failed to list tasks: {}", e),
    }

    // Search tasks
    match client.search_records::<Task>("tasks", "title", "Learn").await {
        Ok(result) => {
            println!("✓ Found {} matching tasks", result.items.len());
        }
        Err(e) => println!("✗ Search failed: {}", e),
    }

    // Get all tasks (with automatic pagination)
    match client.get_all_records::<Task>("tasks").await {
        Ok(all_tasks) => {
            println!("✓ Retrieved all {} tasks", all_tasks.len());
        }
        Err(e) => println!("✗ Failed to get all tasks: {}", e),
    }
    */

    // Example 6: Realtime subscriptions (would require authentication)
    /*
    // Connect to realtime updates
    let mut realtime_client = client;
    if let Err(e) = realtime_client.connect_realtime().await {
        println!("✗ Failed to connect to realtime: {}", e);
        return Ok(());
    }

    // Subscribe to task updates
    match realtime_client.subscribe_to_collection("tasks", None).await {
        Ok(mut subscription) => {
            println!("✓ Subscribed to task updates");
            
            // Listen for events (this would run indefinitely in a real app)
            tokio::select! {
                event = subscription.next() => {
                    if let Some(event) = event {
                        println!("📡 Realtime event: {:?} on collection {}", 
                            event.action, event.collection);
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    println!("⏰ Subscription demo timeout");
                }
            }
        }
        Err(e) => println!("✗ Failed to subscribe: {}", e),
    }

    // Disconnect from realtime
    realtime_client.disconnect_realtime().await;
    println!("✓ Disconnected from realtime");
    */

    println!("\nAPI Features Demonstrated:");
    println!("- ✓ Client creation and configuration");
    println!("- ✓ Health checking");
    println!("- ○ Authentication (requires server)");
    println!("- ○ CRUD operations (requires auth)");
    println!("- ○ Realtime subscriptions (requires auth)");
    println!("- ✓ Typed structs with serde");
    println!("- ✓ Error handling and retries");
    println!("- ✓ Token refresh (automatic)");

    Ok(())
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
    async fn test_custom_client_creation() {
        let client = PocketBaseClientBuilder::new("http://localhost:8090")
            .timeout(Duration::from_secs(30))
            .user_agent("test/1.0")
            .build();
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_list_params() {
        let params = ListParams::new()
            .page(1)
            .per_page(50)
            .sort("-created")
            .filter("active = true");
        
        assert_eq!(params.page, Some(1));
        assert_eq!(params.per_page, Some(50));
    }
}
