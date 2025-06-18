//! Audio extraction and processing

use crate::{Result, TranscriptionError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Audio information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    path: PathBuf,
    duration: Duration,
    sample_rate: u32,
    channels: u32,
    format: String,
    file_size: u64,
    bitrate: Option<u32>,
}

impl AudioInfo {
    /// Create new audio info
    pub fn new(
        path: PathBuf,
        duration: Duration,
        sample_rate: u32,
        channels: u32,
        format: String,
        file_size: u64,
    ) -> Self {
        Self {
            path,
            duration,
            sample_rate,
            channels,
            format,
            file_size,
            bitrate: None,
        }
    }
    
    /// Get audio file path
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Get duration
    pub fn duration(&self) -> Duration {
        self.duration
    }
    
    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    /// Get channel count
    pub fn channels(&self) -> u32 {
        self.channels
    }
    
    /// Get audio format
    pub fn format(&self) -> &str {
        &self.format
    }
    
    /// Get file size
    pub fn file_size(&self) -> u64 {
        self.file_size
    }
    
    /// Get bitrate if available
    pub fn bitrate(&self) -> Option<u32> {
        self.bitrate
    }
    
    /// Set bitrate
    pub fn with_bitrate(mut self, bitrate: u32) -> Self {
        self.bitrate = Some(bitrate);
        self
    }
}

/// Audio extractor for high-performance audio processing
#[derive(Debug, Clone)]
pub struct AudioExtractor {
    target_sample_rate: u32,
    target_format: String,
}

impl Default for AudioExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioExtractor {
    /// Create new audio extractor with optimal settings for Whisper
    pub fn new() -> Self {
        Self {
            target_sample_rate: 16000, // 16kHz optimal for Whisper
            target_format: "wav".to_string(),
        }
    }
    
    /// Get target sample rate
    pub fn target_sample_rate(&self) -> u32 {
        self.target_sample_rate
    }
    
    /// Get target format
    pub fn target_format(&self) -> &str {
        &self.target_format
    }
    
    /// Get expected audio output path for a video file
    pub fn get_audio_output_path(&self, video_path: &Path) -> PathBuf {
        let parent = video_path.parent().unwrap_or(Path::new("."));
        let stem = video_path.file_stem().unwrap().to_string_lossy();
        parent.join(format!("{}.{}", stem, self.target_format))
    }
    
    /// Check if audio artifact already exists
    pub fn audio_exists(&self, video_path: &Path) -> bool {
        self.get_audio_output_path(video_path).exists()
    }
    
    /// Extract audio from video file
    pub async fn extract_audio(&self, video_path: &Path) -> Result<AudioInfo> {
        let output_path = self.get_audio_output_path(video_path);
        
        // Check if audio already exists
        if output_path.exists() {
            return self.get_audio_info(&output_path).await;
        }
        
        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Extract audio using FFmpeg
        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", video_path.to_str().ok_or_else(|| TranscriptionError::AudioExtraction("Invalid video path".to_string()))?,
                "-vn", // No video stream
                "-acodec", "pcm_s16le", // 16-bit PCM
                "-ar", &self.target_sample_rate.to_string(), // Sample rate
                "-ac", "1", // Mono channel
                "-af", "volume=0.95", // Slight volume boost
                "-f", &self.target_format, // Format
                "-y", // Overwrite existing
                output_path.to_str().ok_or_else(|| TranscriptionError::AudioExtraction("Invalid output path".to_string()))?,
            ])
            .status()
            .await?;
        
        if !status.success() {
            return Err(TranscriptionError::FFmpeg(format!("Audio extraction failed for {}", video_path.display())));
        }
        
        // Get audio info
        self.get_audio_info(&output_path).await
    }
    
    /// Get audio information from existing file
    pub async fn get_audio_info(&self, audio_path: &Path) -> Result<AudioInfo> {
        if !audio_path.exists() {
            return Err(TranscriptionError::AudioExtraction(format!("Audio file not found: {}", audio_path.display())));
        }
        
        // Use FFprobe to get audio information
        let output = tokio::process::Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                "-select_streams", "a:0", // First audio stream
                audio_path.to_str().ok_or_else(|| TranscriptionError::AudioExtraction("Invalid audio path".to_string()))?,
            ])
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(TranscriptionError::FFmpeg(format!("ffprobe failed for {}", audio_path.display())));
        }
        
        let json_str = String::from_utf8(output.stdout)
            .map_err(|e| TranscriptionError::AudioExtraction(format!("Invalid ffprobe output: {}", e)))?;
        
        let ffprobe_data: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| TranscriptionError::AudioExtraction(format!("Failed to parse ffprobe output: {}", e)))?;
        
        let format = &ffprobe_data["format"];
        let streams = ffprobe_data["streams"].as_array()
            .ok_or_else(|| TranscriptionError::AudioExtraction("No streams in ffprobe output".to_string()))?;
        let audio_stream = streams.first()
            .ok_or_else(|| TranscriptionError::AudioExtraction("No audio stream found".to_string()))?;
        
        let duration_seconds: f64 = format["duration"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        
        let file_size = tokio::fs::metadata(audio_path).await?.len();
        
        let audio_info = AudioInfo::new(
            audio_path.to_path_buf(),
            Duration::from_secs_f64(duration_seconds),
            audio_stream["sample_rate"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(self.target_sample_rate),
            audio_stream["channels"].as_u64().unwrap_or(1) as u32,
            audio_stream["codec_name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            file_size,
        );
        
        let audio_info = if let Some(bitrate_str) = audio_stream["bit_rate"].as_str() {
            if let Ok(bitrate) = bitrate_str.parse::<u32>() {
                audio_info.with_bitrate(bitrate)
            } else {
                audio_info
            }
        } else {
            audio_info
        };
        
        Ok(audio_info)
    }
}