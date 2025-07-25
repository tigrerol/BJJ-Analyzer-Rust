use anyhow::Result;
use clap::{Arg, Command};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, error};

mod video;
mod audio;
mod processing;
mod config;
// mod transcription_api; // not yet implemented
mod bjj;
mod transcription;
mod llm;
mod state;
mod chapters;
// mod series; // not yet implemented

#[cfg(feature = "api")]
mod api;

use crate::config::Config;
use crate::processing::BatchProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with file output
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("bjj_analyzer.log")
        .expect("Failed to create log file");
    
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stdout.and(log_file))
        .init();

    info!("🔧 Logging system initialized");
    info!("📝 Log file: bjj_analyzer.log");

    let matches = Command::new("BJJ Video Analyzer (Rust)")
        .version("0.1.0")
        .author("TigreRoll")
        .about("High-performance BJJ video analysis and processing")
        .arg(
            Arg::new("video-dir")
                .short('d')
                .long("video-dir")
                .value_name("DIR")
                .help("Directory containing videos to process")
                .required_unless_present("clear-cache")
        )
        .arg(
            Arg::new("output-dir")
                .short('o')
                .long("output-dir")
                .value_name("DIR")
                .help("Output directory for results")
                .default_value("./output")
        )
        .arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .value_name("NUM")
                .help("Number of parallel workers")
                .default_value("4")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("clear-cache")
                .long("clear-cache")
                .help("Clear all chapter cache files and exit")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("reset-chapters")
                .long("reset-chapters")
                .help("Reset chapter detection state to force re-scraping")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("api-port")
                .long("api-port")
                .value_name("PORT")
                .help("Enable API server on specified port (requires 'api' feature)")
        )
        .get_matches();

    let clear_cache = matches.get_flag("clear-cache");
    let reset_chapters = matches.get_flag("reset-chapters");
    
    // Handle cache clearing first
    if clear_cache {
        info!("🧹 Clearing chapter files...");
        use crate::chapters::ChapterDetector;
        
        let detector = ChapterDetector::new().await?;
        let files = detector.list_chapter_files().await.unwrap_or_default();
        let mut cleared_count = 0;
        
        for file_path in files {
            if tokio::fs::remove_file(&file_path).await.is_ok() {
                cleared_count += 1;
                info!("🗑️ Removed: {}", file_path.display());
            }
        }
        
        info!("✅ Cleared {} chapter files", cleared_count);
        return Ok(());
    }

    let video_dir = PathBuf::from(matches.get_one::<String>("video-dir").unwrap_or(&".".to_string()));
    let output_dir = PathBuf::from(matches.get_one::<String>("output-dir").unwrap());
    let workers: usize = matches.get_one::<String>("workers").unwrap().parse()?;
    let verbose = matches.get_flag("verbose");
    let api_port = matches.get_one::<String>("api-port");

    if verbose {
        info!("Verbose logging enabled");
    }

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        warn!("Failed to load config, using defaults: {}", e);
        Config::default()
    });

    info!("🚀 BJJ Video Analyzer (Rust) starting...");
    info!("📁 Input directory: {}", video_dir.display());
    info!("📂 Output directory: {}", output_dir.display());
    info!("🔧 Workers: {}", workers);
    info!("🤖 LLM correction enabled: {}", config.llm.enable_correction);
    info!("🤖 LLM provider: {:?}", config.llm.provider);
    info!("🤖 LLM endpoint: {:?}", config.llm.endpoint);

    // Validate input directory
    if !video_dir.exists() {
        error!("Input directory does not exist: {}", video_dir.display());
        return Err(anyhow::anyhow!("Input directory not found"));
    }

    // Create output directory
    tokio::fs::create_dir_all(&output_dir).await?;

    // Initialize batch processor
    let processor = BatchProcessor::new(config, workers).await?;
    
    // Start API server if requested
    #[cfg(feature = "api")]
    let _api_handle = if let Some(port_str) = api_port {
        let port: u16 = port_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid port number: {}", port_str))?;
        
        info!("🌐 Starting API server on port {}", port);
        let state_manager = processor.get_state_manager_for_dir(&video_dir).await?;
        let config = Arc::new(processor.get_config().clone());
        let api_server = crate::api::ApiServer::new(state_manager, config, port);
        Some(api_server.start_background())
    } else {
        None
    };
    
    #[cfg(not(feature = "api"))]
    if api_port.is_some() {
        warn!("API server requested but 'api' feature not enabled. Rebuild with --features api");
    }
    
    // Handle chapter state reset if requested
    if reset_chapters {
        info!("🔄 Resetting chapter detection state for all videos...");
        let reset_count = processor.reset_chapter_detection_state(&video_dir).await?;
        info!("✅ Reset chapter detection state for {} videos", reset_count);
    }

    // Check if running in API server mode
    #[cfg(feature = "api")]
    if api_port.is_some() {
        info!("🌐 Running in API server mode - processing will be triggered via API calls");
        info!("🎯 Access the web interface or use API endpoints to process videos");
        info!("🛑 Press Ctrl+C to stop the server");
        
        // Wait for the API server (it's running in background)
        if let Some(api_handle) = _api_handle {
            // Keep the server running indefinitely
            if let Err(e) = api_handle.await? {
                error!("API server error: {}", e);
            }
        }
        return Ok(());
    }

    // Standard batch processing mode (when no --api-port specified)
    let start_time = std::time::Instant::now();
    let results = processor.process_directory(video_dir, output_dir).await?;
    let duration = start_time.elapsed();

    // Print results
    info!("🎉 Processing completed in {:.2}s", duration.as_secs_f64());
    info!("✅ Successful: {}", results.successful);
    info!("❌ Failed: {}", results.failed);
    info!("📊 Success rate: {:.1}%", 
        if results.total > 0 { 
            results.successful as f64 / results.total as f64 * 100.0 
        } else { 
            0.0 
        }
    );

    Ok(())
}