use anyhow::Result;
use bjj_analyzer_rust::chapters::cache::ChapterCacheManager;
use bjj_analyzer_rust::state::{StateManager, ProcessingStage};
use std::path::PathBuf;
use tracing::{info, warn};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cache-manager")]
#[command(about = "Chapter cache management utility")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, default_value = "chapters_cache")]
    cache_dir: PathBuf,
    
    #[arg(long, default_value = ".bjj_analyzer_state")]
    state_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// List all cached series
    List,
    /// Get cache statistics
    Stats,
    /// Invalidate cache for a specific series
    Invalidate {
        /// Cache key to invalidate
        cache_key: String,
    },
    /// Invalidate cache for an instructor/series combination
    InvalidateSeries {
        /// Instructor name (e.g., "AdamWardzinski")
        instructor: String,
        /// Series name (e.g., "ClosedGuardReintroduced")  
        series: String,
    },
    /// Clear all cache entries
    Clear,
    /// Clean up expired cache entries
    Cleanup,
    /// Reset chapter detection state for a video file
    ResetChapterState {
        /// Video file path
        video_path: PathBuf,
    },
    /// Reset all processing states for a video
    ResetVideoState {
        /// Video file path
        video_path: PathBuf,
    },
    /// Reset all video states in the state directory
    ResetAllStates,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let cli = Cli::parse();
    
    // Initialize cache manager
    let cache_manager = ChapterCacheManager::new(cli.cache_dir, 24); // 24 hour TTL
    cache_manager.initialize().await?;
    
    // Initialize state manager
    let state_manager = StateManager::new(cli.state_dir).await?;

    match cli.command {
        Commands::List => {
            let series_list = cache_manager.list_cached_series().await?;
            
            if series_list.is_empty() {
                info!("📭 No cached series found");
                return Ok(());
            }

            info!("📚 Found {} cached series:", series_list.len());
            
            for series in series_list {
                let status = if series.is_valid { "✅ Valid" } else { "❌ Expired" };
                info!("  {} - {} chapters, {} hours old, {}", 
                     series.cache_key, 
                     series.chapter_count, 
                     series.age_hours,
                     status);
                info!("    URL: {}", series.product_url);
            }
        }
        
        Commands::Stats => {
            let stats = cache_manager.get_cache_stats().await?;
            info!("📊 Cache Statistics:");
            info!("  Total files: {}", stats.total_files);
            info!("  Valid files: {}", stats.valid_files);
            info!("  Expired files: {}", stats.expired_files);
            info!("  Total chapters: {}", stats.total_chapters);
        }
        
        Commands::Invalidate { cache_key } => {
            let removed = cache_manager.invalidate_cache(&cache_key).await?;
            if removed {
                info!("✅ Successfully invalidated cache for: {}", cache_key);
            } else {
                warn!("⚠️ Cache key not found: {}", cache_key);
            }
        }
        
        Commands::InvalidateSeries { instructor, series } => {
            let instructor_vec = vec![instructor.clone()];
            let series_vec = vec![series.clone()];
            
            let removed = cache_manager.invalidate_series_cache(&instructor_vec, &series_vec).await?;
            if removed {
                info!("✅ Successfully invalidated cache for: {} - {}", instructor, series);
            } else {
                warn!("⚠️ No cache found for: {} - {}", instructor, series);
            }
        }
        
        Commands::Clear => {
            let count = cache_manager.clear_all_cache().await?;
            info!("🧹 Cleared {} cache files", count);
        }
        
        Commands::Cleanup => {
            let count = cache_manager.cleanup_expired_cache().await?;
            info!("🗑️ Cleaned up {} expired cache files", count);
        }
        
        Commands::ResetChapterState { video_path } => {
            state_manager.reset_chapter_detection(&video_path).await?;
            info!("✅ Reset chapter detection state for: {}", video_path.display());
        }
        
        Commands::ResetVideoState { video_path } => {
            state_manager.reset_all_stages(&video_path).await?;
            info!("✅ Reset all processing stages for: {}", video_path.display());
        }
        
        Commands::ResetAllStates => {
            let count = state_manager.reset_all_videos().await?;
            info!("🧹 Reset {} video processing states", count);
        }
    }

    Ok(())
}