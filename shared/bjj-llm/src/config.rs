//! LLM configuration and provider types

use serde::{Deserialize, Serialize};

/// LLM provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LLMProvider {
    LMStudio,
    Gemini,
    OpenAI,
}

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    provider: LLMProvider,
    endpoint: Option<String>,
    api_key: Option<String>,
    model: String,
    max_tokens: u32,
    temperature: f32,
    timeout_seconds: u64,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::LMStudio,
            endpoint: Some("http://localhost:1234/v1/chat/completions".to_string()),
            api_key: None,
            model: "local-model".to_string(),
            max_tokens: 4096,
            temperature: 0.1,
            timeout_seconds: 60,
        }
    }
}

impl LLMConfig {
    /// Create new LLM config
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get provider
    pub fn provider(&self) -> LLMProvider {
        self.provider.clone()
    }
    
    /// Set provider
    pub fn with_provider(mut self, provider: LLMProvider) -> Self {
        self.provider = provider;
        self
    }
    
    /// Get endpoint
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
    
    /// Set endpoint
    pub fn with_endpoint(mut self, endpoint: Option<String>) -> Self {
        self.endpoint = endpoint;
        self
    }
    
    /// Get API key
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
    
    /// Set API key
    pub fn with_api_key(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key;
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
    
    /// Get max tokens
    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }
    
    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
    
    /// Get temperature
    pub fn temperature(&self) -> f32 {
        self.temperature
    }
    
    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
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
}