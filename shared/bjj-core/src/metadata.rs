//! Video metadata structures and utilities

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Video metadata containing essential video information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoMetadata {
    /// Original filename
    pub filename: String,
    
    /// Video resolution (width, height)
    pub resolution: (u32, u32),
    
    /// Frame rate in FPS
    pub frame_rate: f64,
    
    /// Duration in seconds
    pub duration_seconds: f64,
    
    /// When this metadata was created
    pub created_at: DateTime<Utc>,
    
    /// When this metadata was last updated
    pub updated_at: DateTime<Utc>,
    
    /// File size in bytes (if available)
    pub file_size: Option<u64>,
    
    /// MD5 hash of the video file (if calculated)
    pub file_hash: Option<String>,
}

impl VideoMetadata {
    /// Create new video metadata
    pub fn new(
        filename: String,
        width: u32,
        height: u32,
        frame_rate: f64,
        duration_seconds: f64,
    ) -> Self {
        let now = Utc::now();
        
        Self {
            filename,
            resolution: (width, height),
            frame_rate,
            duration_seconds,
            created_at: now,
            updated_at: now,
            file_size: None,
            file_hash: None,
        }
    }
    
    /// Update file size
    pub fn with_file_size(mut self, size: u64) -> Self {
        self.file_size = Some(size);
        self.updated_at = Utc::now();
        self
    }
    
    /// Update file hash
    pub fn with_file_hash(mut self, hash: String) -> Self {
        self.file_hash = Some(hash);
        self.updated_at = Utc::now();
        self
    }
    
    /// Get formatted duration string
    pub fn duration_formatted(&self) -> String {
        let hours = (self.duration_seconds / 3600.0) as u32;
        let minutes = ((self.duration_seconds % 3600.0) / 60.0) as u32;
        let seconds = (self.duration_seconds % 60.0) as u32;
        
        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }
    
    /// Get formatted file size string
    pub fn file_size_formatted(&self) -> String {
        match self.file_size {
            Some(size) => {
                if size >= 1_073_741_824 {
                    format!("{:.1} GB", size as f64 / 1_073_741_824.0)
                } else if size >= 1_048_576 {
                    format!("{:.1} MB", size as f64 / 1_048_576.0)
                } else if size >= 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else {
                    format!("{} B", size)
                }
            }
            None => "Unknown".to_string(),
        }
    }
}