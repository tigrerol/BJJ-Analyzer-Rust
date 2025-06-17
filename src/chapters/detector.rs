/// Chapter detection coordinator that orchestrates different detection methods
use super::{ChapterInfo, SearchTerms, VideoMetadata};
use super::scraper::BJJFanaticsScraper;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Configuration for chapter detection
#[derive(Debug, Clone)]
pub struct ChapterDetectionConfig {
    /// Enable chapter detection
    pub enable_detection: bool,
    /// Chapters output directory
    pub chapters_dir: PathBuf,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Maximum number of chapters expected
    pub max_chapters: usize,
}

impl Default for ChapterDetectionConfig {
    fn default() -> Self {
        Self {
            enable_detection: true,
            chapters_dir: PathBuf::from("chapters"),
            request_timeout_seconds: 30,
            max_chapters: 100,
        }
    }
}

/// Main chapter detector that coordinates different detection methods
#[derive(Clone)]
pub struct ChapterDetector {
    config: ChapterDetectionConfig,
    scraper: BJJFanaticsScraper,
}

impl ChapterDetector {
    /// Create a new chapter detector with default config
    pub async fn new() -> Result<Self> {
        Self::with_config(ChapterDetectionConfig::default()).await
    }

    /// Create a new chapter detector with custom config
    pub async fn with_config(config: ChapterDetectionConfig) -> Result<Self> {
        // Ensure chapters directory exists
        tokio::fs::create_dir_all(&config.chapters_dir).await?;

        // Initialize scraper
        let scraper = BJJFanaticsScraper::new(config.request_timeout_seconds);

        info!("🔍 Chapter detector initialized");
        info!("📁 Chapters directory: {}", config.chapters_dir.display());

        Ok(Self {
            config,
            scraper,
        })
    }

    /// Detect chapters for a video file
    pub async fn detect_chapters(&self, video_path: &Path) -> Result<Vec<ChapterInfo>> {
        if !self.config.enable_detection {
            info!("🔕 Chapter detection disabled");
            return Ok(Vec::new());
        }

        let filename = video_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Get the directory containing the video file
        let video_dir = video_path.parent().unwrap_or_else(|| Path::new("."));

        info!("🔍 Starting chapter detection for: {}", filename);
        info!("📁 Video directory: {}", video_dir.display());

        // Use scraper to get chapters (checks for existing file first)
        match self.scraper.scrape_chapters(filename, video_dir, Some(&self.config.chapters_dir)).await {
            Ok(chapters) => {
                if chapters.is_empty() {
                    warn!("⚠️ No chapters found for: {}", filename);
                } else {
                    info!("✅ Found {} chapters", chapters.len());
                }
                Ok(chapters)
            }
            Err(e) => {
                warn!("❌ Failed to get chapters for {}: {}", filename, e);
                Ok(Vec::new()) // Return empty list instead of error for graceful degradation
            }
        }
    }

    /// Force refresh chapters for a video file (re-scrapes even if file exists)
    pub async fn force_refresh_chapters(&self, video_path: &Path) -> Result<Vec<ChapterInfo>> {
        if !self.config.enable_detection {
            info!("🔕 Chapter detection disabled");
            return Ok(Vec::new());
        }

        let filename = video_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Get the directory containing the video file
        let video_dir = video_path.parent().unwrap_or_else(|| Path::new("."));

        info!("🔄 Force refreshing chapters for: {}", filename);

        match self.scraper.force_scrape_chapters(filename, video_dir, Some(&self.config.chapters_dir)).await {
            Ok(chapters) => {
                if chapters.is_empty() {
                    warn!("⚠️ No chapters found for: {}", filename);
                } else {
                    info!("✅ Found {} chapters", chapters.len());
                }
                Ok(chapters)
            }
            Err(e) => {
                warn!("❌ Failed to force refresh chapters for {}: {}", filename, e);
                Ok(Vec::new())
            }
        }
    }

    /// Get list of existing chapter files
    pub async fn list_chapter_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.config.chapters_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "txt") &&
               path.file_name().map_or(false, |name| name.to_string_lossy().contains("_chapters")) {
                files.push(path);
            }
        }

        Ok(files)
    }

    /// Delete a specific chapter file
    pub async fn delete_chapter_file(&self, series_name: &str) -> Result<bool> {
        let filename = format!("{}_chapters.txt", series_name);
        let file_path = self.config.chapters_dir.join(filename);
        
        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await?;
            info!("🗑️ Deleted chapter file: {}", file_path.display());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Validate chapters against video duration
    pub fn validate_chapters(&self, chapters: &[ChapterInfo], video_duration: f64) -> Vec<ChapterInfo> {
        if chapters.is_empty() {
            return Vec::new();
        }

        let mut valid_chapters = Vec::new();
        
        for chapter in chapters {
            // Skip chapters that are beyond video duration
            if chapter.timestamp > video_duration {
                warn!("⚠️ Skipping chapter '{}' at {}s (beyond video duration {}s)", 
                      chapter.title, chapter.timestamp, video_duration);
                continue;
            }

            // Skip chapters with invalid timestamps
            if chapter.timestamp < 0.0 {
                warn!("⚠️ Skipping chapter '{}' with negative timestamp", chapter.title);
                continue;
            }

            // Skip chapters with very short titles
            if chapter.title.len() < 3 {
                warn!("⚠️ Skipping chapter with too short title: '{}'", chapter.title);
                continue;
            }

            valid_chapters.push(chapter.clone());
        }

        // Ensure chapters are sorted by timestamp
        valid_chapters.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));

        // Remove duplicates based on timestamp proximity
        let mut deduped_chapters = Vec::new();
        for chapter in valid_chapters {
            let is_duplicate = deduped_chapters.iter().any(|existing: &ChapterInfo| {
                (existing.timestamp - chapter.timestamp).abs() < 5.0 // Within 5 seconds
            });

            if !is_duplicate {
                deduped_chapters.push(chapter);
            }
        }

        if deduped_chapters.len() != chapters.len() {
            info!("📝 Validated chapters: {} valid out of {} total", 
                  deduped_chapters.len(), chapters.len());
        }

        deduped_chapters
    }

}