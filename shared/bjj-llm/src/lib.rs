//! BJJ LLM - LLM integrations and content generation for BJJ videos

pub mod config;
pub mod providers;
pub mod filename_parsing;
pub mod correction;

pub use config::{LLMConfig, LLMProvider};
pub use providers::{LLM, ChatMessage, LLMResponse, create_llm};
pub use filename_parsing::{ParsedFilename, FilenameParser};
pub use correction::{CorrectionResponse, TextReplacement, TranscriptionCorrector};

/// Result type for LLM operations
pub type Result<T> = std::result::Result<T, LLMError>;

/// Error types for LLM operations
#[derive(thiserror::Error, Debug)]
pub enum LLMError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Provider not available: {0:?}")]
    ProviderUnavailable(LLMProvider),
    
    #[error("Parsing error: {0}")]
    Parsing(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("LLM response error: {0}")]
    ResponseError(String),
    
    #[error("Processing error: {0}")]
    Processing(String),
}