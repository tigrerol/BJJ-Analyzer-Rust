/// BJJ Video Analyzer - Rust Implementation
/// 
/// High-performance video processing library for Brazilian Jiu-Jitsu instructional content.
/// Designed to replace Python implementation with significant performance improvements.

pub mod video;
pub mod audio;
pub mod processing;
pub mod config;
pub mod api;
pub mod bjj;
pub mod transcription;
pub mod llm;
pub mod state;
pub mod chapters;

#[cfg(feature = "python-bindings")]
pub mod python_bridge;

// Re-export main types for easy access
pub use crate::config::Config;
pub use crate::processing::{BatchProcessor, ProcessingResult};
pub use crate::video::{VideoProcessor, VideoInfo};
pub use crate::audio::{AudioExtractor, AudioInfo};
pub use crate::bjj::{BJJDictionary, BJJTermCategory};
pub use crate::transcription::{WhisperTranscriber, TranscriptionResult, SRTGenerator};
pub use crate::llm::{LLMConfig, LLMProvider};
pub use crate::llm::correction::TranscriptionCorrector;
pub use crate::state::{StateManager, VideoProcessingState, ProcessingStage};
pub use crate::chapters::{ChapterDetector, ChapterInfo};

// Re-export Python bindings if feature is enabled
#[cfg(feature = "python-bindings")]
pub use python_bridge::*;