use super::{create_llm, ChatMessage, LLMConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

/// Structured result from LLM filename parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFilename {
    /// The instructor's name (e.g., "Craig Jones", "John Danaher")
    pub instructor: Option<String>,
    /// The series or course name (e.g., "Just Stand Up", "Enter the System")
    pub series_name: Option<String>,
    /// Part/volume number if applicable
    pub part_number: Option<u32>,
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

/// Parse a BJJ video filename using LLM with fallback to regex parsing
pub async fn parse_filename_with_llm(
    filename: &str,
    config: LLMConfig,
    prompt_path: Option<&Path>,
) -> Result<ParsedFilename> {
    debug!("Starting LLM filename parsing for: {}", filename);
    
    // Create LLM client
    let llm = create_llm(&config)?;
    
    // Check if LLM is available
    if !llm.is_available().await {
        warn!("LLM provider not available for filename parsing, falling back to regex");
        return fallback_regex_parsing(filename);
    }
    
    // Load prompt template
    let prompt = if let Some(path) = prompt_path.filter(|p| p.exists()) {
        tokio::fs::read_to_string(path).await?
    } else {
        default_filename_parsing_prompt().to_string()
    };
    
    // Prepare messages for LLM
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Parse this BJJ video filename: {}", filename),
        },
    ];
    
    // Send request to LLM
    match llm.chat(messages).await {
        Ok(response) => {
            debug!("LLM response received: {}", response.content);
            
            // Clean the response content (remove markdown code blocks if present)
            let cleaned_content = clean_llm_response(&response.content);
            debug!("Cleaned LLM response: {}", cleaned_content);
            
            // Try to parse as JSON first
            match serde_json::from_str::<ParsedFilename>(&cleaned_content) {
                Ok(parsed) => {
                    info!("Successfully parsed filename with LLM: {} -> {:?}", filename, parsed);
                    Ok(parsed)
                }
                Err(e) => {
                    // Try to extract from text response
                    debug!("JSON parsing failed ({}), attempting text parsing", e);
                    parse_text_response(&response.content, filename)
                }
            }
        }
        Err(e) => {
            warn!("LLM request failed: {}, falling back to regex parsing", e);
            fallback_regex_parsing(filename)
        }
    }
}

/// Clean LLM response by removing markdown code blocks and extra whitespace
fn clean_llm_response(content: &str) -> String {
    let content = content.trim();
    
    // Remove markdown code blocks (```json ... ``` or ``` ... ```)
    if content.starts_with("```") {
        if let Some(start) = content.find('\n') {
            if let Some(end) = content.rfind("```") {
                if end > start {
                    return content[start + 1..end].trim().to_string();
                }
            }
        }
    }
    
    // Remove any remaining backticks and extra whitespace
    content.replace("```", "").trim().to_string()
}

/// Default prompt template for filename parsing
fn default_filename_parsing_prompt() -> &'static str {
    r#"You are an expert at parsing Brazilian Jiu-Jitsu (BJJ) video filenames. 

Your task is to extract structured information from BJJ instructional video filenames. These files typically follow patterns like:
- "JustStandUpbyCraigJones3.mp4" 
- "ClosedGuardReintroducedbyAdamWardzinski1.mp4"
- "BackAttacksByJohnDanaher2.mp4"
- "TestFiles_MikeyMusumeciGuardMagic4.mp4"

Common BJJ instructors include: John Danaher, Gordon Ryan, Craig Jones, Marcelo Garcia, Bernardo Faria, Keenan Cornelius, Mikey Musumeci, Adam Wardzinski, and many others.

Common techniques/positions: Guard, Mount, Side Control, Back Control, Half Guard, Closed Guard, Open Guard, Butterfly Guard, X-Guard, De La Riva, Berimbolo, Leg Locks, Heel Hooks, Knee Bar, Triangle, Armbar, Choke, etc.

Return a JSON object with this exact structure:
{
  "instructor": "instructor name or null",
  "technique": "main technique/concept or null", 
  "position": "specific position or null",
  "series_name": "course/series name or null",
  "part_number": number or null,
  "keywords": ["additional", "relevant", "keywords"]
}

Rules:
1. Extract the instructor's full name (first and last name)
2. Identify the main technique or conceptual focus
3. Determine if there's a specific position being taught
4. Extract the series/course name 
5. Find any part/volume numbers
6. Include relevant BJJ terminology as keywords
7. Use null for fields that cannot be determined
8. Be conservative - only include information you're confident about
9. Handle prefixes like "TestFiles_" or directory names by ignoring them

Return only the JSON object, no additional text."#
}

/// Parse LLM text response when JSON parsing fails
fn parse_text_response(text: &str, original_filename: &str) -> Result<ParsedFilename> {
    debug!("Attempting to parse text response: {}", text);
    
    let mut parsed = ParsedFilename::default();
    
    // Look for common patterns in text responses
    let lines: Vec<&str> = text.lines().collect();
    
    for line in lines {
        let line = line.trim();
        if line.is_empty() { continue; }
        
        // Try to extract key-value pairs
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            
            if value.is_empty() || value.eq_ignore_ascii_case("null") || value.eq_ignore_ascii_case("unknown") {
                continue;
            }
            
            match key.as_str() {
                "instructor" | "instructor name" => {
                    parsed.instructor = Some(value.to_string());
                }
                "series_name" | "series name" | "course name" => {
                    parsed.series_name = Some(value.to_string());
                }
                "part_number" | "part number" | "volume" => {
                    if let Ok(num) = value.parse::<u32>() {
                        parsed.part_number = Some(num);
                    }
                }
                _ => {
                    // Ignore other fields
                }
            }
        }
    }
    
    // If we got some useful information, return it
    if parsed.instructor.is_some() || parsed.series_name.is_some() {
        info!("Extracted from text response: {:?}", parsed);
        Ok(parsed)
    } else {
        warn!("Could not extract useful information from text response");
        fallback_regex_parsing(original_filename)
    }
}

/// Fallback to the original regex-based parsing when LLM is unavailable
fn fallback_regex_parsing(filename: &str) -> Result<ParsedFilename> {
    debug!("Using fallback regex parsing for: {}", filename);
    
    // Remove directory prefix if present (e.g., "Test Files2_")
    let basename = if filename.contains('_') {
        filename
            .split('_')
            .skip(1) // Skip directory prefix
            .collect::<Vec<&str>>()
            .join("_")
    } else {
        filename.to_string()
    };
    
    // Remove file extension
    let basename = basename
        .replace(".mp4", "")
        .replace(".avi", "")
        .replace(".mkv", "")
        .replace(".mov", "");
    
    let mut parsed = ParsedFilename::default();
    
    // Try to find "by" separator pattern
    let by_positions: Vec<_> = basename.match_indices("by").collect();
    
    for (by_pos, _) in by_positions {
        let before_char = basename.chars().nth(by_pos.saturating_sub(1));
        let after_char = basename.chars().nth(by_pos + 2);
        
        // Check if this looks like a valid separator
        if (before_char.map_or(true, |c| c.is_lowercase() || !c.is_alphabetic()) &&
            after_char.map_or(false, |c| c.is_uppercase())) {
            
            let series_part = &basename[..by_pos];
            let instructor_part = &basename[by_pos + 2..];
            
            // Clean up instructor name (remove numbers at end)
            let instructor = instructor_part
                .trim_end_matches(char::is_numeric)
                .to_string();
            
            // Convert camelCase to readable format
            parsed.series_name = Some(camel_case_to_readable(series_part));
            parsed.instructor = Some(camel_case_to_readable(&instructor));
            
            // Extract part number if present
            if let Some(num_str) = instructor_part.chars().rev().take_while(|c| c.is_numeric()).collect::<String>().chars().rev().collect::<String>().parse::<u32>().ok() {
                parsed.part_number = Some(num_str);
            }
            
            debug!("Regex fallback parsed: {:?}", parsed);
            return Ok(parsed);
        }
    }
    
    // If no "by" pattern found, treat whole filename as series
    parsed.series_name = Some(camel_case_to_readable(&basename));
    
    debug!("Regex fallback (no instructor found): {:?}", parsed);
    Ok(parsed)
}

/// Convert camelCase to readable format
/// Example: "JustStandUp" -> "Just Stand Up"
fn camel_case_to_readable(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() && !result.is_empty() {
            result.push(' ');
        }
        result.push(ch);
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_llm_response_with_markdown() {
        let input = "```json\n{\n  \"instructor\": \"Mikey Musumeci\",\n  \"series_name\": \"Guard Magic\",\n  \"part_number\": 4\n}\n```";
        let expected = "{\n  \"instructor\": \"Mikey Musumeci\",\n  \"series_name\": \"Guard Magic\",\n  \"part_number\": 4\n}";
        assert_eq!(clean_llm_response(input), expected);
    }

    #[test]
    fn test_clean_llm_response_without_markdown() {
        let input = "{\n  \"instructor\": \"Craig Jones\",\n  \"series_name\": \"Just Stand Up\",\n  \"part_number\": 3\n}";
        assert_eq!(clean_llm_response(input), input);
    }

    #[test]
    fn test_clean_llm_response_with_extra_backticks() {
        let input = "```{\n  \"instructor\": \"John Danaher\",\n  \"series_name\": \"Back Attacks\"\n}```";
        let expected = "{\n  \"instructor\": \"John Danaher\",\n  \"series_name\": \"Back Attacks\"\n}";
        assert_eq!(clean_llm_response(input), expected);
    }

    #[test]
    fn test_camel_case_conversion() {
        assert_eq!(camel_case_to_readable("JustStandUp"), "Just Stand Up");
        assert_eq!(camel_case_to_readable("BackAttacks"), "Back Attacks");
        assert_eq!(camel_case_to_readable("CraigJones"), "Craig Jones");
    }

    #[test]
    fn test_fallback_parsing() {
        let result = fallback_regex_parsing("JustStandUpbyCraigJones3.mp4").unwrap();
        assert_eq!(result.instructor, Some("Craig Jones".to_string()));
        assert_eq!(result.series_name, Some("Just Stand Up".to_string()));
        assert_eq!(result.part_number, Some(3));
    }

    #[test]
    fn test_fallback_parsing_with_prefix() {
        let result = fallback_regex_parsing("TestFiles2_ClosedGuardbyAdamWardzinski1.mp4").unwrap();
        assert_eq!(result.instructor, Some("Adam Wardzinski".to_string()));
        assert_eq!(result.series_name, Some("Closed Guard".to_string()));
        assert_eq!(result.part_number, Some(1));
    }
}