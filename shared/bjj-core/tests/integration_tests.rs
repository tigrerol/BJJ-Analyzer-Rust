use bjj_core::{VideoMetadata, VideoFile, ArtifactDetector, ProcessingStage};
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_video_metadata_creation() {
    let metadata = VideoMetadata::new(
        "test_video.mp4".to_string(),
        1920,
        1080,
        30.0,
        600.0, // 10 minutes
    );
    
    assert_eq!(metadata.filename, "test_video.mp4");
    assert_eq!(metadata.resolution, (1920, 1080));
    assert_eq!(metadata.frame_rate, 30.0);
    assert_eq!(metadata.duration_seconds, 600.0);
    assert!(metadata.created_at.timestamp() > 0);
}

#[tokio::test]
async fn test_video_file_artifact_detection() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mp4");
    
    // Create mock video file
    fs::write(&video_path, b"mock video content").await.unwrap();
    
    let video_file = VideoFile::new(video_path).await.unwrap();
    
    // Should detect no artifacts initially
    assert!(!video_file.has_audio_artifact());
    assert!(!video_file.has_transcript_artifact());
    assert!(!video_file.has_corrected_transcript());
    assert!(!video_file.has_subtitles());
    
    assert_eq!(video_file.get_processing_stage(), ProcessingStage::Pending);
}

#[tokio::test]
async fn test_artifact_detector_with_existing_files() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test_video.mp4");
    let audio_path = temp_dir.path().join("test_video.wav");
    let transcript_path = temp_dir.path().join("test_video.txt");
    
    // Create mock files
    fs::write(&video_path, b"mock video").await.unwrap();
    fs::write(&audio_path, b"mock audio").await.unwrap();
    fs::write(&transcript_path, b"mock transcript").await.unwrap();
    
    let video_file = VideoFile::new(video_path).await.unwrap();
    
    // Should detect existing artifacts
    assert!(video_file.has_audio_artifact());
    assert!(video_file.has_transcript_artifact());
    assert!(!video_file.has_corrected_transcript()); // No _corrected.txt file
    
    assert_eq!(video_file.get_processing_stage(), ProcessingStage::Transcribed);
}

#[tokio::test]
async fn test_artifact_detector_scan_directory() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple video files with different processing stages
    let video1 = temp_dir.path().join("video1.mp4");
    let video2 = temp_dir.path().join("video2.mp4");
    let video2_wav = temp_dir.path().join("video2.wav");
    let video3 = temp_dir.path().join("video3.mp4");
    let video3_corrected = temp_dir.path().join("video3_corrected.txt");
    
    fs::write(&video1, b"video1").await.unwrap();
    fs::write(&video2, b"video2").await.unwrap();
    fs::write(&video2_wav, b"audio2").await.unwrap();
    fs::write(&video3, b"video3").await.unwrap();
    fs::write(&video3_corrected, b"corrected3").await.unwrap();
    
    let detector = ArtifactDetector::new();
    let results = detector.scan_directory(temp_dir.path()).await.unwrap();
    
    assert_eq!(results.len(), 3);
    
    // Check processing stages
    let video1_result = results.iter().find(|v| v.filename() == "video1.mp4").unwrap();
    assert_eq!(video1_result.get_processing_stage(), ProcessingStage::Pending);
    
    let video2_result = results.iter().find(|v| v.filename() == "video2.mp4").unwrap();
    assert_eq!(video2_result.get_processing_stage(), ProcessingStage::AudioExtracted);
    
    let video3_result = results.iter().find(|v| v.filename() == "video3.mp4").unwrap();
    assert_eq!(video3_result.get_processing_stage(), ProcessingStage::LLMCorrected);
}

#[tokio::test]
async fn test_video_file_artifact_paths() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("TestVideo.mp4");
    
    fs::write(&video_path, b"mock video").await.unwrap();
    
    let video_file = VideoFile::new(video_path).await.unwrap();
    
    // Test artifact path generation
    assert_eq!(
        video_file.audio_artifact_path(),
        temp_dir.path().join("TestVideo.wav")
    );
    
    assert_eq!(
        video_file.transcript_artifact_path(),
        temp_dir.path().join("TestVideo.txt")
    );
    
    assert_eq!(
        video_file.corrected_transcript_path(),
        temp_dir.path().join("TestVideo_corrected.txt")
    );
    
    assert_eq!(
        video_file.subtitle_artifact_path(),
        temp_dir.path().join("TestVideo.srt")
    );
}