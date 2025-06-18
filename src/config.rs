use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::llm::LLMProvider;

/// Configuration for the BJJ Video Analyzer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Processing configuration
    pub processing: ProcessingConfig,
    
    /// Audio extraction settings
    pub audio: AudioConfig,
    
    /// Transcription service settings
    pub transcription: TranscriptionConfig,
    
    /// LLM correction settings
    pub llm: LLMConfig,
    
    /// Chapter detection settings
    pub chapters: ChapterConfig,
    
    /// Output and storage settings
    pub output: OutputConfig,
    
    /// Performance and resource settings
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Supported video file extensions
    pub supported_extensions: Vec<String>,
    
    /// Maximum file size in bytes (0 = no limit)
    pub max_file_size: u64,
    
    /// Skip files that already have output
    pub skip_existing: bool,
    
    /// Enable video validation before processing
    pub validate_videos: bool,
    
    /// Enable scene detection for chapter analysis
    pub enable_scene_detection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Target sample rate for transcription
    pub target_sample_rate: u32,
    
    /// Target audio format
    pub target_format: String,
    
    /// Enable audio enhancement
    pub enable_enhancement: bool,
    
    /// Audio enhancement filters
    pub enhancement_filters: String,
    
    /// Enable audio chunking for large files
    pub enable_chunking: bool,
    
    /// Chunk duration in seconds
    pub chunk_duration: u32,
    
    /// Cleanup temporary audio files
    pub cleanup_temp_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Transcription service provider
    pub provider: TranscriptionProvider,
    
    /// API endpoint for transcription service
    pub api_endpoint: Option<String>,
    
    /// API key for transcription service
    pub api_key: Option<String>,
    
    /// Model to use for transcription
    pub model: String,
    
    /// Language hint for transcription
    pub language: Option<String>,
    
    /// Enable automatic language detection
    pub auto_detect_language: bool,
    
    /// Maximum retries for failed transcriptions
    pub max_retries: u32,
    
    /// Timeout for transcription requests (seconds)
    pub timeout: u32,
    
    /// Enable BJJ-specific prompts
    pub use_bjj_prompts: bool,
    
    /// Path to BJJ terms dictionary file
    pub bjj_terms_file: Option<PathBuf>,
    
    /// Output formats for transcription
    pub output_formats: Vec<TranscriptionFormat>,
    
    /// Enable GPU acceleration for Whisper
    pub use_gpu: bool,
    
    /// Temperature setting for Whisper (0.0 = deterministic)
    pub temperature: f32,
    
    /// Best of setting for Whisper quality
    pub best_of: u32,
    
    /// Beam size for Whisper search
    pub beam_size: u32,
    
    /// Enable fallback to local Whisper if remote fails
    pub enable_fallback: bool,
    
    /// Connection timeout for remote server (seconds)
    pub connection_timeout: u32,
    
    /// Upload chunk size for large audio files (bytes)
    pub upload_chunk_size: u64,
    
    /// Enable word-level timestamps for remote transcription
    pub word_timestamps: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TranscriptionProvider {
    OpenAI,
    AssemblyAI,
    GoogleCloud,
    Azure,
    Local,
    Remote,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Base output directory
    pub base_dir: PathBuf,
    
    /// Directory structure template
    pub dir_structure: String,
    
    /// Enable detailed logging
    pub enable_logging: bool,
    
    /// Log level
    pub log_level: String,
    
    /// Log file path
    pub log_file: Option<PathBuf>,
    
    /// Save processing metadata
    pub save_metadata: bool,
    
    /// Export formats
    pub export_formats: Vec<ExportFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    JSON,
    CSV,
    XML,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TranscriptionFormat {
    Text,
    SRT,
    VTT,
    JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum number of concurrent workers
    pub max_workers: usize,
    
    /// Memory limit per worker (MB)
    pub memory_limit_mb: usize,
    
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    
    /// Monitoring port for metrics
    pub monitoring_port: u16,
    
    /// Enable caching
    pub enable_caching: bool,
    
    /// Cache directory
    pub cache_dir: Option<PathBuf>,
    
    /// Cache TTL in seconds
    pub cache_ttl: u32,
}


/// LLM configuration for transcription correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Enable LLM-based transcription correction
    pub enable_correction: bool,
    
    /// LLM provider to use
    pub provider: LLMProvider,
    
    /// API endpoint (for LMStudio and custom providers)
    pub endpoint: Option<String>,
    
    /// API key (for cloud providers)
    pub api_key: Option<String>,
    
    /// Model to use
    pub model: String,
    
    /// Maximum tokens to generate
    pub max_tokens: u32,
    
    /// Temperature for generation (0.0 = deterministic)
    pub temperature: f32,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Path to custom correction prompt file
    pub prompt_file: Option<PathBuf>,
    
    /// Prompt configuration for various LLM tasks
    pub prompts: PromptConfig,
}

/// Configuration for all LLM prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Base directory for prompt files
    pub prompt_dir: PathBuf,
    
    /// Transcription correction prompt file
    pub correction_file: String,
    
    /// High-level summary prompt file
    pub summary_high_level_file: String,
    
    /// Technical summary prompt file
    pub summary_technical_file: String,
    
    /// Mermaid diagram generation prompt file
    pub mermaid_flowchart_file: String,
    
    /// Whisper transcription prompt template file
    pub whisper_transcription_file: String,
    
    /// Filename parsing prompt file
    pub filename_parsing_file: String,
}

/// Configuration for chapter detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterConfig {
    /// Enable chapter detection
    pub enable_detection: bool,
    
    /// Directory for chapter files
    pub chapters_dir: PathBuf,
    
    /// HTTP request timeout in seconds
    pub request_timeout_seconds: u64,
    
    /// Maximum number of chapters expected per video
    pub max_chapters: usize,
}

impl PromptConfig {
    /// Load prompt content from a specific file
    pub async fn load_prompt(&self, filename: &str) -> Result<String> {
        let path = self.prompt_dir.join(filename);
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Ok(content.trim().to_string()),
            Err(e) => Err(anyhow::anyhow!("Failed to load prompt from {}: {}", path.display(), e))
        }
    }
    
    /// Load correction prompt
    pub async fn load_correction_prompt(&self) -> Result<String> {
        self.load_prompt(&self.correction_file).await
    }
    
    /// Load high-level summary prompt
    pub async fn load_summary_high_level_prompt(&self) -> Result<String> {
        self.load_prompt(&self.summary_high_level_file).await
    }
    
    /// Load technical summary prompt
    pub async fn load_summary_technical_prompt(&self) -> Result<String> {
        self.load_prompt(&self.summary_technical_file).await
    }
    
    /// Load mermaid flowchart prompt
    pub async fn load_mermaid_flowchart_prompt(&self) -> Result<String> {
        self.load_prompt(&self.mermaid_flowchart_file).await
    }
    
    /// Load whisper transcription prompt
    pub async fn load_whisper_transcription_prompt(&self) -> Result<String> {
        self.load_prompt(&self.whisper_transcription_file).await
    }
    
    /// Load filename parsing prompt
    pub async fn load_filename_parsing_prompt(&self) -> Result<String> {
        self.load_prompt(&self.filename_parsing_file).await
    }
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        // Try to load from various locations
        let config_paths = [
            "bjj-analyzer.toml",
            "config/bjj-analyzer.toml",
            "~/.config/bjj-analyzer/config.toml",
            "/etc/bjj-analyzer/config.toml",
        ];

        for path in &config_paths {
            if let Ok(config_str) = std::fs::read_to_string(path) {
                match toml::from_str(&config_str) {
                    Ok(config) => {
                        tracing::info!("📄 Loaded configuration from: {}", path);
                        return Ok(config);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse config file {}: {}", path, e);
                    }
                }
            }
        }

        // Try environment variables
        if let Ok(config) = Self::from_env() {
            return Ok(config);
        }

        Err(anyhow!("No configuration file found"))
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(workers) = std::env::var("BJJ_ANALYZER_WORKERS") {
            config.performance.max_workers = workers.parse().unwrap_or(4);
        }

        if let Ok(sample_rate) = std::env::var("BJJ_ANALYZER_SAMPLE_RATE") {
            config.audio.target_sample_rate = sample_rate.parse().unwrap_or(16000);
        }

        if let Ok(api_key) = std::env::var("BJJ_ANALYZER_API_KEY") {
            config.transcription.api_key = Some(api_key);
        }

        if let Ok(output_dir) = std::env::var("BJJ_ANALYZER_OUTPUT_DIR") {
            config.output.base_dir = PathBuf::from(output_dir);
        }

        if let Ok(log_level) = std::env::var("BJJ_ANALYZER_LOG_LEVEL") {
            config.output.log_level = log_level;
        }

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &str) -> Result<()> {
        let config_str = toml::to_string_pretty(self)?;
        std::fs::write(path, config_str)?;
        tracing::info!("💾 Configuration saved to: {}", path);
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate performance settings
        if self.performance.max_workers == 0 {
            return Err(anyhow!("max_workers must be greater than 0"));
        }

        // Validate audio settings
        if self.audio.target_sample_rate == 0 {
            return Err(anyhow!("target_sample_rate must be greater than 0"));
        }

        // Validate output directory
        if !self.output.base_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.output.base_dir) {
                return Err(anyhow!("Cannot create output directory: {}", e));
            }
        }

        // Validate transcription settings
        match self.transcription.provider {
            TranscriptionProvider::OpenAI | TranscriptionProvider::AssemblyAI => {
                if self.transcription.api_key.is_none() {
                    return Err(anyhow!("API key required for external transcription provider"));
                }
            }
            TranscriptionProvider::Remote => {
                if self.transcription.api_endpoint.is_none() {
                    return Err(anyhow!("API endpoint required for remote transcription provider"));
                }
            }
            _ => {}
        }

        tracing::info!("✅ Configuration validation passed");
        Ok(())
    }

    /// Get runtime configuration summary
    pub fn summary(&self) -> String {
        format!(
            "BJJ Analyzer Configuration:\n\
            - Workers: {}\n\
            - Audio Sample Rate: {}Hz\n\
            - Transcription Provider: {:?}\n\
            - Output Directory: {}\n\
            - Supported Extensions: {}\n\
            - Enhancement Enabled: {}\n\
            - Caching Enabled: {}",
            self.performance.max_workers,
            self.audio.target_sample_rate,
            self.transcription.provider,
            self.output.base_dir.display(),
            self.processing.supported_extensions.join(", "),
            self.audio.enable_enhancement,
            self.performance.enable_caching
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            processing: ProcessingConfig {
                supported_extensions: vec![
                    "mp4".to_string(),
                    "mkv".to_string(),
                    "avi".to_string(),
                    "mov".to_string(),
                    "webm".to_string(),
                    "m4v".to_string(),
                ],
                max_file_size: 0, // No limit
                skip_existing: true,
                validate_videos: true,
                enable_scene_detection: true,
            },
            audio: AudioConfig {
                target_sample_rate: 16000, // Optimal for Whisper
                target_format: "wav".to_string(),
                enable_enhancement: true,
                enhancement_filters: "highpass=f=80,lowpass=f=8000,volume=1.2,dynaudnorm=g=3".to_string(),
                enable_chunking: false,
                chunk_duration: 300, // 5 minutes
                cleanup_temp_files: true,
            },
            transcription: TranscriptionConfig {
                provider: TranscriptionProvider::Local,
                api_endpoint: None,
                api_key: None,
                model: "base".to_string(),
                language: None,
                auto_detect_language: true,
                max_retries: 3,
                timeout: 3600, // 60 minutes for large files
                use_bjj_prompts: true,
                bjj_terms_file: Some(PathBuf::from("config/bjj_terms.txt")),
                output_formats: vec![TranscriptionFormat::SRT, TranscriptionFormat::Text],
                use_gpu: false,
                temperature: 0.0,
                best_of: 3,
                beam_size: 5,
                enable_fallback: true,
                connection_timeout: 30, // 30 seconds for API calls
                upload_chunk_size: 10 * 1024 * 1024, // 10MB chunks
                word_timestamps: true,
            },
            llm: LLMConfig {
                enable_correction: true,
                provider: LLMProvider::LMStudio,
                endpoint: Some("http://localhost:1234/v1/chat/completions".to_string()),
                api_key: None,
                model: "local-model".to_string(),
                max_tokens: 8192, // Set to maximum for most models
                temperature: 0.1, // Low temperature for consistent corrections
                timeout_seconds: 120, // 2 minutes timeout
                prompt_file: Some(PathBuf::from("config/prompts/correction.txt")),
                prompts: PromptConfig {
                    prompt_dir: PathBuf::from("config/prompts"),
                    correction_file: "correction.txt".to_string(),
                    summary_high_level_file: "summary_high_level.txt".to_string(),
                    summary_technical_file: "summary_technical.txt".to_string(),
                    mermaid_flowchart_file: "mermaid_flowchart.txt".to_string(),
                    whisper_transcription_file: "whisper_transcription.txt".to_string(),
                    filename_parsing_file: "filename_parsing.txt".to_string(),
                },
            },
            chapters: ChapterConfig {
                enable_detection: true,
                chapters_dir: PathBuf::from("chapters"),
                request_timeout_seconds: 30,
                max_chapters: 100,
            },
            output: OutputConfig {
                base_dir: PathBuf::from("./output"),
                dir_structure: "{date}/{video_name}".to_string(),
                enable_logging: true,
                log_level: "info".to_string(),
                log_file: None,
                save_metadata: true,
                export_formats: vec![ExportFormat::JSON],
            },
            performance: PerformanceConfig {
                max_workers: num_cpus::get().min(8), // Use available cores, max 8
                memory_limit_mb: 1024, // 1GB per worker
                enable_monitoring: false,
                monitoring_port: 9090,
                enable_caching: true,
                cache_dir: Some(PathBuf::from("./cache")),
                cache_ttl: 3600, // 1 hour
            },
        }
    }
}

/// Configuration builder for programmatic config creation
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn with_workers(mut self, workers: usize) -> Self {
        self.config.performance.max_workers = workers;
        self
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.config.audio.target_sample_rate = sample_rate;
        self
    }

    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.config.output.base_dir = dir;
        self
    }

    pub fn with_transcription_provider(mut self, provider: TranscriptionProvider) -> Self {
        self.config.transcription.provider = provider;
        self
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.config.transcription.api_key = Some(api_key);
        self
    }

    pub fn enable_enhancement(mut self, enable: bool) -> Self {
        self.config.audio.enable_enhancement = enable;
        self
    }

    pub fn enable_caching(mut self, enable: bool) -> Self {
        self.config.performance.enable_caching = enable;
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.audio.target_sample_rate, 16000);
        assert!(config.processing.validate_videos);
        assert!(config.audio.enable_enhancement);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .with_workers(8)
            .with_sample_rate(44100)
            .enable_enhancement(false)
            .build();

        assert_eq!(config.performance.max_workers, 8);
        assert_eq!(config.audio.target_sample_rate, 44100);
        assert!(!config.audio.enable_enhancement);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }
}