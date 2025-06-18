//! Artifact detection and processing stage management

use crate::{VideoFile, Result, BJJCoreError};
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// Processing stages for video files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingStage {
    /// No processing has started
    Pending,
    
    /// Audio has been extracted (.wav file exists)
    AudioExtracted,
    
    /// Video has been transcribed (.txt file exists)
    Transcribed,
    
    /// Transcript has been corrected by LLM (_corrected.txt exists)
    LLMCorrected,
    
    /// Subtitles have been generated (.srt file exists)
    SubtitlesGenerated,
    
    /// All processing is complete
    Completed,
}

impl ProcessingStage {
    /// Get progress percentage for this stage
    pub fn progress_percentage(&self) -> u8 {
        match self {
            ProcessingStage::Pending => 0,
            ProcessingStage::AudioExtracted => 20,
            ProcessingStage::Transcribed => 50,
            ProcessingStage::LLMCorrected => 75,
            ProcessingStage::SubtitlesGenerated => 90,
            ProcessingStage::Completed => 100,
        }
    }
    
    /// Get human-readable status string
    pub fn status_string(&self) -> &'static str {
        match self {
            ProcessingStage::Pending => "Pending",
            ProcessingStage::AudioExtracted => "Audio Extracted",
            ProcessingStage::Transcribed => "Transcribed",
            ProcessingStage::LLMCorrected => "LLM Corrected",
            ProcessingStage::SubtitlesGenerated => "Subtitles Generated",
            ProcessingStage::Completed => "Completed",
        }
    }
}

/// Artifact detector for scanning video directories and determining processing status
#[derive(Debug, Clone)]
pub struct ArtifactDetector {
    /// Supported video extensions
    video_extensions: Vec<String>,
}

impl Default for ArtifactDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactDetector {
    /// Create new artifact detector
    pub fn new() -> Self {
        Self {
            video_extensions: vec![
                ".mp4".to_string(),
                ".mkv".to_string(),
                ".avi".to_string(),
                ".mov".to_string(),
                ".wmv".to_string(),
            ],
        }
    }
    
    /// Scan directory for video files and determine their processing status
    pub async fn scan_directory(&self, directory: &Path) -> Result<Vec<VideoFile>> {
        if !directory.exists() {
            return Err(BJJCoreError::Path(format!("Directory does not exist: {}", directory.display())));
        }
        
        let mut video_files = Vec::new();
        
        for entry in WalkDir::new(directory).min_depth(1).max_depth(1) {
            let entry = entry.map_err(|e| BJJCoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            let path = entry.path();
            
            if self.is_video_file(path) {
                match VideoFile::new(path.to_path_buf()).await {
                    Ok(video_file) => video_files.push(video_file),
                    Err(e) => {
                        tracing::warn!("Failed to create VideoFile for {}: {}", path.display(), e);
                        // Continue processing other files
                    }
                }
            }
        }
        
        Ok(video_files)
    }
    
    /// Check if a file is a supported video file
    pub fn is_video_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                let ext_lower = format!(".{}", ext_str.to_lowercase());
                return self.video_extensions.contains(&ext_lower);
            }
        }
        false
    }
    
    /// Scan for unprocessed videos (those that need transcription)
    pub async fn scan_unprocessed(&self, directory: &Path) -> Result<Vec<VideoFile>> {
        let all_videos = self.scan_directory(directory).await?;
        
        Ok(all_videos
            .into_iter()
            .filter(|video| {
                matches!(
                    video.get_processing_stage(),
                    ProcessingStage::Pending | ProcessingStage::AudioExtracted
                )
            })
            .collect())
    }
    
    /// Scan for videos ready for curation (transcription complete)
    pub async fn scan_ready_for_curation(&self, directory: &Path) -> Result<Vec<VideoFile>> {
        let all_videos = self.scan_directory(directory).await?;
        
        Ok(all_videos
            .into_iter()
            .filter(|video| {
                matches!(
                    video.get_processing_stage(),
                    ProcessingStage::Transcribed | ProcessingStage::LLMCorrected | ProcessingStage::SubtitlesGenerated | ProcessingStage::Completed
                )
            })
            .collect())
    }
}