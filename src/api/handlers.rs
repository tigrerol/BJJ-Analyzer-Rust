//! API request handlers

use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::state::{StateManager, ProcessingStage};
use crate::chapters::{ChapterDetector, ChapterInfo};
use crate::llm::filename_parsing::{parse_filename_with_llm, ParsedFilename};
use crate::llm::LLMConfig;
use crate::config::Config;

/// Handle health check requests
pub async fn health_check() -> Result<Value> {
    Ok(serde_json::json!({
        "status": "healthy",
        "service": "bjj-video-analyzer",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handle video listing requests
pub async fn list_videos(state_manager: &Arc<StateManager>, config: &Config) -> Result<Value> {
    let states = state_manager.get_all_states().await?;
    
    let mut videos: Vec<Value> = Vec::new();
    
    for (filename, state) in &states {
        let progress = calculate_progress(&state.current_stage);
        let updated_at = chrono::DateTime::from_timestamp(state.last_updated as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        
        // Extract instructor and series from filename using LLM
        let (instructor, series) = parse_instructor_and_series_with_llm(filename, config).await;
        
        // Try to load chapters for this video - pass the actual video path from state
        let chapters = load_chapters_for_video_with_path(filename, &state.video_path, &instructor, &series)
            .await
            .unwrap_or_default();
        
        let video_status = format_status(&state.current_stage);
        
        videos.push(serde_json::json!({
            "id": filename,
            "filename": filename,
            "status": video_status,
            "current_stage": format!("{:?}", state.current_stage),
            "progress": progress,
            "last_updated": updated_at.to_rfc3339(),
            "error": state.error_message,
            "chapters": chapters,
            "metadata": {
                "duration": state.metadata.duration_seconds,
                "duration_seconds": state.metadata.duration_seconds,
                "file_size": estimate_file_size(&state.video_path),
                "resolution": state.metadata.resolution,
                "frame_rate": state.metadata.frame_rate,
                "instructor": instructor,
                "series": series,
                "transcription_model": state.metadata.transcription_model,
                "segment_count": state.metadata.segment_count,
                "corrections_applied": state.metadata.corrections_applied,
                "chapters_detected": state.metadata.chapters_detected,
                "total_processing_time": state.metadata.total_processing_time,
                "chapter_count": chapters.len()
            }
        }));
    }
    
    Ok(serde_json::json!({
        "videos": videos,
        "total": videos.len()
    }))
}

/// Handle video status requests
pub async fn get_video_status(state_manager: &Arc<StateManager>, config: &Config, video_id: &str) -> Result<Value> {
    match state_manager.get_state(video_id).await? {
        Some(state) => {
            let progress = calculate_progress(&state.current_stage);
            let updated_at = chrono::DateTime::from_timestamp(state.last_updated as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now());
            
            // Extract instructor and series from filename using LLM
            let (instructor, series) = parse_instructor_and_series_with_llm(video_id, config).await;
            
            // Try to load chapters for this video - pass the actual video path from state
            let chapters = load_chapters_for_video_with_path(video_id, &state.video_path, &instructor, &series)
                .await
                .unwrap_or_default();
            
            let video_status = format_status(&state.current_stage);
            
            Ok(serde_json::json!({
                "id": video_id,
                "filename": video_id,
                "status": video_status,
                "current_stage": format!("{:?}", state.current_stage),
                "progress": progress,
                "last_updated": updated_at.to_rfc3339(),
                "error": state.error_message,
                "chapters": chapters,
                "metadata": {
                    "duration": state.metadata.duration_seconds,
                    "duration_seconds": state.metadata.duration_seconds,
                    "file_size": estimate_file_size(&state.video_path),
                    "resolution": state.metadata.resolution,
                    "frame_rate": state.metadata.frame_rate,
                    "instructor": instructor,
                    "series": series,
                    "transcription_model": state.metadata.transcription_model,
                    "segment_count": state.metadata.segment_count,
                    "corrections_applied": state.metadata.corrections_applied,
                    "chapters_detected": state.metadata.chapters_detected,
                    "total_processing_time": state.metadata.total_processing_time,
                    "chapter_count": chapters.len()
                }
            }))
        }
        None => Err(anyhow::anyhow!("Video not found: {}", video_id))
    }
}

/// Handle start video processing requests
pub async fn start_video_processing(state_manager: &Arc<StateManager>, video_id: &str) -> Result<Value> {
    // Check if video exists in state
    match state_manager.get_state(video_id).await? {
        Some(_state) => {
            // Reset state to start processing from beginning
            state_manager.reset_state(video_id).await?;
            
            Ok(serde_json::json!({
                "message": "Processing started",
                "video_id": video_id,
                "status": "Pending",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Err(anyhow::anyhow!("Video not found: {}", video_id))
    }
}

/// Handle start multiple video processing requests
pub async fn start_multiple_video_processing(state_manager: &Arc<StateManager>, payload: &Value) -> Result<Value> {
    // For now, just acknowledge the request
    // In the future, this would process the payload and start processing multiple videos
    Ok(serde_json::json!({
        "message": "Multiple video processing request received",
        "status": "accepted",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "note": "Multiple video processing not yet fully implemented"
    }))
}

/// Handle processing status requests
pub async fn get_processing_status(state_manager: &Arc<StateManager>) -> Result<Value> {
    let states = state_manager.get_all_states().await?;
    
    let mut processing_videos = 0;
    let mut completed_videos = 0;
    let mut failed_videos = 0;
    let mut pending_videos = 0;
    
    for (_, state) in &states {
        match state.current_stage {
            ProcessingStage::Completed => completed_videos += 1,
            ProcessingStage::Error => failed_videos += 1,
            ProcessingStage::Pending => pending_videos += 1,
            _ => processing_videos += 1,
        }
    }
    
    Ok(serde_json::json!({
        "total_videos": states.len(),
        "processing_videos": processing_videos,
        "completed_videos": completed_videos,
        "failed_videos": failed_videos,
        "active_workers": 1, // TODO: Implement proper worker tracking
        "queue_size": pending_videos,
        "uptime": 0, // TODO: Implement uptime tracking
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Calculate progress percentage based on processing stage
fn calculate_progress(stage: &ProcessingStage) -> f64 {
    match stage {
        ProcessingStage::Pending => 0.0,
        ProcessingStage::VideoAnalysis => 10.0,
        ProcessingStage::AudioExtraction => 20.0,
        ProcessingStage::AudioEnhancement => 30.0,
        ProcessingStage::Transcription => 50.0,
        ProcessingStage::LLMCorrection => 70.0,
        ProcessingStage::ChapterDetection => 80.0,
        ProcessingStage::SubtitleGeneration => 90.0,
        ProcessingStage::Completed => 100.0,
        ProcessingStage::Error => 0.0,
    }
}

/// Parse instructor and series from filename using LLM with fallback to regex
/// Example: "Test Files2_JustStandUpbyCraigJones1.mp4" -> ("Craig Jones", "Just Stand Up")
async fn parse_instructor_and_series_with_llm(filename: &str, config: &Config) -> (Option<String>, Option<String>) {
    tracing::debug!("Parsing filename with LLM: {}", filename);
    
    // Convert Config to LLMConfig
    let llm_config = LLMConfig {
        provider: config.llm.provider.clone(),
        endpoint: config.llm.endpoint.clone(),
        api_key: config.llm.api_key.clone(),
        model: config.llm.model.clone(),
        max_tokens: config.llm.max_tokens,
        temperature: config.llm.temperature,
        timeout_seconds: config.llm.timeout_seconds,
    };
    
    // Load the filename parsing prompt
    let prompt_path = config.llm.prompts.prompt_dir.join(&config.llm.prompts.filename_parsing_file);
    
    match parse_filename_with_llm(filename, llm_config, Some(&prompt_path)).await {
        Ok(parsed) => {
            tracing::info!("LLM parsed filename successfully: {:?}", parsed);
            (parsed.instructor, parsed.series_name)
        }
        Err(e) => {
            tracing::warn!("LLM filename parsing failed: {}, using fallback", e);
            parse_instructor_and_series_fallback(filename)
        }
    }
}

/// Fallback regex-based parsing when LLM is unavailable
fn parse_instructor_and_series_fallback(filename: &str) -> (Option<String>, Option<String>) {
    tracing::debug!("Using fallback regex parsing for: {}", filename);
    
    // Remove directory prefix if present
    let basename = if filename.contains('_') {
        filename
            .split('_')
            .skip(1) // Skip directory prefix like "Test Files2"
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
    
    // Try to extract instructor pattern: "SeriesTitlebyInstructorName"
    // Look for "by" with proper case handling
    let by_positions: Vec<_> = basename.match_indices("by").collect();
    
    for (by_pos, _) in by_positions {
        // Check if this "by" is likely the separator (not part of a word)
        let before_char = basename.chars().nth(by_pos.saturating_sub(1));
        let after_char = basename.chars().nth(by_pos + 2);
        
        // If "by" is surrounded by uppercase letters or at word boundaries, it's likely our separator
        if (before_char.map_or(true, |c| c.is_lowercase() || !c.is_alphabetic()) &&
            after_char.map_or(false, |c| c.is_uppercase())) {
            
            let series_part = &basename[..by_pos];
            let instructor_part = &basename[by_pos + 2..];
            
            // Clean up instructor name (remove numbers at end)
            let instructor = instructor_part
                .trim_end_matches(char::is_numeric)
                .to_string();
            
            // Convert camelCase to readable format
            let series = camel_case_to_readable(series_part);
            let instructor_readable = camel_case_to_readable(&instructor);
            
            tracing::debug!("Regex fallback parsed - Series: {:?}, Instructor: {:?}", series, instructor_readable);
            return (Some(instructor_readable), Some(series));
        }
    }
    
    // Final fallback: no clear pattern found
    tracing::warn!("Could not parse instructor/series from: {}", basename);
    (None, Some(camel_case_to_readable(&basename)))
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

/// Estimate file size based on video path
fn estimate_file_size(video_path: &Path) -> u64 {
    std::fs::metadata(video_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

/// Format processing stage to UI-compatible status string
fn format_status(stage: &ProcessingStage) -> String {
    match stage {
        ProcessingStage::Pending => "pending".to_string(),
        ProcessingStage::VideoAnalysis => "processing".to_string(),
        ProcessingStage::AudioExtraction => "processing".to_string(),
        ProcessingStage::AudioEnhancement => "processing".to_string(),
        ProcessingStage::Transcription => "processing".to_string(),
        ProcessingStage::LLMCorrection => "processing".to_string(),
        ProcessingStage::ChapterDetection => "processing".to_string(),
        ProcessingStage::SubtitleGeneration => "processing".to_string(),
        ProcessingStage::Completed => "completed".to_string(),
        ProcessingStage::Error => "failed".to_string(),
    }
}

/// Handle series listing requests
pub async fn list_series(state_manager: &Arc<StateManager>, config: &Config) -> Result<Value> {
    let states = state_manager.get_all_states().await?;
    let mut series_map: HashMap<String, SeriesInfo> = HashMap::new();
    
    // Group videos by series
    for (filename, state) in &states {
        let (instructor, series_name) = parse_instructor_and_series_with_llm(filename, config).await;
        
        if let (Some(instructor), Some(series_name)) = (instructor, series_name) {
            let series_key = format!("{}_{}", instructor.replace(" ", "_"), series_name.replace(" ", "_"));
            
            let video_info = VideoInfo {
                id: filename.clone(),
                filename: filename.clone(),
                status: format_status(&state.current_stage),
                duration: state.metadata.duration_seconds,
            };
            
            if let Some(series_info) = series_map.get_mut(&series_key) {
                series_info.videos.push(video_info);
                series_info.total_duration += state.metadata.duration_seconds;
            } else {
                let mut series_info = SeriesInfo {
                    id: series_key.clone(),
                    name: series_name,
                    instructor,
                    product_url: None,
                    total_duration: state.metadata.duration_seconds,
                    videos: vec![video_info],
                };
                series_map.insert(series_key, series_info);
            }
        }
    }
    
    // Calculate completion percentages
    let series_list: Vec<Value> = series_map
        .into_values()
        .map(|mut series| {
            let completed_count = series.videos.iter()
                .filter(|v| v.status == "completed")
                .count();
            let completion_percentage = if series.videos.is_empty() {
                0.0
            } else {
                (completed_count as f64 / series.videos.len() as f64) * 100.0
            };
            
            serde_json::json!({
                "id": series.id,
                "name": series.name,
                "instructor": series.instructor,
                "product_url": series.product_url,
                "total_duration": series.total_duration,
                "completion_status": {
                    "percentage": completion_percentage
                },
                "videos": series.videos.iter().map(|v| serde_json::json!({
                    "id": v.id,
                    "filename": v.filename,
                    "status": v.status
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    
    Ok(serde_json::json!(series_list))
}

/// Handle series detail requests
pub async fn get_series(state_manager: &Arc<StateManager>, config: &Config, series_id: &str) -> Result<Value> {
    let series_list = list_series(state_manager, config).await?;
    
    if let Some(series_array) = series_list.as_array() {
        for series in series_array {
            if let Some(id) = series.get("id").and_then(|v| v.as_str()) {
                if id == series_id {
                    return Ok(series.clone());
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("Series not found: {}", series_id))
}

/// Load chapters for a specific video with known path
async fn load_chapters_for_video_with_path(
    filename: &str,
    video_path: &Path,
    _instructor: &Option<String>,
    _series: &Option<String>,
) -> Result<Vec<serde_json::Value>> {
    // Try to detect chapters using the chapter detector
    match ChapterDetector::new().await {
        Ok(detector) => {
            tracing::info!("🔍 Loading chapters for video at path: {}", video_path.display());
            
            // Try to detect chapters
            match detector.detect_chapters(video_path).await {
                Ok(chapters) => {
                    tracing::info!("✅ Found {} chapters for {}", chapters.len(), filename);
                    let chapter_values: Vec<serde_json::Value> = chapters
                        .into_iter()
                        .map(|chapter| serde_json::json!({
                            "title": chapter.title,
                            "timestamp": chapter.timestamp
                        }))
                        .collect();
                    Ok(chapter_values)
                }
                Err(e) => {
                    tracing::warn!("❌ Failed to load chapters for {}: {}", filename, e);
                    Ok(Vec::new()) // Return empty if detection fails
                }
            }
        }
        Err(e) => {
            tracing::error!("❌ Failed to create chapter detector: {}", e);
            Ok(Vec::new()) // Return empty if detector creation fails
        }
    }
}

/// Load chapters for a specific video (legacy - for compatibility)
async fn load_chapters_for_video(
    filename: &str,
    _instructor: &Option<String>,
    _series: &Option<String>,
) -> Result<Vec<serde_json::Value>> {
    // Try to detect chapters using the chapter detector
    match ChapterDetector::new().await {
        Ok(detector) => {
            // Extract the actual video path from the filename
            // The filename might include directory prefix like "Test Files2_JustStandUpbyCraigJones3.mp4"
            let video_path = if filename.contains('_') {
                // Extract directory from prefix
                let parts: Vec<&str> = filename.splitn(2, '_').collect();
                if parts.len() == 2 {
                    let dir_name = parts[0].replace(" Files", " Files");
                    let file_name = parts[1];
                    Path::new(&dir_name).join(file_name)
                } else {
                    PathBuf::from(filename)
                }
            } else {
                PathBuf::from(filename)
            };
            
            tracing::info!("🔍 Loading chapters for video at path: {}", video_path.display());
            
            // Try to detect chapters
            match detector.detect_chapters(&video_path).await {
                Ok(chapters) => {
                    tracing::info!("✅ Found {} chapters", chapters.len());
                    let chapter_values: Vec<serde_json::Value> = chapters
                        .into_iter()
                        .map(|chapter| serde_json::json!({
                            "title": chapter.title,
                            "timestamp": chapter.timestamp
                        }))
                        .collect();
                    Ok(chapter_values)
                }
                Err(e) => {
                    tracing::warn!("❌ Failed to load chapters: {}", e);
                    Ok(Vec::new()) // Return empty if detection fails
                }
            }
        }
        Err(e) => {
            tracing::error!("❌ Failed to create chapter detector: {}", e);
            Ok(Vec::new()) // Return empty if detector creation fails
        }
    }
}

#[derive(Debug, Clone)]
struct SeriesInfo {
    id: String,
    name: String,
    instructor: String,
    product_url: Option<String>,
    total_duration: f64,
    videos: Vec<VideoInfo>,
}

#[derive(Debug, Clone)]
struct VideoInfo {
    id: String,
    filename: String,
    status: String,
    duration: f64,
}

/// Handle corrections listing requests
pub async fn list_corrections() -> Result<Value> {
    // Load corrections from file if it exists
    match load_corrections().await {
        Ok(corrections) => Ok(serde_json::json!(corrections)),
        Err(_) => Ok(serde_json::json!([])), // Return empty array if no corrections file
    }
}

/// Handle series correction submission
pub async fn submit_series_correction(payload: &Value) -> Result<Value> {
    let correction = SeriesCorrection {
        id: generate_correction_id(),
        series_name: payload.get("series_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        instructor: payload.get("instructor").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        videos: payload.get("videos").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        product_url: payload.get("product_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        timestamp: chrono::Utc::now().timestamp(),
        correction_type: "series".to_string(),
    };
    
    // Save correction
    save_correction(&correction).await?;
    
    Ok(serde_json::json!({
        "message": "Series correction submitted successfully",
        "correction_id": correction.id,
        "success": true
    }))
}

/// Handle product correction submission
pub async fn submit_product_correction(payload: &Value) -> Result<Value> {
    let correction = ProductCorrection {
        id: generate_correction_id(),
        video_filename: payload.get("video_filename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        product_url: payload.get("product_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        confidence: payload.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0),
        timestamp: chrono::Utc::now().timestamp(),
        correction_type: "product".to_string(),
    };
    
    // Save correction
    save_product_correction(&correction).await?;
    
    Ok(serde_json::json!({
        "message": "Product correction submitted successfully",
        "correction_id": correction.id,
        "success": true
    }))
}

/// Load corrections from storage
async fn load_corrections() -> Result<Vec<Value>> {
    let corrections_path = Path::new("corrections.json");
    
    if corrections_path.exists() {
        let content = tokio::fs::read_to_string(corrections_path).await?;
        let corrections: Vec<Value> = serde_json::from_str(&content)?;
        Ok(corrections)
    } else {
        Ok(Vec::new())
    }
}

/// Save a series correction
async fn save_correction(correction: &SeriesCorrection) -> Result<()> {
    let mut corrections = load_corrections().await.unwrap_or_default();
    corrections.push(serde_json::to_value(correction)?);
    
    let corrections_path = Path::new("corrections.json");
    let content = serde_json::to_string_pretty(&corrections)?;
    tokio::fs::write(corrections_path, content).await?;
    
    Ok(())
}

/// Save a product correction
async fn save_product_correction(correction: &ProductCorrection) -> Result<()> {
    let mut corrections = load_corrections().await.unwrap_or_default();
    corrections.push(serde_json::to_value(correction)?);
    
    let corrections_path = Path::new("corrections.json");
    let content = serde_json::to_string_pretty(&corrections)?;
    tokio::fs::write(corrections_path, content).await?;
    
    Ok(())
}

/// Generate a unique correction ID
fn generate_correction_id() -> String {
    format!("correction_{}", chrono::Utc::now().timestamp_millis())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeriesCorrection {
    id: String,
    series_name: String,
    instructor: String,
    videos: Vec<String>,
    product_url: Option<String>,
    timestamp: i64,
    correction_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductCorrection {
    id: String,
    video_filename: String,
    product_url: String,
    confidence: f64,
    timestamp: i64,
    correction_type: String,
}