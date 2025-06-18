//! Whisper transcription integration

use crate::{Result, TranscriptionError, TranscriptionConfig, AudioInfo};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Transcription segment with timing information
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
}

impl TranscriptionSegment {
    /// Create new transcription segment
    pub fn new(id: u32, start: f64, end: f64, text: String) -> Self {
        Self {
            id,
            start,
            end,
            text,
            confidence: None,
        }
    }
    
    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }
    
    /// Get duration of this segment
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.end - self.start)
    }
}

/// Complete transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Full transcription text
    text: String,
    /// Detected language
    language: Option<String>,
    /// Individual segments with timestamps
    segments: Vec<TranscriptionSegment>,
    /// Processing duration
    processing_time: Duration,
    /// Model used for transcription
    model_used: String,
    /// Path to generated text file
    text_path: Option<PathBuf>,
    /// Path to generated SRT file
    srt_path: Option<PathBuf>,
}

impl TranscriptionResult {
    /// Create new transcription result
    pub fn new(
        text: String,
        language: Option<String>,
        segments: Vec<TranscriptionSegment>,
        processing_time: Duration,
        model_used: String,
    ) -> Self {
        Self {
            text,
            language,
            segments,
            processing_time,
            model_used,
            text_path: None,
            srt_path: None,
        }
    }
    
    /// Get transcription text
    pub fn text(&self) -> &str {
        &self.text
    }
    
    /// Get detected language
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
    
    /// Get segments
    pub fn segments(&self) -> &[TranscriptionSegment] {
        &self.segments
    }
    
    /// Get processing time
    pub fn processing_time(&self) -> Duration {
        self.processing_time
    }
    
    /// Get model used
    pub fn model_used(&self) -> &str {
        &self.model_used
    }
    
    /// Get text file path
    pub fn text_path(&self) -> Option<&Path> {
        self.text_path.as_deref()
    }
    
    /// Set text file path
    pub fn with_text_path(mut self, path: PathBuf) -> Self {
        self.text_path = Some(path);
        self
    }
    
    /// Get SRT file path
    pub fn srt_path(&self) -> Option<&Path> {
        self.srt_path.as_deref()
    }
    
    /// Set SRT file path
    pub fn with_srt_path(mut self, path: PathBuf) -> Self {
        self.srt_path = Some(path);
        self
    }
}

/// Whisper transcriber
#[derive(Debug, Clone)]
pub struct WhisperTranscriber {
    config: TranscriptionConfig,
}

impl WhisperTranscriber {
    /// Create new Whisper transcriber
    pub fn new(config: TranscriptionConfig) -> Self {
        Self { config }
    }
    
    /// Get model name
    pub fn model(&self) -> &str {
        self.config.model()
    }
    
    /// Check if GPU support is enabled
    pub fn supports_gpu(&self) -> bool {
        self.config.use_gpu()
    }
    
    /// Check if transcription already exists for a video
    pub fn transcription_exists(&self, video_path: &Path) -> bool {
        // Check for corrected transcript first, then regular transcript
        let parent = video_path.parent().unwrap_or(Path::new("."));
        let stem = video_path.file_stem().unwrap().to_string_lossy();
        
        let corrected_path = parent.join(format!("{}_corrected.txt", stem));
        let regular_path = parent.join(format!("{}.txt", stem));
        
        corrected_path.exists() || regular_path.exists()
    }
    
    /// Get output paths for transcription files
    pub fn get_output_paths(&self, video_path: &Path) -> (PathBuf, PathBuf) {
        let parent = video_path.parent().unwrap_or(Path::new("."));
        let stem = video_path.file_stem().unwrap().to_string_lossy();
        
        let text_path = parent.join(format!("{}.txt", stem));
        let srt_path = parent.join(format!("{}.srt", stem));
        
        (text_path, srt_path)
    }
    
    /// Transcribe audio file using Whisper
    pub async fn transcribe_audio(&self, audio_info: &AudioInfo) -> Result<TranscriptionResult> {
        let start_time = std::time::Instant::now();
        
        if !audio_info.path().exists() {
            return Err(TranscriptionError::Transcription(format!("Audio file not found: {}", audio_info.path().display())));
        }
        
        tracing::info!("Starting Whisper transcription for: {}", audio_info.path().display());
        
        // Build Whisper.cpp command
        let mut cmd = tokio::process::Command::new("whisper-cli");
        
        // Basic arguments - output both text and JSON files
        cmd.args([
            "-f", audio_info.path().to_str().ok_or_else(|| TranscriptionError::Transcription("Invalid audio path".to_string()))?,
            "-otxt", // Output text file
            "-oj",   // Output JSON file
        ]);
        
        // Model path resolution for whisper-cli
        self.add_model_args(&mut cmd)?;
        
        // Language detection or explicit language
        if let Some(language) = self.config.language() {
            if language != "auto" {
                cmd.args(["-l", language]);
            }
        }
        
        // GPU support (whisper.cpp uses --no-gpu to disable)
        if !self.config.use_gpu() {
            cmd.args(["--no-gpu"]);
        }
        
        // Output file path (without extension)
        let output_stem = audio_info.path().with_extension("");
        if let Some(output_path) = output_stem.to_str() {
            cmd.args(["-of", output_path]);
        }
        
        // Execute Whisper
        tracing::debug!("Running Whisper command: {:?}", cmd);
        let output = cmd.output().await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::error!("Whisper stderr: {}", stderr);
            tracing::error!("Whisper stdout: {}", stdout);
            return Err(TranscriptionError::Whisper(format!("Whisper failed: {}", stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("Whisper stdout: {}", stdout);
        tracing::debug!("Whisper stderr: {}", stderr);
        
        // Parse Whisper JSON output from file (created by -oj)
        let output_stem = audio_info.path().with_extension("");
        let json_path = format!("{}.json", output_stem.to_string_lossy());
        let json_path = std::path::Path::new(&json_path);
        
        tracing::debug!("Expected JSON path: {}", json_path.display());
        tracing::debug!("JSON path exists: {}", json_path.exists());
        
        if !json_path.exists() {
            // List files in the directory to see what was actually created
            if let Some(parent) = json_path.parent() {
                if let Ok(entries) = std::fs::read_dir(parent) {
                    tracing::debug!("Files in directory:");
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.contains(&audio_info.path().file_stem().unwrap().to_string_lossy().to_string()) {
                                tracing::debug!("  - {}", name);
                            }
                        }
                    }
                }
            }
            return Err(TranscriptionError::Transcription(format!("Whisper JSON output not found: {}", json_path.display())));
        }
        
        let json_content = tokio::fs::read_to_string(&json_path).await?;
        let whisper_result: serde_json::Value = serde_json::from_str(&json_content)
            .map_err(|e| TranscriptionError::Transcription(format!("Failed to parse Whisper JSON: {}", e)))?;
        
        // Extract text and segments from whisper-cli JSON format
        let language = whisper_result["result"]["language"]
            .as_str()
            .map(|s| s.to_string());
        
        let empty_vec = Vec::new();
        let transcription_array = whisper_result["transcription"]
            .as_array()
            .unwrap_or(&empty_vec);
        
        // Combine all text segments
        let text = transcription_array
            .iter()
            .filter_map(|segment| segment["text"].as_str())
            .collect::<Vec<&str>>()
            .join("")
            .trim()
            .to_string();
        
        let segments: Vec<TranscriptionSegment> = transcription_array
            .iter()
            .enumerate()
            .filter_map(|(id, segment)| {
                let start_ms = segment["offsets"]["from"].as_u64()?;
                let end_ms = segment["offsets"]["to"].as_u64()?;
                let text = segment["text"].as_str()?.trim().to_string();
                
                if !text.is_empty() {
                    let start_secs = start_ms as f64 / 1000.0;
                    let end_secs = end_ms as f64 / 1000.0;
                    Some(TranscriptionSegment::new(id as u32, start_secs, end_secs, text))
                } else {
                    None
                }
            })
            .collect();
        
        let processing_time = start_time.elapsed();
        
        tracing::info!(
            "Whisper transcription completed in {:.2}s: {} segments, {} chars",
            processing_time.as_secs_f64(),
            segments.len(),
            text.len()
        );
        
        // Clean up temporary files that whisper-cli creates
        let _ = tokio::fs::remove_file(&json_path).await;
        
        let result = TranscriptionResult::new(
            text,
            language,
            segments,
            processing_time,
            self.config.model().to_string(),
        );
        
        // Save transcription results to files
        self.save_transcription_result(&result, audio_info.path()).await?;
        
        Ok(result)
    }
    
    /// Save transcription result to text and SRT files
    async fn save_transcription_result(&self, result: &TranscriptionResult, audio_path: &Path) -> Result<()> {
        // Determine output paths based on audio file path
        let video_path = audio_path.with_extension("mp4"); // Assume video is mp4
        let (text_path, srt_path) = self.get_output_paths(&video_path);
        
        // Save transcript text
        tokio::fs::write(&text_path, result.text()).await?;
        tracing::debug!("Saved transcript to: {}", text_path.display());
        
        // Generate and save SRT content
        let srt_content = self.generate_srt_content(result);
        tokio::fs::write(&srt_path, srt_content).await?;
        tracing::debug!("Saved SRT to: {}", srt_path.display());
        
        Ok(())
    }
    
    /// Generate SRT subtitle content from transcription result
    fn generate_srt_content(&self, result: &TranscriptionResult) -> String {
        let mut srt_content = String::new();
        
        for (index, segment) in result.segments().iter().enumerate() {
            let start_time = self.format_srt_timestamp(segment.start);
            let end_time = self.format_srt_timestamp(segment.end);
            
            srt_content.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                index + 1,
                start_time,
                end_time,
                segment.text
            ));
        }
        
        srt_content
    }
    
    /// Format timestamp for SRT format (HH:MM:SS,mmm)
    fn format_srt_timestamp(&self, seconds: f64) -> String {
        let total_seconds = seconds as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;
        let milliseconds = ((seconds - total_seconds as f64) * 1000.0) as u64;
        
        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, milliseconds)
    }
    
    /// Transcribe video file (extracts audio first if needed)
    pub async fn transcribe_video(&self, video_path: &Path) -> Result<TranscriptionResult> {
        use crate::audio::AudioExtractor;
        
        // Check if transcription already exists
        if self.transcription_exists(video_path) {
            return Err(TranscriptionError::Transcription("Transcription already exists".to_string()));
        }
        
        // Extract audio if needed
        let audio_extractor = AudioExtractor::new();
        let audio_info = if audio_extractor.audio_exists(video_path) {
            // Audio already exists, get info
            let audio_path = audio_extractor.get_audio_output_path(video_path);
            audio_extractor.get_audio_info(&audio_path).await?
        } else {
            // Extract audio from video
            tracing::info!("Extracting audio from: {}", video_path.display());
            audio_extractor.extract_audio(video_path).await?
        };
        
        // Transcribe the audio
        let result = self.transcribe_audio(&audio_info).await?;
        
        let (text_path, srt_path) = self.get_output_paths(video_path);
        
        Ok(result
            .with_text_path(text_path)
            .with_srt_path(srt_path))
    }
    
    /// Add model arguments to whisper-cli command
    fn add_model_args(&self, cmd: &mut tokio::process::Command) -> Result<()> {
        let model_paths = [
            &format!("models/ggml-{}.bin", self.config.model()),
            "models/ggml-tiny.bin", // Our downloaded tiny model
            "models/ggml-base.bin", // Fallback to base model
            "/usr/local/share/whisper-cpp/ggml-base.bin",
            "/opt/homebrew/share/whisper-cpp/ggml-base.bin", 
            &format!("/usr/local/share/whisper-cpp/ggml-{}.bin", self.config.model()),
            &format!("/opt/homebrew/share/whisper-cpp/ggml-{}.bin", self.config.model()),
            "/usr/local/Cellar/whisper-cpp/1.7.5/share/whisper-cpp/for-tests-ggml-tiny.bin",
        ];
        
        tracing::info!("🔍 Looking for Whisper model files...");
        let mut model_found = false;
        for model_path in &model_paths {
            tracing::debug!("🔍 Checking: {}", model_path);
            if std::path::Path::new(model_path).exists() {
                let metadata = std::fs::metadata(model_path).unwrap_or_else(|_| std::fs::metadata(".").unwrap());
                tracing::info!("✅ Found model: {} ({:.1} MB)", model_path, metadata.len() as f64 / 1_000_000.0);
                cmd.args(["-m", model_path]);
                model_found = true;
                break;
            } else {
                tracing::debug!("❌ Not found: {}", model_path);
            }
        }
        
        if !model_found {
            tracing::warn!("⚠️  No model found, using default");
        }
        
        Ok(())
    }
}