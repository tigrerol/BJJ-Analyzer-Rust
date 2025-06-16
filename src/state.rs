use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

/// Represents the processing state for a single video
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProcessingState {
    /// Original video file path
    pub video_path: PathBuf,
    
    /// Video file hash for integrity checking
    pub video_hash: String,
    
    /// Video file modification time
    pub video_modified: u64,
    
    /// Current processing stage
    pub current_stage: ProcessingStage,
    
    /// Completed stages
    pub completed_stages: Vec<ProcessingStage>,
    
    /// Generated file paths
    pub generated_files: GeneratedFiles,
    
    /// Processing metadata
    pub metadata: ProcessingMetadata,
    
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Processing stages in the BJJ video analysis pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessingStage {
    /// Video analysis and validation
    VideoAnalysis,
    
    /// Audio extraction from video
    AudioExtraction,
    
    /// Audio enhancement (optional)
    AudioEnhancement,
    
    /// Speech-to-text transcription
    Transcription,
    
    /// LLM-based transcription correction
    LLMCorrection,
    
    /// SRT subtitle generation
    SubtitleGeneration,
    
    /// Final processing and cleanup
    Completed,
}

/// Generated files during processing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneratedFiles {
    /// Extracted audio file
    pub audio_path: Option<PathBuf>,
    
    /// Enhanced audio file (if enhancement was applied)
    pub enhanced_audio_path: Option<PathBuf>,
    
    /// Raw transcription text file
    pub raw_transcript_path: Option<PathBuf>,
    
    /// LLM-corrected transcript file
    pub corrected_transcript_path: Option<PathBuf>,
    
    /// SRT subtitle file
    pub srt_path: Option<PathBuf>,
    
    /// JSON metadata file
    pub metadata_path: Option<PathBuf>,
}

/// Processing metadata and statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessingMetadata {
    /// Video duration in seconds
    pub duration_seconds: f64,
    
    /// Video resolution
    pub resolution: (u32, u32),
    
    /// Video frame rate
    pub frame_rate: f32,
    
    /// Audio sample rate
    pub audio_sample_rate: Option<u32>,
    
    /// Transcription model used
    pub transcription_model: Option<String>,
    
    /// Number of transcript segments
    pub segment_count: Option<usize>,
    
    /// LLM corrections applied
    pub corrections_applied: Option<usize>,
    
    /// Processing time for each stage (seconds)
    pub stage_times: HashMap<ProcessingStage, f64>,
    
    /// Total processing time
    pub total_processing_time: f64,
}

/// State manager for video processing pipeline
#[derive(Debug, Clone)]
pub struct StateManager {
    /// Base directory for state files
    state_dir: PathBuf,
    
    /// In-memory state cache (thread-safe)
    state_cache: Arc<RwLock<HashMap<String, VideoProcessingState>>>,
}

impl StateManager {
    /// Create a new state manager
    pub async fn new(state_dir: PathBuf) -> Result<Self> {
        // Create state directory if it doesn't exist
        fs::create_dir_all(&state_dir).await?;
        
        let mut manager = Self {
            state_dir,
            state_cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Load existing state files
        manager.load_existing_states().await?;
        
        let cache_len = manager.state_cache.read().await.len();
        info!("📊 State manager initialized with {} cached states", cache_len);
        
        Ok(manager)
    }
    
    /// Load existing state files from disk
    async fn load_existing_states(&mut self) -> Result<()> {
        let mut entries = fs::read_dir(&self.state_dir).await?;
        let mut loaded_count = 0;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_state_file(&path).await {
                    Ok(state) => {
                        let key = self.generate_state_key(&state.video_path);
                        self.state_cache.write().await.insert(key, state);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load state file {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        debug!("📁 Loaded {} state files from disk", loaded_count);
        Ok(())
    }
    
    /// Load a single state file
    async fn load_state_file(&self, path: &Path) -> Result<VideoProcessingState> {
        let content = fs::read_to_string(path).await?;
        let state: VideoProcessingState = serde_json::from_str(&content)?;
        Ok(state)
    }
    
    /// Generate a unique key for a video file
    fn generate_state_key(&self, video_path: &Path) -> String {
        // Use a combination of file name and parent directory for uniqueness
        format!("{}_{}", 
            video_path.parent().unwrap_or(Path::new("")).file_name()
                .unwrap_or_default().to_string_lossy(),
            video_path.file_name().unwrap_or_default().to_string_lossy()
        )
    }
    
    /// Get or create processing state for a video
    pub async fn get_or_create_state(&self, video_path: &Path) -> Result<VideoProcessingState> {
        let key = self.generate_state_key(video_path);
        
        // Check if we have cached state
        {
            let cache = self.state_cache.read().await;
            if let Some(cached_state) = cache.get(&key) {
                // Verify the video file hasn't changed
                if self.is_video_unchanged(video_path, cached_state).await? {
                    debug!("📋 Using cached state for: {}", video_path.display());
                    return Ok(cached_state.clone());
                } else {
                    info!("🔄 Video file changed, invalidating cached state: {}", video_path.display());
                }
            }
        }
        
        // Create new state
        let state = self.create_new_state(video_path).await?;
        self.state_cache.write().await.insert(key, state.clone());
        
        info!("🆕 Created new processing state for: {}", video_path.display());
        Ok(state)
    }
    
    /// Check if video file is unchanged since last processing
    async fn is_video_unchanged(&self, video_path: &Path, state: &VideoProcessingState) -> Result<bool> {
        let metadata = fs::metadata(video_path).await?;
        let current_modified = metadata.modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Check both modification time and file size for basic integrity
        let size_matches = metadata.len() > 0; // Basic sanity check
        let time_matches = current_modified == state.video_modified;
        
        Ok(size_matches && time_matches)
    }
    
    /// Create a new processing state for a video
    async fn create_new_state(&self, video_path: &Path) -> Result<VideoProcessingState> {
        let metadata = fs::metadata(video_path).await?;
        let modified_time = metadata.modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Generate a simple hash based on file size and path
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        video_path.to_string_lossy().to_string().hash(&mut hasher);
        metadata.len().hash(&mut hasher);
        let video_hash = format!("{:x}", hasher.finish());
        
        Ok(VideoProcessingState {
            video_path: video_path.to_path_buf(),
            video_hash,
            video_modified: modified_time,
            current_stage: ProcessingStage::VideoAnalysis,
            completed_stages: Vec::new(),
            generated_files: GeneratedFiles::default(),
            metadata: ProcessingMetadata::default(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        })
    }
    
    /// Update processing state
    pub async fn update_state(&self, state: VideoProcessingState) -> Result<()> {
        let key = self.generate_state_key(&state.video_path);
        
        // Update cache
        self.state_cache.write().await.insert(key.clone(), state.clone());
        
        // Save to disk
        self.save_state_to_disk(&state).await?;
        
        debug!("💾 Updated state for: {}", state.video_path.display());
        Ok(())
    }
    
    /// Save state to disk
    async fn save_state_to_disk(&self, state: &VideoProcessingState) -> Result<()> {
        let filename = format!("{}.json", 
            state.video_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .replace(' ', "_")
                .replace('.', "_"));
        
        let state_file_path = self.state_dir.join(filename);
        let json_content = serde_json::to_string_pretty(state)?;
        fs::write(&state_file_path, json_content).await?;
        
        Ok(())
    }
    
    /// Check if a processing stage can be skipped
    pub fn can_skip_stage(&self, state: &VideoProcessingState, stage: &ProcessingStage) -> bool {
        state.completed_stages.contains(stage)
    }
    
    /// Mark a stage as completed
    pub fn mark_stage_completed(&self, state: &mut VideoProcessingState, stage: ProcessingStage, duration: f64) {
        if !state.completed_stages.contains(&stage) {
            state.completed_stages.push(stage.clone());
        }
        
        state.metadata.stage_times.insert(stage, duration);
        state.current_stage = self.next_stage(&state.current_stage);
        state.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
    
    /// Get the next processing stage
    fn next_stage(&self, current: &ProcessingStage) -> ProcessingStage {
        match current {
            ProcessingStage::VideoAnalysis => ProcessingStage::AudioExtraction,
            ProcessingStage::AudioExtraction => ProcessingStage::AudioEnhancement,
            ProcessingStage::AudioEnhancement => ProcessingStage::Transcription,
            ProcessingStage::Transcription => ProcessingStage::LLMCorrection,
            ProcessingStage::LLMCorrection => ProcessingStage::SubtitleGeneration,
            ProcessingStage::SubtitleGeneration => ProcessingStage::Completed,
            ProcessingStage::Completed => ProcessingStage::Completed,
        }
    }
    
    /// Get processing statistics
    pub async fn get_statistics(&self) -> Result<StateManagerStats> {
        let cache = self.state_cache.read().await;
        let total_states = cache.len();
        let completed = cache.values()
            .filter(|state| state.current_stage == ProcessingStage::Completed)
            .count();
        let in_progress = total_states - completed;
        
        Ok(StateManagerStats {
            total_videos: total_states,
            completed_videos: completed,
            in_progress_videos: in_progress,
            state_cache_size: cache.len(),
        })
    }
    
    /// Clean up old or invalid state files
    pub async fn cleanup_invalid_states(&self) -> Result<usize> {
        let mut removed_count = 0;
        let mut keys_to_remove = Vec::new();
        
        // First pass: identify keys to remove
        {
            let cache = self.state_cache.read().await;
            for (key, state) in cache.iter() {
                // Check if video file still exists
                if !state.video_path.exists() {
                    keys_to_remove.push(key.clone());
                    removed_count += 1;
                }
            }
        }
        
        // Second pass: remove from cache and disk
        {
            let mut cache = self.state_cache.write().await;
            for key in keys_to_remove {
                if let Some(state) = cache.remove(&key) {
                    let _ = self.remove_state_file(&state).await;
                }
            }
        }
        
        if removed_count > 0 {
            info!("🧹 Cleaned up {} invalid state files", removed_count);
        }
        
        Ok(removed_count)
    }
    
    /// Remove state file from disk
    async fn remove_state_file(&self, state: &VideoProcessingState) -> Result<()> {
        let filename = format!("{}.json", 
            state.video_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .replace(' ', "_")
                .replace('.', "_"));
        
        let state_file_path = self.state_dir.join(filename);
        if state_file_path.exists() {
            fs::remove_file(&state_file_path).await?;
        }
        
        Ok(())
    }
}

/// State manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateManagerStats {
    pub total_videos: usize,
    pub completed_videos: usize,
    pub in_progress_videos: usize,
    pub state_cache_size: usize,
}

use std::hash::{Hash, Hasher};
impl Hash for ProcessingStage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}