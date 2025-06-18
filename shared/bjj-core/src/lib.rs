//! BJJ Core - Shared data structures and utilities for video processing

pub mod metadata;
pub mod artifacts;
pub mod video_file;

pub use metadata::VideoMetadata;
pub use artifacts::{ArtifactDetector, ProcessingStage};
pub use video_file::VideoFile;

/// Result type for BJJ Core operations
pub type Result<T> = std::result::Result<T, BJJCoreError>;

/// Error types for BJJ Core operations
#[derive(thiserror::Error, Debug)]
pub enum BJJCoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Path error: {0}")]
    Path(String),
    
    #[error("Invalid video file: {0}")]
    InvalidVideo(String),
    
    #[error("Artifact not found: {0}")]
    ArtifactNotFound(String),
}