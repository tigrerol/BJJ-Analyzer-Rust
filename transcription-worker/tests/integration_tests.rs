use transcription_worker::{WorkerConfig, TranscriptionWorker, WorkerMode};
use bjj_core::{VideoFile, ProcessingStage};
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_worker_config_creation() {
    let config = WorkerConfig::default();
    
    assert_eq!(config.batch_size(), 10);
    assert_eq!(config.mode(), WorkerMode::Batch);
    assert!(config.enable_llm_correction());
    assert_eq!(config.worker_name(), "transcription-worker-1");
}

#[tokio::test]
async fn test_worker_creation() {
    let config = WorkerConfig::default();
    let worker = TranscriptionWorker::new(config);
    
    assert_eq!(worker.name(), "transcription-worker-1");
    assert!(!worker.is_running());
}

#[tokio::test]
async fn test_worker_scan_for_work() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create mock video files
    let video1 = temp_dir.path().join("video1.mp4");
    let video2 = temp_dir.path().join("video2.mp4");
    let video2_wav = temp_dir.path().join("video2.wav");
    
    fs::write(&video1, b"mock video 1").await.unwrap();
    fs::write(&video2, b"mock video 2").await.unwrap();
    fs::write(&video2_wav, b"mock audio 2").await.unwrap();
    
    let config = WorkerConfig::default()
        .with_video_dir(temp_dir.path().to_path_buf());
    let worker = TranscriptionWorker::new(config);
    
    let work_items = worker.scan_for_work().await.unwrap();
    
    // Should find 1 video needing transcription (video1)
    // video2 already has audio extracted
    assert_eq!(work_items.len(), 2);
    
    let video1_work = work_items.iter().find(|w| w.video_path().ends_with("video1.mp4")).unwrap();
    assert_eq!(video1_work.current_stage(), ProcessingStage::Pending);
    
    let video2_work = work_items.iter().find(|w| w.video_path().ends_with("video2.mp4")).unwrap();
    assert_eq!(video2_work.current_stage(), ProcessingStage::AudioExtracted);
}

#[tokio::test]
async fn test_worker_respects_batch_size() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 5 mock video files
    for i in 1..=5 {
        let video = temp_dir.path().join(format!("video{}.mp4", i));
        fs::write(&video, b"mock video").await.unwrap();
    }
    
    let config = WorkerConfig::default()
        .with_video_dir(temp_dir.path().to_path_buf())
        .with_batch_size(3); // Limit to 3
        
    let worker = TranscriptionWorker::new(config);
    let batch = worker.get_next_batch().await.unwrap();
    
    assert_eq!(batch.len(), 3); // Should respect batch size limit
}

#[tokio::test] 
async fn test_worker_skips_completed_videos() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create video with all artifacts
    let video = temp_dir.path().join("completed.mp4");
    let wav = temp_dir.path().join("completed.wav");
    let txt = temp_dir.path().join("completed.txt");
    let corrected = temp_dir.path().join("completed_corrected.txt");
    let srt = temp_dir.path().join("completed.srt");
    
    fs::write(&video, b"mock video").await.unwrap();
    fs::write(&wav, b"mock audio").await.unwrap();
    fs::write(&txt, b"mock transcript").await.unwrap();
    fs::write(&corrected, b"mock corrected").await.unwrap();
    fs::write(&srt, b"mock srt").await.unwrap();
    
    let config = WorkerConfig::default()
        .with_video_dir(temp_dir.path().to_path_buf());
    let worker = TranscriptionWorker::new(config);
    
    let work_items = worker.scan_for_work().await.unwrap();
    
    // Should find no work - video is complete
    assert_eq!(work_items.len(), 0);
}

#[tokio::test]
async fn test_worker_process_single_video() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    fs::write(&video_path, b"mock video").await.unwrap();
    
    let config = WorkerConfig::default()
        .with_video_dir(temp_dir.path().to_path_buf())
        .with_dry_run(true); // Don't actually process
        
    let worker = TranscriptionWorker::new(config);
    let video_file = VideoFile::new(video_path.clone()).await.unwrap();
    
    let result = worker.process_video(&video_file).await;
    
    assert!(result.is_ok());
    // In dry run mode, should simulate success
}

#[tokio::test]
async fn test_worker_mode_configuration() {
    // Test batch mode
    let batch_config = WorkerConfig::default()
        .with_mode(WorkerMode::Batch);
    assert_eq!(batch_config.mode(), WorkerMode::Batch);
    
    // Test continuous mode
    let continuous_config = WorkerConfig::default()
        .with_mode(WorkerMode::Continuous);
    assert_eq!(continuous_config.mode(), WorkerMode::Continuous);
}

#[tokio::test]
async fn test_worker_stats_tracking() {
    let config = WorkerConfig::default();
    let worker = TranscriptionWorker::new(config);
    
    let stats = worker.get_stats();
    assert_eq!(stats.videos_processed, 0);
    assert_eq!(stats.videos_failed, 0);
    assert_eq!(stats.total_processing_time_secs, 0.0);
}