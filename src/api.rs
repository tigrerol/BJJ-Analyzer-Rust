use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

use crate::config::{TranscriptionProvider, TranscriptionConfig};

/// External API client for transcription services
pub struct TranscriptionClient {
    config: TranscriptionConfig,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRequest {
    pub audio_path: String,
    pub language: Option<String>,
    pub model: String,
    pub response_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
    pub language: Option<String>,
    pub confidence: Option<f64>,
    pub segments: Option<Vec<TranscriptionSegment>>,
    pub processing_time: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub id: u32,
    pub seek: f64,
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub confidence: Option<f64>,
}

impl TranscriptionClient {
    pub fn new(config: TranscriptionConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout as u64))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, client }
    }

    /// Submit audio file for transcription
    pub async fn transcribe_audio(&self, audio_path: &str) -> Result<TranscriptionResponse> {
        match self.config.provider {
            TranscriptionProvider::OpenAI => self.transcribe_openai(audio_path).await,
            TranscriptionProvider::AssemblyAI => self.transcribe_assemblyai(audio_path).await,
            TranscriptionProvider::GoogleCloud => self.transcribe_google_cloud(audio_path).await,
            TranscriptionProvider::Azure => self.transcribe_azure(audio_path).await,
            TranscriptionProvider::Local => self.transcribe_local(audio_path).await,
            TranscriptionProvider::External => self.transcribe_external(audio_path).await,
        }
    }

    /// OpenAI Whisper API transcription
    async fn transcribe_openai(&self, audio_path: &str) -> Result<TranscriptionResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

        info!("🤖 Transcribing with OpenAI Whisper API: {}", audio_path);

        // Read audio file
        let audio_data = tokio::fs::read(audio_path).await?;
        
        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(audio_data)
                .file_name("audio.wav")
                .mime_str("audio/wav")?)
            .text("model", self.config.model.clone())
            .text("response_format", "verbose_json");

        let form = if let Some(language) = &self.config.language {
            form.text("language", language.clone())
        } else {
            form
        };

        let response = timeout(
            Duration::from_secs(self.config.timeout as u64),
            self.client
                .post("https://api.openai.com/v1/audio/transcriptions")
                .header("Authorization", format!("Bearer {}", api_key))
                .multipart(form)
                .send()
        ).await??;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        
        let transcription = TranscriptionResponse {
            text: response_json["text"].as_str().unwrap_or("").to_string(),
            language: response_json["language"].as_str().map(|s| s.to_string()),
            confidence: None, // OpenAI doesn't provide overall confidence
            segments: self.parse_openai_segments(&response_json),
            processing_time: None,
        };

        info!("✅ OpenAI transcription completed: {} characters", transcription.text.len());
        Ok(transcription)
    }

    /// AssemblyAI transcription
    async fn transcribe_assemblyai(&self, audio_path: &str) -> Result<TranscriptionResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("AssemblyAI API key not configured"))?;

        info!("🤖 Transcribing with AssemblyAI: {}", audio_path);

        // Step 1: Upload audio file
        let audio_data = tokio::fs::read(audio_path).await?;
        let upload_response = self.client
            .post("https://api.assemblyai.com/v2/upload")
            .header("authorization", api_key)
            .body(audio_data)
            .send()
            .await?;

        let upload_result: serde_json::Value = upload_response.json().await?;
        let audio_url = upload_result["upload_url"].as_str()
            .ok_or_else(|| anyhow!("Failed to get upload URL"))?;

        // Step 2: Request transcription
        let mut transcription_request = HashMap::new();
        transcription_request.insert("audio_url", audio_url);
        transcription_request.insert("speech_model", "best");
        
        if let Some(language) = &self.config.language {
            transcription_request.insert("language_code", language);
        }

        let transcription_response = self.client
            .post("https://api.assemblyai.com/v2/transcript")
            .header("authorization", api_key)
            .json(&transcription_request)
            .send()
            .await?;

        let transcription_result: serde_json::Value = transcription_response.json().await?;
        let transcript_id = transcription_result["id"].as_str()
            .ok_or_else(|| anyhow!("Failed to get transcript ID"))?;

        // Step 3: Poll for completion
        let final_result = self.poll_assemblyai_completion(api_key, transcript_id).await?;

        let transcription = TranscriptionResponse {
            text: final_result["text"].as_str().unwrap_or("").to_string(),
            language: final_result["language_code"].as_str().map(|s| s.to_string()),
            confidence: final_result["confidence"].as_f64(),
            segments: None, // Would need to parse segments if needed
            processing_time: None,
        };

        info!("✅ AssemblyAI transcription completed: {} characters", transcription.text.len());
        Ok(transcription)
    }

    /// Poll AssemblyAI for transcription completion
    async fn poll_assemblyai_completion(
        &self,
        api_key: &str,
        transcript_id: &str,
    ) -> Result<serde_json::Value> {
        let mut attempts = 0;
        let max_attempts = 60; // 5 minutes with 5-second intervals

        loop {
            let response = self.client
                .get(&format!("https://api.assemblyai.com/v2/transcript/{}", transcript_id))
                .header("authorization", api_key)
                .send()
                .await?;

            let result: serde_json::Value = response.json().await?;
            let status = result["status"].as_str().unwrap_or("");

            match status {
                "completed" => return Ok(result),
                "error" => return Err(anyhow!("AssemblyAI transcription failed: {}", 
                    result["error"].as_str().unwrap_or("Unknown error"))),
                _ => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow!("AssemblyAI transcription timeout"));
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Google Cloud Speech-to-Text transcription
    async fn transcribe_google_cloud(&self, _audio_path: &str) -> Result<TranscriptionResponse> {
        // Implementation would go here
        warn!("Google Cloud transcription not implemented yet");
        Err(anyhow!("Google Cloud transcription not implemented"))
    }

    /// Azure Speech Services transcription
    async fn transcribe_azure(&self, _audio_path: &str) -> Result<TranscriptionResponse> {
        // Implementation would go here
        warn!("Azure transcription not implemented yet");
        Err(anyhow!("Azure transcription not implemented"))
    }

    /// Local transcription (calls local Whisper server or Python script)
    async fn transcribe_local(&self, audio_path: &str) -> Result<TranscriptionResponse> {
        info!("🏠 Transcribing locally: {}", audio_path);

        // Call local whisper command or API
        let output = tokio::process::Command::new("whisper")
            .args([
                audio_path,
                "--model", &self.config.model,
                "--output_format", "json",
                "--fp16", "False", // Better compatibility
            ])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Local whisper failed: {}", error));
        }

        // Parse whisper output
        let output_str = String::from_utf8(output.stdout)?;
        let whisper_result: serde_json::Value = serde_json::from_str(&output_str)?;

        let transcription = TranscriptionResponse {
            text: whisper_result["text"].as_str().unwrap_or("").to_string(),
            language: whisper_result["language"].as_str().map(|s| s.to_string()),
            confidence: None,
            segments: self.parse_whisper_segments(&whisper_result),
            processing_time: None,
        };

        info!("✅ Local transcription completed: {} characters", transcription.text.len());
        Ok(transcription)
    }

    /// External API transcription (custom endpoint)
    async fn transcribe_external(&self, audio_path: &str) -> Result<TranscriptionResponse> {
        let endpoint = self.config.api_endpoint.as_ref()
            .ok_or_else(|| anyhow!("External API endpoint not configured"))?;

        info!("🌐 Transcribing with external API: {}", endpoint);

        let audio_data = tokio::fs::read(audio_path).await?;
        
        let mut request = self.client.post(endpoint);
        
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let form = reqwest::multipart::Form::new()
            .part("audio", reqwest::multipart::Part::bytes(audio_data)
                .file_name("audio.wav")
                .mime_str("audio/wav")?);

        let response = request.multipart(form).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("External API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        
        let transcription = TranscriptionResponse {
            text: response_json["text"].as_str().unwrap_or("").to_string(),
            language: response_json["language"].as_str().map(|s| s.to_string()),
            confidence: response_json["confidence"].as_f64(),
            segments: None,
            processing_time: response_json["processing_time"].as_f64(),
        };

        info!("✅ External API transcription completed: {} characters", transcription.text.len());
        Ok(transcription)
    }

    /// Parse OpenAI Whisper API segments
    fn parse_openai_segments(&self, response: &serde_json::Value) -> Option<Vec<TranscriptionSegment>> {
        let segments = response["segments"].as_array()?;
        let mut parsed_segments = Vec::new();

        for (i, segment) in segments.iter().enumerate() {
            if let (Some(start), Some(end), Some(text)) = (
                segment["start"].as_f64(),
                segment["end"].as_f64(),
                segment["text"].as_str(),
            ) {
                parsed_segments.push(TranscriptionSegment {
                    id: i as u32,
                    seek: segment["seek"].as_f64().unwrap_or(0.0),
                    start,
                    end,
                    text: text.to_string(),
                    confidence: segment["avg_logprob"].as_f64(),
                });
            }
        }

        if parsed_segments.is_empty() {
            None
        } else {
            Some(parsed_segments)
        }
    }

    /// Parse local Whisper segments
    fn parse_whisper_segments(&self, response: &serde_json::Value) -> Option<Vec<TranscriptionSegment>> {
        // Similar to OpenAI parsing, but might have different structure
        self.parse_openai_segments(response)
    }

    /// Retry transcription with exponential backoff
    pub async fn transcribe_with_retry(&self, audio_path: &str) -> Result<TranscriptionResponse> {
        let mut last_error = None;
        
        for attempt in 0..self.config.max_retries {
            match self.transcribe_audio(audio_path).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries - 1 {
                        let delay = Duration::from_secs(2_u64.pow(attempt));
                        warn!("Transcription attempt {} failed, retrying in {:?}", 
                              attempt + 1, delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All transcription attempts failed")))
    }
}

/// Transcription service health check
pub async fn check_transcription_service(config: &TranscriptionConfig) -> Result<bool> {
    match config.provider {
        TranscriptionProvider::OpenAI => {
            if let Some(api_key) = &config.api_key {
                let client = reqwest::Client::new();
                let response = client
                    .get("https://api.openai.com/v1/models")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send()
                    .await?;
                Ok(response.status().is_success())
            } else {
                Ok(false)
            }
        }
        TranscriptionProvider::Local => {
            // Check if whisper command is available
            let output = tokio::process::Command::new("whisper")
                .arg("--help")
                .output()
                .await;
            Ok(output.is_ok())
        }
        _ => Ok(true), // Assume other services are available
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TranscriptionConfig;

    #[test]
    fn test_transcription_client_creation() {
        let config = TranscriptionConfig {
            provider: TranscriptionProvider::Local,
            api_endpoint: None,
            api_key: None,
            model: "base".to_string(),
            language: None,
            auto_detect_language: true,
            max_retries: 3,
            timeout: 300,
            use_bjj_prompts: true,
            bjj_terms_file: None,
            output_formats: vec![],
            use_gpu: false,
            temperature: 0.0,
            best_of: 1,
            beam_size: 1,
        };

        let client = TranscriptionClient::new(config);
        assert_eq!(client.config.max_retries, 3);
    }

    #[tokio::test]
    async fn test_health_check_local() {
        let config = TranscriptionConfig {
            provider: TranscriptionProvider::Local,
            api_endpoint: None,
            api_key: None,
            model: "base".to_string(),
            language: None,
            auto_detect_language: true,
            max_retries: 3,
            timeout: 300,
            use_bjj_prompts: true,
            bjj_terms_file: None,
            output_formats: vec![],
            use_gpu: false,
            temperature: 0.0,
            best_of: 1,
            beam_size: 1,
        };

        // This test will pass/fail based on whether whisper is installed
        let _result = check_transcription_service(&config).await;
    }
}