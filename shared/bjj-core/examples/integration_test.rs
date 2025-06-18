use bjj_core::{ArtifactDetector, ProcessingStage};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Testing BJJ Video Processing Integration (Core Only)");
    
    // Test with Test Files2 which has artifacts
    let test_dir2 = Path::new("/Users/rolandlechner/SW Development/Test Files2");
    if test_dir2.exists() {
        println!("\n🧪 Testing with processed artifacts:");
        test_video_analysis(test_dir2).await?;
    }
    
    let test_dir = Path::new("/Users/rolandlechner/SW Development/Test Files");
    if test_dir.exists() {
        println!("\n🧪 Testing with unprocessed videos:");
        test_video_analysis(test_dir).await?;
    }
    
    Ok(())
}

async fn test_video_analysis(test_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📹 Analyzing videos in: {}", test_dir.display());
    
    let detector = ArtifactDetector::new();
    let videos = detector.scan_directory(test_dir).await?;
    
    println!("✅ Found {} videos", videos.len());
    
    for video in &videos {
        println!("\n{}", "=".repeat(50));
        println!("📹 {}", video.filename());
        
        let stage = video.get_processing_stage();
        println!("📊 Processing Stage: {:?} ({}%)", stage, stage.progress_percentage());
        
        // Show artifact status
        println!("📁 Artifacts:");
        println!("   🎵 Audio (.wav): {}", if video.has_audio_artifact() { "✅" } else { "❌" });
        println!("   📄 Transcript (.txt): {}", if video.has_transcript_artifact() { "✅" } else { "❌" });
        println!("   🔧 Corrected (_corrected.txt): {}", if video.has_corrected_transcript() { "✅" } else { "❌" });
        println!("   📺 Subtitles (.srt): {}", if video.has_subtitles() { "✅" } else { "❌" });
        
        // Show artifact paths
        println!("📂 Expected Paths:");
        println!("   🎵 {}", video.audio_artifact_path().display());
        println!("   📄 {}", video.transcript_artifact_path().display());
        println!("   🔧 {}", video.corrected_transcript_path().display());
        println!("   📺 {}", video.subtitle_artifact_path().display());
        
        // Recommended action
        println!("🎯 Next Action:");
        match stage {
            ProcessingStage::Pending => println!("   ➡️  Extract audio"),
            ProcessingStage::AudioExtracted => println!("   ➡️  Transcribe audio"),
            ProcessingStage::Transcribed => println!("   ➡️  Apply LLM correction"),
            ProcessingStage::LLMCorrected => println!("   ➡️  Generate subtitles"),
            ProcessingStage::SubtitlesGenerated => println!("   ➡️  Ready for curation"),
            ProcessingStage::Completed => println!("   ✅ Complete"),
        }
    }
    
    // Summary statistics
    println!("\n{}", "=".repeat(50));
    println!("📊 Summary:");
    
    let mut pending = 0;
    let mut audio_extracted = 0;
    let mut transcribed = 0;
    let mut llm_corrected = 0;
    let mut subtitles_generated = 0;
    let mut completed = 0;
    
    for video in &videos {
        match video.get_processing_stage() {
            ProcessingStage::Pending => pending += 1,
            ProcessingStage::AudioExtracted => audio_extracted += 1,
            ProcessingStage::Transcribed => transcribed += 1,
            ProcessingStage::LLMCorrected => llm_corrected += 1,
            ProcessingStage::SubtitlesGenerated => subtitles_generated += 1,
            ProcessingStage::Completed => completed += 1,
        }
    }
    
    println!("   Pending: {} videos", pending);
    println!("   Audio Extracted: {} videos", audio_extracted);
    println!("   Transcribed: {} videos", transcribed);
    println!("   LLM Corrected: {} videos", llm_corrected);
    println!("   Subtitles Generated: {} videos", subtitles_generated);
    println!("   Completed: {} videos", completed);
    
    let unprocessed = detector.scan_unprocessed(test_dir).await?;
    let ready_for_curation = detector.scan_ready_for_curation(test_dir).await?;
    
    println!("\n🎯 Workflow Status:");
    println!("   🔄 {} videos need transcription worker", unprocessed.len());
    println!("   ✅ {} videos ready for curation manager", ready_for_curation.len());
    
    Ok(())
}