use bjj_core::{ArtifactDetector, VideoFile, ProcessingStage};
use bjj_transcription::{AudioExtractor, WhisperTranscriber, TranscriptionConfig};
use bjj_llm::{LLMConfig, TranscriptionCorrector};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerMode {
    Batch,
    Continuous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    batch_size: usize,
    mode: WorkerMode,
    enable_llm_correction: bool,
    worker_name: String,
    video_dir: PathBuf,
    dry_run: bool,
    scan_interval_secs: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            mode: WorkerMode::Batch,
            enable_llm_correction: true,
            worker_name: "transcription-worker-1".to_string(),
            video_dir: PathBuf::from("."),
            dry_run: false,
            scan_interval_secs: 60,
        }
    }
}

impl WorkerConfig {
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn mode(&self) -> WorkerMode {
        self.mode
    }

    pub fn enable_llm_correction(&self) -> bool {
        self.enable_llm_correction
    }

    pub fn worker_name(&self) -> &str {
        &self.worker_name
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn with_mode(mut self, mode: WorkerMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_video_dir(mut self, dir: PathBuf) -> Self {
        self.video_dir = dir;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_worker_name(mut self, name: String) -> Self {
        self.worker_name = name;
        self
    }

    pub fn with_scan_interval(mut self, seconds: u64) -> Self {
        self.scan_interval_secs = seconds;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    pub videos_processed: usize,
    pub videos_failed: usize,
    pub total_processing_time_secs: f64,
    #[serde(skip)]
    pub started_at: Option<Instant>,
}

impl Default for WorkerStats {
    fn default() -> Self {
        Self {
            videos_processed: 0,
            videos_failed: 0,
            total_processing_time_secs: 0.0,
            started_at: None,
        }
    }
}

#[derive(Debug)]
pub struct WorkItem {
    video_file: VideoFile,
}

impl WorkItem {
    pub fn new(video_file: VideoFile) -> Self {
        Self { video_file }
    }

    pub fn video_path(&self) -> &Path {
        self.video_file.video_path()
    }

    pub fn current_stage(&self) -> ProcessingStage {
        self.video_file.get_processing_stage()
    }
}

pub struct TranscriptionWorker {
    config: WorkerConfig,
    detector: ArtifactDetector,
    audio_extractor: AudioExtractor,
    transcriber: WhisperTranscriber,
    corrector: Option<TranscriptionCorrector>,
    stats: WorkerStats,
    running: bool,
}

impl TranscriptionWorker {
    pub fn new(config: WorkerConfig) -> Self {
        let detector = ArtifactDetector::new();
        let audio_extractor = AudioExtractor::new();
        let transcription_config = TranscriptionConfig::default();
        let transcriber = WhisperTranscriber::new(transcription_config);
        
        let corrector = if config.enable_llm_correction {
            let llm_config = LLMConfig::default();
            Some(TranscriptionCorrector::new(llm_config))
        } else {
            None
        };

        Self {
            config,
            detector,
            audio_extractor,
            transcriber,
            corrector,
            stats: WorkerStats::default(),
            running: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.config.worker_name
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn get_stats(&self) -> WorkerStats {
        self.stats.clone()
    }

    pub async fn scan_for_work(&self) -> Result<Vec<WorkItem>> {
        let videos = self.detector.scan_directory(&self.config.video_dir).await?;
        
        let work_items: Vec<WorkItem> = videos
            .into_iter()
            .filter(|video| !matches!(video.get_processing_stage(), ProcessingStage::Completed))
            .map(WorkItem::new)
            .collect();

        Ok(work_items)
    }

    pub async fn get_next_batch(&self) -> Result<Vec<WorkItem>> {
        let mut work_items = self.scan_for_work().await?;
        work_items.truncate(self.config.batch_size);
        Ok(work_items)
    }

    pub async fn process_video(&self, video_file: &VideoFile) -> Result<()> {
        if self.config.dry_run {
            tracing::info!("Dry run: Would process {}", video_file.filename());
            return Ok(());
        }

        let start_time = Instant::now();
        let mut current_video = video_file.clone();

        // Process through stages until complete or error
        loop {
            let stage = current_video.get_processing_stage();
            tracing::info!("Processing {} at stage: {:?}", current_video.filename(), stage);

            match stage {
                ProcessingStage::Pending => {
                    tracing::info!("Extracting audio from {}", current_video.filename());
                    self.audio_extractor.extract_audio(current_video.video_path()).await
                        .context("Failed to extract audio")?;
                }
                ProcessingStage::AudioExtracted => {
                    tracing::info!("Transcribing audio for {}", current_video.filename());
                    
                    // Get real audio info from extracted file
                    let audio_path = current_video.audio_artifact_path();
                    if !audio_path.exists() {
                        return Err(anyhow::anyhow!("Audio file not found: {}", audio_path.display()));
                    }
                    
                    let audio_info = self.audio_extractor.get_audio_info(&audio_path).await
                        .context("Failed to get audio info")?;
                    
                    self.transcriber.transcribe_audio(&audio_info).await
                        .context("Failed to transcribe audio")?;
                }
                ProcessingStage::Transcribed => {
                    if let Some(corrector) = &self.corrector {
                        tracing::info!("Applying LLM correction to {}", current_video.filename());
                        
                        // Apply corrections to both .txt and .srt files simultaneously
                        corrector.correct_transcript_files(current_video.video_path()).await
                            .context("Failed to apply LLM corrections")?;
                    } else {
                        tracing::info!("LLM correction disabled, skipping to subtitles");
                    }
                }
                ProcessingStage::LLMCorrected => {
                    tracing::info!("Generating subtitles for {}", current_video.filename());
                    self.generate_subtitles(&current_video).await
                        .context("Failed to generate subtitles")?;
                }
                ProcessingStage::SubtitlesGenerated | ProcessingStage::Completed => {
                    tracing::info!("{} processing complete", current_video.filename());
                    break;
                }
            }

            // Re-scan the video file to update its processing stage
            current_video = VideoFile::new(current_video.video_path().to_path_buf()).await
                .context("Failed to re-scan video file")?;
            
            // Safety check to prevent infinite loops
            let new_stage = current_video.get_processing_stage();
            if new_stage == stage {
                tracing::warn!("Processing stage did not advance from {:?}, breaking loop", stage);
                break;
            }
        }

        let processing_time = start_time.elapsed();
        tracing::info!(
            "Completed processing {} in {:.2}s",
            current_video.filename(),
            processing_time.as_secs_f64()
        );

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        self.running = true;
        self.stats.started_at = Some(Instant::now());

        match self.config.mode {
            WorkerMode::Batch => self.run_batch_mode().await,
            WorkerMode::Continuous => self.run_continuous_mode().await,
        }
    }

    async fn run_batch_mode(&mut self) -> Result<()> {
        tracing::info!("Starting batch mode processing");
        
        let work_items = self.get_next_batch().await?;
        tracing::info!("Found {} videos to process", work_items.len());

        for work_item in work_items {
            let result = self.process_video(&work_item.video_file).await;
            
            match result {
                Ok(_) => {
                    self.stats.videos_processed += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to process {}: {}", work_item.video_file.filename(), e);
                    self.stats.videos_failed += 1;
                }
            }
        }

        self.running = false;
        
        // Update total processing time
        if let Some(start_time) = self.stats.started_at {
            self.stats.total_processing_time_secs = start_time.elapsed().as_secs_f64();
        }
        
        tracing::info!(
            "Batch processing complete. Processed: {}, Failed: {}, Total time: {:.1}s",
            self.stats.videos_processed,
            self.stats.videos_failed,
            self.stats.total_processing_time_secs
        );

        Ok(())
    }

    async fn run_continuous_mode(&mut self) -> Result<()> {
        tracing::info!("Starting continuous mode processing");
        
        loop {
            let work_items = self.get_next_batch().await?;
            
            if work_items.is_empty() {
                tracing::debug!("No work found, sleeping for {} seconds", self.config.scan_interval_secs);
                tokio::time::sleep(Duration::from_secs(self.config.scan_interval_secs)).await;
                continue;
            }

            tracing::info!("Found {} videos to process", work_items.len());

            for work_item in work_items {
                let result = self.process_video(&work_item.video_file).await;
                
                match result {
                    Ok(_) => {
                        self.stats.videos_processed += 1;
                    }
                    Err(e) => {
                        tracing::error!("Failed to process {}: {}", work_item.video_file.filename(), e);
                        self.stats.videos_failed += 1;
                    }
                }
            }
        }
    }

    async fn generate_subtitles(&self, video_file: &VideoFile) -> Result<()> {
        // TODO: Implement actual subtitle generation
        // For now, this is a placeholder
        tracing::debug!("Subtitle generation not yet implemented for {}", video_file.filename());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_config_defaults() {
        let config = WorkerConfig::default();
        assert_eq!(config.batch_size(), 10);
        assert_eq!(config.mode(), WorkerMode::Batch);
        assert!(config.enable_llm_correction());
        assert_eq!(config.worker_name(), "transcription-worker-1");
    }

    #[test]
    fn test_worker_config_builder() {
        let config = WorkerConfig::default()
            .with_batch_size(20)
            .with_mode(WorkerMode::Continuous)
            .with_worker_name("test-worker".to_string());

        assert_eq!(config.batch_size(), 20);
        assert_eq!(config.mode(), WorkerMode::Continuous);
        assert_eq!(config.worker_name(), "test-worker");
    }
}