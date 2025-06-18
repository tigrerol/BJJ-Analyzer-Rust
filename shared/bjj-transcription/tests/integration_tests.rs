use bjj_transcription::{AudioExtractor, AudioInfo, WhisperTranscriber, TranscriptionResult, TranscriptionConfig};
use bjj_core::VideoFile;
use tempfile::TempDir;
use tokio::fs;
use std::time::Duration;

#[tokio::test]
async fn test_audio_extractor_creation() {
    let extractor = AudioExtractor::new();
    
    assert_eq!(extractor.target_sample_rate(), 16000);
    assert_eq!(extractor.target_format(), "wav");
}

#[tokio::test] 
async fn test_audio_extractor_artifact_paths() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mp4");
    
    // Create mock video file
    fs::write(&video_path, b"mock video content").await.unwrap();
    
    let extractor = AudioExtractor::new();
    let audio_path = extractor.get_audio_output_path(&video_path);
    
    assert_eq!(audio_path, temp_dir.path().join("test_video.wav"));
}

#[tokio::test]
async fn test_audio_info_creation() {
    let temp_dir = TempDir::new().unwrap();
    let audio_path = temp_dir.path().join("test_audio.wav");
    
    let audio_info = AudioInfo::new(
        audio_path.clone(),
        Duration::from_secs(300), // 5 minutes
        16000, // 16kHz
        1,     // mono
        "pcm_s16le".to_string(),
        1024,  // 1KB file size
    );
    
    assert_eq!(audio_info.path(), &audio_path);
    assert_eq!(audio_info.duration(), Duration::from_secs(300));
    assert_eq!(audio_info.sample_rate(), 16000);
    assert_eq!(audio_info.channels(), 1);
    assert_eq!(audio_info.format(), "pcm_s16le");
    assert_eq!(audio_info.file_size(), 1024);
}

#[tokio::test]
async fn test_transcription_config_default() {
    let config = TranscriptionConfig::default();
    
    assert_eq!(config.provider(), "local");
    assert_eq!(config.model(), "base");
    assert!(config.use_gpu());
    assert_eq!(config.language(), Some("en"));
}

#[tokio::test]
async fn test_whisper_transcriber_creation() {
    let config = TranscriptionConfig::default();
    let transcriber = WhisperTranscriber::new(config);
    
    assert_eq!(transcriber.model(), "base");
    assert!(transcriber.supports_gpu());
}

#[tokio::test]
async fn test_transcription_result_creation() {
    let result = TranscriptionResult::new(
        "Test transcription text".to_string(),
        Some("en".to_string()),
        vec![], // Empty segments for simplicity
        Duration::from_secs(10),
        "base".to_string(),
    );
    
    assert_eq!(result.text(), "Test transcription text");
    assert_eq!(result.language(), Some("en"));
    assert_eq!(result.processing_time(), Duration::from_secs(10));
    assert_eq!(result.model_used(), "base");
}

#[tokio::test]
async fn test_transcription_workflow_integration() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mp4");
    
    // Create mock video file
    fs::write(&video_path, b"mock video content").await.unwrap();
    
    let video_file = VideoFile::new(video_path).await.unwrap();
    let extractor = AudioExtractor::new();
    
    // Test audio output path generation
    let audio_path = extractor.get_audio_output_path(video_file.video_path());
    
    assert!(audio_path.to_string_lossy().ends_with(".wav"));
    assert_eq!(
        audio_path.file_stem().unwrap(),
        video_file.video_path().file_stem().unwrap()
    );
}

#[tokio::test]
async fn test_artifact_aware_processing() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mp4");
    let audio_path = temp_dir.path().join("test_video.wav");
    let transcript_path = temp_dir.path().join("test_video.txt");
    
    // Create mock files
    fs::write(&video_path, b"mock video").await.unwrap();
    fs::write(&audio_path, b"mock audio").await.unwrap();
    fs::write(&transcript_path, b"mock transcript").await.unwrap();
    
    let video_file = VideoFile::new(video_path).await.unwrap();
    let extractor = AudioExtractor::new();
    
    // Should detect existing audio artifact
    assert!(extractor.audio_exists(video_file.video_path()));
    
    // Should detect transcription is needed (even though .txt exists, no _corrected.txt)
    let config = TranscriptionConfig::default();
    let transcriber = WhisperTranscriber::new(config);
    
    // Since test_video.txt exists, transcription_exists should return true
    assert!(transcriber.transcription_exists(video_file.video_path()));
    
    // Test the output paths
    let (text_path, srt_path) = transcriber.get_output_paths(video_file.video_path());
    assert!(text_path.to_string_lossy().ends_with(".txt"));
    assert!(srt_path.to_string_lossy().ends_with(".srt"));
}