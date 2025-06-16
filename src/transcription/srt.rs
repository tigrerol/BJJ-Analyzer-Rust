use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::time::Duration;

/// SRT (SubRip Subtitle) entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SRTEntry {
    /// Sequential number
    pub index: u32,
    /// Start timestamp
    pub start: Duration,
    /// End timestamp
    pub end: Duration,
    /// Subtitle text
    pub text: String,
}

impl SRTEntry {
    /// Create a new SRT entry
    pub fn new(index: u32, start: Duration, end: Duration, text: String) -> Self {
        Self {
            index,
            start,
            end,
            text: text.trim().to_string(),
        }
    }
}

impl fmt::Display for SRTEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n{} --> {}\n{}\n",
            self.index,
            format_duration(self.start),
            format_duration(self.end),
            self.text
        )
    }
}

/// SRT file generator and formatter
#[derive(Debug, Clone)]
pub struct SRTGenerator {
    entries: Vec<SRTEntry>,
}

impl SRTGenerator {
    /// Create a new SRT generator
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    
    /// Add an entry to the SRT file
    pub fn add_entry(&mut self, entry: SRTEntry) {
        self.entries.push(entry);
    }
    
    /// Add multiple entries
    pub fn add_entries(&mut self, entries: Vec<SRTEntry>) {
        self.entries.extend(entries);
    }
    
    /// Sort entries by start time
    pub fn sort_entries(&mut self) {
        self.entries.sort_by(|a, b| a.start.cmp(&b.start));
        
        // Re-index entries after sorting
        for (i, entry) in self.entries.iter_mut().enumerate() {
            entry.index = (i + 1) as u32;
        }
    }
    
    /// Generate SRT content as string
    pub fn generate(&self) -> String {
        let mut srt_content = String::new();
        
        for entry in &self.entries {
            srt_content.push_str(&entry.to_string());
            srt_content.push('\n');
        }
        
        srt_content
    }
    
    /// Save SRT to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = self.generate();
        tokio::fs::write(path.as_ref(), content).await?;
        Ok(())
    }
    
    /// Get total duration of the SRT file
    pub fn get_total_duration(&self) -> Duration {
        self.entries
            .iter()
            .map(|entry| entry.end)
            .max()
            .unwrap_or(Duration::from_secs(0))
    }
    
    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Get entries in a time range
    pub fn get_entries_in_range(&self, start: Duration, end: Duration) -> Vec<&SRTEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.start >= start && entry.end <= end)
            .collect()
    }
    
    /// Get all entries
    pub fn get_entries(&self) -> &[SRTEntry] {
        &self.entries
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// Merge overlapping entries
    pub fn merge_overlapping(&mut self) {
        if self.entries.len() < 2 {
            return;
        }
        
        self.sort_entries();
        let mut merged = Vec::new();
        let mut current = self.entries[0].clone();
        
        for entry in self.entries.iter().skip(1) {
            if entry.start <= current.end {
                // Overlapping - merge them
                current.end = entry.end.max(current.end);
                current.text.push(' ');
                current.text.push_str(&entry.text);
            } else {
                // No overlap - add current and start new
                merged.push(current);
                current = entry.clone();
            }
        }
        merged.push(current);
        
        self.entries = merged;
        self.sort_entries();
    }
    
    /// Split long entries at a maximum duration
    pub fn split_long_entries(&mut self, max_duration: Duration) {
        let mut new_entries = Vec::new();
        
        for entry in &self.entries {
            let duration = entry.end - entry.start;
            
            if duration <= max_duration {
                new_entries.push(entry.clone());
            } else {
                // Split the entry
                let words: Vec<&str> = entry.text.split_whitespace().collect();
                let num_splits = (duration.as_secs_f64() / max_duration.as_secs_f64()).ceil() as usize;
                let words_per_split = words.len() / num_splits + 1;
                
                for (i, chunk) in words.chunks(words_per_split).enumerate() {
                    let start_offset = Duration::from_secs_f64(
                        (i as f64 / num_splits as f64) * duration.as_secs_f64()
                    );
                    let end_offset = Duration::from_secs_f64(
                        ((i + 1) as f64 / num_splits as f64) * duration.as_secs_f64()
                    );
                    
                    let split_entry = SRTEntry::new(
                        0, // Will be re-indexed later
                        entry.start + start_offset,
                        entry.start + end_offset,
                        chunk.join(" "),
                    );
                    new_entries.push(split_entry);
                }
            }
        }
        
        self.entries = new_entries;
        self.sort_entries();
    }
    
    /// Validate SRT entries for common issues
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        
        for (i, entry) in self.entries.iter().enumerate() {
            // Check for negative duration
            if entry.end <= entry.start {
                issues.push(format!("Entry {}: End time is not after start time", i + 1));
            }
            
            // Check for empty text
            if entry.text.trim().is_empty() {
                issues.push(format!("Entry {}: Empty text", i + 1));
            }
            
            // Check for very short duration (likely an error)
            let duration = entry.end - entry.start;
            if duration < Duration::from_millis(100) {
                issues.push(format!("Entry {}: Very short duration ({:?})", i + 1, duration));
            }
            
            // Check for very long duration (might need splitting)
            if duration > Duration::from_secs(30) {
                issues.push(format!("Entry {}: Very long duration ({:?})", i + 1, duration));
            }
        }
        
        // Check for overlapping entries
        for i in 0..self.entries.len().saturating_sub(1) {
            if self.entries[i].end > self.entries[i + 1].start {
                issues.push(format!(
                    "Entries {} and {}: Overlapping timestamps",
                    i + 1,
                    i + 2
                ));
            }
        }
        
        issues
    }
}

impl Default for SRTGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// SRT formatting utilities
pub struct SRTFormatter;

impl SRTFormatter {
    /// Format duration for SRT timestamp (HH:MM:SS,mmm)
    pub fn format_timestamp(duration: Duration) -> String {
        format_duration(duration)
    }
    
    /// Parse SRT timestamp to Duration
    pub fn parse_timestamp(timestamp: &str) -> Result<Duration> {
        parse_duration(timestamp)
    }
    
    /// Clean text for SRT display
    pub fn clean_text(text: &str) -> String {
        text.trim()
            .replace('\n', " ")
            .replace('\r', " ")
            .replace('\t', " ")
            // Replace multiple spaces with single space
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Wrap text at specified line length
    pub fn wrap_text(text: &str, max_line_length: usize) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_line_length {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines.join("\n")
    }
}

/// Format duration as SRT timestamp (HH:MM:SS,mmm)
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let milliseconds = duration.subsec_millis();
    
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, milliseconds)
}

/// Parse SRT timestamp to Duration
fn parse_duration(timestamp: &str) -> Result<Duration> {
    let parts: Vec<&str> = timestamp.split(" --> ").collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid timestamp format"));
    }
    
    parse_single_timestamp(parts[0])
}

/// Parse a single timestamp (HH:MM:SS,mmm)
fn parse_single_timestamp(timestamp: &str) -> Result<Duration> {
    let time_parts: Vec<&str> = timestamp.split(',').collect();
    if time_parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid timestamp format"));
    }
    
    let hms_parts: Vec<&str> = time_parts[0].split(':').collect();
    if hms_parts.len() != 3 {
        return Err(anyhow::anyhow!("Invalid time format"));
    }
    
    let hours: u64 = hms_parts[0].parse()?;
    let minutes: u64 = hms_parts[1].parse()?;
    let seconds: u64 = hms_parts[2].parse()?;
    let milliseconds: u64 = time_parts[1].parse()?;
    
    let total_seconds = hours * 3600 + minutes * 60 + seconds;
    let total_millis = total_seconds * 1000 + milliseconds;
    
    Ok(Duration::from_millis(total_millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt_entry_creation() {
        let entry = SRTEntry::new(
            1,
            Duration::from_secs(10),
            Duration::from_secs(15),
            "Test subtitle".to_string(),
        );
        
        assert_eq!(entry.index, 1);
        assert_eq!(entry.start, Duration::from_secs(10));
        assert_eq!(entry.end, Duration::from_secs(15));
        assert_eq!(entry.text, "Test subtitle");
    }

    #[test]
    fn test_srt_entry_display() {
        let entry = SRTEntry::new(
            1,
            Duration::from_secs(10),
            Duration::from_secs(15),
            "Test subtitle".to_string(),
        );
        
        let output = entry.to_string();
        assert!(output.contains("1"));
        assert!(output.contains("00:00:10,000 --> 00:00:15,000"));
        assert!(output.contains("Test subtitle"));
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(Duration::from_secs(3661)), "01:01:01,000");
        assert_eq!(format_duration(Duration::from_millis(1500)), "00:00:01,500");
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00:00,000");
    }

    #[test]
    fn test_srt_generator() {
        let mut generator = SRTGenerator::new();
        
        generator.add_entry(SRTEntry::new(
            1,
            Duration::from_secs(0),
            Duration::from_secs(5),
            "First subtitle".to_string(),
        ));
        
        generator.add_entry(SRTEntry::new(
            2,
            Duration::from_secs(5),
            Duration::from_secs(10),
            "Second subtitle".to_string(),
        ));
        
        let content = generator.generate();
        assert!(content.contains("First subtitle"));
        assert!(content.contains("Second subtitle"));
        assert_eq!(generator.len(), 2);
    }

    #[test]
    fn test_text_cleaning() {
        let dirty_text = "  This\thas\n\rmultiple   spaces  ";
        let clean = SRTFormatter::clean_text(dirty_text);
        assert_eq!(clean, "This has multiple spaces");
    }

    #[test]
    fn test_text_wrapping() {
        let long_text = "This is a very long line that should be wrapped at a specific length";
        let wrapped = SRTFormatter::wrap_text(long_text, 20);
        let lines: Vec<&str> = wrapped.split('\n').collect();
        
        for line in lines {
            assert!(line.len() <= 20);
        }
    }

    #[test]
    fn test_validation() {
        let mut generator = SRTGenerator::new();
        
        // Add invalid entry (end before start)
        generator.add_entry(SRTEntry::new(
            1,
            Duration::from_secs(10),
            Duration::from_secs(5),
            "Invalid".to_string(),
        ));
        
        // Add empty text entry
        generator.add_entry(SRTEntry::new(
            2,
            Duration::from_secs(15),
            Duration::from_secs(20),
            "".to_string(),
        ));
        
        let issues = generator.validate();
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|issue| issue.contains("End time is not after start time")));
        assert!(issues.iter().any(|issue| issue.contains("Empty text")));
    }
}