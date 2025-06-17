use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

use super::SearchTerms;

/// Manages product pages URLs from a text file and provides matching functionality
#[derive(Debug, Clone)]
pub struct ProductPagesFile {
    urls: Vec<String>,
}

impl ProductPagesFile {
    /// Load product pages from a directory containing "product-pages.txt"
    pub async fn load_from_directory(video_dir: &Path) -> Result<Self> {
        let product_pages_path = video_dir.join("product-pages.txt");
        
        if !product_pages_path.exists() {
            return Err(anyhow!(
                "Product pages file not found: {}. Please create this file with BJJfanatics URLs (one per line).",
                product_pages_path.display()
            ));
        }

        info!("📄 Loading product pages from: {}", product_pages_path.display());
        
        let content = fs::read_to_string(&product_pages_path).await?;
        let urls: Vec<String> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                // Remove line prefixes like "7→" from URLs
                if let Some(arrow_pos) = line.find('→') {
                    line[arrow_pos + '→'.len_utf8()..].trim().to_string()
                } else {
                    line.to_string()
                }
            })
            .filter(|line| line.starts_with("http")) // Only keep valid URLs
            .collect();

        if urls.is_empty() {
            return Err(anyhow!("No valid URLs found in product pages file"));
        }

        info!("✅ Loaded {} product URLs", urls.len());
        for url in &urls {
            debug!("  - {}", url);
        }

        Ok(Self { urls })
    }

    /// Find the best matching URL for the given search terms
    pub fn find_matching_url(&self, search_terms: &SearchTerms) -> Result<String> {
        info!("🔍 Finding matching URL for instructor: {:?}, series: {:?}", 
              search_terms.instructor, search_terms.series);

        let mut best_match: Option<(String, f64)> = None;

        for url in &self.urls {
            let score = self.calculate_match_score(url, search_terms);
            debug!("URL: {} | Score: {:.2}", url, score);

            if let Some((_, best_score)) = &best_match {
                if score > *best_score {
                    best_match = Some((url.clone(), score));
                }
            } else if score > 0.0 {
                best_match = Some((url.clone(), score));
            }
        }

        match best_match {
            Some((url, score)) => {
                info!("✅ Best match: {} (score: {:.2})", url, score);
                if score < 0.5 {
                    warn!("⚠️ Low confidence match (score: {:.2})", score);
                }
                Ok(url)
            }
            None => {
                let available_urls = self.urls.join("\n  - ");
                Err(anyhow!(
                    "No matching URL found for instructor: {:?}, series: {:?}\n\
                     Available URLs:\n  - {}",
                    search_terms.instructor, search_terms.series, available_urls
                ))
            }
        }
    }

    /// Calculate match score between URL and search terms
    fn calculate_match_score(&self, url: &str, search_terms: &SearchTerms) -> f64 {
        // Extract slug from URL (e.g., "just-stand-up-by-craig-jones" from full URL)
        let slug = self.extract_url_slug(url);
        let slug_words = self.normalize_text(&slug);
        
        // Normalize search terms
        let instructor_words: Vec<String> = search_terms.instructor
            .iter()
            .map(|w| self.normalize_word(w))
            .collect();
        
        let series_words: Vec<String> = search_terms.series
            .iter()
            .map(|w| self.normalize_word(w))
            .collect();

        debug!("  Slug words: {:?}", slug_words);
        debug!("  Instructor: {:?}", instructor_words);
        debug!("  Series: {:?}", series_words);

        let mut score = 0.0;
        let mut total_weight = 0.0;

        // Instructor matching (higher weight)
        let instructor_weight = 0.6;
        let instructor_score = self.calculate_word_match_score(&slug_words, &instructor_words);
        score += instructor_score * instructor_weight;
        total_weight += instructor_weight;

        // Series matching
        let series_weight = 0.4;
        let series_score = self.calculate_word_match_score(&slug_words, &series_words);
        score += series_score * series_weight;
        total_weight += series_weight;

        if total_weight > 0.0 {
            score / total_weight
        } else {
            0.0
        }
    }

    /// Extract the product slug from a BJJfanatics URL
    fn extract_url_slug(&self, url: &str) -> String {
        if let Some(slug_start) = url.rfind("/products/") {
            let slug_part = &url[slug_start + 10..]; // Skip "/products/"
            // Remove any query parameters or fragments
            slug_part.split('?').next().unwrap_or(slug_part)
                .split('#').next().unwrap_or(slug_part)
                .to_string()
        } else {
            url.to_string()
        }
    }

    /// Normalize text into searchable words
    fn normalize_text(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
            .filter(|word| !word.is_empty())
            .map(|word| word.to_string())
            .collect()
    }

    /// Normalize a single word
    fn normalize_word(&self, word: &str) -> String {
        word.to_lowercase().trim().to_string()
    }

    /// Calculate match score between two sets of words
    fn calculate_word_match_score(&self, slug_words: &[String], search_words: &[String]) -> f64 {
        if search_words.is_empty() {
            return 0.0;
        }

        let matched_count = search_words
            .iter()
            .filter(|search_word| {
                slug_words.iter().any(|slug_word| {
                    slug_word == *search_word || 
                    slug_word.contains(*search_word) || 
                    search_word.contains(slug_word)
                })
            })
            .count();

        matched_count as f64 / search_words.len() as f64
    }

    /// Get all URLs for debugging
    pub fn get_urls(&self) -> &[String] {
        &self.urls
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_product_pages() {
        let temp_dir = TempDir::new().unwrap();
        let product_pages_path = temp_dir.path().join("product-pages.txt");
        
        let content = "https://bjjfanatics.com/products/just-stand-up-by-craig-jones\n\
                      https://bjjfanatics.com/products/closed-guard-by-adam-wardzinski\n\
                      # This is a comment\n\
                      \n\
                      https://bjjfanatics.com/products/back-attacks-by-john-danaher";
        
        fs::write(&product_pages_path, content).await.unwrap();
        
        let product_pages = ProductPagesFile::load_from_directory(temp_dir.path()).await.unwrap();
        assert_eq!(product_pages.urls.len(), 3);
        assert!(product_pages.urls[0].contains("craig-jones"));
    }

    #[test]
    fn test_url_slug_extraction() {
        let product_pages = ProductPagesFile { urls: vec![] };
        
        let slug = product_pages.extract_url_slug("https://bjjfanatics.com/products/just-stand-up-by-craig-jones");
        assert_eq!(slug, "just-stand-up-by-craig-jones");
        
        let slug = product_pages.extract_url_slug("https://bjjfanatics.com/products/test?param=value");
        assert_eq!(slug, "test");
    }

    #[test]
    fn test_text_normalization() {
        let product_pages = ProductPagesFile { urls: vec![] };
        
        let words = product_pages.normalize_text("just-stand-up-by-craig-jones");
        assert_eq!(words, vec!["just", "stand", "up", "by", "craig", "jones"]);
        
        let words = product_pages.normalize_text("Test_Multiple-Separators");
        assert_eq!(words, vec!["test", "multiple", "separators"]);
    }
}