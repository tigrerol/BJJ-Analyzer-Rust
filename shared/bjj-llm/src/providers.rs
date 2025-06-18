//! LLM provider implementations

use crate::{LLMConfig, LLMProvider, Result, LLMError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Chat message for LLM communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    /// Create new chat message
    pub fn new(role: String, content: String) -> Self {
        Self { role, content }
    }
    
    /// Get role
    pub fn role(&self) -> &str {
        &self.role
    }
    
    /// Get content
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// LLM response
#[derive(Debug, Clone)]
pub struct LLMResponse {
    content: String,
    tokens_used: Option<u32>,
}

impl LLMResponse {
    /// Create new LLM response
    pub fn new(content: String, tokens_used: Option<u32>) -> Self {
        Self { content, tokens_used }
    }
    
    /// Get content
    pub fn content(&self) -> &str {
        &self.content
    }
    
    /// Get tokens used
    pub fn tokens_used(&self) -> Option<u32> {
        self.tokens_used
    }
}

/// Trait for LLM providers
#[async_trait]
pub trait LLM: Send + Sync {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<LLMResponse>;
    async fn is_available(&self) -> bool;
    fn provider_type(&self) -> LLMProvider;
}

/// Mock LLM provider for testing
pub struct MockLLMProvider {
    provider_type: LLMProvider,
    available: bool,
}

impl MockLLMProvider {
    pub fn new(provider_type: LLMProvider) -> Self {
        Self {
            provider_type,
            available: true,
        }
    }
}

#[async_trait]
impl LLM for MockLLMProvider {
    async fn chat(&self, _messages: Vec<ChatMessage>) -> Result<LLMResponse> {
        Ok(LLMResponse::new("Mock response".to_string(), Some(10)))
    }
    
    async fn is_available(&self) -> bool {
        self.available
    }
    
    fn provider_type(&self) -> LLMProvider {
        self.provider_type.clone()
    }
}

/// Create LLM instance based on configuration
pub fn create_llm(config: &LLMConfig) -> Result<Box<dyn LLM>> {
    // For testing purposes, return mock provider
    // Real implementation would create actual providers
    Ok(Box::new(MockLLMProvider::new(config.provider())))
}