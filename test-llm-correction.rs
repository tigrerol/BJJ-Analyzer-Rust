#!/usr/bin/env -S cargo +nightly -Zscript
//! Test LLM correction functionality
//! 
//! Run with: cargo +nightly -Zscript test-llm-correction.rs

use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Add the necessary dependencies for this script
    let llm_config = bjj_llm::LLMConfig::default();
    let corrector = bjj_llm::TranscriptionCorrector::new(llm_config);
    
    // Test file path
    let test_video_path = Path::new("/Users/rolandlechner/SW Development/Test Files/test_transcript_for_llm.mp4");
    
    println!("🧪 Testing LLM correction on: {}", test_video_path.display());
    
    // Test the correction
    match corrector.correct_transcript_files(test_video_path).await {
        Ok(_) => {
            println!("✅ LLM correction completed successfully!");
            
            // Check if _corrected.txt file was created
            let corrected_path = test_video_path.with_extension("").with_extension("_corrected.txt");
            if corrected_path.exists() {
                println!("✅ Corrected file created: {}", corrected_path.display());
            } else {
                println!("❌ Corrected file not found");
            }
        }
        Err(e) => {
            println!("❌ LLM correction failed: {}", e);
        }
    }
    
    Ok(())
}