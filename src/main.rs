use anyhow::Result;
use clap::{Arg, Command};
use std::path::PathBuf;
use tracing::{info, warn, error};

mod video;
mod audio;
mod processing;
mod config;
mod api;
mod bjj;
mod transcription;

use crate::config::Config;
use crate::processing::BatchProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("bjj_analyzer_rust=info,warn")
        .init();

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
                .required(true)
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
        .get_matches();

    let video_dir = PathBuf::from(matches.get_one::<String>("video-dir").unwrap());
    let output_dir = PathBuf::from(matches.get_one::<String>("output-dir").unwrap());
    let workers: usize = matches.get_one::<String>("workers").unwrap().parse()?;
    let verbose = matches.get_flag("verbose");

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

    // Validate input directory
    if !video_dir.exists() {
        error!("Input directory does not exist: {}", video_dir.display());
        return Err(anyhow::anyhow!("Input directory not found"));
    }

    // Create output directory
    tokio::fs::create_dir_all(&output_dir).await?;

    // Initialize batch processor
    let processor = BatchProcessor::new(config, workers).await?;

    // Start processing
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