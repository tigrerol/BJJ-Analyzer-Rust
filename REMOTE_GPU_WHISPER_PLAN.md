# Remote GPU Whisper Integration Plan

## **🎯 Architecture Overview**
Transform the current local whisper.cpp integration into a hybrid system that can offload transcription to a remote GPU machine while maintaining local processing for other tasks.

## **🏗️ High-Level Design**

### **Current State**
- Local whisper.cpp via command line (`whisper-cli`)
- All processing happens on local machine
- No GPU acceleration capability

### **Target State**
- **Local Machine**: Video analysis, audio extraction, LLM correction, file generation
- **Remote GPU Machine**: High-speed whisper transcription only
- **Hybrid Fallback**: Local whisper.cpp if remote unavailable

## **📋 Implementation Plan**

### **Phase 1: Remote Whisper Service (2-3 hours)**
- [ ] **Create remote whisper server** on GPU machine
  - REST API endpoint for transcription requests
  - Accept audio file uploads or audio data
  - Return structured transcription with timestamps
  - Support BJJ prompts and model selection
  - Health check and status endpoints

- [ ] **Design API contract**
  ```
  POST /transcribe
  - Accept: audio file + model + prompt + language
  - Return: JSON with segments, timestamps, text
  ```

### **Phase 2: Client Integration (1-2 hours)**
- [ ] **Create RemoteWhisperClient** in Rust
  - HTTP client for API communication
  - File upload with progress tracking
  - Response parsing and validation
  - Connection pooling and retries

- [ ] **Extend WhisperTranscriber**
  - Add remote backend option alongside local whisper.cpp
  - Automatic fallback: Remote → Local → Error
  - Configuration for remote endpoint

### **Phase 3: Configuration & Management (1 hour)**
- [ ] **Configuration updates**
  ```toml
  [transcription]
  provider = "remote" # or "local" or "auto"
  remote_endpoint = "http://gpu-server:8080"
  timeout = 3600
  fallback_to_local = true
  ```

- [ ] **Connection management**
  - Health checks before processing
  - Automatic endpoint discovery
  - Load balancing for multiple GPU machines

### **Phase 4: Optimization Features (1-2 hours)**
- [ ] **Performance enhancements**
  - Parallel uploads for multiple videos
  - Progress streaming from remote server
  - Bandwidth optimization (audio compression)
  - Resume capability for interrupted transfers

- [ ] **Monitoring and logging**
  - Remote processing time tracking
  - Network transfer metrics
  - Error rate monitoring
  - Cost analysis (local vs remote processing time)

## **🛠️ Technical Implementation**

### **Remote Server Stack Options**
1. **Python FastAPI + whisper.cpp** (Recommended)
   - Fast, lightweight, easy deployment
   - Direct whisper.cpp integration
   - GPU acceleration via CUDA/Metal

2. **Rust Axum + whisper-rs** 
   - Native Rust integration
   - Better performance consistency
   - More complex setup

3. **Docker containerized service**
   - Easy deployment and scaling
   - Isolated environment
   - Version management

### **Client-Side Changes**
```rust
// New remote transcription backend
pub struct RemoteWhisperClient {
    endpoint: String,
    client: reqwest::Client,
    timeout: Duration,
}

// Updated transcriber with remote option  
impl WhisperTranscriber {
    async fn run_remote_whisper_command(&self, ...) -> Result<WhisperOutput>
    async fn fallback_to_local(&self, ...) -> Result<WhisperOutput>
}
```

### **Configuration Schema**
```toml
[transcription]
provider = "auto" # auto, remote, local
remote_endpoint = "http://192.168.1.100:8080"
remote_timeout = 3600
enable_fallback = true
upload_chunk_size = "10MB"
compression_level = 3

[transcription.remote]
api_key = "optional-auth-token"
model_preference = "large-v3"
concurrent_uploads = 2
retry_attempts = 3
```

## **🚀 Deployment Strategy**

### **GPU Server Setup**
1. **Hardware requirements**
   - NVIDIA GPU with CUDA support (RTX 3080+ recommended)
   - 16GB+ RAM for large models
   - SSD storage for model files
   - Fast network connection

2. **Software stack**
   - CUDA toolkit installation
   - whisper.cpp with GPU support
   - Web server (FastAPI/Nginx)
   - Model downloads (base, large, large-v3)

### **Network Considerations**
- **Local network**: Fast, low latency, no bandwidth costs
- **VPN connection**: Secure remote access
- **Cloud instance**: Scalable but with bandwidth costs
- **Hybrid approach**: Local for small files, remote for large files

## **📊 Benefits Analysis**

### **Performance Gains**
- **GPU acceleration**: 5-10x faster transcription
- **Parallel processing**: Multiple videos simultaneously
- **Larger models**: Access to large-v3 for better accuracy
- **Resource optimization**: Free up local CPU for other tasks

### **Flexibility**
- **Scalability**: Add more GPU workers as needed
- **Cost efficiency**: Share GPU resources across multiple machines
- **Model variety**: Quick switching between whisper models
- **Fallback reliability**: Always works if local whisper available

## **🔄 Migration Path**
1. **Phase 1**: Implement alongside existing local whisper
2. **Phase 2**: Add remote option with manual configuration
3. **Phase 3**: Enable auto-detection and fallback
4. **Phase 4**: Optimize for production use

## **⚡ Quick Start Option**
- Set up simple Python FastAPI server on GPU machine
- Modify existing `run_whisper_cpp_command` to try remote first
- 2-3 hours to basic working system
- Gradual enhancement over multiple sessions

## **Example Server Implementation (Python FastAPI)**

```python
from fastapi import FastAPI, File, UploadFile, Form
import whisper
import tempfile
import os

app = FastAPI()

# Load model once at startup
model = whisper.load_model("large-v3")

@app.post("/transcribe")
async def transcribe_audio(
    audio: UploadFile = File(...),
    language: str = Form("en"),
    prompt: str = Form(""),
    model_size: str = Form("large-v3")
):
    # Save uploaded file temporarily
    with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as tmp_file:
        tmp_file.write(await audio.read())
        tmp_path = tmp_file.name
    
    try:
        # Transcribe with whisper
        result = model.transcribe(
            tmp_path,
            language=language,
            initial_prompt=prompt,
            word_timestamps=True
        )
        
        # Format response for Rust client
        return {
            "text": result["text"],
            "language": result["language"],
            "segments": [
                {
                    "start": seg["start"],
                    "end": seg["end"], 
                    "text": seg["text"]
                }
                for seg in result["segments"]
            ]
        }
    finally:
        os.unlink(tmp_path)

@app.get("/health")
async def health_check():
    return {"status": "healthy", "model": "large-v3"}
```

## **Example Client Integration (Rust)**

```rust
pub struct RemoteWhisperClient {
    client: reqwest::Client,
    endpoint: String,
    timeout: Duration,
}

impl RemoteWhisperClient {
    pub async fn transcribe_audio(&self, audio_path: &Path, prompt: &str) -> Result<WhisperOutput> {
        let form = reqwest::multipart::Form::new()
            .file("audio", audio_path).await?
            .text("prompt", prompt.to_string())
            .text("language", "en");

        let response = self.client
            .post(&format!("{}/transcribe", self.endpoint))
            .multipart(form)
            .timeout(self.timeout)
            .send()
            .await?;

        let transcription: WhisperOutput = response.json().await?;
        Ok(transcription)
    }
}
```