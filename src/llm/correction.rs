use super::{create_llm, ChatMessage, LLMConfig};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

/// Structured replacement for text correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextReplacement {
    /// Original text to replace
    pub original: String,
    /// Corrected text
    pub replacement: String,
    /// Optional explanation of the correction
    pub reason: Option<String>,
}

/// Collection of replacements returned by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResponse {
    /// List of replacements to apply
    pub replacements: Vec<TextReplacement>,
    /// Any additional notes from the LLM
    pub notes: Option<String>,
}

/// Transcription corrector using LLM
pub struct TranscriptionCorrector {
    llm: Box<dyn super::LLM>,
    correction_prompt: String,
}

impl TranscriptionCorrector {
    /// Create new transcription corrector
    pub async fn new(config: LLMConfig, prompt_path: Option<&Path>) -> Result<Self> {
        let llm = create_llm(&config)?;

        // Check if LLM is available
        if !llm.is_available().await {
            return Err(anyhow!("LLM provider {:?} is not available", config.provider));
        }

        // Load correction prompt
        let correction_prompt = if let Some(path) = prompt_path {
            if path.exists() {
                tokio::fs::read_to_string(path).await.unwrap_or_else(|_| {
                    warn!("Failed to read prompt file, using default prompt");
                    Self::default_prompt().to_string()
                })
            } else {
                warn!("Prompt file not found, using default prompt");
                Self::default_prompt().to_string()
            }
        } else {
            Self::default_prompt().to_string()
        };

        info!("✅ Transcription corrector initialized with {:?} provider", config.provider);

        Ok(Self {
            llm,
            correction_prompt,
        })
    }

    /// Get structured corrections for transcription text using LLM
    pub async fn get_corrections(&self, text: &str) -> Result<CorrectionResponse> {
        debug!("Getting structured corrections for transcription text ({} chars)", text.len());

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: self.correction_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("Analyze this BJJ transcription and return only the corrections needed:\n\n{}", text),
            },
        ];

        let response = self.llm.chat(messages).await?;

        debug!(
            "LLM correction analysis completed (tokens: {:?})", 
            response.tokens_used
        );

        // Parse the structured response
        self.parse_correction_response(&response.content)
    }

    /// Apply a list of replacements to text
    pub fn apply_replacements(&self, text: &str, replacements: &[TextReplacement]) -> String {
        let mut corrected_text = text.to_string();
        
        // Sort replacements by original text length (descending) to avoid partial replacements
        let mut sorted_replacements = replacements.to_vec();
        sorted_replacements.sort_by(|a, b| b.original.len().cmp(&a.original.len()));
        
        for replacement in sorted_replacements {
            corrected_text = corrected_text.replace(&replacement.original, &replacement.replacement);
        }
        
        corrected_text
    }

    /// Parse LLM response into structured corrections
    fn parse_correction_response(&self, response: &str) -> Result<CorrectionResponse> {
        // Try to parse as JSON first
        if let Ok(parsed) = serde_json::from_str::<CorrectionResponse>(response) {
            return Ok(parsed);
        }

        // Fallback: parse simple text format
        // Look for patterns like "original -> replacement" or "original → replacement"
        let mut replacements = Vec::new();
        
        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            
            // Try different separators
            for separator in &[" -> ", " → ", " => ", ": "] {
                if let Some(pos) = line.find(separator) {
                    let original = line[..pos].trim().trim_matches('"').to_string();
                    let rest = &line[pos + separator.len()..];
                    
                    // Extract replacement (might have explanation in parentheses)
                    let (replacement, reason) = if let Some(paren_pos) = rest.find('(') {
                        let repl = rest[..paren_pos].trim().trim_matches('"').to_string();
                        let reason_text = rest[paren_pos+1..].trim_end_matches(')').trim().to_string();
                        (repl, if reason_text.is_empty() { None } else { Some(reason_text) })
                    } else {
                        (rest.trim().trim_matches('"').to_string(), None)
                    };
                    
                    if !original.is_empty() && !replacement.is_empty() && original != replacement {
                        replacements.push(TextReplacement {
                            original,
                            replacement,
                            reason,
                        });
                        break;
                    }
                }
            }
        }
        
        Ok(CorrectionResponse {
            replacements,
            notes: None,
        })
    }

    /// Check if corrector is available
    pub async fn is_available(&self) -> bool {
        self.llm.is_available().await
    }

    /// Default correction prompt for BJJ transcriptions
    fn default_prompt() -> &'static str {
        r#"You are an expert Brazilian Jiu-Jitsu (BJJ) instructor helping to identify transcription errors in BJJ instructional videos.

IMPORTANT: Return ONLY the corrections needed in this exact format:

```
original text -> corrected text
original text -> corrected text
```

Focus on these common BJJ transcription errors:

BJJ Terminology Corrections:
- "coast guard" → "closed guard"
- "half cord" → "half guard" 
- "x cord" → "x guard"
- "full cord" → "full guard"
- "butterfly cord" → "butterfly guard"
- "spider cord" → "spider guard"
- "de la hiva" → "de la Riva"
- "berimbo" → "berimbolo"
- "guilatine" → "guillotine"
- "darce" → "d'arce"
- "omoplata" → "omoplata"
- "americana" → "americana"
- "kimura" → "kimura"
- "arm bar" → "armbar"
- "heel hook" → "heel hook"
- "knee on belly" → "knee on belly"
- "north south" → "north-south"
- "50/50" → "50/50"
- "single leg x" → "single leg X"
- "ashi garami" → "ashi garami"
- "imanari roll" → "Imanari roll"
- "berimbolo" → "berimbolo"
- "worm guard" → "worm guard"
- "k guard" → "K guard"
- "rdlr" → "RDLR"
- "dlr" → "DLR"

Rules:
1. Only return lines that need correction
2. Use format: "original -> replacement"
3. Do NOT return the full transcription
4. Do NOT add commentary or explanations
5. If no corrections needed, return: "No corrections needed"

Example response:
```
coast guard -> closed guard
half cord -> half guard
de la hiva -> de la Riva
```"#
    }
}

/// Convenience function to get structured corrections for transcription
pub async fn get_transcription_corrections(
    text: &str,
    config: LLMConfig,
    prompt_path: Option<&Path>,
) -> Result<CorrectionResponse> {
    match TranscriptionCorrector::new(config, prompt_path).await {
        Ok(corrector) => corrector.get_corrections(text).await,
        Err(e) => {
            warn!("Failed to initialize transcription corrector: {}", e);
            warn!("Skipping LLM correction step");
            Ok(CorrectionResponse {
                replacements: Vec::new(),
                notes: Some(format!("LLM unavailable: {}", e)),
            })
        }
    }
}

/// Convenience function to correct transcription text using structured replacements
pub async fn correct_transcription_text(
    text: &str,
    config: LLMConfig,
    prompt_path: Option<&Path>,
) -> Result<String> {
    let corrections = get_transcription_corrections(text, config, prompt_path).await?;
    
    if corrections.replacements.is_empty() {
        debug!("No corrections needed");
        return Ok(text.to_string());
    }
    
    // Apply replacements directly without needing a corrector instance
    let corrected_text = apply_text_replacements(text, &corrections.replacements);
    
    info!("Applied {} corrections to transcription", corrections.replacements.len());
    for replacement in &corrections.replacements {
        debug!("Correction: '{}' -> '{}'", replacement.original, replacement.replacement);
    }
    
    Ok(corrected_text)
}

/// Apply a list of text replacements to text
pub fn apply_text_replacements(text: &str, replacements: &[TextReplacement]) -> String {
    let mut corrected_text = text.to_string();
    
    // Sort replacements by original text length (descending) to avoid partial replacements
    let mut sorted_replacements = replacements.to_vec();
    sorted_replacements.sort_by(|a, b| b.original.len().cmp(&a.original.len()));
    
    for replacement in sorted_replacements {
        corrected_text = corrected_text.replace(&replacement.original, &replacement.replacement);
    }
    
    corrected_text
}