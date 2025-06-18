//! API module for the BJJ Video Analyzer
//! 
//! Provides REST API endpoints for the web UI and external integrations.

use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::state::StateManager;
use crate::config::Config;

pub mod handlers;
pub mod models;
pub mod server;

/// API Server for handling REST requests and WebSocket connections
#[derive(Debug)]
pub struct ApiServer {
    state_manager: Arc<StateManager>,
    config: Arc<Config>,
    port: u16,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(state_manager: Arc<StateManager>, config: Arc<Config>, port: u16) -> Self {
        Self {
            state_manager,
            config,
            port,
        }
    }
    
    /// Start the API server in the background
    pub fn start_background(self) -> JoinHandle<Result<()>> {
        tokio::spawn(async move {
            self.start().await
        })
    }
    
    /// Start the API server
    async fn start(self) -> Result<()> {
        info!("🚀 Starting API server on port {}", self.port);
        
        // Start the actual HTTP server
        server::start_http_server(self.state_manager, self.config, self.port).await
    }
}