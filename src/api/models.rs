//! API data models

use serde::{Deserialize, Serialize};

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Video information for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoApiInfo {
    pub filename: String,
    pub duration: Option<f64>,
    pub status: String,
    pub progress: f64,
}

/// Processing status for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingStatus {
    pub active_jobs: u32,
    pub completed_jobs: u32,
    pub failed_jobs: u32,
    pub queue_size: u32,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}