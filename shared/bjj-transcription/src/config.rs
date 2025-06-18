//! Transcription configuration

use serde::{Deserialize, Serialize};

/// Configuration for transcription operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Provider: "local", "remote", "openai"
    provider: String,
    
    /// Whisper model: "tiny", "base", "small", "medium", "large"
    model: String,
    
    /// Use GPU acceleration
    use_gpu: bool,
    
    /// Target language (None for auto-detect)
    language: Option<String>,
    
    /// Remote server endpoint (for remote provider)
    remote_endpoint: Option<String>,
    
    /// API timeout in seconds
    timeout_seconds: u64,
    
    /// Enable BJJ-specific prompting
    bjj_context: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            provider: "local".to_string(),
            model: "base".to_string(),
            use_gpu: true,
            language: Some("en".to_string()),
            remote_endpoint: None,
            timeout_seconds: 300, // 5 minutes
            bjj_context: true,
        }
    }
}

impl TranscriptionConfig {
    /// Create new transcription config
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get provider
    pub fn provider(&self) -> &str {
        &self.provider
    }
    
    /// Set provider
    pub fn with_provider(mut self, provider: String) -> Self {
        self.provider = provider;
        self
    }
    
    /// Get model
    pub fn model(&self) -> &str {
        &self.model
    }
    
    /// Set model
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
    
    /// Get GPU usage
    pub fn use_gpu(&self) -> bool {
        self.use_gpu
    }
    
    /// Set GPU usage
    pub fn with_gpu(mut self, use_gpu: bool) -> Self {
        self.use_gpu = use_gpu;
        self
    }
    
    /// Get language
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
    
    /// Set language
    pub fn with_language(mut self, language: Option<String>) -> Self {
        self.language = language;
        self
    }
    
    /// Get remote endpoint
    pub fn remote_endpoint(&self) -> Option<&str> {
        self.remote_endpoint.as_deref()
    }
    
    /// Set remote endpoint
    pub fn with_remote_endpoint(mut self, endpoint: Option<String>) -> Self {
        self.remote_endpoint = endpoint;
        self
    }
    
    /// Get timeout
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
    
    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
    
    /// Get BJJ context usage
    pub fn bjj_context(&self) -> bool {
        self.bjj_context
    }
    
    /// Set BJJ context usage
    pub fn with_bjj_context(mut self, bjj_context: bool) -> Self {
        self.bjj_context = bjj_context;
        self
    }
}