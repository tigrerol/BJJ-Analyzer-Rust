/// BJJfanatics.com web scraper for chapter information
use super::{ChapterInfo, SearchTerms};
use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};
use crate::llm::filename_parsing::{parse_filename_with_llm, ParsedFilename};
use crate::llm::LLMConfig;
use crate::config::Config;

/// BJJfanatics web scraper
#[derive(Clone)]
pub struct BJJFanaticsScraper {
    client: Client,
    timeout: Duration,
}

impl BJJFanaticsScraper {
    /// Create a new scraper instance
    pub fn new(timeout_seconds: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// Main entry point: scrape chapters for a video filename using product pages file
    /// Only scrapes if no chapters file exists for the series
    pub async fn scrape_chapters(&self, video_filename: &str, video_dir: &Path, output_dir: Option<&Path>) -> Result<Vec<ChapterInfo>> {
        info!("🔍 Starting BJJfanatics scraping for: {}", video_filename);

        // Parse filename to extract search terms
        let search_terms = self.parse_video_filename(video_filename)?;
        debug!("Parsed search terms: {:?}", search_terms);

        // Check if chapters file already exists in video directory first
        let video_dir_chapters_path = self.get_chapters_file_path(&search_terms, Some(video_dir));
        if video_dir_chapters_path.exists() {
            info!("📁 Chapters file already exists in video directory: {}", video_dir_chapters_path.display());
            return self.load_chapters_from_file(&video_dir_chapters_path).await;
        }
        
        // Fallback: check output directory (for backwards compatibility)
        let output_dir_chapters_path = self.get_chapters_file_path(&search_terms, output_dir);
        if output_dir_chapters_path.exists() {
            info!("📁 Chapters file already exists in output directory: {}", output_dir_chapters_path.display());
            return self.load_chapters_from_file(&output_dir_chapters_path).await;
        }

        // No chapters file exists - use product pages file
        info!("🌐 No chapters file found, using product pages file...");
        
        // Load product pages file and find matching URL
        let product_pages = super::ProductPagesFile::load_from_directory(video_dir).await?;
        let product_url = product_pages.find_matching_url(&search_terms)?;
        info!("📄 Found matching product URL: {}", product_url);

        // Scrape the product page (gets ALL chapters for the series)
        let all_chapters = self.scrape_product_page(&product_url).await?;
        info!("📚 Extracted {} total chapters from series", all_chapters.len());

        // Save chapters to file in video directory for future use
        if !all_chapters.is_empty() {
            if let Err(e) = self.write_chapters_to_file(&all_chapters, &search_terms, Some(video_dir)).await {
                warn!("Failed to save chapters to file: {}", e);
            }
        }

        Ok(all_chapters)
    }

    /// Force scrape chapters (ignores existing files)
    pub async fn force_scrape_chapters(&self, video_filename: &str, video_dir: &Path, output_dir: Option<&Path>) -> Result<Vec<ChapterInfo>> {
        info!("🔄 Force scraping chapters for: {}", video_filename);

        // Parse filename to extract search terms
        let search_terms = self.parse_video_filename(video_filename)?;
        debug!("Parsed search terms: {:?}", search_terms);

        // Load product pages file and find matching URL
        let product_pages = super::ProductPagesFile::load_from_directory(video_dir).await?;
        let product_url = product_pages.find_matching_url(&search_terms)?;
        info!("📄 Found matching product URL: {}", product_url);

        // Scrape the product page (gets ALL chapters for the series)
        let all_chapters = self.scrape_product_page(&product_url).await?;
        info!("📚 Extracted {} total chapters from series", all_chapters.len());

        // Save chapters to file in video directory (overwrite existing)
        if !all_chapters.is_empty() {
            if let Err(e) = self.write_chapters_to_file(&all_chapters, &search_terms, Some(video_dir)).await {
                warn!("Failed to save chapters to file: {}", e);
            }
        }

        Ok(all_chapters)
    }

    /// Map chapters to specific video based on precise duration calculation
    pub fn map_chapters_to_video(
        &self, 
        all_chapters: Vec<ChapterInfo>, 
        video_duration_seconds: f64,
        volume_number: Option<u32>
    ) -> Vec<ChapterInfo> {
        if all_chapters.is_empty() {
            return Vec::new();
        }

        let volume_number = volume_number.unwrap_or(1);
        
        info!("🎯 Calculating chapters for video - Duration: {:.1}s, Volume: {}", 
              video_duration_seconds, volume_number);

        // Calculate video boundaries based on cumulative durations
        let video_boundaries = self.calculate_video_boundaries(&all_chapters, video_duration_seconds);
        
        if video_boundaries.is_empty() {
            warn!("⚠️ Could not calculate video boundaries, filtering by duration only");
            return self.filter_chapters_by_duration(&all_chapters, video_duration_seconds);
        }

        // Find the boundary for the requested volume
        if let Some(boundary) = video_boundaries.get((volume_number - 1) as usize) {
            let chapters_for_video = self.extract_chapters_for_boundary(&all_chapters, boundary);
            info!("✅ Calculated {} chapters for volume {} (time range: {:.1}s - {:.1}s)", 
                  chapters_for_video.len(), volume_number, boundary.start_time, boundary.end_time);
            return chapters_for_video;
        }

        // Fallback: if volume not found, filter by duration
        warn!("⚠️ Volume {} not found in boundaries, filtering by duration", volume_number);
        self.filter_chapters_by_duration(&all_chapters, video_duration_seconds)
    }

    /// Calculate video boundaries based on actual chapter timestamps and video duration
    fn calculate_video_boundaries(&self, chapters: &[ChapterInfo], single_video_duration: f64) -> Vec<VideoBoundary> {
        if chapters.is_empty() || single_video_duration <= 0.0 {
            return Vec::new();
        }

        // Sort chapters by timestamp to ensure proper order
        let mut sorted_chapters = chapters.to_vec();
        sorted_chapters.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));

        let max_timestamp = sorted_chapters.last().map(|ch| ch.timestamp).unwrap_or(0.0);
        
        // If all chapters fit within single video duration, it's a single volume
        if max_timestamp <= single_video_duration {
            debug!("All chapters fit in single video duration ({:.1}s <= {:.1}s)", max_timestamp, single_video_duration);
            return vec![VideoBoundary {
                volume: 1,
                start_time: 0.0,
                end_time: single_video_duration,
            }];
        }

        // Calculate how many videos are needed based on total content vs single video duration
        let total_content_duration = max_timestamp;
        let estimated_video_count = (total_content_duration / single_video_duration).ceil() as u32;
        
        info!("📊 Calculating boundaries: Total content {:.1}s, Single video {:.1}s, Estimated {} videos", 
              total_content_duration, single_video_duration, estimated_video_count);

        // Create boundaries for each video
        let mut boundaries = Vec::new();
        for i in 0..estimated_video_count {
            let start_time = i as f64 * single_video_duration;
            let end_time = ((i + 1) as f64 * single_video_duration).min(total_content_duration);
            
            boundaries.push(VideoBoundary {
                volume: i + 1,
                start_time,
                end_time,
            });
        }

        debug!("Calculated {} video boundaries", boundaries.len());
        for boundary in &boundaries {
            debug!("  Volume {}: {:.1}s - {:.1}s", boundary.volume, boundary.start_time, boundary.end_time);
        }

        boundaries
    }

    /// Extract chapters that fall within a specific video boundary
    fn extract_chapters_for_boundary(&self, chapters: &[ChapterInfo], boundary: &VideoBoundary) -> Vec<ChapterInfo> {
        let mut video_chapters: Vec<ChapterInfo> = chapters.iter()
            .filter(|chapter| chapter.timestamp >= boundary.start_time && chapter.timestamp < boundary.end_time)
            .map(|chapter| {
                // Adjust timestamp to be relative to the start of this video
                ChapterInfo {
                    title: chapter.title.clone(),
                    timestamp: chapter.timestamp - boundary.start_time,
                    description: chapter.description.clone(),
                }
            })
            .collect();

        // Sort by adjusted timestamp
        video_chapters.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));

        video_chapters
    }

    /// Filter chapters that have timestamps within the video duration (simple fallback)
    fn filter_chapters_by_duration(&self, chapters: &[ChapterInfo], max_duration: f64) -> Vec<ChapterInfo> {
        chapters.iter()
            .filter(|chapter| chapter.timestamp <= max_duration)
            .cloned()
            .collect()
    }

    /// Parse video filename to extract search terms using LLM with fallback to regex
    pub async fn parse_video_filename_with_llm(&self, filename: &str, config: Option<&Config>) -> Result<SearchTerms> {
        if let Some(config) = config {
            // Try LLM parsing first
            match self.parse_filename_with_llm_internal(filename, config).await {
                Ok(search_terms) => return Ok(search_terms),
                Err(e) => {
                    warn!("LLM filename parsing failed: {}, falling back to regex", e);
                }
            }
        }
        
        // Fallback to regex parsing
        self.parse_video_filename_regex(filename)
    }
    
    /// Parse video filename using LLM
    async fn parse_filename_with_llm_internal(&self, filename: &str, config: &Config) -> Result<SearchTerms> {
        info!("🤖 Parsing filename with LLM: {}", filename);
        
        let llm_config = LLMConfig {
            provider: config.llm.provider.clone(),
            endpoint: config.llm.endpoint.clone(),
            api_key: config.llm.api_key.clone(),
            model: config.llm.model.clone(),
            max_tokens: config.llm.max_tokens,
            temperature: config.llm.temperature,
            timeout_seconds: config.llm.timeout_seconds,
        };
        
        let prompt_path = config.llm.prompts.prompt_dir.join(&config.llm.prompts.filename_parsing_file);
        
        let parsed = parse_filename_with_llm(filename, llm_config, Some(&prompt_path)).await?;
        
        let mut search_terms = SearchTerms::new();
        
        // Convert ParsedFilename to SearchTerms
        if let Some(instructor) = parsed.instructor {
            search_terms.instructor = self.split_camel_case(&instructor);
        }
        
        if let Some(series_name) = parsed.series_name {
            search_terms.series = self.split_camel_case(&series_name);
        }
        
        search_terms.volume = parsed.part_number;
        
        info!("🤖 LLM parsed search terms: {:?}", search_terms);
        Ok(search_terms)
    }

    /// Parse video filename to extract search terms (legacy regex method)
    pub fn parse_video_filename(&self, filename: &str) -> Result<SearchTerms> {
        self.parse_video_filename_regex(filename)
    }
    
    /// Parse video filename using regex patterns (fallback method)
    fn parse_video_filename_regex(&self, filename: &str) -> Result<SearchTerms> {
        let mut search_terms = SearchTerms::new();
        
        info!("🔍 Parsing filename: {}", filename);
        
        // Remove file extension
        let name = filename.trim_end_matches(".mp4")
            .trim_end_matches(".mkv")
            .trim_end_matches(".avi")
            .trim_end_matches(".mov");
            
        info!("📝 Cleaned filename: {}", name);

        // BJJ video filename patterns (CamelCase format)
        let patterns = vec![
            // "SeriesNameByFirstnameLastname1" - clear "By" separator (uppercase)
            r"^(.+?)By([A-Z][a-z]+[A-Z][a-z]+)(?:Vol|VOL)?(\d+)(?:New)?$",
            // "SeriesNamebyFirstnameLastname1" - clear "by" separator (lowercase)
            r"^(.+?)by([A-Z][a-z]+[A-Z][a-z]+)(?:Vol|VOL)?(\d+)(?:New)?$",
            // "SeriesNameByFirstnameLastname" without volume (uppercase)
            r"^(.+?)By([A-Z][a-z]+[A-Z][a-z]+)$",
            // "SeriesNamebyFirstnameLastname" without volume (lowercase)
            r"^(.+?)by([A-Z][a-z]+[A-Z][a-z]+)$",
            // Special pattern for known instructor names + Vol
            r"^(MikeyMusumeci|GordonRyan|CraigJones|KeenanCornelius)(.*)(?:Vol|VOL)(\d+)(?:New)?$",
            // General "FirstnameLastnameSeriesNameVol1" - more flexible instructor capture
            r"^([A-Z][a-z]*[A-Z][a-z]+)(.+?)(?:Vol|VOL)(\d+)(?:New)?$",
            // "FirstnameLastnameSeriesName1" - ends with just a number
            r"^([A-Z][a-z]*[A-Z][a-z]+)(.+?)(\d+)(?:New)?$",
            // "FirstnameLastnameSeriesName" without volume
            r"^([A-Z][a-z]*[A-Z][a-z]+)(.+?)$",
        ];

        for (i, pattern) in patterns.iter().enumerate() {
            info!("🧪 Testing pattern {}: {}", i, pattern);
            if let Ok(re) = Regex::new(pattern) {
                if let Some(captures) = re.captures(name) {
                    info!("✅ Pattern {} matched! Captures: {:?}", i, captures);
                    
                    match i {
                        0 => {
                            // "SeriesNameByFirstnameLastname1" - series, instructor, volume (uppercase)
                            if captures.len() == 4 {
                                let series = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let instructor = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                                let volume = captures.get(3).and_then(|m| m.as_str().parse::<u32>().ok());

                                search_terms.series = self.split_camel_case(series);
                                search_terms.instructor = self.split_camel_case(instructor);
                                search_terms.volume = volume;
                                
                                debug!("'By' pattern with volume: instructor={:?}, series={:?}, volume={:?}", 
                                    search_terms.instructor, search_terms.series, search_terms.volume);
                                return Ok(search_terms);
                            }
                        }
                        1 => {
                            // "SeriesNamebyFirstnameLastname1" - series, instructor, volume (lowercase)
                            if captures.len() == 4 {
                                let series = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let instructor = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                                let volume = captures.get(3).and_then(|m| m.as_str().parse::<u32>().ok());

                                search_terms.series = self.split_camel_case(series);
                                search_terms.instructor = self.split_camel_case(instructor);
                                search_terms.volume = volume;
                                
                                info!("📊 'by' pattern with volume: instructor={:?}, series={:?}, volume={:?}", 
                                    search_terms.instructor, search_terms.series, search_terms.volume);
                                return Ok(search_terms);
                            }
                        }
                        2 => {
                            // "SeriesNameByFirstnameLastname" - no volume (uppercase)
                            if captures.len() == 3 {
                                let series = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let instructor = captures.get(2).map(|m| m.as_str()).unwrap_or("");

                                search_terms.series = self.split_camel_case(series);
                                search_terms.instructor = self.split_camel_case(instructor);
                                
                                debug!("'By' pattern without volume: instructor={:?}, series={:?}", 
                                    search_terms.instructor, search_terms.series);
                                return Ok(search_terms);
                            }
                        }
                        3 => {
                            // "SeriesNamebyFirstnameLastname" - no volume (lowercase)
                            if captures.len() == 3 {
                                let series = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let instructor = captures.get(2).map(|m| m.as_str()).unwrap_or("");

                                search_terms.series = self.split_camel_case(series);
                                search_terms.instructor = self.split_camel_case(instructor);
                                
                                debug!("'by' pattern without volume: instructor={:?}, series={:?}", 
                                    search_terms.instructor, search_terms.series);
                                return Ok(search_terms);
                            }
                        }
                        4 => {
                            // Special known instructor names + Vol
                            if captures.len() == 4 {
                                let instructor = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let series = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                                let volume = captures.get(3).and_then(|m| m.as_str().parse::<u32>().ok());

                                search_terms.instructor = self.split_camel_case(instructor);
                                search_terms.series = if series.is_empty() { 
                                    vec!["Unknown".to_string()] 
                                } else { 
                                    self.split_camel_case(series) 
                                };
                                search_terms.volume = volume;
                                
                                debug!("Special instructor + Vol pattern: instructor={:?}, series={:?}, volume={:?}", 
                                    search_terms.instructor, search_terms.series, search_terms.volume);
                                return Ok(search_terms);
                            }
                        }
                        5 => {
                            // General "FirstnameLastnameSeriesNameVol1" - instructor, series, volume
                            if captures.len() == 4 {
                                let instructor = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let series = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                                let volume = captures.get(3).and_then(|m| m.as_str().parse::<u32>().ok());

                                search_terms.instructor = self.split_camel_case(instructor);
                                search_terms.series = self.split_camel_case(series);
                                search_terms.volume = volume;
                                
                                debug!("General FirstLast + Series + Vol pattern: instructor={:?}, series={:?}, volume={:?}", 
                                    search_terms.instructor, search_terms.series, search_terms.volume);
                                return Ok(search_terms);
                            }
                        }
                        6 => {
                            // "FirstnameLastnameSeriesName1" - instructor, series, volume  
                            if captures.len() == 4 {
                                let instructor = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let series = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                                let volume = captures.get(3).and_then(|m| m.as_str().parse::<u32>().ok());

                                search_terms.instructor = self.split_camel_case(instructor);
                                search_terms.series = self.split_camel_case(series);
                                search_terms.volume = volume;
                                
                                debug!("FirstLast + Series + Number pattern: instructor={:?}, series={:?}, volume={:?}", 
                                    search_terms.instructor, search_terms.series, search_terms.volume);
                                return Ok(search_terms);
                            }
                        }
                        7 => {
                            // "FirstnameLastnameSeriesName" - no volume
                            if captures.len() == 3 {
                                let instructor = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                                let series = captures.get(2).map(|m| m.as_str()).unwrap_or("");

                                search_terms.instructor = self.split_camel_case(instructor);
                                search_terms.series = self.split_camel_case(series);
                                
                                debug!("FirstLast + Series pattern: instructor={:?}, series={:?}", 
                                    search_terms.instructor, search_terms.series);
                                return Ok(search_terms);
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }

        // Fallback: Assume FirstnameLastname + SeriesName pattern
        warn!("❌ No pattern matched, using fallback parsing for: {}", name);
        
        // Split into words
        let words = self.split_camel_case(name);
        
        if words.len() >= 2 {
            // Assume first two words are Firstname + Lastname, rest is series
            search_terms.instructor = words[..2].to_vec();
            
            // Check for volume at the end
            if let Some(last_word) = words.last() {
                if let Ok(volume) = last_word.parse::<u32>() {
                    // Last word is a number - it's the volume
                    search_terms.volume = Some(volume);
                    if words.len() > 3 {
                        search_terms.series = words[2..words.len()-1].to_vec();
                    } else {
                        search_terms.series = vec!["Unknown".to_string()];
                    }
                } else {
                    // No volume number, all remaining words are series
                    search_terms.series = words[2..].to_vec();
                }
            } else {
                search_terms.series = vec!["Unknown".to_string()];
            }
        } else if words.len() == 1 {
            // Single word - try to extract volume number
            if let Ok(re) = Regex::new(r"^(.+?)(\d+)$") {
                if let Some(captures) = re.captures(name) {
                    let text_part = captures.get(1).map(|m| m.as_str()).unwrap_or(name);
                    let volume = captures.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
                    
                    search_terms.instructor = vec![text_part.to_string()];
                    search_terms.series = vec!["Unknown".to_string()];
                    search_terms.volume = volume;
                } else {
                    search_terms.instructor = words;
                    search_terms.series = vec!["Unknown".to_string()];
                }
            }
        } else {
            // Empty - return defaults
            search_terms.instructor = vec!["Unknown".to_string()];
            search_terms.series = vec!["Unknown".to_string()];
        }
        
        debug!("Fallback parsing: instructor={:?}, series={:?}, volume={:?}", 
            search_terms.instructor, search_terms.series, search_terms.volume);

        Ok(search_terms)
    }

    /// Split CamelCase strings into separate words
    fn split_camel_case(&self, input: &str) -> Vec<String> {
        if let Ok(re) = Regex::new(r"([a-z])([A-Z])") {
            let spaced = re.replace_all(input, "$1 $2");
            spaced.split_whitespace()
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![input.to_string()]
        }
    }

    /// Split combined instructor and series name
    /// Uses heuristics to identify where instructor name ends and series begins
    fn split_instructor_and_series(&self, combined: &str) -> (Vec<String>, Vec<String>) {
        // Common BJJ instructor names (helps identify instructor part)
        let common_instructor_names = vec![
            "Ryan", "Musumeci", "Garcia", "Daher", "Telles", "Miyao", "Mendes", 
            "Galvao", "Cobrinha", "Lepri", "Atos", "Gordon", "Mikey", "Keenan",
            "Bernardo", "Lachlan", "Craig", "Jones", "Tonon", "Danaher",
            "GordonRyan", "MikeyMusumeci", "CraigJones", "KeenanCornelius"
        ];

        // Split into words first
        let words = self.split_camel_case(combined);
        
        // First, check if the combined string starts with a known instructor name
        for instructor_name in &common_instructor_names {
            if combined.starts_with(instructor_name) {
                // Found instructor at the beginning
                let instructor = self.split_camel_case(instructor_name);
                let remainder = &combined[instructor_name.len()..];
                let series = if remainder.is_empty() {
                    vec!["Unknown".to_string()]
                } else {
                    self.split_camel_case(remainder)
                };
                return (instructor, series);
            }
        }

        // Look for known instructor names in words
        for (i, word) in words.iter().enumerate() {
            if common_instructor_names.iter().any(|name| word.contains(name) || name.contains(word)) {
                // Found instructor name - split here
                let instructor = words[..=i].to_vec();
                let series = if i + 1 < words.len() {
                    words[i + 1..].to_vec()
                } else {
                    vec!["Unknown".to_string()]
                };
                return (instructor, series);
            }
        }

        // Fallback: assume first 1-2 words are instructor, rest is series
        if words.len() >= 3 {
            // If 3+ words, take first 2 as instructor
            let instructor = words[..2].to_vec();
            let series = words[2..].to_vec();
            (instructor, series)
        } else if words.len() == 2 {
            // If exactly 2 words, first is instructor, second is series
            let instructor = vec![words[0].clone()];
            let series = vec![words[1].clone()];
            (instructor, series)
        } else if words.len() == 1 {
            // Single word - treat as instructor with unknown series
            let instructor = words.clone();
            let series = vec!["Unknown".to_string()];
            (instructor, series)
        } else {
            // Empty - return defaults
            (vec!["Unknown".to_string()], vec!["Unknown".to_string()])
        }
    }



    /// Scrape chapter information from a product page
    async fn scrape_product_page(&self, product_url: &str) -> Result<Vec<ChapterInfo>> {
        info!("📄 Fetching product page: {}", product_url);

        let response = self.client.get(product_url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("HTTP error {}: {}", response.status(), product_url));
        }

        let html_content = response.text().await?;
        debug!("📄 Downloaded {} characters of HTML content", html_content.len());


        let document = Html::parse_document(&html_content);
        
        // Try multiple selectors based on successful Python implementation
        
        // Method 1: Primary - Extract from course content structure (like Python)
        if let Some(chapters) = self.extract_from_course_content(&document) {
            if !chapters.is_empty() && self.validate_chapters(&chapters) {
                info!("✅ Found {} chapters using course content method", chapters.len());
                return Ok(chapters);
            }
        }

        // Method 2: Fallback - product-tabs sections
        if let Some(chapters) = self.extract_from_product_tabs(&document) {
            if !chapters.is_empty() && self.validate_chapters(&chapters) {
                info!("✅ Found {} chapters using product tabs method", chapters.len());
                return Ok(chapters);
            }
        }

        // Method 3: General selectors (following Python fallback approach)
        let chapter_selectors = vec![
            "div[class*='content'] table tr",
            "div[class*='curriculum'] table tr", 
            "div[class*='chapters'] table tr",
            "table tr",
            "div.product-tabs li",
            "ul li", 
            "ol li",
        ];

        for selector_str in chapter_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                let chapters = self.extract_chapters_with_selector(&document, &selector)?;
                if !chapters.is_empty() && self.validate_chapters(&chapters) {
                    info!("✅ Found {} chapters using selector: {}", chapters.len(), selector_str);
                    return Ok(chapters);
                }
            }
        }

        // Fallback: look for timestamp patterns in any text
        warn!("No chapters found with specific selectors, trying text pattern matching");
        let text_chapters = self.extract_chapters_from_text(&html_content)?;
        
        if !text_chapters.is_empty() && self.validate_chapters(&text_chapters) {
            return Ok(text_chapters);
        }

        // If all else fails, return empty - don't return invalid data
        warn!("No valid chapters found for URL: {}", product_url);
        Ok(Vec::new())
    }


    /// Validate that chapters contain reasonable data (not JavaScript or HTML)
    fn validate_chapters(&self, chapters: &[ChapterInfo]) -> bool {
        if chapters.is_empty() {
            return false;
        }

        for chapter in chapters {
            // Check for common indicators of invalid content
            if chapter.title.len() > 200 {
                // Very long titles are likely JavaScript/HTML
                debug!("Rejecting chapter with overly long title: {} chars", chapter.title.len());
                return false;
            }
            
            if chapter.title.contains("GMT") || 
               chapter.title.contains("document.cookie") ||
               chapter.title.contains("JSON.stringify") ||
               chapter.title.contains("function") ||
               chapter.title.contains("var ") ||
               chapter.title.contains("const ") ||
               chapter.title.contains("let ") ||
               chapter.title.contains("window.") ||
               chapter.title.contains("};") {
                debug!("Rejecting chapter containing JavaScript: {}", &chapter.title[..50.min(chapter.title.len())]);
                return false;
            }

            if chapter.title.contains("<div") ||
               chapter.title.contains("<span") ||
               chapter.title.contains("<script") ||
               chapter.title.contains("</div>") ||
               chapter.title.contains("</span>") {
                debug!("Rejecting chapter containing HTML: {}", &chapter.title[..50.min(chapter.title.len())]);
                return false;
            }

            // Timestamps should be reasonable (0-10800 seconds = 3 hours max)
            if chapter.timestamp < 0.0 || chapter.timestamp > 10800.0 {
                debug!("Rejecting chapter with invalid timestamp: {}", chapter.timestamp);
                return false;
            }
        }

        true
    }

    /// Method 1: Extract from course content structure (following Python implementation)
    fn extract_from_course_content(&self, document: &Html) -> Option<Vec<ChapterInfo>> {
        // Look for the specific BJJ Fanatics course content structure
        // div.product__course-content-accordion (the main container)
        debug!("🔍 Trying to find div.product__course-content-accordion");
        let container_selector = Selector::parse("div.product__course-content-accordion").ok()?;
        let container = document.select(&container_selector).next();
        
        if container.is_none() {
            debug!("❌ No div.product__course-content-accordion found");
            return None;
        }
        let container = container.unwrap();
        
        info!("✅ Found course content accordion container");
        
        let mut all_chapters = Vec::new();
        
        // Find all volume sections (h3.product__course-title)
        let volume_selector = Selector::parse("h3.product__course-title").ok()?;
        let content_selector = Selector::parse("div.product__course-content").ok()?;
        
        let volume_headers: Vec<_> = container.select(&volume_selector).collect();
        let content_divs: Vec<_> = container.select(&content_selector).collect();
        
        debug!("Found {} volume headers and {} content sections", volume_headers.len(), content_divs.len());
        
        // Process each volume
        for (volume_idx, content_div) in content_divs.iter().enumerate() {
            // Get volume title from corresponding header
            let volume_title = if volume_idx < volume_headers.len() {
                volume_headers[volume_idx].text().collect::<String>().trim().to_string()
            } else {
                format!("Volume {}", volume_idx + 1)
            };
            
            debug!("Processing volume: {}", volume_title);
            
            // Find table in this content section
            let table_selector = Selector::parse("table tbody").ok()?;
            if let Some(tbody) = content_div.select(&table_selector).next() {
                // Extract chapters from table rows
                let row_selector = Selector::parse("tr").ok()?;
                for row in tbody.select(&row_selector) {
                    let cell_selector = Selector::parse("td").ok()?;
                    let cells: Vec<_> = row.select(&cell_selector).collect();
                    if cells.len() >= 2 {
                        let title = cells[0].text().collect::<Vec<_>>().join(" ").trim().to_string();
                        let time_range = cells[1].text().collect::<Vec<_>>().join(" ").trim().to_string();
                        
                        debug!("Found chapter: '{}' with time: '{}'", title, time_range);
                        
                        if self.is_valid_chapter_data(&title, &time_range) {
                            // For multi-volume series, don't use cumulative time - let each volume have its own timestamps
                            if let Some(timestamp_seconds) = self.parse_timestamp_range(&time_range, 0.0) {
                                let chapter_title = if !title.is_empty() {
                                    format!("{} - {}", volume_title, title)
                                } else {
                                    format!("{} - Unknown Chapter", volume_title)
                                };
                                
                                all_chapters.push(ChapterInfo {
                                    title: self.clean_chapter_title(&chapter_title),
                                    timestamp: timestamp_seconds,
                                    description: None,
                                });
                                debug!("Extracted chapter: '{}' at {}s", chapter_title, timestamp_seconds);
                            }
                        }
                    }
                }
                
                // Note: Each volume has its own timestamp range (not cumulative)
            }
        }
        
        if !all_chapters.is_empty() {
            // Sort by timestamp
            all_chapters.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));
            debug!("Successfully extracted {} chapters from course content", all_chapters.len());
            return Some(all_chapters);
        }
        
        None
    }

    /// Find volume title for a table (h3 element before the table)
    fn find_volume_title(&self, _table_element: &scraper::ElementRef) -> String {
        // For now, return empty string - this is a complex DOM traversal
        // The Python version handles this, but for Rust it's more complex
        // We can implement this later if needed
        String::new()
    }

    /// Method 2: Extract from product-tabs sections (following Python implementation)
    fn extract_from_product_tabs(&self, document: &Html) -> Option<Vec<ChapterInfo>> {
        if let Ok(selector) = Selector::parse("div.product-tabs") {
            let mut all_chapters = Vec::new();
            
            for section in document.select(&selector) {
                if let Ok(li_selector) = Selector::parse("li") {
                    for item in section.select(&li_selector) {
                        let text = item.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        if self.looks_like_chapter_entry(&text) {
                            if let Some(chapter) = self.parse_chapter_text(&text) {
                                all_chapters.push(chapter);
                                debug!("Extracted product tabs chapter: {}", text);
                            }
                        }
                    }
                }
            }
            
            if !all_chapters.is_empty() {
                // Sort by timestamp and remove duplicates
                all_chapters.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));
                all_chapters.dedup_by(|a, b| a.title == b.title && (a.timestamp - b.timestamp).abs() < 5.0);
                return Some(all_chapters);
            }
        }
        
        None
    }

    /// Check if text looks like a chapter entry (following Python logic)
    fn looks_like_chapter_entry(&self, text: &str) -> bool {
        // Must contain a colon and be reasonable length
        if !text.contains(':') || text.len() < 5 || text.len() > 300 {
            return false;
        }

        // Filter out non-chapter content
        let exclude_patterns = vec![
            "price", "shipping", "cart", "buy", "purchase",
            "email", "phone", "address", "copyright", "policy",
            "login", "register", "account", "password", "sign",
            "search", "menu", "navigation", "footer", "header"
        ];

        let text_lower = text.to_lowercase();
        for pattern in exclude_patterns {
            if text_lower.contains(pattern) {
                return false;
            }
        }

        // Must have time pattern
        let time_patterns = vec![
            r"\d+:\d+",           // MM:SS
            r"\d+\.\d+",          // Decimal minutes  
            r"\d+min",            // Minutes
            r"\d+sec",            // Seconds
            r"\d+\s*-\s*\d+",     // Range like "0:00 - 1:00"
        ];

        use regex::Regex;
        time_patterns.iter().any(|pattern| {
            if let Ok(re) = Regex::new(pattern) {
                re.is_match(text)
            } else {
                false
            }
        })
    }

    /// Extract chapters using a specific CSS selector
    fn extract_chapters_with_selector(&self, document: &Html, selector: &Selector) -> Result<Vec<ChapterInfo>> {
        let mut chapters = Vec::new();

        for element in document.select(selector) {
            // Check if this is a table row - handle differently like Python implementation
            if element.value().name() == "tr" {
                // Extract from table cells (td elements)
                let cells: Vec<_> = element.select(&Selector::parse("td").unwrap()).collect();
                if cells.len() >= 2 {
                    let title = cells[0].text().collect::<Vec<_>>().join(" ").trim().to_string();
                    let timestamp = cells[1].text().collect::<Vec<_>>().join(" ").trim().to_string();
                    
                    // Skip header rows
                    if title.to_uppercase() != "CHAPTER TITLE" && 
                       timestamp.to_uppercase() != "START TIME" &&
                       !title.is_empty() && !timestamp.is_empty() &&
                       self.is_valid_chapter_data(&title, &timestamp) {
                        
                        if let Some(timestamp_seconds) = self.parse_timestamp(&timestamp) {
                            chapters.push(ChapterInfo {
                                title: self.clean_chapter_title(&title),
                                timestamp: timestamp_seconds,
                                description: None,
                            });
                            debug!("Extracted table chapter: {} -> {}", title, timestamp);
                        }
                    }
                }
            } else {
                // Handle non-table elements (li, div, etc.)
                let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                
                if let Some(chapter) = self.parse_chapter_text(&text) {
                    chapters.push(chapter);
                }
            }
        }

        Ok(chapters)
    }

    /// Check if title and timestamp look like valid chapter data (from Python implementation)
    pub fn is_valid_chapter_data(&self, title: &str, timestamp: &str) -> bool {
        if title.is_empty() || timestamp.is_empty() {
            return false;
        }
        
        // Skip header rows
        let title_upper = title.to_uppercase();
        let timestamp_upper = timestamp.to_uppercase();
        
        if ["CHAPTER TITLE", "TITLE", "NAME"].contains(&title_upper.as_str()) ||
           ["START TIME", "TIME", "TIMESTAMP"].contains(&timestamp_upper.as_str()) {
            return false;
        }
        
        // Must have some time pattern
        let time_patterns = vec![
            r"\d+:\d+",         // MM:SS
            r"\d+\.\d+",        // Decimal minutes
            r"\d+min",          // Minutes
            r"\d+sec",          // Seconds
            r"\d+\s*-\s*\d+",   // Range like "0:00 - 1:00"
        ];
        
        use regex::Regex;
        time_patterns.iter().any(|pattern| {
            if let Ok(re) = Regex::new(pattern) {
                re.is_match(timestamp)
            } else {
                false
            }
        })
    }

    /// Extract chapters by searching for timestamp patterns in text
    fn extract_chapters_from_text(&self, html_content: &str) -> Result<Vec<ChapterInfo>> {
        let mut chapters = Vec::new();

        // Look for patterns like "Chapter Title - 1:23" or "1:23 - Chapter Title"
        let timestamp_patterns = vec![
            r"(\d{1,2}:\d{2})\s*[-–]\s*(.+)",  // "1:23 - Title"
            r"(.+?)\s*[-–]\s*(\d{1,2}:\d{2})", // "Title - 1:23"
            r"(\d{1,2}:\d{2})\s+(.+)",        // "1:23 Title"
        ];

        for pattern in timestamp_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(html_content) {
                    if cap.len() >= 3 {
                        let (timestamp_str, title) = if cap[1].contains(':') {
                            (&cap[1], &cap[2])
                        } else {
                            (&cap[2], &cap[1])
                        };

                        if let Some(timestamp) = self.parse_timestamp(timestamp_str) {
                            let clean_title = self.clean_chapter_title(title);
                            if !clean_title.is_empty() && clean_title.len() > 3 {
                                chapters.push(ChapterInfo {
                                    title: clean_title,
                                    timestamp,
                                    description: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates and sort by timestamp
        chapters.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));
        chapters.dedup_by(|a, b| a.title == b.title && (a.timestamp - b.timestamp).abs() < 5.0);

        Ok(chapters)
    }

    /// Parse a single chapter text entry
    fn parse_chapter_text(&self, text: &str) -> Option<ChapterInfo> {
        // Look for timestamp patterns
        let patterns = vec![
            r"^(\d{1,2}:\d{2})\s*[-–]?\s*(.+)$",  // "1:23 - Title" or "1:23 Title"
            r"^(.+?)\s*[-–]\s*(\d{1,2}:\d{2})$",  // "Title - 1:23"
        ];

        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(cap) = re.captures(text.trim()) {
                    let (timestamp_str, title) = if cap[1].contains(':') {
                        (&cap[1], &cap[2])
                    } else {
                        (&cap[2], &cap[1])
                    };

                    if let Some(timestamp) = self.parse_timestamp(timestamp_str) {
                        let clean_title = self.clean_chapter_title(title);
                        if !clean_title.is_empty() && clean_title.len() > 3 {
                            return Some(ChapterInfo {
                                title: clean_title,
                                timestamp,
                                description: None,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Parse timestamp string to seconds
    pub fn parse_timestamp(&self, timestamp_str: &str) -> Option<f64> {
        // Handle both colon and period separators (e.g., "3:48:00" or "3.48.00")
        let normalized = timestamp_str.replace('.', ":");
        let parts: Vec<&str> = normalized.split(':').collect();
        
        match parts.len() {
            2 => {
                // MM:SS format
                if let (Ok(minutes), Ok(seconds)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    Some((minutes * 60 + seconds) as f64)
                } else {
                    None
                }
            }
            3 => {
                // For BJJfanatics, assume MM:SS:FF format (minutes:seconds:frames) for values under 60 in first part
                if let (Ok(first), Ok(second), Ok(third)) = 
                    (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                    
                    // If first part is under 60, treat as MM:SS:FF (minutes:seconds:frames)
                    // Otherwise treat as HH:MM:SS (hours:minutes:seconds)
                    if first < 60 {
                        // MM:SS:FF - ignore frames (third part)
                        Some((first * 60 + second) as f64)
                    } else {
                        // HH:MM:SS
                        Some((first * 3600 + second * 60 + third) as f64)
                    }
                } else {
                    None
                }
            }
            _ => None
        }
    }

    /// Parse timestamp range string to start time in seconds, adding cumulative time
    /// Handles formats like "0:00 - 1:00", "1:00 - 6:56", "39:50 +"
    pub fn parse_timestamp_range(&self, time_range_str: &str, cumulative_time: f64) -> Option<f64> {
        let time_range = time_range_str.trim();
        debug!("Parsing timestamp range: '{}' with cumulative: {}s", time_range, cumulative_time);
        
        // Handle formats like "39:50 +" (no end time)
        if time_range.ends_with(" +") || time_range.ends_with("+") {
            let start_time_str = time_range.trim_end_matches(" +").trim_end_matches("+").trim();
            if let Some(start_seconds) = self.parse_timestamp(start_time_str) {
                let total_time = cumulative_time + start_seconds;
                debug!("Parsed open-ended range: {} -> {}s (total)", start_time_str, total_time);
                return Some(total_time);
            }
        }
        
        // Handle range formats like "0:00 - 1:00" or "1:00 - 6:56"
        if time_range.contains(" - ") {
            let parts: Vec<&str> = time_range.split(" - ").collect();
            if parts.len() == 2 {
                let start_time_str = parts[0].trim();
                if let Some(start_seconds) = self.parse_timestamp(start_time_str) {
                    let total_time = cumulative_time + start_seconds;
                    debug!("Parsed range: {} -> {}s (total)", start_time_str, total_time);
                    return Some(total_time);
                }
            }
        }
        
        // Fallback: try to parse as single timestamp
        if let Some(start_seconds) = self.parse_timestamp(time_range) {
            let total_time = cumulative_time + start_seconds;
            debug!("Parsed single timestamp: {} -> {}s (total)", time_range, total_time);
            return Some(total_time);
        }
        
        debug!("Failed to parse timestamp range: '{}'", time_range);
        None
    }

    /// Clean and normalize chapter title
    pub fn clean_chapter_title(&self, title: &str) -> String {
        title.trim()
            .trim_matches(|c| c == '-' || c == '–' || c == '|' || c == ':')
            .trim()
            .to_string()
    }

    /// Write chapters to a file with the series name
    pub async fn write_chapters_to_file(
        &self,
        chapters: &[ChapterInfo],
        search_terms: &SearchTerms,
        output_dir: Option<&Path>
    ) -> Result<std::path::PathBuf> {
        if chapters.is_empty() {
            return Err(anyhow!("No chapters to write"));
        }

        // Create series name from search terms
        let series_name = self.create_series_filename(search_terms);
        
        // Determine output directory
        let output_dir = output_dir.unwrap_or_else(|| Path::new("chapters"));
        
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).await?;
            info!("📁 Created directory: {}", output_dir.display());
        }

        // Create filename
        let filename = format!("{}_chapters.txt", series_name);
        let file_path = output_dir.join(&filename);

        // Format chapters content
        let content = self.format_chapters_content(chapters, search_terms);

        // Write to file
        fs::write(&file_path, content).await?;
        
        info!("📝 Wrote {} chapters to: {}", chapters.len(), file_path.display());
        Ok(file_path)
    }

    /// Create a clean filename from series information
    pub fn create_series_filename(&self, search_terms: &SearchTerms) -> String {
        let mut parts = Vec::new();

        // Add series name
        if !search_terms.series.is_empty() {
            parts.extend(search_terms.series.iter().cloned());
        }

        // Add instructor name
        if !search_terms.instructor.is_empty() {
            parts.push("by".to_string());
            parts.extend(search_terms.instructor.iter().cloned());
        }

        // Join and clean up for filename
        let filename = parts.join("_")
            .to_lowercase()
            .replace(' ', "_")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

        // Remove duplicate underscores
        let re = Regex::new(r"_+").unwrap();
        re.replace_all(&filename, "_").trim_matches('_').to_string()
    }

    /// Format chapters content for file output
    fn format_chapters_content(&self, chapters: &[ChapterInfo], search_terms: &SearchTerms) -> String {
        let mut content = String::new();

        // Header
        content.push_str(&format!("# {} Chapters\n\n", self.create_series_title(search_terms)));
        
        // Series information
        content.push_str("## Series Information\n");
        content.push_str(&format!("- **Instructor**: {}\n", search_terms.instructor.join(" ")));
        content.push_str(&format!("- **Series**: {}\n", search_terms.series.join(" ")));
        if let Some(volume) = search_terms.volume {
            content.push_str(&format!("- **Volume**: {}\n", volume));
        }
        content.push_str(&format!("- **Total Chapters**: {}\n", chapters.len()));
        content.push_str(&format!("- **Duration**: {:.1} minutes\n", 
                                 chapters.last().map(|ch| ch.timestamp / 60.0).unwrap_or(0.0)));
        content.push_str("\n");

        // Chapters list
        content.push_str("## Chapters\n\n");
        
        for (i, chapter) in chapters.iter().enumerate() {
            let minutes = (chapter.timestamp / 60.0) as u32;
            let seconds = (chapter.timestamp % 60.0) as u32;
            
            content.push_str(&format!(
                "{}. **{}** - {}:{:02}\n",
                i + 1,
                chapter.title,
                minutes,
                seconds
            ));
        }

        // Footer
        content.push_str("\n---\n");
        content.push_str(&format!("*Generated by BJJ Analyzer Rust - {}*\n", 
                                 chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        content
    }

    /// Create a readable series title from search terms
    fn create_series_title(&self, search_terms: &SearchTerms) -> String {
        let mut title_parts = Vec::new();

        if !search_terms.series.is_empty() {
            title_parts.push(search_terms.series.join(" "));
        }

        if !search_terms.instructor.is_empty() {
            title_parts.push(format!("by {}", search_terms.instructor.join(" ")));
        }

        if title_parts.is_empty() {
            "BJJ Instructional".to_string()
        } else {
            title_parts.join(" ")
        }
    }

    /// Get the expected path for chapters file based on series information
    fn get_chapters_file_path(&self, search_terms: &SearchTerms, output_dir: Option<&Path>) -> PathBuf {
        let output_dir = output_dir.unwrap_or_else(|| Path::new("chapters"));
        let filename = format!("{}_chapters.txt", self.create_series_filename(search_terms));
        output_dir.join(filename)
    }

    /// Load chapters from an existing file
    async fn load_chapters_from_file(&self, file_path: &Path) -> Result<Vec<ChapterInfo>> {
        let content = fs::read_to_string(file_path).await?;
        self.parse_chapters_file_content(&content)
    }

    /// Parse chapters from file content
    fn parse_chapters_file_content(&self, content: &str) -> Result<Vec<ChapterInfo>> {
        let mut chapters = Vec::new();
        
        // Look for lines that match the chapter format: "1. **Title** - M:SS"
        let chapter_regex = Regex::new(r"^\d+\.\s+\*\*(.+?)\*\*\s+-\s+(\d+):(\d{2})").unwrap();
        
        for line in content.lines() {
            if let Some(captures) = chapter_regex.captures(line) {
                let title = captures.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                let minutes: u32 = captures.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                let seconds: u32 = captures.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                
                let timestamp = (minutes * 60 + seconds) as f64;
                
                chapters.push(ChapterInfo {
                    title,
                    timestamp,
                    description: None,
                });
            }
        }
        
        info!("📁 Loaded {} chapters from file", chapters.len());
        Ok(chapters)
    }
}

/// Represents a calculated boundary for a video in a series
#[derive(Debug, Clone)]
struct VideoBoundary {
    volume: u32,
    start_time: f64,  // Start time in seconds (from series perspective)
    end_time: f64,    // End time in seconds (from series perspective)
}