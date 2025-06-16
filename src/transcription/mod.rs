pub mod whisper;
pub mod srt;

pub use whisper::{WhisperTranscriber, TranscriptionResult, TranscriptionSegment};
pub use srt::{SRTGenerator, SRTEntry, SRTFormatter};