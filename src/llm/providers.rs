use super::{ChatMessage, LLM, LLMConfig, LLMProvider, LLMResponse};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

/// LMStudio provider implementation
pub struct LMStudioProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct LMStudioRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct LMStudioResponse {
    choices: Vec<LMStudioChoice>,
    usage: Option<LMStudioUsage>,
}

#[derive(Debug, Deserialize)]
struct LMStudioChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct LMStudioUsage {
    total_tokens: u32,
}

impl LMStudioProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LLM for LMStudioProvider {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<LLMResponse> {
        let endpoint = self
            .config
            .endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("LMStudio endpoint not configured"))?;

        let request = LMStudioRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        debug!("Sending request to LMStudio at {}", endpoint);
        
        let response = self
            .client
            .post(endpoint)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("LMStudio API error {}: {}", status, text));
        }

        let llm_response: LMStudioResponse = response.json().await?;
        
        let content = llm_response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No response from LMStudio"))?
            .message
            .content
            .clone();

        let tokens_used = llm_response
            .usage
            .map(|u| u.total_tokens);

        Ok(LLMResponse {
            content,
            tokens_used,
        })
    }

    async fn is_available(&self) -> bool {
        let endpoint = match &self.config.endpoint {
            Some(ep) => ep,
            None => return false,
        };

        // Try to reach the health endpoint
        let health_endpoint = endpoint.replace("/v1/chat/completions", "/health");
        
        match self.client.get(&health_endpoint).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    fn provider_type(&self) -> LLMProvider {
        LLMProvider::LMStudio
    }
}

/// Gemini provider implementation
pub struct GeminiProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Deserialize)]
struct GeminiUsage {
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

impl GeminiProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(anyhow!("Gemini API key required"));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LLM for GeminiProvider {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<LLMResponse> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("Gemini API key not configured"))?;

        // Convert chat messages to Gemini format
        let content = messages
            .iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n");

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: content }],
            }],
            generation_config: GeminiGenerationConfig {
                max_output_tokens: self.config.max_tokens,
                temperature: self.config.temperature,
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model, api_key
        );

        debug!("Sending request to Gemini API");
        
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini API error {}: {}", status, text));
        }

        let gemini_response: GeminiResponse = response.json().await?;
        
        let content = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow!("No response from Gemini"))?;

        let tokens_used = gemini_response
            .usage_metadata
            .map(|u| u.total_token_count);

        Ok(LLMResponse {
            content,
            tokens_used,
        })
    }

    async fn is_available(&self) -> bool {
        // Simple check by trying to list models
        if let Some(api_key) = &self.config.api_key {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models?key={}",
                api_key
            );
            
            match self.client.get(&url).send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        } else {
            false
        }
    }

    fn provider_type(&self) -> LLMProvider {
        LLMProvider::Gemini
    }
}

/// OpenAI provider implementation
pub struct OpenAIProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    total_tokens: u32,
}

impl OpenAIProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        if config.api_key.is_none() {
            return Err(anyhow!("OpenAI API key required"));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LLM for OpenAIProvider {
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<LLMResponse> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        let url = "https://api.openai.com/v1/chat/completions";

        debug!("Sending request to OpenAI API");
        
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error {}: {}", status, text));
        }

        let openai_response: OpenAIResponse = response.json().await?;
        
        let content = openai_response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No response from OpenAI"))?
            .message
            .content
            .clone();

        let tokens_used = openai_response
            .usage
            .map(|u| u.total_tokens);

        Ok(LLMResponse {
            content,
            tokens_used,
        })
    }

    async fn is_available(&self) -> bool {
        if let Some(api_key) = &self.config.api_key {
            let url = "https://api.openai.com/v1/models";
            
            match self
                .client
                .get(url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
            {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        } else {
            false
        }
    }

    fn provider_type(&self) -> LLMProvider {
        LLMProvider::OpenAI
    }
}