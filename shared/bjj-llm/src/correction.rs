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
    pub async fn correct(&self, text: &str) -> crate::Result<CorrectionResponse> {
        // Mock implementation for testing - apply some common BJJ corrections
        let mut corrected_text = text.to_string();
        let mut response = CorrectionResponse::new();
        
        // Common BJJ terminology corrections
        let corrections = [
            ("gard", "guard"),
            ("jujitsu", "jiu-jitsu"),
            ("ju jitsu", "jiu-jitsu"),
            ("jujutu", "jiu-jitsu"),
            ("kimora", "kimura"),
            ("armbar", "armbar"),
            ("choak", "choke"),
            ("submition", "submission"),
            ("posision", "position"),
            ("takedown", "takedown"),
        ];
        
        for (wrong, correct) in corrections {
            if corrected_text.to_lowercase().contains(wrong) {
                corrected_text = corrected_text.replace(wrong, correct);
                corrected_text = corrected_text.replace(&wrong.to_uppercase(), &correct.to_uppercase());
                response = response.add_replacement(wrong, correct, Some("BJJ terminology correction".to_string()));
            }
        }
        
        let count = response.replacements().len();
        Ok(response.with_notes(format!("Applied {} corrections", count)))
    }
    
    /// Apply corrections to transcript and SRT files
    pub async fn correct_transcript_files(&self, video_path: &std::path::Path) -> crate::Result<()> {
        use tokio::fs;
        
        let parent_dir = video_path.parent().unwrap_or(std::path::Path::new("."));
        let stem = video_path.file_stem().unwrap().to_string_lossy();
        
        let txt_path = parent_dir.join(format!("{}.txt", stem));
        let srt_path = parent_dir.join(format!("{}.srt", stem));
        let corrected_txt_path = parent_dir.join(format!("{}_corrected.txt", stem));
        
        // Read original transcript
        if !txt_path.exists() {
            return Err(crate::LLMError::Processing("Transcript file not found".to_string()).into());
        }
        
        let original_text = fs::read_to_string(&txt_path).await
            .map_err(|e| crate::LLMError::Processing(format!("Failed to read transcript: {}", e)))?;
        
        // Apply corrections
        let correction_response = self.correct(&original_text).await?;
        
        // Apply replacements to the text
        let mut corrected_text = original_text.clone();
        for replacement in correction_response.replacements() {
            corrected_text = corrected_text.replace(replacement.original(), replacement.replacement());
        }
        
        // Save corrected transcript
        fs::write(&corrected_txt_path, &corrected_text).await
            .map_err(|e| crate::LLMError::Processing(format!("Failed to write corrected transcript: {}", e)))?;
        
        // Update SRT file if it exists
        if srt_path.exists() {
            let original_srt = fs::read_to_string(&srt_path).await
                .map_err(|e| crate::LLMError::Processing(format!("Failed to read SRT: {}", e)))?;
            
            let mut corrected_srt = original_srt;
            for replacement in correction_response.replacements() {
                corrected_srt = corrected_srt.replace(replacement.original(), replacement.replacement());
            }
            
            fs::write(&srt_path, &corrected_srt).await
                .map_err(|e| crate::LLMError::Processing(format!("Failed to write corrected SRT: {}", e)))?;
        }
        
        tracing::info!("Applied {} corrections to transcript files", correction_response.replacements().len());
        Ok(())
    }
}