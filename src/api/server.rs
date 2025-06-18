//! HTTP server implementation for the API

use anyhow::Result;
use axum::{
    extract::{ws::{WebSocket, Message}, State, WebSocketUpgrade},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use tokio::time::{interval, Duration};

use crate::state::StateManager;
use crate::config::Config;
use super::{handlers, models::ApiResponse};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub state_manager: Arc<StateManager>,
    pub config: Arc<Config>,
}

/// Configure and start the HTTP server
pub async fn start_http_server(state_manager: Arc<StateManager>, config: Arc<Config>, port: u16) -> Result<()> {
    info!("🚀 Starting HTTP server on port {}", port);
    
    let app_state = AppState { state_manager, config };
    
    // Configure CORS to allow browser access
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);
    
    // Build the application with routes
    let app = Router::new()
        // Health check endpoints (both paths for compatibility)
        .route("/health", get(health_handler))
        .route("/api/health", get(health_handler))
        
        // Video management endpoints
        .route("/api/videos", get(list_videos_handler))
        .route("/api/videos/:id", get(video_status_handler))
        .route("/api/videos/:id/status", get(video_status_handler))
        .route("/api/videos/:id/process", post(process_video_handler))
        .route("/api/videos/process", post(process_multiple_videos_handler))
        
        // Series endpoints (placeholder for now)
        .route("/api/series", get(list_series_handler))
        .route("/api/series/:id", get(series_handler))
        
        // Corrections endpoints (placeholder for now)
        .route("/api/corrections", get(list_corrections_handler))
        .route("/api/corrections/series", post(submit_series_correction_handler))
        .route("/api/corrections/products", post(submit_product_correction_handler))
        
        // Processing status endpoints
        .route("/api/status", get(processing_status_handler))
        
        // WebSocket endpoints (multiple paths for compatibility)
        .route("/ws", get(websocket_handler))
        .route("/api/status/live", get(websocket_handler))
        
        // Serve static files for the UI (if present)
        .route("/", get(serve_ui))
        .route("/*path", get(serve_static))
        
        // Add state and middleware
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        );
    
    // Bind and serve
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🌐 API server listening on http://0.0.0.0:{}", port);
    info!("🔗 WebSocket endpoint available at ws://0.0.0.0:{}/ws", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Health check handler
async fn health_handler() -> impl IntoResponse {
    match handlers::health_check().await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// List videos handler
async fn list_videos_handler(State(state): State<AppState>) -> impl IntoResponse {
    match handlers::list_videos(&state.state_manager, &state.config).await {
        Ok(data) => {
            // Extract videos array directly for UI compatibility
            if let Some(videos) = data.get("videos") {
                (StatusCode::OK, Json(videos)).into_response()
            } else {
                (StatusCode::OK, Json(serde_json::json!([]))).into_response()
            }
        },
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Video status handler
async fn video_status_handler(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match handlers::get_video_status(&state.state_manager, &state.config, &id).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::NOT_FOUND;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Process video handler
async fn process_video_handler(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match handlers::start_video_processing(&state.state_manager, &id).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::BAD_REQUEST;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Processing status handler
async fn processing_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    match handlers::get_processing_status(&state.state_manager).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Process multiple videos handler
async fn process_multiple_videos_handler(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>
) -> impl IntoResponse {
    match handlers::start_multiple_video_processing(&state.state_manager, &payload).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::BAD_REQUEST;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// List series handler
async fn list_series_handler(State(state): State<AppState>) -> impl IntoResponse {
    match handlers::list_series(&state.state_manager, &state.config).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Series handler
async fn series_handler(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>
) -> impl IntoResponse {
    match handlers::get_series(&state.state_manager, &state.config, &id).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::NOT_FOUND;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// List corrections handler
async fn list_corrections_handler() -> impl IntoResponse {
    match handlers::list_corrections().await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Submit series correction handler
async fn submit_series_correction_handler(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    match handlers::submit_series_correction(&payload).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::BAD_REQUEST;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// Submit product correction handler
async fn submit_product_correction_handler(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    match handlers::submit_product_correction(&payload).await {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            let status = StatusCode::BAD_REQUEST;
            (status, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// WebSocket handler for real-time updates
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket_connection(socket, state))
}

/// Handle WebSocket connections
async fn websocket_connection(mut socket: WebSocket, state: AppState) {
    info!("🔌 New WebSocket connection established");
    
    // Send initial status message
    if let Ok(status_data) = handlers::get_processing_status(&state.state_manager).await {
        let initial_message = serde_json::json!({
            "type": "SystemStatus",
            "status": status_data
        });
        
        if let Ok(msg_text) = serde_json::to_string(&initial_message) {
            if socket.send(Message::Text(msg_text)).await.is_err() {
                warn!("Failed to send initial status message");
                return;
            }
        }
    }
    
    // Set up periodic status updates
    let mut interval = interval(Duration::from_secs(5));
    
    loop {
        tokio::select! {
            // Handle incoming messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Handle incoming text messages (e.g., client requests)
                        if text == "ping" {
                            if socket.send(Message::Text("pong".to_string())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("🔌 WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        break;
                    }
                    _ => {}
                }
            }
            
            // Send periodic status updates
            _ = interval.tick() => {
                match handlers::get_processing_status(&state.state_manager).await {
                    Ok(status_data) => {
                        let status_message = serde_json::json!({
                            "type": "SystemStatus",
                            "status": status_data
                        });
                        
                        if let Ok(msg_text) = serde_json::to_string(&status_message) {
                            if socket.send(Message::Text(msg_text)).await.is_err() {
                                info!("🔌 WebSocket connection closed during status update");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get processing status for WebSocket: {}", e);
                    }
                }
            }
        }
    }
    
    info!("🔌 WebSocket connection ended");
}

/// Serve the main UI page
async fn serve_ui() -> impl IntoResponse {
    // Check if UI directory exists and serve index.html
    if std::path::Path::new("ui/index.html").exists() {
        match tokio::fs::read("ui/index.html").await {
            Ok(content) => (
                StatusCode::OK,
                [("content-type", "text/html")],
                content
            ).into_response(),
            Err(_) => not_found_response().into_response(),
        }
    } else {
        // Fallback to simple API info page
        let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>BJJ Video Analyzer API</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .endpoint { background: #f5f5f5; padding: 10px; margin: 10px 0; }
        code { background: #e8e8e8; padding: 2px 4px; }
    </style>
</head>
<body>
    <h1>BJJ Video Analyzer API</h1>
    <p>The API server is running. Available endpoints:</p>
    
    <div class="endpoint">
        <strong>GET /health</strong> - Health check
    </div>
    <div class="endpoint">
        <strong>GET /api/videos</strong> - List all videos
    </div>
    <div class="endpoint">
        <strong>GET /api/videos/:id/status</strong> - Get video processing status
    </div>
    <div class="endpoint">
        <strong>POST /api/videos/:id/process</strong> - Start video processing
    </div>
    <div class="endpoint">
        <strong>GET /api/status</strong> - Get overall processing status
    </div>
    <div class="endpoint">
        <strong>WebSocket /ws</strong> - Real-time updates
    </div>
    
    <p>Place the BJJ-Analyzer-UI files in the ui directory to serve the web interface.</p>
</body>
</html>
        "#;
        (
            StatusCode::OK,
            [("content-type", "text/html")],
            html.as_bytes().to_vec()
        ).into_response()
    }
}

/// Serve static files from UI directory
async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> impl IntoResponse {
    let file_path = format!("ui/{}", path);
    
    match tokio::fs::read(&file_path).await {
        Ok(content) => {
            let content_type = match path.split('.').last() {
                Some("html") => "text/html",
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                Some("json") => "application/json",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("svg") => "image/svg+xml",
                _ => "application/octet-stream",
            };
            
            (
                StatusCode::OK,
                [("content-type", content_type)],
                content
            ).into_response()
        }
        Err(_) => not_found_response().into_response(),
    }
}

/// 404 response for files
fn not_found_response() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        [("content-type", "text/plain")],
        "Not Found".as_bytes().to_vec()
    )
}