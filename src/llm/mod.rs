pub mod providers;
pub mod correction;
pub mod filename_parsing;

use anyhow::Result;
use async_trait::async_trait;
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
    pub provider: LLMProvider,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_seconds: u64,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::LMStudio,
            endpoint: Some("http://localhost:1234/v1/chat/completions".to_string()),
            api_key: None,
            model: "local-model".to_string(),
            max_tokens: 4096, // Will be overridden to model maximum
            temperature: 0.1,
            timeout_seconds: 60,
        }
    }
}

/// Chat message for LLM communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// LLM response
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: String,
    pub tokens_used: Option<u32>,
}

/// Trait for LLM providers
#[async_trait]
pub trait LLM: Send + Sync {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<LLMResponse>;
    async fn is_available(&self) -> bool;
    fn provider_type(&self) -> LLMProvider;
}

/// Create LLM instance based on configuration
pub fn create_llm(config: &LLMConfig) -> Result<Box<dyn LLM>> {
    match config.provider {
        LLMProvider::LMStudio => Ok(Box::new(providers::LMStudioProvider::new(config.clone())?)),
        LLMProvider::Gemini => Ok(Box::new(providers::GeminiProvider::new(config.clone())?)),
        LLMProvider::OpenAI => Ok(Box::new(providers::OpenAIProvider::new(config.clone())?)),
    }
}