use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use tracing::{info, warn, error, debug};

use crate::config::Config;
use crate::video::{VideoProcessor, VideoInfo};
use crate::audio::{AudioExtractor, AudioInfo};
use crate::bjj::BJJDictionary;
use crate::transcription::{WhisperTranscriber, TranscriptionResult};
use crate::state::{StateManager, ProcessingStage as StateProcessingStage};
use crate::chapters::{ChapterDetector, ChapterInfo, ChapterDetectionConfig};

/// Processing result for a single video
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProcessingResult {
    pub video_info: VideoInfo,
    pub audio_info: Option<AudioInfo>,
    pub transcription_result: Option<TranscriptionResult>,
    pub srt_path: Option<PathBuf>,
    pub text_path: Option<PathBuf>,
    pub chapters: Vec<ChapterInfo>,
    pub processing_time: Duration,
    pub status: ProcessingStatus,
    pub error_message: Option<String>,
    pub stages_completed: Vec<ProcessingStage>,
}

/// Overall batch processing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub total_time: Duration,
    pub results: Vec<VideoProcessingResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStage {
    Discovery,
    VideoAnalysis,
    AudioExtraction,
    AudioEnhancement,
    TranscriptionPrep,
    Transcription,
    LLMCorrection,
    ChapterDetection,
    TranscriptionPost,
    Completed,
}

/// High-performance batch processor using async/await and worker pools
pub struct BatchProcessor {
    config: Config,
    video_processor: VideoProcessor,
    audio_extractor: AudioExtractor,
    whisper_transcriber: WhisperTranscriber,
    bjj_dictionary: BJJDictionary,
    chapter_detector: ChapterDetector,
    worker_semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl BatchProcessor {
    pub async fn new(config: Config, max_workers: usize) -> Result<Self> {
        info!("🔧 Initializing BatchProcessor with {} workers", max_workers);

        // Load BJJ dictionary
        let mut bjj_dictionary = if let Some(ref bjj_file) = config.transcription.bjj_terms_file {
            if bjj_file.exists() {
                BJJDictionary::from_file(bjj_file).await?
            } else {
                warn!("BJJ terms file not found: {}, using default dictionary", bjj_file.display());
                BJJDictionary::new()
            }
        } else {
            BJJDictionary::new()
        };
        
        // Load external prompt template if configured
        let whisper_prompt_path = config.llm.prompts.prompt_dir.join(&config.llm.prompts.whisper_transcription_file);
        if whisper_prompt_path.exists() {
            bjj_dictionary.load_prompt_from_file(&whisper_prompt_path).await?;
        } else {
            info!("🔔 Whisper prompt file not found: {}, using default template", whisper_prompt_path.display());
        }

        // Initialize Whisper transcriber
        let whisper_transcriber = WhisperTranscriber::new(
            config.transcription.clone(),
            bjj_dictionary.clone(),
        );

        // Initialize chapter detector
        let chapter_config = ChapterDetectionConfig {
            enable_detection: config.chapters.enable_detection,
            chapters_dir: config.chapters.chapters_dir.clone(),
            request_timeout_seconds: config.chapters.request_timeout_seconds,
            max_chapters: config.chapters.max_chapters,
        };
        let chapter_detector = ChapterDetector::with_config(chapter_config).await?;

        info!("📚 BJJ Dictionary loaded: {} terms, {} corrections", 
              bjj_dictionary.get_stats().total_terms,
              bjj_dictionary.get_stats().total_corrections);

        // State manager will be initialized per video directory

        Ok(Self {
            config,
            video_processor: VideoProcessor::new(),
            audio_extractor: AudioExtractor::new(),
            whisper_transcriber,
            bjj_dictionary,
            chapter_detector,
            worker_semaphore: Arc::new(Semaphore::new(max_workers)),
            max_concurrent: max_workers,
        })
    }

    /// Process all videos in a directory
    pub async fn process_directory(
        &self,
        input_dir: PathBuf,
        output_dir: PathBuf,
    ) -> Result<ProcessingResult> {
        let start_time = Instant::now();
        
        info!("🚀 Starting batch processing...");
        info!("📁 Input: {}", input_dir.display());
        info!("📂 Output: {}", output_dir.display());

        // Initialize state manager tied to input video directory
        let state_dir = input_dir.join(".bjj_analyzer_state");
        let state_manager = StateManager::new(state_dir.clone()).await?;
        info!("💾 State directory: {}", state_dir.display());

        // Create output directory structure
        tokio::fs::create_dir_all(&output_dir).await?;
        let audio_dir = output_dir.join("audio");
        tokio::fs::create_dir_all(&audio_dir).await?;

        // Discover all videos
        info!("🔍 Discovering videos...");
        let video_paths = self.video_processor.discover_videos(&input_dir).await?;
        
        if video_paths.is_empty() {
            warn!("No videos found in {}", input_dir.display());
            return Ok(ProcessingResult {
                total: 0,
                successful: 0,
                failed: 0,
                total_time: start_time.elapsed(),
                results: Vec::new(),
            });
        }

        info!("📹 Found {} videos to process", video_paths.len());

        // Process videos in parallel with concurrency control
        let results = self.process_videos_parallel(video_paths, &audio_dir, state_manager).await?;

        let total_time = start_time.elapsed();
        let successful = results.iter().filter(|r| matches!(r.status, ProcessingStatus::Completed)).count();
        let failed = results.len() - successful;

        // Save results to JSON
        let results_path = output_dir.join("processing_results.json");
        let processing_result = ProcessingResult {
            total: results.len(),
            successful,
            failed,
            total_time,
            results: results.clone(),
        };

        let json_data = serde_json::to_string_pretty(&processing_result)?;
        tokio::fs::write(&results_path, json_data).await?;

        info!("💾 Results saved to: {}", results_path.display());

        Ok(processing_result)
    }

    /// Process multiple videos in parallel with controlled concurrency
    async fn process_videos_parallel(
        &self,
        video_paths: Vec<PathBuf>,
        output_dir: &Path,
        state_manager: StateManager,
    ) -> Result<Vec<VideoProcessingResult>> {
        let (tx, mut rx) = mpsc::channel(self.max_concurrent);
        let total_videos = video_paths.len();

        // Spawn tasks for each video
        for (index, video_path) in video_paths.into_iter().enumerate() {
            let processor = self.clone_processor_state(state_manager.clone());
            let output_dir = output_dir.to_path_buf();
            let tx = tx.clone();
            let semaphore = Arc::clone(&self.worker_semaphore);

            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                info!("📹 Processing video {}/{}: {}", 
                      index + 1, total_videos, video_path.display());

                let result = processor.process_single_video(&video_path, &output_dir).await;
                
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send result: {}", e);
                }
            });
        }

        // Drop the original sender to close the channel when all tasks complete
        drop(tx);

        // Collect results
        let mut results = Vec::new();
        while let Some(result) = rx.recv().await {
            match result {
                Ok(video_result) => {
                    match video_result.status {
                        ProcessingStatus::Completed => {
                            info!("✅ Completed: {} in {:.2}s", 
                                  video_result.video_info.filename,
                                  video_result.processing_time.as_secs_f64());
                        }
                        ProcessingStatus::Failed => {
                            warn!("❌ Failed: {} - {}", 
                                  video_result.video_info.filename,
                                  video_result.error_message.as_deref().unwrap_or("Unknown error"));
                        }
                        _ => {}
                    }
                    results.push(video_result);
                }
                Err(e) => {
                    error!("Processing error: {}", e);
                }
            }
        }

        Ok(results)
    }

    /// Create a lightweight clone of processor state for parallel processing
    fn clone_processor_state(&self, state_manager: StateManager) -> ProcessorState {
        ProcessorState {
            config: self.config.clone(),
            video_processor: VideoProcessor::new(),
            audio_extractor: AudioExtractor::new(),
            whisper_transcriber: self.whisper_transcriber.clone(),
            bjj_dictionary: self.bjj_dictionary.clone(),
            chapter_detector: self.chapter_detector.clone(),
            state_manager,
        }
    }

    /// Reset chapter detection state for all videos in a directory
    pub async fn reset_chapter_detection_state(&self, input_dir: &PathBuf) -> Result<usize> {
        let state_dir = input_dir.join(".bjj_analyzer_state");
        let state_manager = StateManager::new(state_dir).await?;
        
        // Discover videos in the directory
        let videos = self.video_processor.discover_videos(input_dir).await?;
        let mut reset_count = 0;
        
        for video_path in videos {
            if let Ok(mut state) = state_manager.get_or_create_state(&video_path).await {
                // Check if ChapterDetection was completed
                let had_chapter_detection = state.completed_stages.contains(&StateProcessingStage::ChapterDetection);
                
                if had_chapter_detection {
                    // Remove ChapterDetection from completed stages
                    state.completed_stages.retain(|stage| stage != &StateProcessingStage::ChapterDetection);
                    state.metadata.chapters_detected = None;
                    
                    // Update the state
                    state_manager.update_state(state).await?;
                    reset_count += 1;
                    
                    info!("🔄 Reset chapter detection state for: {}", video_path.file_name().unwrap_or_default().to_string_lossy());
                }
            }
        }
        
        Ok(reset_count)
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> ProcessingStats {
        ProcessingStats {
            max_workers: self.max_concurrent,
            available_permits: self.worker_semaphore.available_permits(),
        }
    }
}

/// Lightweight processor state for parallel tasks
#[derive(Clone)]
struct ProcessorState {
    config: Config,
    video_processor: VideoProcessor,
    audio_extractor: AudioExtractor,
    whisper_transcriber: WhisperTranscriber,
    bjj_dictionary: BJJDictionary,
    chapter_detector: ChapterDetector,
    state_manager: StateManager,
}

impl ProcessorState {
    async fn process_single_video(
        &self,
        video_path: &Path,
        output_dir: &Path,
    ) -> Result<VideoProcessingResult> {
        let start_time = Instant::now();
        
        // Get or create processing state
        let mut state = self.state_manager.get_or_create_state(video_path).await?;
        info!("📋 Processing state for {}: {:?} (completed: {:?})", 
              video_path.display(), state.current_stage, state.completed_stages);
        
        let mut result = VideoProcessingResult {
            video_info: VideoInfo {
                path: video_path.to_path_buf(),
                filename: video_path.file_name().unwrap().to_string_lossy().to_string(),
                duration: Duration::from_secs(0),
                width: 0,
                height: 0,
                fps: 0.0,
                format: String::new(),
                file_size: 0,
                audio_streams: Vec::new(),
            },
            audio_info: None,
            transcription_result: None,
            srt_path: None,
            text_path: None,
            chapters: Vec::new(),
            processing_time: Duration::from_secs(0),
            status: ProcessingStatus::InProgress,
            error_message: None,
            stages_completed: state.completed_stages.iter().map(|s| match s {
                StateProcessingStage::VideoAnalysis => ProcessingStage::VideoAnalysis,
                StateProcessingStage::AudioExtraction => ProcessingStage::AudioExtraction,
                StateProcessingStage::AudioEnhancement => ProcessingStage::AudioEnhancement,
                StateProcessingStage::Transcription => ProcessingStage::Transcription,
                StateProcessingStage::LLMCorrection => ProcessingStage::LLMCorrection,
                StateProcessingStage::ChapterDetection => ProcessingStage::ChapterDetection,
                StateProcessingStage::SubtitleGeneration => ProcessingStage::TranscriptionPost,
                StateProcessingStage::Completed => ProcessingStage::Completed,
            }).collect(),
        };

        // Stage 1: Video Analysis
        if !self.state_manager.can_skip_stage(&state, &StateProcessingStage::VideoAnalysis) {
            info!("📊 Analyzing video: {}", video_path.display());
            let stage_start = Instant::now();
            
            match self.video_processor.get_video_info(video_path).await {
                Ok(video_info) => {
                    result.video_info = video_info.clone();
                    
                    // Update state with metadata
                    state.metadata.duration_seconds = video_info.duration.as_secs_f64();
                    state.metadata.resolution = (video_info.width, video_info.height);
                    state.metadata.frame_rate = video_info.fps as f32;
                    
                    self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::VideoAnalysis, stage_start.elapsed().as_secs_f64());
                    self.state_manager.update_state(state.clone()).await?;
                    
                    info!("✅ Video analysis completed: {}x{} at {:.1}fps", video_info.width, video_info.height, video_info.fps);
                }
                Err(e) => {
                    result.status = ProcessingStatus::Failed;
                    result.error_message = Some(format!("Video analysis failed: {}", e));
                    result.processing_time = start_time.elapsed();
                    return Ok(result);
                }
            }
        } else {
            info!("⏭️  Skipping video analysis (already completed)");
            // Load metadata from state
            result.video_info.duration = Duration::from_secs_f64(state.metadata.duration_seconds);
            result.video_info.width = state.metadata.resolution.0;
            result.video_info.height = state.metadata.resolution.1;
            result.video_info.fps = state.metadata.frame_rate as f64;
        }

        // Stage 2: Audio Extraction
        if !self.state_manager.can_skip_stage(&state, &StateProcessingStage::AudioExtraction) {
            info!("🎵 Extracting audio from: {}", result.video_info.filename);
            let stage_start = Instant::now();
            
            match self.audio_extractor.extract_for_transcription(video_path, output_dir).await {
                Ok(audio_info) => {
                    result.audio_info = Some(audio_info.clone());
                    
                    // Update state with audio info
                    state.generated_files.audio_path = Some(audio_info.path.clone());
                    state.metadata.audio_sample_rate = Some(audio_info.sample_rate);
                    
                    self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::AudioExtraction, stage_start.elapsed().as_secs_f64());
                    self.state_manager.update_state(state.clone()).await?;
                    
                    info!("✅ Audio extracted: {:.1}s duration, {}Hz", audio_info.duration.as_secs_f64(), audio_info.sample_rate);
                }
                Err(e) => {
                    warn!("⚠️ Audio extraction failed for {}: {}", result.video_info.filename, e);
                    self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::AudioExtraction, stage_start.elapsed().as_secs_f64());
                    self.state_manager.update_state(state.clone()).await?;
                }
            }
        } else {
            info!("⏭️  Skipping audio extraction (already completed)");
            // Load audio info from state if available
            if let Some(audio_path) = &state.generated_files.audio_path {
                if audio_path.exists() {
                    // Create a basic AudioInfo from the path - we'll need to add this method
                    if let Ok(metadata) = tokio::fs::metadata(audio_path).await {
                        use crate::audio::AudioInfo;
                        result.audio_info = Some(AudioInfo {
                            path: audio_path.to_path_buf(),
                            duration: Duration::from_secs_f64(state.metadata.duration_seconds),
                            sample_rate: state.metadata.audio_sample_rate.unwrap_or(44100),
                            channels: 1, // Default assumption
                            file_size: metadata.len(),
                            format: "wav".to_string(),
                            bitrate: None, // Unknown from cached state
                        });
                    }
                }
            }
        }

        // Stage 3: Transcription
        if let Some(ref audio_info) = result.audio_info {
            if !self.state_manager.can_skip_stage(&state, &StateProcessingStage::Transcription) {
                info!("🎤 Starting Whisper transcription: {} ({:?} duration)", 
                      audio_info.path.display(), audio_info.duration);
                
                let stage_start = Instant::now();
                match self.whisper_transcriber.transcribe_audio(audio_info, output_dir).await {
                    Ok(transcription_result) => {
                        // Store paths in state
                        state.generated_files.raw_transcript_path = transcription_result.text_path.clone();
                        state.generated_files.srt_path = transcription_result.srt_path.clone();
                        state.metadata.transcription_model = Some(transcription_result.model_used.clone());
                        state.metadata.segment_count = Some(transcription_result.segments.len());
                        
                        result.srt_path = transcription_result.srt_path.clone();
                        result.text_path = transcription_result.text_path.clone();
                        result.transcription_result = Some(transcription_result);
                        
                        self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::Transcription, stage_start.elapsed().as_secs_f64());
                        self.state_manager.update_state(state.clone()).await?;
                        
                        info!("✅ Transcription completed: {} characters", 
                              result.transcription_result.as_ref().unwrap().text.len());
                    }
                    Err(e) => {
                        error!("❌ Transcription failed: {}", e);
                        self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::Transcription, stage_start.elapsed().as_secs_f64());
                        self.state_manager.update_state(state.clone()).await?;
                    }
                }
            } else {
                info!("⏭️  Skipping transcription (already completed)");
                // Load transcription from state files
                if let Some(text_path) = &state.generated_files.raw_transcript_path {
                    if text_path.exists() {
                        match tokio::fs::read_to_string(text_path).await {
                            Ok(text) => {
                                use crate::transcription::TranscriptionResult;
                                result.transcription_result = Some(TranscriptionResult {
                                    text,
                                    language: Some("en".to_string()),
                                    segments: Vec::new(), // Will be loaded if needed
                                    srt_path: state.generated_files.srt_path.clone(),
                                    text_path: Some(text_path.to_path_buf()),
                                    processing_time: Duration::from_secs(0),
                                    model_used: state.metadata.transcription_model.clone().unwrap_or_else(|| "unknown".to_string()),
                                    bjj_prompt: None,
                                });
                                result.text_path = Some(text_path.to_path_buf());
                                result.srt_path = state.generated_files.srt_path.clone();
                            }
                            Err(e) => warn!("Failed to load cached transcription: {}", e),
                        }
                    }
                }
            }
        }

        // Stage 4: LLM Correction (if enabled)
        if let Some(ref mut transcription_result) = result.transcription_result {
            if self.config.llm.enable_correction && !self.state_manager.can_skip_stage(&state, &StateProcessingStage::LLMCorrection) {
                info!("🤖 Starting LLM transcription correction: {}", result.video_info.filename);
                let stage_start = Instant::now();
                
                let llm_config = crate::llm::LLMConfig {
                    provider: match self.config.llm.provider {
                        crate::config::LLMProvider::LMStudio => crate::llm::LLMProvider::LMStudio,
                        crate::config::LLMProvider::Gemini => crate::llm::LLMProvider::Gemini,
                        crate::config::LLMProvider::OpenAI => crate::llm::LLMProvider::OpenAI,
                    },
                    endpoint: self.config.llm.endpoint.clone(),
                    api_key: self.config.llm.api_key.clone(),
                    model: self.config.llm.model.clone(),
                    max_tokens: self.config.llm.max_tokens,
                    temperature: self.config.llm.temperature,
                    timeout_seconds: self.config.llm.timeout_seconds,
                };

                use crate::llm::correction::get_transcription_corrections;
                match get_transcription_corrections(
                    &transcription_result.text,
                    llm_config,
                    self.config.llm.prompt_file.as_deref(),
                ).await {
                    Ok(corrections) => {
                        state.metadata.corrections_applied = Some(corrections.replacements.len());
                        
                        if !corrections.replacements.is_empty() {
                            info!("✨ Applying {} LLM corrections to text and SRT files", corrections.replacements.len());
                            
                            use crate::llm::correction::apply_text_replacements;
                            let corrected_text = apply_text_replacements(&transcription_result.text, &corrections.replacements);
                            transcription_result.text = corrected_text;
                            
                            let video_stem = video_path.file_stem().unwrap().to_string_lossy();
                            
                            // Handle .txt file correction
                            if let Some(ref original_text_path) = transcription_result.text_path {
                                let old_text_path = output_dir.join(format!("{}_old.txt", video_stem));
                                let new_text_path = output_dir.join(format!("{}.txt", video_stem));
                                
                                // Rename original to _old.txt
                                if let Err(e) = tokio::fs::rename(original_text_path, &old_text_path).await {
                                    warn!("Failed to rename original text file: {}", e);
                                } else {
                                    // Write corrected text as the main file
                                    if let Err(e) = tokio::fs::write(&new_text_path, &transcription_result.text).await {
                                        warn!("Failed to write corrected text file: {}", e);
                                    } else {
                                        transcription_result.text_path = Some(new_text_path.clone());
                                        state.generated_files.raw_transcript_path = Some(new_text_path.clone());
                                        state.generated_files.corrected_transcript_path = Some(old_text_path);
                                        info!("📝 Corrected text file saved: {}", new_text_path.display());
                                    }
                                }
                            }
                            
                            // Handle .srt file correction
                            if let Some(ref original_srt_path) = transcription_result.srt_path {
                                let old_srt_path = output_dir.join(format!("{}_old.srt", video_stem));
                                let new_srt_path = output_dir.join(format!("{}.srt", video_stem));
                                
                                // Read original SRT content
                                match tokio::fs::read_to_string(original_srt_path).await {
                                    Ok(srt_content) => {
                                        // Apply corrections to SRT content
                                        let corrected_srt = apply_text_replacements(&srt_content, &corrections.replacements);
                                        
                                        // Rename original to _old.srt
                                        if let Err(e) = tokio::fs::rename(original_srt_path, &old_srt_path).await {
                                            warn!("Failed to rename original SRT file: {}", e);
                                        } else {
                                            // Write corrected SRT as the main file
                                            if let Err(e) = tokio::fs::write(&new_srt_path, corrected_srt).await {
                                                warn!("Failed to write corrected SRT file: {}", e);
                                            } else {
                                                transcription_result.srt_path = Some(new_srt_path.clone());
                                                state.generated_files.srt_path = Some(new_srt_path.clone());
                                                info!("🎬 Corrected SRT file saved: {}", new_srt_path.display());
                                            }
                                        }
                                    }
                                    Err(e) => warn!("Failed to read original SRT file for correction: {}", e),
                                }
                            }
                        } else {
                            info!("📝 No corrections needed");
                        }
                        
                        self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::LLMCorrection, stage_start.elapsed().as_secs_f64());
                        self.state_manager.update_state(state.clone()).await?;
                    }
                    Err(e) => {
                        warn!("⚠️ LLM correction failed: {}", e);
                        self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::LLMCorrection, stage_start.elapsed().as_secs_f64());
                        self.state_manager.update_state(state.clone()).await?;
                    }
                }
            } else if self.config.llm.enable_correction {
                info!("⏭️  Skipping LLM correction (already completed)");
                // Load corrected text if available
                if let Some(corrected_path) = &state.generated_files.corrected_transcript_path {
                    if corrected_path.exists() {
                        match tokio::fs::read_to_string(corrected_path).await {
                            Ok(corrected_text) => {
                                transcription_result.text = corrected_text;
                                info!("📝 Loaded corrected transcript");
                            }
                            Err(e) => warn!("Failed to load corrected transcript: {}", e),
                        }
                    }
                }
            } else {
                info!("🔕 LLM correction disabled");
                self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::LLMCorrection, 0.0);
                self.state_manager.update_state(state.clone()).await?;
            }
        }

        // Stage 5: Chapter Detection (always run - uses internal file-based caching)
        if true {
            info!("🔍 Starting chapter detection for: {}", result.video_info.filename);
            let stage_start = Instant::now();
            
            match self.chapter_detector.detect_chapters(video_path).await {
                Ok(chapters) => {
                    let chapter_count = chapters.len();
                    if chapter_count > 0 {
                        info!("📚 Found {} chapters for {}", chapter_count, result.video_info.filename);
                        
                        // Validate chapters against video duration
                        let validated_chapters = self.chapter_detector.validate_chapters(
                            &chapters, 
                            result.video_info.duration.as_secs_f64()
                        );
                        
                        result.chapters = validated_chapters.clone();
                        state.metadata.chapters_detected = Some(validated_chapters.len());
                        
                        // Log first few chapters for debugging
                        for (i, chapter) in validated_chapters.iter().take(3).enumerate() {
                            debug!("Chapter {}: {} at {}s", i + 1, chapter.title, chapter.timestamp);
                        }
                        if validated_chapters.len() > 3 {
                            debug!("... and {} more chapters", validated_chapters.len() - 3);
                        }
                    } else {
                        info!("📭 No chapters found for {}", result.video_info.filename);
                        state.metadata.chapters_detected = Some(0);
                    }
                    
                    self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::ChapterDetection, stage_start.elapsed().as_secs_f64());
                    self.state_manager.update_state(state.clone()).await?;
                }
                Err(e) => {
                    warn!("⚠️ Chapter detection failed for {}: {}", result.video_info.filename, e);
                    state.metadata.chapters_detected = Some(0);
                    self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::ChapterDetection, stage_start.elapsed().as_secs_f64());
                    self.state_manager.update_state(state.clone()).await?;
                }
            }
        } else {
            info!("⏭️  Skipping chapter detection (already completed)");
            // Load chapters from state if available
            if let Some(chapter_count) = state.metadata.chapters_detected {
                info!("📚 Loaded {} chapters from cache", chapter_count);
                // In a full implementation, we would load actual chapter data from cache
            }
        }

        // Mark as completed
        self.state_manager.mark_stage_completed(&mut state, StateProcessingStage::Completed, 0.0);
        state.metadata.total_processing_time = start_time.elapsed().as_secs_f64();
        self.state_manager.update_state(state.clone()).await?;
        
        result.status = ProcessingStatus::Completed;
        result.processing_time = start_time.elapsed();
        result.stages_completed = state.completed_stages.iter().map(|s| match s {
            StateProcessingStage::VideoAnalysis => ProcessingStage::VideoAnalysis,
            StateProcessingStage::AudioExtraction => ProcessingStage::AudioExtraction,
            StateProcessingStage::AudioEnhancement => ProcessingStage::AudioEnhancement,
            StateProcessingStage::Transcription => ProcessingStage::Transcription,
            StateProcessingStage::LLMCorrection => ProcessingStage::LLMCorrection,
            StateProcessingStage::ChapterDetection => ProcessingStage::ChapterDetection,
            StateProcessingStage::SubtitleGeneration => ProcessingStage::TranscriptionPost,
            StateProcessingStage::Completed => ProcessingStage::Completed,
        }).collect();

        info!("🎉 Video processing completed: {} (total: {:.1}s, skipped: {} stages)", 
              result.video_info.filename, 
              result.processing_time.as_secs_f64(),
              state.completed_stages.len() - 1); // -1 for the Completed stage
        
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct ProcessingStats {
    pub max_workers: usize,
    pub available_permits: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_batch_processor_creation() {
        let config = Config::default();
        let processor = BatchProcessor::new(config, 4).await.unwrap();
        
        let stats = processor.get_stats();
        assert_eq!(stats.max_workers, 4);
        assert_eq!(stats.available_permits, 4);
    }

    #[tokio::test]
    async fn test_empty_directory_processing() {
        let config = Config::default();
        let processor = BatchProcessor::new(config, 2).await.unwrap();
        
        let temp_dir = TempDir::new().unwrap();
        let input_dir = temp_dir.path().to_path_buf();
        let output_dir = temp_dir.path().join("output");
        
        let result = processor.process_directory(input_dir, output_dir).await.unwrap();
        
        assert_eq!(result.total, 0);
        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
    }
}