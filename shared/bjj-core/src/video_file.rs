//! Video file representation and artifact management

use crate::{VideoMetadata, ProcessingStage, Result, BJJCoreError};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Represents a video file and its associated artifacts
#[derive(Debug, Clone)]
pub struct VideoFile {
    /// Path to the original video file
    video_path: PathBuf,
    
    /// Video metadata (if available)
    metadata: Option<VideoMetadata>,
    
    /// Cached processing stage
    processing_stage: ProcessingStage,
}

impl VideoFile {
    /// Create new VideoFile instance and detect artifacts
    pub async fn new(video_path: PathBuf) -> Result<Self> {
        if !video_path.exists() {
            return Err(BJJCoreError::Path(format!("Video file does not exist: {}", video_path.display())));
        }
        
        let mut video_file = Self {
            video_path,
            metadata: None,
            processing_stage: ProcessingStage::Pending,
        };
        
        // Detect processing stage based on artifacts
        video_file.processing_stage = video_file.detect_processing_stage().await;
        
        Ok(video_file)
    }
    
    /// Get the video file path
    pub fn video_path(&self) -> &Path {
        &self.video_path
    }
    
    /// Get the filename without extension
    pub fn filename_stem(&self) -> String {
        self.video_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
    
    /// Get the full filename
    pub fn filename(&self) -> String {
        self.video_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
    
    /// Get the parent directory
    pub fn parent_dir(&self) -> &Path {
        self.video_path.parent().unwrap_or(Path::new("."))
    }
    
    /// Get current processing stage
    pub fn get_processing_stage(&self) -> ProcessingStage {
        self.processing_stage
    }
    
    /// Get video metadata (if loaded)
    pub fn metadata(&self) -> Option<&VideoMetadata> {
        self.metadata.as_ref()
    }
    
    /// Set video metadata
    pub fn set_metadata(&mut self, metadata: VideoMetadata) {
        self.metadata = Some(metadata);
    }
    
    /// Check if audio artifact exists
    pub fn has_audio_artifact(&self) -> bool {
        self.audio_artifact_path().exists()
    }
    
    /// Check if transcript artifact exists
    pub fn has_transcript_artifact(&self) -> bool {
        self.transcript_artifact_path().exists()
    }
    
    /// Check if corrected transcript exists
    pub fn has_corrected_transcript(&self) -> bool {
        self.corrected_transcript_path().exists()
    }
    
    /// Check if subtitle artifact exists
    pub fn has_subtitles(&self) -> bool {
        self.subtitle_artifact_path().exists()
    }
    
    /// Get path to audio artifact (.wav)
    pub fn audio_artifact_path(&self) -> PathBuf {
        self.parent_dir().join(format!("{}.wav", self.filename_stem()))
    }
    
    /// Get path to transcript artifact (.txt)
    pub fn transcript_artifact_path(&self) -> PathBuf {
        self.parent_dir().join(format!("{}.txt", self.filename_stem()))
    }
    
    /// Get path to corrected transcript (_corrected.txt)
    pub fn corrected_transcript_path(&self) -> PathBuf {
        self.parent_dir().join(format!("{}_corrected.txt", self.filename_stem()))
    }
    
    /// Get path to subtitle artifact (.srt)
    pub fn subtitle_artifact_path(&self) -> PathBuf {
        self.parent_dir().join(format!("{}.srt", self.filename_stem()))
    }
    
    /// Detect current processing stage based on existing artifacts
    async fn detect_processing_stage(&self) -> ProcessingStage {
        // Check artifacts in reverse order (most complete first)
        // But also check if files have meaningful content (not just empty files)
        
        if self.has_corrected_transcript() && self.has_subtitles() && self.is_file_meaningful(&self.corrected_transcript_path()).await {
            ProcessingStage::Completed
        } else if self.has_corrected_transcript() && self.is_file_meaningful(&self.corrected_transcript_path()).await {
            ProcessingStage::LLMCorrected
        } else if self.has_transcript_artifact() && self.has_subtitles() && self.is_file_meaningful(&self.transcript_artifact_path()).await {
            // If we have both transcript and SRT but no corrected transcript, we're ready for LLM correction
            ProcessingStage::Transcribed
        } else if self.has_transcript_artifact() && self.is_file_meaningful(&self.transcript_artifact_path()).await {
            ProcessingStage::Transcribed
        } else if self.has_audio_artifact() {
            ProcessingStage::AudioExtracted
        } else {
            ProcessingStage::Pending
        }
    }
    
    /// Check if a file exists and has meaningful content (not empty or very small)
    async fn is_file_meaningful(&self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }
        
        if let Ok(metadata) = fs::metadata(path).await {
            // Consider a file meaningful if it has more than 10 bytes
            metadata.len() > 10
        } else {
            false
        }
    }
    
    /// Refresh processing stage by re-detecting artifacts
    pub async fn refresh_processing_stage(&mut self) {
        self.processing_stage = self.detect_processing_stage().await;
    }
    
    /// Get file size of video file
    pub async fn get_file_size(&self) -> Result<u64> {
        let metadata = fs::metadata(&self.video_path).await?;
        Ok(metadata.len())
    }
    
    /// Calculate MD5 hash of video file
    pub async fn calculate_hash(&self) -> Result<String> {
        let content = fs::read(&self.video_path).await?;
        let hash = md5::compute(&content);
        Ok(format!("{:x}", hash))
    }
}