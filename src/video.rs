use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};

// Optional OpenCV support for advanced features (currently not enabled)

/// Video information extracted from file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub path: PathBuf,
    pub filename: String,
    pub duration: Duration,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub format: String,
    pub file_size: u64,
    pub audio_streams: Vec<AudioStreamInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamInfo {
    pub index: usize,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub duration: Duration,
}

/// High-performance video processor using FFmpeg
#[derive(Clone)]
pub struct VideoProcessor {
    /// Supported video extensions
    supported_extensions: Vec<String>,
}

impl VideoProcessor {
    pub fn new() -> Self {
        Self {
            supported_extensions: vec![
                "mp4".to_string(),
                "mkv".to_string(),
                "avi".to_string(),
                "mov".to_string(),
                "webm".to_string(),
                "m4v".to_string(),
            ],
        }
    }

    /// Discover all video files in a directory recursively
    pub async fn discover_videos(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        use std::pin::Pin;
        use std::future::Future;
        
        fn discover_recursive<'a>(
            supported_extensions: &'a [String], 
            dir: &'a Path
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PathBuf>>> + Send + 'a>> {
            Box::pin(async move {
                let mut videos = Vec::new();
                
                let mut entries = tokio::fs::read_dir(dir).await?;
                
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    
                    if path.is_dir() {
                        // Recursively search subdirectories
                        let mut sub_videos = discover_recursive(supported_extensions, &path).await?;
                        videos.append(&mut sub_videos);
                    } else if let Some(extension) = path.extension() {
                        if let Some(ext_str) = extension.to_str() {
                            if supported_extensions.contains(&ext_str.to_lowercase()) {
                                videos.push(path);
                            }
                        }
                    }
                }
                
                Ok(videos)
            })
        }
        
        discover_recursive(&self.supported_extensions, dir).await
    }

    /// Extract video information using FFmpeg command line
    pub async fn get_video_info(&self, video_path: &Path) -> Result<VideoInfo> {
        // Use ffprobe command line tool for video analysis
        let output = tokio::process::Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                video_path.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("ffprobe failed for {}", video_path.display()));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let ffprobe_data: serde_json::Value = serde_json::from_str(&json_str)?;

        // Extract video information
        let format = &ffprobe_data["format"];
        let streams = ffprobe_data["streams"].as_array().unwrap();

        // Find video stream
        let video_stream = streams
            .iter()
            .find(|s| s["codec_type"] == "video")
            .ok_or_else(|| anyhow!("No video stream found"))?;

        // Find audio streams
        let audio_streams: Vec<AudioStreamInfo> = streams
            .iter()
            .filter(|s| s["codec_type"] == "audio")
            .enumerate()
            .map(|(index, stream)| {
                AudioStreamInfo {
                    index,
                    codec: stream["codec_name"].as_str().unwrap_or("unknown").to_string(),
                    sample_rate: stream["sample_rate"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(44100),
                    channels: stream["channels"].as_u64().unwrap_or(2) as u32,
                    duration: Duration::from_secs_f64(
                        stream["duration"]
                            .as_str()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0.0)
                    ),
                }
            })
            .collect();

        let duration_seconds: f64 = format["duration"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let file_size = tokio::fs::metadata(video_path).await?.len();

        let video_info = VideoInfo {
            path: video_path.to_path_buf(),
            filename: video_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            duration: Duration::from_secs_f64(duration_seconds),
            width: video_stream["width"].as_u64().unwrap_or(0) as u32,
            height: video_stream["height"].as_u64().unwrap_or(0) as u32,
            fps: video_stream["r_frame_rate"]
                .as_str()
                .and_then(|s| {
                    let parts: Vec<&str> = s.split('/').collect();
                    if parts.len() == 2 {
                        let num: f64 = parts[0].parse().ok()?;
                        let den: f64 = parts[1].parse().ok()?;
                        Some(num / den)
                    } else {
                        s.parse().ok()
                    }
                })
                .unwrap_or(0.0),
            format: format["format_name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            file_size,
            audio_streams,
        };

        info!("📹 Analyzed video: {} ({}x{}, {:.1}fps, {:.1}s)", 
              video_info.filename,
              video_info.width,
              video_info.height,
              video_info.fps,
              video_info.duration.as_secs_f64());

        Ok(video_info)
    }

    /// Extract audio from video for transcription
    pub async fn extract_audio(&self, video_info: &VideoInfo, output_path: &Path) -> Result<PathBuf> {
        let audio_path = output_path.with_extension("wav");

        info!("🎵 Extracting audio from {}", video_info.filename);

        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", video_info.path.to_str().unwrap(),
                "-vn", // No video
                "-acodec", "pcm_s16le", // 16-bit PCM
                "-ar", "16000", // 16kHz sample rate (optimal for Whisper)
                "-ac", "1", // Mono
                "-y", // Overwrite output file
                audio_path.to_str().unwrap(),
            ])
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Audio extraction failed for {}", video_info.filename));
        }

        info!("✅ Audio extracted: {}", audio_path.display());
        Ok(audio_path)
    }

    /// Validate video file integrity
    pub async fn validate_video(&self, video_path: &Path) -> Result<bool> {
        let output = tokio::process::Command::new("ffprobe")
            .args([
                "-v", "error",
                "-select_streams", "v:0",
                "-count_frames",
                "-show_entries", "stream=nb_frames",
                "-of", "csv=p=0",
                video_path.to_str().unwrap(),
            ])
            .output()
            .await?;

        Ok(output.status.success())
    }

    /// Get video thumbnail at specific timestamp
    pub async fn extract_thumbnail(
        &self, 
        video_info: &VideoInfo, 
        timestamp: Duration,
        output_path: &Path,
    ) -> Result<PathBuf> {
        let thumbnail_path = output_path.with_extension("jpg");

        let timestamp_str = format!("{:.2}", timestamp.as_secs_f64());

        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-i", video_info.path.to_str().unwrap(),
                "-ss", &timestamp_str,
                "-vframes", "1",
                "-q:v", "2", // High quality
                "-y",
                thumbnail_path.to_str().unwrap(),
            ])
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Thumbnail extraction failed"));
        }

        Ok(thumbnail_path)
    }

    /// Detect scene changes in video (for chapter detection)
    pub async fn detect_scene_changes(&self, video_info: &VideoInfo) -> Result<Vec<Duration>> {
        let output = tokio::process::Command::new("ffprobe")
            .args([
                "-f", "lavfi",
                "-i", &format!("movie={}:f=lavfi,select=gt(scene\\,0.4)", 
                             video_info.path.to_str().unwrap()),
                "-show_entries", "frame=pkt_pts_time",
                "-of", "csv=p=0",
            ])
            .output()
            .await?;

        if !output.status.success() {
            warn!("Scene detection failed for {}", video_info.filename);
            return Ok(Vec::new());
        }

        let output_str = String::from_utf8(output.stdout)?;
        let mut scene_times = Vec::new();

        for line in output_str.lines() {
            if let Ok(time) = line.trim().parse::<f64>() {
                scene_times.push(Duration::from_secs_f64(time));
            }
        }

        info!("🎬 Detected {} scene changes in {}", 
              scene_times.len(), video_info.filename);

        Ok(scene_times)
    }
}

impl Default for VideoProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_video_discovery() {
        let processor = VideoProcessor::new();
        
        // This would need a test video directory
        if let Ok(test_dir) = env::var("TEST_VIDEO_DIR") {
            let videos = processor.discover_videos(Path::new(&test_dir)).await.unwrap();
            assert!(!videos.is_empty());
        }
    }
}