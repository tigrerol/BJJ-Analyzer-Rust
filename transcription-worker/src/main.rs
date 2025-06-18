use transcription_worker::{WorkerConfig, TranscriptionWorker, WorkerMode};
use clap::{Arg, Command};
use std::path::PathBuf;
use tracing_subscriber;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {

    let matches = Command::new("bjj-transcription-worker")
        .version("0.1.0")
        .author("TigreRoll")
        .about("High-performance transcription worker for BJJ videos")
        .arg(
            Arg::new("video-dir")
                .long("video-dir")
                .short('d')
                .value_name("DIRECTORY")
                .help("Directory containing video files to process")
                .required(true)
        )
        .arg(
            Arg::new("batch-size")
                .long("batch-size")
                .short('b')
                .value_name("SIZE")
                .help("Number of videos to process in each batch")
                .default_value("10")
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .short('m')
                .value_name("MODE")
                .help("Processing mode: batch or continuous")
                .default_value("batch")
                .value_parser(["batch", "continuous"])
        )
        .arg(
            Arg::new("worker-name")
                .long("worker-name")
                .value_name("NAME")
                .help("Name for this worker instance")
                .default_value("transcription-worker-1")
        )
        .arg(
            Arg::new("no-llm")
                .long("no-llm")
                .help("Disable LLM-based transcript correction")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Show what would be processed without actually processing")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("scan-interval")
                .long("scan-interval")
                .value_name("SECONDS")
                .help("Interval between scans in continuous mode")
                .default_value("60")
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Configure logging based on verbose flag
    if matches.get_flag("verbose") {
        tracing_subscriber::fmt()
            .with_target(true)
            .with_thread_ids(true)
            .with_env_filter("debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_thread_ids(false)
            .with_env_filter("info")
            .init();
    }

    // Parse arguments
    let video_dir = PathBuf::from(matches.get_one::<String>("video-dir").unwrap());
    let batch_size: usize = matches.get_one::<String>("batch-size").unwrap().parse()?;
    let mode = match matches.get_one::<String>("mode").unwrap().as_str() {
        "batch" => WorkerMode::Batch,
        "continuous" => WorkerMode::Continuous,
        _ => unreachable!(),
    };
    let worker_name = matches.get_one::<String>("worker-name").unwrap().clone();
    let enable_llm_correction = !matches.get_flag("no-llm");
    let dry_run = matches.get_flag("dry-run");
    let scan_interval: u64 = matches.get_one::<String>("scan-interval").unwrap().parse()?;

    // Validate video directory
    if !video_dir.exists() {
        anyhow::bail!("Video directory does not exist: {}", video_dir.display());
    }

    if !video_dir.is_dir() {
        anyhow::bail!("Video path is not a directory: {}", video_dir.display());
    }

    // Create worker configuration
    let config = WorkerConfig::default()
        .with_video_dir(video_dir.clone())
        .with_batch_size(batch_size)
        .with_mode(mode)
        .with_worker_name(worker_name)
        .with_dry_run(dry_run)
        .with_scan_interval(scan_interval);

    // Log startup configuration
    tracing::info!("🚀 BJJ Transcription Worker Starting");
    tracing::info!("📁 Video Directory: {}", video_dir.display());
    tracing::info!("📊 Batch Size: {}", batch_size);
    tracing::info!("🔄 Mode: {:?}", mode);
    tracing::info!("🤖 LLM Correction: {}", if enable_llm_correction { "enabled" } else { "disabled" });
    tracing::info!("🧪 Dry Run: {}", if dry_run { "enabled" } else { "disabled" });
    
    if mode == WorkerMode::Continuous {
        tracing::info!("⏱️  Scan Interval: {} seconds", scan_interval);
    }

    // Create and run worker
    let mut worker = TranscriptionWorker::new(config);
    
    // Show initial scan results
    tracing::info!("🔍 Scanning for work...");
    let work_items = worker.scan_for_work().await?;
    
    if work_items.is_empty() {
        tracing::info!("✅ No videos need processing");
        return Ok(());
    }

    tracing::info!("📹 Found {} videos to process", work_items.len());
    
    // Show processing stages breakdown
    let mut pending = 0;
    let mut audio_extracted = 0;
    let mut transcribed = 0;
    let mut llm_corrected = 0;
    let mut subtitles_generated = 0;
    let mut completed = 0;
    
    for item in &work_items {
        use bjj_core::ProcessingStage;
        match item.current_stage() {
            ProcessingStage::Pending => pending += 1,
            ProcessingStage::AudioExtracted => audio_extracted += 1,
            ProcessingStage::Transcribed => transcribed += 1,
            ProcessingStage::LLMCorrected => llm_corrected += 1,
            ProcessingStage::SubtitlesGenerated => subtitles_generated += 1,
            ProcessingStage::Completed => completed += 1,
        }
    }
    
    if pending > 0 { tracing::info!("   Pending: {} videos", pending); }
    if audio_extracted > 0 { tracing::info!("   AudioExtracted: {} videos", audio_extracted); }
    if transcribed > 0 { tracing::info!("   Transcribed: {} videos", transcribed); }
    if llm_corrected > 0 { tracing::info!("   LLMCorrected: {} videos", llm_corrected); }
    if subtitles_generated > 0 { tracing::info!("   SubtitlesGenerated: {} videos", subtitles_generated); }
    if completed > 0 { tracing::info!("   Completed: {} videos", completed); }

    if dry_run {
        tracing::info!("🧪 Dry run complete - no actual processing performed");
        return Ok(());
    }

    // Start processing
    tracing::info!("🎬 Starting transcription processing...");
    worker.run().await?;

    // Show final statistics
    let stats = worker.get_stats();
    tracing::info!("✅ Processing complete!");
    tracing::info!("📊 Videos processed: {}", stats.videos_processed);
    tracing::info!("❌ Videos failed: {}", stats.videos_failed);
    tracing::info!("⏱️  Total time: {:.2}s", stats.total_processing_time_secs);

    if stats.videos_failed > 0 {
        tracing::warn!("⚠️  Some videos failed processing. Check logs for details.");
        std::process::exit(1);
    }

    Ok(())
}