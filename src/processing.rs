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

/// Processing result for a single video
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProcessingResult {
    pub video_info: VideoInfo,
    pub audio_info: Option<AudioInfo>,
    pub transcription_result: Option<TranscriptionResult>,
    pub srt_path: Option<PathBuf>,
    pub text_path: Option<PathBuf>,
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
    worker_semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl BatchProcessor {
    pub async fn new(config: Config, max_workers: usize) -> Result<Self> {
        info!("🔧 Initializing BatchProcessor with {} workers", max_workers);

        // Load BJJ dictionary
        let bjj_dictionary = if let Some(ref bjj_file) = config.transcription.bjj_terms_file {
            if bjj_file.exists() {
                BJJDictionary::from_file(bjj_file).await?
            } else {
                warn!("BJJ terms file not found: {}, using default dictionary", bjj_file.display());
                BJJDictionary::new()
            }
        } else {
            BJJDictionary::new()
        };

        // Initialize Whisper transcriber
        let whisper_transcriber = WhisperTranscriber::new(
            config.transcription.clone(),
            bjj_dictionary.clone(),
        );

        info!("📚 BJJ Dictionary loaded: {} terms, {} corrections", 
              bjj_dictionary.get_stats().total_terms,
              bjj_dictionary.get_stats().total_corrections);

        Ok(Self {
            config,
            video_processor: VideoProcessor::new(),
            audio_extractor: AudioExtractor::new(),
            whisper_transcriber,
            bjj_dictionary,
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
        let results = self.process_videos_parallel(video_paths, &audio_dir).await?;

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
    ) -> Result<Vec<VideoProcessingResult>> {
        let (tx, mut rx) = mpsc::channel(self.max_concurrent);
        let total_videos = video_paths.len();

        // Spawn tasks for each video
        for (index, video_path) in video_paths.into_iter().enumerate() {
            let processor = self.clone_processor_state();
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

    /// Process a single video through the complete pipeline
    async fn process_single_video(
        &self,
        video_path: &Path,
        output_dir: &Path,
    ) -> Result<VideoProcessingResult> {
        let start_time = Instant::now();
        let mut stages_completed = Vec::new();
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
            processing_time: Duration::from_secs(0),
            status: ProcessingStatus::InProgress,
            error_message: None,
            stages_completed: Vec::new(),
        };

        // Stage 1: Video Analysis
        debug!("📊 Analyzing video: {}", video_path.display());
        match self.video_processor.get_video_info(video_path).await {
            Ok(video_info) => {
                result.video_info = video_info;
                stages_completed.push(ProcessingStage::VideoAnalysis);
            }
            Err(e) => {
                result.status = ProcessingStatus::Failed;
                result.error_message = Some(format!("Video analysis failed: {}", e));
                result.processing_time = start_time.elapsed();
                result.stages_completed = stages_completed;
                return Ok(result);
            }
        }

        // Stage 2: Audio Extraction
        debug!("🎵 Extracting audio from: {}", result.video_info.filename);
        match self.audio_extractor.extract_for_transcription(video_path, output_dir).await {
            Ok(audio_info) => {
                result.audio_info = Some(audio_info);
                stages_completed.push(ProcessingStage::AudioExtraction);
            }
            Err(e) => {
                warn!("Audio extraction failed for {}: {}", result.video_info.filename, e);
                // Continue processing even if audio extraction fails
                stages_completed.push(ProcessingStage::AudioExtraction);
            }
        }

        // Stage 3: Audio Enhancement (if audio was extracted successfully)
        if let Some(ref audio_info) = result.audio_info {
            let enhanced_path = output_dir.join(format!("{}_enhanced.wav", 
                video_path.file_stem().unwrap().to_string_lossy()));
            
            debug!("🔧 Enhancing audio quality: {}", audio_info.path.display());
            match self.audio_extractor.enhance_audio(audio_info, &enhanced_path).await {
                Ok(enhanced_audio) => {
                    result.audio_info = Some(enhanced_audio);
                    stages_completed.push(ProcessingStage::AudioEnhancement);
                }
                Err(e) => {
                    debug!("Audio enhancement failed (using original): {}", e);
                    // Use original audio if enhancement fails
                    stages_completed.push(ProcessingStage::AudioEnhancement);
                }
            }
        }

        // Stage 4: Transcription with Whisper and BJJ prompts
        if let Some(ref audio_info) = result.audio_info {
            debug!("🎤 Starting Whisper transcription: {}", audio_info.path.display());
            
            match self.whisper_transcriber.transcribe_audio(audio_info, output_dir).await {
                Ok(transcription_result) => {
                    // Store transcription results
                    result.srt_path = transcription_result.srt_path.clone();
                    result.text_path = transcription_result.text_path.clone();
                    result.transcription_result = Some(transcription_result);
                    
                    stages_completed.push(ProcessingStage::Transcription);
                    
                    info!("✅ Transcription completed for {}: {} characters", 
                          result.video_info.filename,
                          result.transcription_result.as_ref().unwrap().text.len());
                }
                Err(e) => {
                    warn!("⚠️ Transcription failed for {}: {}", result.video_info.filename, e);
                    // Continue processing even if transcription fails
                    stages_completed.push(ProcessingStage::Transcription);
                }
            }
        }

        // Mark as completed
        stages_completed.push(ProcessingStage::Completed);
        result.status = ProcessingStatus::Completed;
        result.processing_time = start_time.elapsed();
        result.stages_completed = stages_completed;

        Ok(result)
    }

    /// Create a lightweight clone of processor state for parallel processing
    fn clone_processor_state(&self) -> ProcessorState {
        ProcessorState {
            config: self.config.clone(),
            video_processor: VideoProcessor::new(),
            audio_extractor: AudioExtractor::new(),
            whisper_transcriber: self.whisper_transcriber.clone(),
            bjj_dictionary: self.bjj_dictionary.clone(),
        }
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
}

impl ProcessorState {
    async fn process_single_video(
        &self,
        video_path: &Path,
        output_dir: &Path,
    ) -> Result<VideoProcessingResult> {
        let start_time = Instant::now();
        let mut stages_completed = Vec::new();

        // Create base result
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
            processing_time: Duration::from_secs(0),
            status: ProcessingStatus::InProgress,
            error_message: None,
            stages_completed: Vec::new(),
        };

        // Process video (same logic as in BatchProcessor)
        match self.video_processor.get_video_info(video_path).await {
            Ok(video_info) => {
                result.video_info = video_info;
                stages_completed.push(ProcessingStage::VideoAnalysis);

                // Extract audio
                match self.audio_extractor.extract_for_transcription(video_path, output_dir).await {
                    Ok(audio_info) => {
                        result.audio_info = Some(audio_info.clone());
                        stages_completed.push(ProcessingStage::AudioExtraction);
                        
                        // Transcribe audio
                        match self.whisper_transcriber.transcribe_audio(&audio_info, output_dir).await {
                            Ok(transcription_result) => {
                                result.srt_path = transcription_result.srt_path.clone();
                                result.text_path = transcription_result.text_path.clone();
                                result.transcription_result = Some(transcription_result);
                                stages_completed.push(ProcessingStage::Transcription);
                                stages_completed.push(ProcessingStage::Completed);
                                result.status = ProcessingStatus::Completed;
                            }
                            Err(_e) => {
                                // Transcription failed but we still have audio
                                stages_completed.push(ProcessingStage::Completed);
                                result.status = ProcessingStatus::Completed;
                            }
                        }
                    }
                    Err(e) => {
                        result.status = ProcessingStatus::Failed;
                        result.error_message = Some(format!("Audio extraction failed: {}", e));
                    }
                }
            }
            Err(e) => {
                result.status = ProcessingStatus::Failed;
                result.error_message = Some(format!("Video analysis failed: {}", e));
            }
        }

        result.processing_time = start_time.elapsed();
        result.stages_completed = stages_completed;
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