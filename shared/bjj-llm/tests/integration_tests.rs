use bjj_llm::{
    LLMConfig, LLMProvider, ChatMessage, LLMResponse, LLM,
    ParsedFilename, FilenameParser,
    CorrectionResponse, TranscriptionCorrector,
    create_llm
};
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_llm_config_creation() {
    let config = LLMConfig::default();
    
    assert_eq!(config.provider(), LLMProvider::LMStudio);
    assert_eq!(config.model(), "local-model");
    assert_eq!(config.max_tokens(), 4096);
    assert_eq!(config.temperature(), 0.1);
    assert_eq!(config.timeout_seconds(), 60);
}

#[tokio::test]
async fn test_llm_config_builder() {
    let config = LLMConfig::new()
        .with_provider(LLMProvider::Gemini)
        .with_model("gemini-pro".to_string())
        .with_temperature(0.7)
        .with_max_tokens(2048);
    
    assert_eq!(config.provider(), LLMProvider::Gemini);
    assert_eq!(config.model(), "gemini-pro");
    assert_eq!(config.temperature(), 0.7);
    assert_eq!(config.max_tokens(), 2048);
}

#[tokio::test]
async fn test_chat_message_creation() {
    let message = ChatMessage::new("user".to_string(), "Hello, world!".to_string());
    
    assert_eq!(message.role(), "user");
    assert_eq!(message.content(), "Hello, world!");
}

#[tokio::test]
async fn test_llm_response_creation() {
    let response = LLMResponse::new("Response text".to_string(), Some(150));
    
    assert_eq!(response.content(), "Response text");
    assert_eq!(response.tokens_used(), Some(150));
}

#[tokio::test]
async fn test_parsed_filename_creation() {
    let parsed = ParsedFilename::new()
        .with_instructor("Craig Jones".to_string())
        .with_series_name("Just Stand Up".to_string())
        .with_part_number(1);
    
    assert_eq!(parsed.instructor(), Some("Craig Jones"));
    assert_eq!(parsed.series_name(), Some("Just Stand Up"));
    assert_eq!(parsed.part_number(), Some(1));
}

#[tokio::test]
async fn test_filename_parser_creation() {
    let config = LLMConfig::default();
    let parser = FilenameParser::new(config);
    
    // Should create successfully
    assert_eq!(parser.provider_type(), LLMProvider::LMStudio);
}

#[tokio::test]
async fn test_filename_parser_with_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let prompt_path = temp_dir.path().join("filename_prompt.txt");
    
    fs::write(&prompt_path, "Custom BJJ filename parsing prompt").await.unwrap();
    
    let config = LLMConfig::default();
    let parser = FilenameParser::new(config).with_prompt_file(prompt_path);
    
    // Should handle custom prompt
    assert!(parser.has_custom_prompt());
}

#[tokio::test]
async fn test_parsed_filename_json_serialization() {
    let parsed = ParsedFilename::new()
        .with_instructor("Mikey Musumeci".to_string())
        .with_series_name("Guard Magic".to_string())
        .with_part_number(4);
    
    let json = serde_json::to_string(&parsed).unwrap();
    let deserialized: ParsedFilename = serde_json::from_str(&json).unwrap();
    
    assert_eq!(parsed.instructor(), deserialized.instructor());
    assert_eq!(parsed.series_name(), deserialized.series_name());
    assert_eq!(parsed.part_number(), deserialized.part_number());
}

#[tokio::test]
async fn test_correction_response_creation() {
    let response = CorrectionResponse::new()
        .add_replacement("gard", "guard", Some("Spelling correction".to_string()))
        .add_replacement("arm drag", "arm-drag", Some("Hyphenation".to_string()))
        .with_notes("Minor corrections for BJJ terminology".to_string());
    
    assert_eq!(response.replacements().len(), 2);
    assert_eq!(response.notes(), Some("Minor corrections for BJJ terminology"));
    
    let first_replacement = &response.replacements()[0];
    assert_eq!(first_replacement.original(), "gard");
    assert_eq!(first_replacement.replacement(), "guard");
    assert_eq!(first_replacement.reason(), Some("Spelling correction"));
}

#[tokio::test]
async fn test_create_llm_factory() {
    let config = LLMConfig::default();
    
    // Should create LLM instance (might fail if LMStudio not available, but should not panic)
    let result = create_llm(&config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_transcription_corrector_creation() {
    let config = LLMConfig::default();
    
    // Should create corrector
    let corrector = TranscriptionCorrector::new(config);
    assert_eq!(corrector.provider_type(), LLMProvider::LMStudio);
}