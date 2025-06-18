//! Test LLM correction functionality independently

use bjj_llm::{LLMConfig, TranscriptionCorrector};
use std::path::Path;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_env_filter("debug")
        .init();

    // Test file path
    let test_video_path = Path::new("/Users/rolandlechner/SW Development/Test Files/test_transcript_for_llm.mp4");
    
    if !test_video_path.exists() {
        anyhow::bail!("Test video file not found: {}", test_video_path.display());
    }
    
    // Check if transcript exists
    let transcript_path = test_video_path.with_extension("txt");
    if !transcript_path.exists() {
        anyhow::bail!("Transcript file not found: {}", transcript_path.display());
    }
    
    println!("🧪 Testing LLM correction on: {}", test_video_path.display());
    println!("📄 Transcript file: {}", transcript_path.display());
    
    // Create LLM corrector with default config (LMStudio)
    let llm_config = LLMConfig::default();
    println!("🤖 Using LLM provider: {:?}", llm_config.provider());
    println!("🔗 LLM endpoint: {:?}", llm_config.endpoint());
    
    let corrector = TranscriptionCorrector::new(llm_config);
    
    // Test the correction
    println!("⚡ Starting LLM correction...");
    match corrector.correct_transcript_files(test_video_path).await {
        Ok(_) => {
            println!("✅ LLM correction completed successfully!");
            
            // Check if _corrected.txt file was created
            let corrected_path = test_video_path.with_extension("").with_extension("_corrected.txt");
            if corrected_path.exists() {
                println!("✅ Corrected file created: {}", corrected_path.display());
                
                // Show a sample of the correction
                let corrected_content = std::fs::read_to_string(&corrected_path)?;
                println!("📝 Corrected content preview (first 200 chars):");
                println!("{}", corrected_content.chars().take(200).collect::<String>());
                if corrected_content.len() > 200 {
                    println!("...");
                }
            } else {
                println!("❌ Corrected file not found");
            }
        }
        Err(e) => {
            println!("❌ LLM correction failed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}