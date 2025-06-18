//! BJJ Transcription - Audio extraction and transcription for BJJ videos

pub mod audio;
pub mod transcription;
pub mod config;

pub use audio::{AudioExtractor, AudioInfo};
pub use transcription::{WhisperTranscriber, TranscriptionResult, TranscriptionSegment};
pub use config::TranscriptionConfig;

/// Result type for transcription operations
pub type Result<T> = std::result::Result<T, TranscriptionError>;

/// Error types for transcription operations
#[derive(thiserror::Error, Debug)]
pub enum TranscriptionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Audio extraction failed: {0}")]
    AudioExtraction(String),
    
    #[error("Transcription failed: {0}")]
    Transcription(String),
    
    #[error("Remote server error: {0}")]
    RemoteServer(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),
    
    #[error("Whisper error: {0}")]
    Whisper(String),
}