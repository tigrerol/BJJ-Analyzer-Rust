use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn, debug, error};

use crate::audio::AudioInfo;
use crate::bjj::BJJDictionary;
use crate::config::TranscriptionConfig;
use super::srt::{SRTGenerator, SRTEntry};

/// Transcription segment from Whisper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Segment ID
    pub id: u32,
    /// Start time in seconds
    pub start: f64,
    /// End time in seconds
    pub end: f64,
    /// Transcribed text
    pub text: String,
    /// Confidence score (if available)
    pub confidence: Option<f64>,
    /// Average log probability
    pub avg_logprob: Option<f64>,
    /// No speech probability
    pub no_speech_prob: Option<f64>,
}

/// Complete transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Full transcription text
    pub text: String,
    /// Detected language
    pub language: Option<String>,
    /// Individual segments with timestamps
    pub segments: Vec<TranscriptionSegment>,
    /// Path to generated SRT file
    pub srt_path: Option<PathBuf>,
    /// Path to text file
    pub text_path: Option<PathBuf>,
    /// Processing duration
    pub processing_time: Duration,
    /// Model used for transcription
    pub model_used: String,
    /// BJJ prompt used
    pub bjj_prompt: Option<String>,
}

/// Whisper transcriber with BJJ-specific optimization
#[derive(Debug, Clone)]
pub struct WhisperTranscriber {
    /// Configuration
    config: TranscriptionConfig,
    /// BJJ dictionary for context
    bjj_dictionary: BJJDictionary,
    /// Whisper model path or name
    model: String,
    /// Enable GPU acceleration
    use_gpu: bool,
}

impl WhisperTranscriber {
    /// Create a new Whisper transcriber
    pub fn new(config: TranscriptionConfig, bjj_dictionary: BJJDictionary) -> Self {
        let model = config.model.clone();
        
        Self {
            config,
            bjj_dictionary,
            model,
            use_gpu: Self::detect_gpu_support(),
        }
    }
    
    /// Create transcriber with custom model
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
    
    /// Enable or disable GPU acceleration
    pub fn with_gpu(mut self, use_gpu: bool) -> Self {
        self.use_gpu = use_gpu;
        self
    }
    
    /// Transcribe audio file with BJJ-specific optimization
    pub async fn transcribe_audio(
        &self,
        audio_info: &AudioInfo,
        output_dir: &Path,
    ) -> Result<TranscriptionResult> {
        let start_time = std::time::Instant::now();
        
        info!("🎤 Starting Whisper transcription for: {}", audio_info.path.display());
        info!("📊 Audio info: {}Hz, {} channels, {:?} duration", 
              audio_info.sample_rate, audio_info.channels, audio_info.duration);
        
        // Generate BJJ-specific prompt
        let bjj_prompt = self.bjj_dictionary.generate_prompt();
        info!("🥋 Using BJJ prompt: {}", bjj_prompt);
        
        // Prepare output paths
        let base_name = audio_info.path.file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        
        let temp_dir = output_dir.join("temp_whisper");
        tokio::fs::create_dir_all(&temp_dir).await?;
        
        // Run Whisper command
        let whisper_result = self.run_whisper_command(
            &audio_info.path,
            &temp_dir,
            &bjj_prompt,
        ).await?;
        
        // Parse results and create SRT
        let transcription_result = self.process_whisper_output(
            whisper_result,
            &base_name,
            output_dir,
            bjj_prompt,
            start_time.elapsed(),
        ).await?;
        
        // Cleanup temporary files
        if self.config.timeout > 0 {
            let _ = tokio::fs::remove_dir_all(&temp_dir).await;
        }
        
        info!("✅ Transcription completed in {:?}: {} characters, {} segments",
              transcription_result.processing_time,
              transcription_result.text.len(),
              transcription_result.segments.len());
        
        Ok(transcription_result)
    }
    
    /// Run Whisper command-line tool with automatic backend detection
    async fn run_whisper_command(
        &self,
        audio_path: &Path,
        output_dir: &Path,
        bjj_prompt: &str,
    ) -> Result<WhisperOutput> {
        // Try different whisper implementations in order of preference
        let backends = [
            ("whisper-cli", true),  // whisper.cpp via Homebrew (fastest)
            ("whisper-cpp", true),  // whisper.cpp (fastest)
            ("whisper", false),     // Python OpenAI Whisper (fallback)
        ];
        
        for (cmd_name, is_cpp) in &backends {
            if Self::check_command_available(cmd_name).await {
                info!("🚀 Using {} backend for transcription", cmd_name);
                return if *is_cpp {
                    self.run_whisper_cpp_command(audio_path, output_dir, bjj_prompt).await
                } else {
                    self.run_python_whisper_command(audio_path, output_dir, bjj_prompt).await
                };
            }
        }
        
        Err(anyhow!("No Whisper backend found. Please install whisper.cpp or openai-whisper"))
    }
    
    /// Run whisper.cpp (C++ implementation - fastest)
    async fn run_whisper_cpp_command(
        &self,
        audio_path: &Path,
        output_dir: &Path,
        bjj_prompt: &str,
    ) -> Result<WhisperOutput> {
        // Auto-detect whisper-cli vs whisper-cpp command
        let cmd_name = if Self::check_command_available("whisper-cli").await {
            "whisper-cli"
        } else {
            "whisper-cpp"
        };
        
        let mut cmd = Command::new(cmd_name);
        
        // whisper.cpp/whisper-cli argument format
        let base_name = audio_path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let output_file = output_dir.join(&base_name);
        
        cmd.arg("-f").arg(audio_path.to_str().unwrap())
            .arg("--output-json") // JSON output
            .arg("--output-srt") // Also generate SRT
            .arg("--output-file").arg(output_file.to_str().unwrap())
            .arg("-t").arg("4") // 4 threads
            .arg("--temperature").arg("0.0");
        
        // Try to use base model (common default)
        if cmd_name == "whisper-cli" {
            // For homebrew whisper-cli, try default model location
            let model_paths = [
                &format!("models/ggml-{}.bin", self.model),
                "/usr/local/share/whisper-cpp/ggml-base.bin",
                "/opt/homebrew/share/whisper-cpp/ggml-base.bin", 
                &format!("/usr/local/share/whisper-cpp/ggml-{}.bin", self.model),
                "/usr/local/Cellar/whisper-cpp/1.7.5/share/whisper-cpp/for-tests-ggml-tiny.bin",
            ];
            
            for model_path in &model_paths {
                if std::path::Path::new(model_path).exists() {
                    cmd.arg("-m").arg(model_path);
                    info!("🎯 Using Whisper model: {}", model_path);
                    break;
                }
            }
        } else {
            cmd.arg("-m").arg(&format!("models/ggml-{}.bin", self.model));
        }
        
        // BJJ-specific prompt
        if !bjj_prompt.is_empty() {
            cmd.arg("--prompt").arg(bjj_prompt);
        }
        
        // Language settings
        if let Some(language) = &self.config.language {
            cmd.arg("-l").arg(language);
        }
        
        info!("🚀 Running {}: {} model on {}", 
              cmd_name, self.model, audio_path.display());
        
        self.execute_command_and_parse(cmd, output_dir, "whisper.cpp").await
    }
    
    /// Run Python OpenAI Whisper (fallback)
    async fn run_python_whisper_command(
        &self,
        audio_path: &Path,
        output_dir: &Path,
        bjj_prompt: &str,
    ) -> Result<WhisperOutput> {
        let mut cmd = Command::new("whisper");
        
        // Python whisper arguments
        cmd.arg(audio_path.to_str().unwrap())
            .arg("--model").arg(&self.model)
            .arg("--output_dir").arg(output_dir.to_str().unwrap())
            .arg("--output_format").arg("json")
            .arg("--verbose").arg("False")
            .arg("--fp16").arg("False");
        
        // BJJ-specific prompt
        if !bjj_prompt.is_empty() {
            cmd.arg("--initial_prompt").arg(bjj_prompt);
        }
        
        // Language settings
        if let Some(language) = &self.config.language {
            cmd.arg("--language").arg(language);
        }
        
        // GPU settings
        if !self.use_gpu {
            cmd.arg("--device").arg("cpu");
        }
        
        // Quality settings
        cmd.arg("--temperature").arg("0.0")
            .arg("--best_of").arg("3")
            .arg("--beam_size").arg("5");
        
        info!("🚀 Running Python Whisper: {} model on {}", 
              self.model, audio_path.display());
        
        self.execute_command_and_parse(cmd, output_dir, "Python Whisper").await
    }
    
    /// Execute command and parse JSON output
    async fn execute_command_and_parse(
        &self,
        mut cmd: Command,
        output_dir: &Path,
        backend_name: &str,
    ) -> Result<WhisperOutput> {
        // Execute command with timeout
        let timeout_duration = Duration::from_secs(self.config.timeout as u64);
        let output = tokio::time::timeout(timeout_duration, cmd.output()).await??;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("{} command failed: {}", backend_name, stderr);
            return Err(anyhow!("{} transcription failed: {}", backend_name, stderr));
        }
        
        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("{} stdout: {}", backend_name, stdout);
        
        // Find the JSON output file
        let json_files = self.find_whisper_output_files(output_dir, "json").await?;
        if json_files.is_empty() {
            return Err(anyhow!("No {} JSON output found", backend_name));
        }
        
        let json_content = tokio::fs::read_to_string(&json_files[0]).await?;
        let whisper_data: WhisperOutput = serde_json::from_str(&json_content)?;
        
        Ok(whisper_data)
    }
    
    /// Check if a command is available
    async fn check_command_available(cmd_name: &str) -> bool {
        Command::new(cmd_name)
            .arg("--help")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// Process Whisper output and create final result
    async fn process_whisper_output(
        &self,
        whisper_output: WhisperOutput,
        base_name: &str,
        output_dir: &Path,
        bjj_prompt: String,
        processing_time: Duration,
    ) -> Result<TranscriptionResult> {
        // Convert Whisper segments to our format
        let segments: Vec<TranscriptionSegment> = whisper_output.segments
            .into_iter()
            .enumerate()
            .map(|(i, seg)| TranscriptionSegment {
                id: i as u32,
                start: seg.start,
                end: seg.end,
                text: seg.text.trim().to_string(),
                confidence: seg.avg_logprob.map(|p| (p + 1.0) / 2.0), // Normalize to 0-1
                avg_logprob: seg.avg_logprob,
                no_speech_prob: seg.no_speech_prob,
            })
            .collect();
        
        // Generate SRT file
        let srt_path = self.generate_srt_file(&segments, base_name, output_dir).await?;
        
        // Save text file
        let text_path = self.save_text_file(&whisper_output.text, base_name, output_dir).await?;
        
        Ok(TranscriptionResult {
            text: whisper_output.text,
            language: Some(whisper_output.language),
            segments,
            srt_path: Some(srt_path),
            text_path: Some(text_path),
            processing_time,
            model_used: self.model.clone(),
            bjj_prompt: Some(bjj_prompt),
        })
    }
    
    /// Generate SRT file from segments
    async fn generate_srt_file(
        &self,
        segments: &[TranscriptionSegment],
        base_name: &str,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let mut srt_generator = SRTGenerator::new();
        
        for segment in segments {
            if segment.text.trim().is_empty() {
                continue;
            }
            
            let entry = SRTEntry::new(
                segment.id + 1,
                Duration::from_secs_f64(segment.start),
                Duration::from_secs_f64(segment.end),
                segment.text.clone(),
            );
            srt_generator.add_entry(entry);
        }
        
        // Sort and validate
        srt_generator.sort_entries();
        let issues = srt_generator.validate();
        if !issues.is_empty() {
            warn!("SRT validation issues: {:?}", issues);
        }
        
        // Save SRT file
        let srt_path = output_dir.join(format!("{}.srt", base_name));
        srt_generator.save_to_file(&srt_path).await?;
        
        info!("💾 SRT file saved: {} ({} entries)", 
              srt_path.display(), srt_generator.len());
        
        Ok(srt_path)
    }
    
    /// Save transcription text to file
    async fn save_text_file(
        &self,
        text: &str,
        base_name: &str,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let text_path = output_dir.join(format!("{}.txt", base_name));
        tokio::fs::write(&text_path, text).await?;
        
        info!("💾 Text file saved: {} ({} characters)", 
              text_path.display(), text.len());
        
        Ok(text_path)
    }
    
    /// Find Whisper output files
    async fn find_whisper_output_files(&self, dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == extension) {
                files.push(path);
            }
        }
        
        Ok(files)
    }
    
    /// Detect GPU support for Whisper
    fn detect_gpu_support() -> bool {
        // Try to detect CUDA or other GPU support
        std::env::var("CUDA_VISIBLE_DEVICES").is_ok() ||
        std::env::var("WHISPER_USE_GPU").map_or(false, |v| v == "1" || v.to_lowercase() == "true")
    }
    
    /// Check if Whisper is available (any backend)
    pub async fn check_availability() -> Result<String> {
        let backends = [
            ("whisper-cpp", "whisper.cpp (C++ implementation)"),
            ("whisper", "OpenAI Whisper (Python implementation)"),
        ];
        
        for (cmd_name, description) in &backends {
            if Self::check_command_available(cmd_name).await {
                return Ok(format!("{} available", description));
            }
        }
        
        Err(anyhow!(
            "No Whisper backend found. Please install:\n\
            - whisper.cpp (recommended): https://github.com/ggerganov/whisper.cpp\n\
            - Or OpenAI Whisper: pip install openai-whisper"
        ))
    }
    
    /// Get available models
    pub async fn get_available_models() -> Result<Vec<String>> {
        // Standard Whisper models
        Ok(vec![
            "tiny".to_string(),
            "base".to_string(),
            "small".to_string(),
            "medium".to_string(),
            "large".to_string(),
            "large-v1".to_string(),
            "large-v2".to_string(),
            "large-v3".to_string(),
        ])
    }
    
    /// Estimate transcription time
    pub fn estimate_processing_time(&self, audio_duration: Duration) -> Duration {
        // Rough estimate: base model processes ~10x real-time on CPU
        let multiplier = match self.model.as_str() {
            "tiny" => 2.0,
            "base" => 5.0,
            "small" => 10.0,
            "medium" => 20.0,
            "large" | "large-v1" | "large-v2" | "large-v3" => 40.0,
            _ => 10.0,
        };
        
        let gpu_factor = if self.use_gpu { 0.3 } else { 1.0 };
        let estimated_seconds = audio_duration.as_secs_f64() * multiplier * gpu_factor;
        
        Duration::from_secs_f64(estimated_seconds)
    }
}

/// Whisper JSON output format
#[derive(Debug, Clone, Deserialize)]
struct WhisperOutput {
    text: String,
    language: String,
    segments: Vec<WhisperSegment>,
}

#[derive(Debug, Clone, Deserialize)]
struct WhisperSegment {
    id: u32,
    start: f64,
    end: f64,
    text: String,
    #[serde(default)]
    avg_logprob: Option<f64>,
    #[serde(default)]
    no_speech_prob: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TranscriptionConfig, TranscriptionProvider};
    use tempfile::TempDir;

    fn create_test_config() -> TranscriptionConfig {
        TranscriptionConfig {
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
        }
    }

    #[test]
    fn test_transcriber_creation() {
        let config = create_test_config();
        let dict = BJJDictionary::new();
        let transcriber = WhisperTranscriber::new(config, dict);
        
        assert_eq!(transcriber.model, "base");
    }

    #[test]
    fn test_gpu_detection() {
        // This test depends on environment
        let _gpu_available = WhisperTranscriber::detect_gpu_support();
    }

    #[tokio::test]
    async fn test_whisper_availability() {
        // This test will pass/fail based on whether whisper is installed
        let _result = WhisperTranscriber::check_availability().await;
    }

    #[test]
    fn test_processing_time_estimation() {
        let config = create_test_config();
        let dict = BJJDictionary::new();
        let transcriber = WhisperTranscriber::new(config, dict);
        
        let audio_duration = Duration::from_secs(60); // 1 minute
        let estimated = transcriber.estimate_processing_time(audio_duration);
        
        // Should be reasonable estimate (not zero, not too crazy)
        assert!(estimated.as_secs() > 0);
        assert!(estimated.as_secs() < 3600); // Less than 1 hour for 1 minute
    }

    #[test]
    fn test_available_models() {
        tokio_test::block_on(async {
            let models = WhisperTranscriber::get_available_models().await.unwrap();
            assert!(models.contains(&"base".to_string()));
            assert!(models.contains(&"large".to_string()));
        });
    }
}