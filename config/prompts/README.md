# BJJ Video Analyzer - Prompt Templates

This directory contains all the prompt templates used by the BJJ Video Analyzer for various LLM-powered tasks. These prompts can be edited without recompiling the Rust application.

## Available Prompts

### 1. `correction.txt`
**Purpose**: LLM-based transcription correction for BJJ terminology  
**Usage**: Identifies and corrects common speech-to-text errors in BJJ instructional videos  
**Format**: Returns corrections in "original -> corrected" format  
**Example corrections**: "coast guard" → "closed guard", "half cord" → "half guard"

### 2. `whisper_transcription.txt`
**Purpose**: Context prompt for Whisper speech-to-text transcription  
**Usage**: Provides BJJ terminology context to improve transcription accuracy  
**Variables**: `{key_terms}` is replaced with relevant BJJ terms from the dictionary  
**Note**: Keep this prompt concise as Whisper has token limits

### 3. `summary_high_level.txt`
**Purpose**: High-level summary generation for BJJ instructional videos  
**Usage**: Creates accessible summaries focusing on main concepts and learning objectives  
**Target audience**: Students looking for quick overview of video content  
**Output**: Structured summary with main topics, positions, and key points

### 4. `summary_technical.txt`
**Purpose**: Detailed technical breakdown for advanced practitioners  
**Usage**: Generates comprehensive analysis with step-by-step mechanics  
**Target audience**: Experienced practitioners and instructors  
**Output**: Detailed technical breakdown with grips, positions, and troubleshooting

### 5. `mermaid_flowchart.txt`
**Purpose**: Generates Mermaid flowchart diagrams for technique flows  
**Usage**: Creates visual representations of technique progressions and decision points  
**Output**: Mermaid syntax flowchart showing technique flow and alternatives  
**Features**: Color-coded nodes for different types of actions and decisions

## Customization Guidelines

### Prompt Structure
- Keep system messages clear and specific
- Use consistent formatting instructions
- Include examples where helpful
- Specify output format requirements

### BJJ Terminology
- Include common transcription errors and corrections
- Use standard BJJ terminology consistently
- Consider both English and Portuguese/Japanese terms
- Account for different regional naming conventions

### Variables
- `{key_terms}`: Replaced with relevant BJJ terms from dictionary
- Variables are case-sensitive
- Ensure variable names match those used in the code

### Best Practices
1. **Test prompts** with your specific LLM provider
2. **Keep context relevant** to the specific task
3. **Use clear instructions** for output formatting
4. **Include examples** for complex formatting requirements
5. **Consider token limits** for your LLM provider

## Configuration

Prompts are configured in the main application config:

```toml
[llm.prompts]
prompt_dir = "config/prompts"
correction_file = "correction.txt"
summary_high_level_file = "summary_high_level.txt"
summary_technical_file = "summary_technical.txt"
mermaid_flowchart_file = "mermaid_flowchart.txt"
whisper_transcription_file = "whisper_transcription.txt"
```

## Adding New Prompts

1. Create new `.txt` file in this directory
2. Add configuration entry in `config.rs`
3. Update `PromptConfig` struct with new field
4. Add loader method to `PromptConfig` implementation
5. Use the prompt in your processing logic

## Error Handling

- If a prompt file is missing, the application will fall back to hardcoded defaults
- Invalid prompts will be logged as warnings
- The application will continue processing with default prompts if files are inaccessible

## Performance Notes

- Prompts are loaded once at startup
- Template processing is done in-memory
- Large prompts may affect LLM response times and token usage
- Consider your LLM provider's token limits when crafting prompts