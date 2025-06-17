/// Chapter detection and management module
/// 
/// This module provides functionality for detecting chapters in BJJ instructional videos
/// through web scraping of BJJfanatics.com and other methods.

pub mod scraper;
pub mod detector;
pub mod cache;
pub mod product_pages;

// Re-export main types
pub use detector::{ChapterDetector, ChapterDetectionConfig};
pub use scraper::BJJFanaticsScraper;
pub use cache::{ChapterCache, ChapterCacheManager};
pub use product_pages::ProductPagesFile;

use serde::{Deserialize, Serialize};

/// Represents a single chapter in a video
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterInfo {
    /// Chapter title/name
    pub title: String,
    /// Timestamp in seconds from start of video
    pub timestamp: f64,
    /// Optional description
    pub description: Option<String>,
}

/// Video metadata for chapter detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    /// Original filename
    pub filename: String,
    /// Video duration in seconds
    pub duration: f64,
    /// Extracted instructor name
    pub instructor: Option<String>,
    /// Extracted series name
    pub series: Option<String>,
    /// Volume/part number
    pub volume: Option<u32>,
}

/// Search terms extracted from video filename
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTerms {
    /// Instructor name parts
    pub instructor: Vec<String>,
    /// Series/technique name parts
    pub series: Vec<String>,
    /// Volume/part information
    pub volume: Option<u32>,
    /// Additional keywords
    pub keywords: Vec<String>,
}

impl SearchTerms {
    pub fn new() -> Self {
        Self {
            instructor: Vec::new(),
            series: Vec::new(),
            volume: None,
            keywords: Vec::new(),
        }
    }
}

impl Default for SearchTerms {
    fn default() -> Self {
        Self::new()
    }
}