# Remote Whisper GPU Server

Lightweight Docker container for high-performance Whisper transcription with GPU acceleration.

## Features

- **GPU Accelerated**: 5-10x faster transcription with NVIDIA GPUs
- **Lightweight**: ~500MB Docker image vs 2GB+ alternatives  
- **REST API**: Simple HTTP interface for transcription requests
- **Model Management**: Dynamic loading/unloading of Whisper models
- **Health Monitoring**: Built-in health checks and monitoring
- **Production Ready**: Proper error handling, logging, and security

## Quick Start

### Prerequisites
- Docker and Docker Compose
- NVIDIA GPU with CUDA support (for GPU acceleration)
- Docker Desktop with GPU support enabled

### 1. Build and Start
```bash
./deploy.sh start
```

### 2. Test the Server
```bash
# Health check
curl http://localhost:8080/health

# List available models
curl http://localhost:8080/models

# Test transcription (replace with your audio file)
curl -X POST "http://localhost:8080/transcribe" \
  -F "audio=@your_audio_file.wav" \
  -F "model=base" \
  -F "language=en"
```

### 3. View API Documentation
Open http://localhost:8080/docs in your browser for interactive API documentation.

## API Endpoints

### POST /transcribe
Transcribe an audio file using Whisper.

**Parameters:**
- `audio` (file): Audio file to transcribe
- `model` (string): Whisper model to use (default: "base")
- `language` (string, optional): Language code for transcription
- `prompt` (string, optional): Initial prompt for better accuracy
- `temperature` (float): Sampling temperature (default: 0.0)
- `word_timestamps` (bool): Include word-level timestamps (default: true)

**Response:**
```json
{
  "text": "Transcribed text...",
  "language": "en",
  "segments": [...],
  "processing_time": 12.5,
  "model_used": "base"
}
```

### GET /health
Health check endpoint.

### GET /models
List available and loaded models.

### POST /models/{model_name}/load
Preload a specific model.

### DELETE /models/{model_name}
Unload a model to free memory.

## Configuration

Environment variables:

- `HOST`: Server host (default: "0.0.0.0")
- `PORT`: Server port (default: 8080)
- `WORKERS`: Number of worker processes (default: 1)
- `DEFAULT_MODEL`: Default Whisper model to load (default: "base")

## Available Models

- `tiny`, `tiny.en` - Fastest, lowest accuracy
- `base`, `base.en` - Good balance of speed and accuracy ⭐
- `small`, `small.en` - Better accuracy, slower
- `medium`, `medium.en` - High accuracy, slow
- `large`, `large-v1`, `large-v2`, `large-v3` - Best accuracy, slowest

## GPU Requirements

- **Minimum**: NVIDIA GTX 1060 or better
- **Recommended**: RTX 3080 or better for large models
- **Memory**: 4GB+ VRAM for base model, 8GB+ for large models

## Docker Commands

```bash
# Build image
docker build -t whisper-gpu-server .

# Run with GPU support
docker run --gpus all -p 8080:8080 whisper-gpu-server

# View logs
docker-compose logs -f

# Stop service
docker-compose down
```

## Integration with BJJ Analyzer

The Rust BJJ Video Analyzer will automatically detect and use this server when:
1. Server is running on `localhost:8080`
2. Transcription provider is set to "remote" or "auto"
3. Server passes health checks

## Troubleshooting

### GPU Not Detected
- Ensure NVIDIA drivers are installed
- Verify Docker has GPU access: `docker run --rm --gpus all nvidia/cuda:11.0-base nvidia-smi`
- Check Docker Desktop GPU settings

### High Memory Usage
- Use smaller models (`tiny`, `base`)
- Unload unused models via API
- Restart container to clear memory

### Connection Issues
- Check firewall settings
- Verify port 8080 is not in use
- Check server logs: `./deploy.sh logs`

## Performance Tips

- Use `base` model for best speed/accuracy balance
- Preload models for faster first transcription
- Use persistent volume for model caching
- Configure appropriate worker count for your GPU