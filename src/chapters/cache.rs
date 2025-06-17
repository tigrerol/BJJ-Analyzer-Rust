/// Chapter caching system for storing scraped chapter data
use super::{ChapterInfo, VideoMetadata};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Cached chapter data for a video series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterCache {
    /// Cache timestamp
    pub timestamp: u64,
    /// Cache key for identification
    pub cache_key: String,
    /// Source product URL
    pub product_url: String,
    /// Number of chapters
    pub chapter_count: usize,
    /// Chapter data
    pub chapters: Vec<ChapterInfo>,
    /// Video metadata (optional)
    pub video_metadata: Option<VideoMetadata>,
}

/// Manages chapter cache operations
#[derive(Clone)]
pub struct ChapterCacheManager {
    /// Cache directory path
    cache_dir: PathBuf,
    /// Cache TTL in hours
    cache_ttl_hours: u64,
}

impl ChapterCacheManager {
    /// Create a new cache manager
    pub fn new(cache_dir: PathBuf, cache_ttl_hours: u64) -> Self {
        Self {
            cache_dir,
            cache_ttl_hours,
        }
    }

    /// Initialize cache directory
    pub async fn initialize(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.cache_dir).await?;
        info!("📁 Chapter cache directory initialized: {}", self.cache_dir.display());
        Ok(())
    }

    /// Generate cache key from instructor and series info
    pub fn generate_cache_key(&self, instructor: &[String], series: &[String]) -> String {
        // Create a stable identifier from instructor and series info
        let instructor_part = instructor.join("_").to_lowercase();
        let series_part = series.iter().take(3).cloned().collect::<Vec<_>>().join("_").to_lowercase();
        
        // Create hash to handle long names and special characters
        let combined = format!("{}_{}", instructor_part, series_part);
        let mut hasher = DefaultHasher::new();
        combined.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Use first 8 characters of hash for readability
        format!("{}_{:08x}", 
            combined.chars().take(30).collect::<String>().replace(' ', "_"),
            hash
        )
    }

    /// Load cached chapters if valid
    pub async fn load_cached_chapters(&self, cache_key: &str) -> Option<ChapterCache> {
        let cache_path = self.cache_dir.join(format!("{}.json", cache_key));
        
        if !cache_path.exists() {
            debug!("Cache miss: no file found for key {}", cache_key);
            return None;
        }

        match tokio::fs::read_to_string(&cache_path).await {
            Ok(content) => {
                match serde_json::from_str::<ChapterCache>(&content) {
                    Ok(cache) => {
                        // Check if cache is still valid
                        if self.is_cache_valid(&cache) {
                            info!("📚 Cache hit: loaded {} chapters for {}", cache.chapters.len(), cache_key);
                            Some(cache)
                        } else {
                            info!("⏰ Cache expired for key: {}", cache_key);
                            // Optionally clean up expired cache
                            let _ = tokio::fs::remove_file(&cache_path).await;
                            None
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse cache file {}: {}", cache_path.display(), e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read cache file {}: {}", cache_path.display(), e);
                None
            }
        }
    }

    /// Save chapters to cache
    pub async fn save_cached_chapters(
        &self,
        cache_key: &str,
        chapters: Vec<ChapterInfo>,
        product_url: String,
        video_metadata: Option<VideoMetadata>,
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let cache = ChapterCache {
            timestamp: now,
            cache_key: cache_key.to_string(),
            product_url,
            chapter_count: chapters.len(),
            chapters,
            video_metadata,
        };

        let cache_path = self.cache_dir.join(format!("{}.json", cache_key));
        let json_content = serde_json::to_string_pretty(&cache)?;
        
        tokio::fs::write(&cache_path, json_content).await?;
        info!("💾 Saved {} chapters to cache: {}", cache.chapter_count, cache_key);
        
        Ok(())
    }

    /// Check if cache is still valid based on TTL
    fn is_cache_valid(&self, cache: &ChapterCache) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let cache_age_hours = (now - cache.timestamp) / 3600;
        cache_age_hours < self.cache_ttl_hours
    }

    /// Clean up expired cache files
    pub async fn cleanup_expired_cache(&self) -> Result<usize> {
        let mut cleaned_count = 0;
        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                // Try to load and check if expired
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(cache) = serde_json::from_str::<ChapterCache>(&content) {
                        if !self.is_cache_valid(&cache) {
                            if tokio::fs::remove_file(&path).await.is_ok() {
                                cleaned_count += 1;
                                debug!("🗑️ Removed expired cache: {}", path.display());
                            }
                        }
                    }
                }
            }
        }

        if cleaned_count > 0 {
            info!("🧹 Cleaned up {} expired cache files", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let mut stats = CacheStats::default();
        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                stats.total_files += 1;
                
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(cache) = serde_json::from_str::<ChapterCache>(&content) {
                        if self.is_cache_valid(&cache) {
                            stats.valid_files += 1;
                            stats.total_chapters += cache.chapters.len();
                        } else {
                            stats.expired_files += 1;
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Force invalidate a specific cache entry by key
    pub async fn invalidate_cache(&self, cache_key: &str) -> Result<bool> {
        let cache_path = self.cache_dir.join(format!("{}.json", cache_key));
        
        if cache_path.exists() {
            tokio::fs::remove_file(&cache_path).await?;
            info!("🗑️ Force invalidated cache for key: {}", cache_key);
            Ok(true)
        } else {
            debug!("Cache file not found for key: {}", cache_key);
            Ok(false)
        }
    }

    /// Force invalidate cache for a specific instructor/series combination
    pub async fn invalidate_series_cache(&self, instructor: &[String], series: &[String]) -> Result<bool> {
        let cache_key = self.generate_cache_key(instructor, series);
        self.invalidate_cache(&cache_key).await
    }

    /// Force clear all cached chapters (nuclear option)
    pub async fn clear_all_cache(&self) -> Result<usize> {
        let mut cleared_count = 0;
        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if tokio::fs::remove_file(&path).await.is_ok() {
                    cleared_count += 1;
                    debug!("🗑️ Removed cache file: {}", path.display());
                }
            }
        }

        if cleared_count > 0 {
            info!("🧹 Cleared {} cache files", cleared_count);
        }

        Ok(cleared_count)
    }

    /// List all cached series with metadata
    pub async fn list_cached_series(&self) -> Result<Vec<CachedSeriesInfo>> {
        let mut series_list = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(cache) = serde_json::from_str::<ChapterCache>(&content) {
                        let is_valid = self.is_cache_valid(&cache);
                        let age_hours = self.get_cache_age_hours(&cache);
                        
                        series_list.push(CachedSeriesInfo {
                            cache_key: cache.cache_key,
                            product_url: cache.product_url,
                            chapter_count: cache.chapter_count,
                            is_valid,
                            age_hours,
                            timestamp: cache.timestamp,
                        });
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        series_list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(series_list)
    }

    /// Get cache age in hours
    fn get_cache_age_hours(&self, cache: &ChapterCache) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        (now - cache.timestamp) / 3600
    }
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub total_files: usize,
    pub valid_files: usize,
    pub expired_files: usize,
    pub total_chapters: usize,
}

/// Information about a cached series
#[derive(Debug, Clone)]
pub struct CachedSeriesInfo {
    pub cache_key: String,
    pub product_url: String,
    pub chapter_count: usize,
    pub is_valid: bool,
    pub age_hours: u64,
    pub timestamp: u64,
}