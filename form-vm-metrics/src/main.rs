use std::{sync::Arc, time::{Duration, Instant}, collections::HashMap};

use axum::{extract::State, routing::{get, post}, Json, Router};
use clap::Parser;
use form_vm_metrics::{
    system::{collect_system_metrics, SystemMetrics},
    events::MetricsPublisher,
};
use tokio::{sync::{Mutex, mpsc, oneshot}, time::interval};
use serde::{Serialize, Deserialize};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Optional instance ID to associate with metrics
    #[arg(long, short)]
    instance_id: Option<String>,
    
    /// Optional account ID to associate with metrics
    #[arg(long, short)]
    account_id: Option<String>,
    
    /// Optional message queue endpoint
    #[arg(long, default_value = "127.0.0.1")]
    queue_endpoint: String,
    
    /// Optional message queue port
    #[arg(long, default_value_t = 3003)]
    queue_port: u16,
    
    /// Optional path or URL to threshold configuration source
    #[arg(long)]
    threshold_config: Option<String>,
    
    /// Port to serve metrics API on
    #[arg(long, default_value_t = 8080)]
    port: u16,
}

// Track service start time for uptime reporting
static mut SERVICE_START_TIME: Option<Instant> = None;

// Track registered webhooks
static WEBHOOKS: Mutex<Vec<WebhookConfig>> = Mutex::const_new(Vec::new());

#[derive(Serialize, Deserialize, Clone, Debug)]
struct WebhookConfig {
    /// Unique ID for this webhook
    id: String,
    
    /// URL to call when events occur
    url: String,
    
    /// Types of events to receive (e.g., "metrics", "threshold_violation")
    event_types: Vec<String>,
    
    /// Optional secret for validating webhook calls
    secret: Option<String>,
    
    /// When this webhook was registered
    registered_at: i64,
}

#[derive(Serialize, Deserialize)]
struct WebhookRegistrationRequest {
    /// URL to call when events occur
    url: String,
    
    /// Types of events to receive (e.g., "metrics", "threshold_violation") 
    event_types: Vec<String>,
    
    /// Optional secret for validating webhook calls
    secret: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct WebhookRegistrationResponse {
    /// Unique ID for the registered webhook
    id: String,
    
    /// Status of the registration
    status: String,
    
    /// URL that was registered
    url: String,
    
    /// Types of events this webhook will receive
    event_types: Vec<String>,
    
    /// When this webhook was registered
    registered_at: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set service start time
    unsafe {
        SERVICE_START_TIME = Some(Instant::now());
    }
    
    let args = Args::parse();
    
    // Create initial system metrics
    let mut system_metrics = SystemMetrics::default();
    
    // Set instance and account IDs if provided
    if let Some(id) = &args.instance_id {
        system_metrics.instance_id = Some(id.clone());
    }
    
    if let Some(id) = &args.account_id {
        system_metrics.account_id = Some(id.clone());
    }
    
    // Create shared metrics state
    let metrics = Arc::new(Mutex::new(system_metrics));
    
    // Create the metrics publisher
    let mut metrics_publisher = MetricsPublisher::with_config(
        args.queue_endpoint,
        args.queue_port,
        "usage_events".to_string(),
        0,
    );
    
    // Add threshold detection if config source is provided
    if let Some(config_source) = args.threshold_config {
        println!("Initializing threshold detection with config source: {}", config_source);
        // Clone before calling to avoid move
        metrics_publisher = match metrics_publisher.clone().with_threshold_detection(config_source).await {
            Ok(publisher) => {
                println!("Threshold detection enabled");
                publisher
            },
            Err(e) => {
                eprintln!("Failed to initialize threshold detection: {}", e);
                metrics_publisher
            }
        };
    }
    
    // Channel for signaling collector to stop
    let (collector_sender, mut collector_receiver) = oneshot::channel();
    
    // Start the metrics collection loop
    let collector_metrics = metrics.clone();
    let metrics_collection_handle = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            tokio::select! {
                _ = &mut collector_receiver => { break }
                _ = async {
                    // Collect metrics
                    let updated_metrics = collect_system_metrics(collector_metrics.clone()).await;
                    
                    // Publish metrics to the message queue
                    let metrics_guard = updated_metrics.lock().await;
                    if let Err(e) = metrics_publisher.publish_metrics(&metrics_guard).await {
                        eprintln!("Failed to publish metrics: {}", e);
                    }
                    
                    // Publish to registered webhooks
                    if let Err(e) = publish_to_webhooks(&metrics_guard, "metrics").await {
                        eprintln!("Failed to publish to webhooks: {}", e);
                    }
                    
                    // Process any threshold violations
                    // (This would be implemented as part of the threshold manager)
                } => {}
            }
        }
    });
    
    // Create a channel for shutting down the server
    let (server_shutdown_tx, server_shutdown_rx) = mpsc::channel(1);
    
    // Start the metrics API server
    let server_metrics = metrics.clone();
    let server = serve(server_metrics, args.port, server_shutdown_rx);
    
    println!("Starting metrics service");
    println!("API available at http://localhost:{}/get", args.port);
    
    // Handle Ctrl+C to gracefully shut down
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c signal");
        println!("Shutting down...");
        
        // Signal collector to stop
        let _ = collector_sender.send(());
        
        // Signal server to stop
        let _ = server_shutdown_tx.send(()).await;
    });
    
    // Wait for the server to complete
    server.await?;
    
    // Wait for metrics collection to complete
    metrics_collection_handle.await?;
    
    println!("Shutdown complete");
    
    Ok(())
}

async fn serve(
    metrics: Arc<Mutex<SystemMetrics>>,
    port: u16,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        // Get current system metrics
        .route("/get", get(get_metrics))
        // Simple health check for liveness probes
        .route("/health", get(health_check))
        // Detailed health status for monitoring
        .route("/api/v1/health/status", get(health_status))
        // New webhook routes
        .route("/api/v1/webhooks", post(register_webhook))
        .route("/api/v1/webhooks", get(list_webhooks))
        .route("/api/v1/webhooks/:id", axum::routing::delete(delete_webhook))
        .with_state(metrics);
        
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
        })
        .await?;
        
    Ok(())
}

/// Get the current system metrics
///
/// Returns the latest collected system metrics including CPU, memory, disk, 
/// network, and GPU usage. This endpoint provides a point-in-time snapshot
/// and does not include historical data.
///
/// # Response Format
/// 
/// ```json
/// {
///   "timestamp": 1626350430,
///   "instance_id": "instance-abc123",
///   "account_id": "account-xyz789",
///   "cpu": { ... },
///   "memory": { ... },
///   "disks": [ ... ],
///   "network": { ... },
///   "gpus": [ ... ],
///   "load": { ... }
/// }
/// ```
async fn get_metrics(
    State(state): State<Arc<Mutex<SystemMetrics>>>
) -> Json<SystemMetrics> {
    Json(state.lock().await.clone())
}

/// Defines the structure of the health status response
#[derive(Serialize, Deserialize)]
struct HealthStatus {
    /// Overall status of the service: "ok", "degraded", or "error"
    status: String,
    /// Number of seconds the service has been running
    uptime_seconds: u64,
    /// Status of individual components
    components: HealthComponents,
    /// Service version
    version: String,
}

/// Status of individual service components
#[derive(Serialize, Deserialize)]
struct HealthComponents {
    /// Status of metrics collection
    metrics_collection: ComponentStatus,
    /// Status of event publishing
    event_publishing: ComponentStatus,
    /// Status of the API layer
    api: ComponentStatus,
}

/// Detailed status of a single component
#[derive(Serialize, Deserialize)]
struct ComponentStatus {
    /// Status: "ok", "degraded", or "error"
    status: String,
    /// Unix timestamp of last successful operation
    last_success: Option<i64>,
    /// Additional details about the component status
    details: Option<String>,
}

/// Basic health check endpoint
///
/// Returns a simple "healthy" string if the service is running.
/// This endpoint is intended for basic liveness probes in container
/// orchestration systems.
///
/// # Response Format
/// 
/// Simple text: "healthy"
async fn health_check() -> &'static str {
    "healthy"
}

/// Detailed health status endpoint
///
/// Returns comprehensive information about the service health, including
/// uptime, component status, and version information. This endpoint is
/// intended for monitoring systems and dashboards.
///
/// # Response Format
/// 
/// ```json
/// {
///   "status": "ok",
///   "uptime_seconds": 3600,
///   "components": {
///     "metrics_collection": {
///       "status": "ok",
///       "last_success": 1626350430,
///       "details": "Last metrics collection at timestamp 1626350430"
///     },
///     "event_publishing": {
///       "status": "ok",
///       "last_success": 1626350430,
///       "details": "Event publishing appears operational"
///     },
///     "api": {
///       "status": "ok",
///       "last_success": null,
///       "details": "API is responding to requests"
///     }
///   },
///   "version": "0.1.0"
/// }
/// ```
async fn health_status(
    State(state): State<Arc<Mutex<SystemMetrics>>>
) -> Json<HealthStatus> {
    // Get uptime
    let uptime_seconds = unsafe {
        SERVICE_START_TIME.map_or(0, |start_time| start_time.elapsed().as_secs())
    };
    
    // Check when metrics were last collected
    let metrics_last_updated = {
        let metrics = state.lock().await;
        metrics.timestamp
    };
    
    // Build health status response
    let health = HealthStatus {
        status: "ok".to_string(),
        uptime_seconds,
        components: HealthComponents {
            metrics_collection: ComponentStatus {
                status: "ok".to_string(),
                last_success: Some(metrics_last_updated),
                details: Some(format!("Last metrics collection at timestamp {}", metrics_last_updated)),
            },
            event_publishing: ComponentStatus {
                status: "ok".to_string(),
                last_success: Some(metrics_last_updated), // Using the same timestamp for now
                details: Some("Event publishing appears operational".to_string()),
            },
            api: ComponentStatus {
                status: "ok".to_string(),
                last_success: None,
                details: Some("API is responding to requests".to_string()),
            },
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    
    Json(health)
}

/// Register a new webhook
///
/// Registers a new webhook for receiving real-time event notifications.
/// Clients can specify which types of events they want to receive and
/// provide an optional secret for validating the webhook calls.
///
/// # Request Format
/// 
/// ```json
/// {
///   "url": "https://example.com/webhook",
///   "event_types": ["metrics", "threshold_violation"],
///   "secret": "optional_shared_secret"
/// }
/// ```
///
/// # Response Format
/// 
/// ```json
/// {
///   "id": "webhook_abc123",
///   "status": "registered",
///   "url": "https://example.com/webhook",
///   "event_types": ["metrics", "threshold_violation"],
///   "registered_at": 1626350430
/// }
/// ```
async fn register_webhook(
    Json(request): Json<WebhookRegistrationRequest>,
) -> Result<Json<WebhookRegistrationResponse>, axum::http::StatusCode> {
    // Validate URL
    if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }
    
    // Validate event types
    let valid_event_types = vec!["metrics", "threshold_violation"];
    for event_type in &request.event_types {
        if !valid_event_types.contains(&event_type.as_str()) {
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    }
    
    // Generate a unique ID
    let id = format!("webhook_{}", uuid::Uuid::new_v4().to_string().replace("-", "").chars().take(8).collect::<String>());
    
    // Create webhook config
    let webhook = WebhookConfig {
        id: id.clone(),
        url: request.url.clone(),
        event_types: request.event_types.clone(),
        secret: request.secret.clone(),
        registered_at: chrono::Utc::now().timestamp(),
    };
    
    // Store the webhook
    WEBHOOKS.lock().await.push(webhook.clone());
    
    // Return the registration response
    Ok(Json(WebhookRegistrationResponse {
        id,
        status: "registered".to_string(),
        url: request.url,
        event_types: request.event_types,
        registered_at: webhook.registered_at,
    }))
}

/// List registered webhooks
///
/// Returns a list of all registered webhooks. 
/// The secrets are not included in the response for security reasons.
async fn list_webhooks() -> Json<Vec<WebhookConfig>> {
    // Get webhooks without secrets
    let webhooks = WEBHOOKS.lock().await.clone();
    let public_webhooks = webhooks.into_iter().map(|mut webhook| {
        webhook.secret = None;
        webhook
    }).collect();
    
    Json(public_webhooks)
}

/// Delete a webhook by ID
///
/// Unregisters a webhook with the specified ID.
async fn delete_webhook(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    let mut webhooks = WEBHOOKS.lock().await;
    
    let initial_len = webhooks.len();
    webhooks.retain(|webhook| webhook.id != id);
    
    if webhooks.len() < initial_len {
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err(axum::http::StatusCode::NOT_FOUND)
    }
}

/// Publish events to registered webhooks
///
/// Sends the events to all registered webhooks that are interested
/// in the specified event type.
async fn publish_to_webhooks(metrics: &SystemMetrics, event_type: &str) -> Result<(), String> {
    let webhooks = WEBHOOKS.lock().await.clone();
    
    if webhooks.is_empty() {
        return Ok(());
    }
    
    let client = reqwest::Client::new();
    
    for webhook in webhooks {
        if webhook.event_types.iter().any(|t| t == event_type) {
            // Create the payload
            let payload = serde_json::json!({
                "event_type": event_type,
                "timestamp": chrono::Utc::now().timestamp(),
                "data": metrics
            });
            
            // Build the request
            let mut request = client.post(&webhook.url)
                .json(&payload)
                .header("Content-Type", "application/json")
                .header("User-Agent", "Form-VM-Metrics-Webhook")
                .header("X-Webhook-Event", event_type);
                
            // Add signature if a secret is provided
            if let Some(secret) = &webhook.secret {
                let payload_str = serde_json::to_string(&payload).unwrap_or_default();
                let signature = hmac_sha256(secret, &payload_str);
                request = request.header("X-Webhook-Signature", signature);
            }
            
            // Send the request (don't wait for response)
            tokio::spawn(async move {
                match request.send().await {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send webhook to {}: {}", webhook.url, e),
                }
            });
        }
    }
    
    Ok(())
}

/// Create an HMAC-SHA256 signature using the provided secret and payload
fn hmac_sha256(secret: &str, payload: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    
    hex::encode(code_bytes)
}
