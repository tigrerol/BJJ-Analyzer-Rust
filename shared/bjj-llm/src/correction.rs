//! Transcription correction using LLM

use crate::{LLMConfig, LLMProvider};
use serde::{Deserialize, Serialize};

/// Text replacement for correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextReplacement {
    original: String,
    replacement: String,
    reason: Option<String>,
}

impl TextReplacement {
    /// Create new text replacement
    pub fn new(original: String, replacement: String, reason: Option<String>) -> Self {
        Self { original, replacement, reason }
    }
    
    /// Get original text
    pub fn original(&self) -> &str {
        &self.original
    }
    
    /// Get replacement text
    pub fn replacement(&self) -> &str {
        &self.replacement
    }
    
    /// Get reason
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

/// Collection of corrections from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResponse {
    replacements: Vec<TextReplacement>,
    notes: Option<String>,
}

impl CorrectionResponse {
    /// Create new correction response
    pub fn new() -> Self {
        Self {
            replacements: Vec::new(),
            notes: None,
        }
    }
    
    /// Add replacement
    pub fn add_replacement(mut self, original: &str, replacement: &str, reason: Option<String>) -> Self {
        self.replacements.push(TextReplacement::new(
            original.to_string(),
            replacement.to_string(),
            reason,
        ));
        self
    }
    
    /// Set notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }
    
    /// Get replacements
    pub fn replacements(&self) -> &[TextReplacement] {
        &self.replacements
    }
    
    /// Get notes
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

impl Default for CorrectionResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// Transcription corrector
pub struct TranscriptionCorrector {
    config: LLMConfig,
}

impl TranscriptionCorrector {
    /// Create new transcription corrector
    pub fn new(config: LLMConfig) -> Self {
        Self { config }
    }
    
    /// Get provider type
    pub fn provider_type(&self) -> LLMProvider {
        self.config.provider()
    }
    
    /// Correct transcription text
    pub async fn correct(&self, _text: &str) -> crate::Result<CorrectionResponse> {
        // Mock implementation for testing
        Ok(CorrectionResponse::new()
            .add_replacement("gard", "guard", Some("Spelling correction".to_string())))
    }
}