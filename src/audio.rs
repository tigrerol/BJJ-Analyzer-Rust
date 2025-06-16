use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};

/// Audio information and processing capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    pub path: PathBuf,
    pub duration: Duration,
    pub sample_rate: u32,
    pub channels: u32,
    pub format: String,
    pub bitrate: Option<u32>,
    pub file_size: u64,
}

/// High-performance audio extractor and processor
#[derive(Clone)]
pub struct AudioExtractor {
    /// Default sample rate for transcription (Whisper optimal)
    pub target_sample_rate: u32,
    /// Target audio format for processing
    pub target_format: String,
}

impl AudioExtractor {
    pub fn new() -> Self {
        Self {
            target_sample_rate: 16000, // 16kHz optimal for Whisper
            target_format: "wav".to_string(),
        }
    }

    /// Extract audio from video with optimal settings for transcription
    pub async fn extract_for_transcription(
        &self,
        video_path: &Path,
        output_dir: &Path,
    ) -> Result<AudioInfo> {
        let filename = video_path
            .file_stem()
            .ok_or_else(|| anyhow!("Invalid video filename"))?
            .to_string_lossy();
        
        let audio_path = output_dir.join(format!("{}.wav", filename));

        info!("🎵 Extracting audio for transcription: {}", video_path.display());

        // Ensure output directory exists
        tokio::fs::create_dir_all(output_dir).await?;

        // Extract audio with optimal settings for Whisper
        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", video_path.to_str().unwrap(),
                "-vn", // No video stream
                "-acodec", "pcm_s16le", // 16-bit PCM
                "-ar", &self.target_sample_rate.to_string(), // Sample rate
                "-ac", "1", // Mono channel
                "-af", "volume=0.95", // Slight volume boost
                "-f", "wav", // WAV format
                "-y", // Overwrite existing
                audio_path.to_str().unwrap(),
            ])
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Audio extraction failed for {}", video_path.display()));
        }

        // Get audio info
        let audio_info = self.get_audio_info(&audio_path).await?;
        
        info!("✅ Audio extracted: {} ({:.1}s, {}Hz)", 
              audio_info.path.display(),
              audio_info.duration.as_secs_f64(),
              audio_info.sample_rate);

        Ok(audio_info)
    }

    /// Get detailed audio information
    pub async fn get_audio_info(&self, audio_path: &Path) -> Result<AudioInfo> {
        let output = tokio::process::Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                "-select_streams", "a:0", // First audio stream
                audio_path.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("ffprobe failed for {}", audio_path.display()));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let ffprobe_data: serde_json::Value = serde_json::from_str(&json_str)?;

        let format = &ffprobe_data["format"];
        let streams = ffprobe_data["streams"].as_array().unwrap();
        let audio_stream = streams.first()
            .ok_or_else(|| anyhow!("No audio stream found"))?;

        let duration_seconds: f64 = format["duration"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let file_size = tokio::fs::metadata(audio_path).await?.len();

        let audio_info = AudioInfo {
            path: audio_path.to_path_buf(),
            duration: Duration::from_secs_f64(duration_seconds),
            sample_rate: audio_stream["sample_rate"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(self.target_sample_rate),
            channels: audio_stream["channels"].as_u64().unwrap_or(1) as u32,
            format: audio_stream["codec_name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            bitrate: audio_stream["bit_rate"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            file_size,
        };

        Ok(audio_info)
    }

    /// Split audio into chunks for batch transcription
    pub async fn split_audio(
        &self,
        audio_info: &AudioInfo,
        chunk_duration: Duration,
        output_dir: &Path,
    ) -> Result<Vec<AudioInfo>> {
        let filename = audio_info.path
            .file_stem()
            .unwrap()
            .to_string_lossy();

        let total_duration = audio_info.duration.as_secs_f64();
        let chunk_seconds = chunk_duration.as_secs_f64();
        let num_chunks = (total_duration / chunk_seconds).ceil() as usize;

        info!("✂️ Splitting audio into {} chunks of {:.1}s each", 
              num_chunks, chunk_seconds);

        let mut chunks = Vec::new();

        for i in 0..num_chunks {
            let start_time = i as f64 * chunk_seconds;
            let chunk_path = output_dir.join(format!("{}_chunk_{:03}.wav", filename, i));

            let status = tokio::process::Command::new("ffmpeg")
                .args([
                    "-i", audio_info.path.to_str().unwrap(),
                    "-ss", &start_time.to_string(),
                    "-t", &chunk_seconds.to_string(),
                    "-c", "copy", // Copy without re-encoding
                    "-y",
                    chunk_path.to_str().unwrap(),
                ])
                .status()
                .await?;

            if status.success() {
                let chunk_info = self.get_audio_info(&chunk_path).await?;
                chunks.push(chunk_info);
            } else {
                warn!("Failed to create chunk {}", i);
            }
        }

        info!("✅ Created {} audio chunks", chunks.len());
        Ok(chunks)
    }

    /// Enhance audio quality for better transcription
    pub async fn enhance_audio(
        &self,
        audio_info: &AudioInfo,
        output_path: &Path,
    ) -> Result<AudioInfo> {
        info!("🔧 Enhancing audio quality: {}", audio_info.path.display());

        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", audio_info.path.to_str().unwrap(),
                // Audio enhancement filters
                "-af", "highpass=f=80,lowpass=f=8000,volume=1.2,dynaudnorm=g=3",
                "-ar", &self.target_sample_rate.to_string(),
                "-ac", "1",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Audio enhancement failed"));
        }

        let enhanced_info = self.get_audio_info(output_path).await?;
        
        info!("✅ Audio enhanced: {}", output_path.display());
        Ok(enhanced_info)
    }

    /// Detect silence periods in audio (useful for chapter detection)
    pub async fn detect_silence(&self, audio_info: &AudioInfo) -> Result<Vec<(Duration, Duration)>> {
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", audio_info.path.to_str().unwrap(),
                "-af", "silencedetect=n=-30dB:d=2", // Detect 2+ seconds of silence below -30dB
                "-f", "null",
                "-",
            ])
            .output()
            .await?;

        let stderr = String::from_utf8(output.stderr)?;
        let mut silence_periods = Vec::new();
        let mut current_start: Option<Duration> = None;

        for line in stderr.lines() {
            if line.contains("silence_start:") {
                if let Some(start_str) = line.split("silence_start: ").nth(1) {
                    if let Ok(start_seconds) = start_str.trim().parse::<f64>() {
                        current_start = Some(Duration::from_secs_f64(start_seconds));
                    }
                }
            } else if line.contains("silence_end:") && current_start.is_some() {
                if let Some(end_str) = line.split("silence_end: ").nth(1) {
                    if let Some(end_part) = end_str.split_whitespace().next() {
                        if let Ok(end_seconds) = end_part.parse::<f64>() {
                            let start = current_start.take().unwrap();
                            let end = Duration::from_secs_f64(end_seconds);
                            silence_periods.push((start, end));
                        }
                    }
                }
            }
        }

        info!("🔇 Detected {} silence periods", silence_periods.len());
        Ok(silence_periods)
    }

    /// Get audio volume levels over time (for dynamic content detection)
    pub async fn analyze_volume_levels(&self, audio_info: &AudioInfo) -> Result<Vec<f64>> {
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", audio_info.path.to_str().unwrap(),
                "-af", "volumedetect",
                "-f", "null",
                "-",
            ])
            .output()
            .await?;

        let stderr = String::from_utf8(output.stderr)?;
        let mut volume_levels = Vec::new();

        // Parse volume information from output
        for line in stderr.lines() {
            if line.contains("mean_volume:") {
                if let Some(volume_str) = line.split("mean_volume: ").nth(1) {
                    if let Some(db_str) = volume_str.split(" dB").next() {
                        if let Ok(volume_db) = db_str.parse::<f64>() {
                            volume_levels.push(volume_db);
                        }
                    }
                }
            }
        }

        Ok(volume_levels)
    }

    /// Clean up temporary audio files
    pub async fn cleanup_temp_files(&self, temp_dir: &Path) -> Result<()> {
        let mut entries = tokio::fs::read_dir(temp_dir).await?;
        let mut cleaned_files = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "wav" || ext == "tmp") {
                if let Err(e) = tokio::fs::remove_file(&path).await {
                    warn!("Failed to remove temp file {}: {}", path.display(), e);
                } else {
                    cleaned_files += 1;
                }
            }
        }

        if cleaned_files > 0 {
            info!("🧹 Cleaned up {} temporary audio files", cleaned_files);
        }

        Ok(())
    }
}

impl Default for AudioExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_extractor_creation() {
        let extractor = AudioExtractor::new();
        assert_eq!(extractor.target_sample_rate, 16000);
        assert_eq!(extractor.target_format, "wav");
    }
}