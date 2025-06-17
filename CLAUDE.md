# CLAUDE.md - BJJ Video Analyzer Rust

This file provides guidance to Claude Code (claude.ai/code) when working with the Rust implementation of the BJJ Video Analyzer.

## Project Overview

This is a high-performance Rust rewrite of the Python-based BJJ Video Analyzer. It provides video processing, transcription, and LLM-based correction for Brazilian Jiu-Jitsu instructional content with significant performance improvements over the Python version.

## Technology Stack

- **Core**: Rust 2021 Edition, Tokio async runtime
- **Video Processing**: FFmpeg (external dependency)
- **Transcription**: OpenAI Whisper integration
- **LLM Integration**: Multiple providers (LMStudio, Gemini, OpenAI)
- **State Management**: JSON-based persistence with thread-safe caching

## Development Commands

### Environment Setup
```bash
# Ensure Rust is installed via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Add cargo to PATH for this session
export PATH="$HOME/.cargo/bin:$PATH"
```

### Building
```bash
# Debug build
export PATH="$HOME/.cargo/bin:$PATH" && cargo build

# Release build
export PATH="$HOME/.cargo/bin:$PATH" && cargo build --release

# Build specific binary
export PATH="$HOME/.cargo/bin:$PATH" && cargo build --bin bjj-analyzer
```

### Running
```bash
# Basic usage
export PATH="$HOME/.cargo/bin:$PATH" && cargo run -- --video-dir "/path/to/videos" --output-dir "./output"

# With custom worker count
export PATH="$HOME/.cargo/bin:$PATH" && cargo run -- --video-dir "/path/to/videos" --workers 8

# Verbose logging
export PATH="$HOME/.cargo/bin:$PATH" && cargo run -- --video-dir "/path/to/videos" --verbose
```

### Testing
```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test
export PATH="$HOME/.cargo/bin:$PATH" && cargo test --release
```

### Code Quality
```bash
# Check for issues
export PATH="$HOME/.cargo/bin:$PATH" && cargo check

# Clippy linting
export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy

# Format code
export PATH="$HOME/.cargo/bin:$PATH" && cargo fmt
```

## Architecture Overview

### State Management System
- **Thread-safe caching**: Uses `Arc<RwLock<HashMap>>` for concurrent access
- **Skip logic**: Avoids re-processing completed stages for faster debugging
- **JSON persistence**: States stored in `.bjj_analyzer_state/` within input directory
- **Integrity checking**: File modification time and size validation

### Processing Pipeline
1. **Video Analysis** → Extract metadata, validate format
2. **Audio Extraction** → Extract WAV audio for transcription
3. **Transcription** → Whisper-based speech-to-text
4. **LLM Correction** → Optional correction of transcription errors
5. **File Management** → Rename and organize output files

### Key Features
- **Parallel processing**: Configurable worker pool with semaphore-based concurrency
- **Resume capability**: Can restart from any interrupted stage
- **LLM correction**: Processes both .txt and .srt files with proper backup naming
- **State persistence**: Independent of output directory location

## Configuration

Configuration is loaded from:
- `config/settings.toml` (if present)
- Environment variables
- Command-line arguments
- Default values

### LLM Configuration
Set environment variables for LLM providers:
```bash
export GOOGLE_API_KEY="your_gemini_key"
export OPENAI_API_KEY="your_openai_key"
# LMStudio runs locally, no API key needed
```

## File Structure

- **src/main.rs** - CLI entry point
- **src/processing.rs** - Main processing pipeline and state integration
- **src/state.rs** - State management system with skip logic
- **src/video.rs** - Video analysis and metadata extraction
- **src/audio.rs** - Audio extraction and processing
- **src/transcription/** - Whisper integration and SRT generation
- **src/llm/** - LLM provider integrations and correction logic
- **src/config.rs** - Configuration management
- **src/bjj/** - BJJ-specific terminology and corrections

## State Management

States are stored in `.bjj_analyzer_state/` within the input video directory:
- **Location**: `{input_dir}/.bjj_analyzer_state/`
- **Format**: JSON files named after video files
- **Thread-safe**: Concurrent access via `Arc<RwLock<HashMap>>`
- **Persistence**: Automatic save after each stage completion

### Processing Stages
1. `VideoAnalysis` - Metadata extraction
2. `AudioExtraction` - Audio file creation
3. `AudioEnhancement` - Optional audio processing
4. `Transcription` - Speech-to-text conversion
5. `LLMCorrection` - AI-based error correction
6. `SubtitleGeneration` - SRT file creation
7. `Completed` - Final cleanup

## LLM Correction Workflow

When LLM correction is enabled:
1. Reads original `.txt` and `.srt` files
2. Applies corrections from LLM provider
3. Renames originals to `*_old.txt` and `*_old.srt`
4. Saves corrected versions as main files
5. Updates state with correction metadata

## Chapter File Management

The system uses a simple file-based approach for chapter data to avoid cache complexity:

### How It Works
1. **Chapter files** are stored in `chapters/` directory as `.txt` files
2. **Naming convention**: `{series}_by_{instructor}_chapters.txt`
3. **Automatic behavior**: If file exists, load chapters; if not, scrape from BJJfanatics
4. **No complex cache**: Simple file presence check

### Chapter File Examples
```
chapters/
├── closed_guard_reintroduced_by_adam_wardzinski_chapters.txt
├── back_attacks_by_john_danaher_chapters.txt
└── half_guard_by_bernardo_faria_chapters.txt
```

### Force Re-scraping
To force fresh chapter data, simply delete the chapter file:

```bash
# Delete specific series chapters (force re-scraping for that series)
rm chapters/closed_guard_reintroduced_by_adam_wardzinski_chapters.txt

# Delete all chapter files (force complete re-scraping)
rm chapters/*_chapters.txt

# Reset video processing state if needed
cargo run --bin cache-manager -- reset-chapter-state "ClosedGuardReintroducedbyAdamWardzinski1.mp4"
```

### Simple Workflow
```bash
# 1. Delete chapter file to force fresh data
rm chapters/series_name_chapters.txt

# 2. Run analyzer - automatically re-scrapes
cargo run -- --video-dir "/path/to/videos"
```

**Benefits:**
- **No cache complexity** - just file presence check
- **Easy to understand** - see exactly what's cached
- **Simple management** - delete file to force re-scraping
- **No TTL confusion** - files persist until manually deleted

## Important Notes

- **Cargo PATH**: Always ensure `~/.cargo/bin` is in PATH for build commands
- **FFmpeg dependency**: Required for video/audio processing
- **State location**: States persist in input directory, not output directory
- **Parallel safety**: All state operations are thread-safe
- **Resume capability**: Can restart processing from any interrupted stage
- **Cache management**: Use cache-manager binary to control chapter cache behavior

## Debugging

- **Log file**: `bjj_analyzer.log` contains detailed processing information
- **Verbose mode**: Use `--verbose` flag for additional console output
- **State inspection**: Check `.bjj_analyzer_state/*.json` files for processing status
- **Skip testing**: Use state management to avoid re-running expensive operations