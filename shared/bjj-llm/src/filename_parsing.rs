//! BJJ filename parsing using LLM

use crate::{LLMConfig, LLMProvider};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Structured result from LLM filename parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFilename {
    instructor: Option<String>,
    series_name: Option<String>,
    part_number: Option<u32>,
}

impl Default for ParsedFilename {
    fn default() -> Self {
        Self {
            instructor: None,
            series_name: None,
            part_number: None,
        }
    }
}

impl ParsedFilename {
    /// Create new parsed filename
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get instructor
    pub fn instructor(&self) -> Option<&str> {
        self.instructor.as_deref()
    }
    
    /// Set instructor
    pub fn with_instructor(mut self, instructor: String) -> Self {
        self.instructor = Some(instructor);
        self
    }
    
    /// Get series name
    pub fn series_name(&self) -> Option<&str> {
        self.series_name.as_deref()
    }
    
    /// Set series name
    pub fn with_series_name(mut self, series_name: String) -> Self {
        self.series_name = Some(series_name);
        self
    }
    
    /// Get part number
    pub fn part_number(&self) -> Option<u32> {
        self.part_number
    }
    
    /// Set part number
    pub fn with_part_number(mut self, part_number: u32) -> Self {
        self.part_number = Some(part_number);
        self
    }
}

/// Filename parser using LLM
pub struct FilenameParser {
    config: LLMConfig,
    prompt_file: Option<PathBuf>,
}

impl FilenameParser {
    /// Create new filename parser
    pub fn new(config: LLMConfig) -> Self {
        Self {
            config,
            prompt_file: None,
        }
    }
    
    /// Set custom prompt file
    pub fn with_prompt_file(mut self, prompt_file: PathBuf) -> Self {
        self.prompt_file = Some(prompt_file);
        self
    }
    
    /// Check if has custom prompt
    pub fn has_custom_prompt(&self) -> bool {
        self.prompt_file.is_some()
    }
    
    /// Get provider type
    pub fn provider_type(&self) -> LLMProvider {
        self.config.provider()
    }
    
    /// Parse filename using LLM
    pub async fn parse(&self, _filename: &str) -> crate::Result<ParsedFilename> {
        // Mock implementation for testing
        Ok(ParsedFilename::new()
            .with_instructor("Mock Instructor".to_string())
            .with_series_name("Mock Series".to_string())
            .with_part_number(1))
    }
}