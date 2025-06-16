use anyhow::Result;
use bjj_analyzer_rust::{BJJDictionary, WhisperTranscriber, Config};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("bjj_analyzer_rust=info")
        .init();

    info!("🔍 Checking transcription backend availability...");

    // Check if any Whisper backend is available
    match WhisperTranscriber::check_availability().await {
        Ok(backend_info) => {
            info!("✅ {}", backend_info);
        }
        Err(e) => {
            info!("❌ {}", e);
            info!("💡 Recommendation: Install whisper.cpp for best performance:");
            info!("   git clone https://github.com/ggerganov/whisper.cpp.git");
            info!("   cd whisper.cpp && make -j");
            info!("   # Download models: ./models/download-ggml-model.sh base");
            return Ok(());
        }
    }

    // Test BJJ dictionary
    info!("📚 Testing BJJ Dictionary...");
    let bjj_dict = BJJDictionary::new();
    let stats = bjj_dict.get_stats();
    info!("   - {} BJJ terms loaded", stats.total_terms);
    info!("   - {} corrections available", stats.total_corrections);
    
    let bjj_prompt = bjj_dict.generate_prompt();
    info!("   - Generated prompt: {}", &bjj_prompt[..100.min(bjj_prompt.len())]);

    // Test transcriber initialization
    info!("🎤 Testing Whisper transcriber initialization...");
    let config = Config::default();
    let transcriber = WhisperTranscriber::new(config.transcription.clone(), bjj_dict);
    info!("   - Transcriber initialized with model: {}", config.transcription.model);
    info!("   - BJJ prompts enabled: {}", config.transcription.use_bjj_prompts);

    // Show available models
    info!("📋 Available Whisper models:");
    let models = WhisperTranscriber::get_available_models().await?;
    for model in models {
        info!("   - {}", model);
    }

    info!("🎉 All transcription components ready!");
    info!("💡 To test with real videos, use: cargo run -- --video-dir /path/to/videos");

    Ok(())
}