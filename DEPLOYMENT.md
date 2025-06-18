# BJJ Transcription Worker - Deployment Guide

This guide covers local deployment and GPU deployment of the BJJ Transcription Worker.

## Overview

The BJJ Transcription Worker is a high-performance Rust application that:
- Extracts audio from video files
- Transcribes audio using Whisper.cpp 
- Applies LLM-based corrections for BJJ terminology
- Generates subtitle files (.srt)
- Provides 300x performance improvement over Python implementation

## Prerequisites

### Local Development
- Docker and Docker Compose
- LMStudio (for LLM corrections) running on `localhost:1234`
- whisper.cpp installed locally (`whisper-cli` binary)

### GPU Deployment
- NVIDIA GPU with CUDA support
- NVIDIA Container Toolkit
- Docker with GPU support

## Quick Start

### 1. Build Container
```bash
# Simple CPU-only container
./build-docker.sh

# Or manually:
docker build -f transcription-worker/Dockerfile.simple -t bjj-transcription-worker:simple .
```

### 2. Test Deployment
```bash
# Automated test
./deploy-and-test.sh

# Or manually:
docker run --rm bjj-transcription-worker:simple --help
```

### 3. Process Videos
```bash
# Using Docker Compose
VIDEO_DIR="/path/to/videos" docker-compose -f docker-compose.simple.yml up

# Or manually:
docker run --rm \
  -v "/path/to/videos:/app/videos:ro" \
  -v "./output:/app/output:rw" \
  -v "$(which whisper-cli):/usr/local/bin/whisper-cli:ro" \
  --add-host host.docker.internal:host-gateway \
  bjj-transcription-worker:simple \
  --video-dir /app/videos --batch-size 1 --verbose
```

## Configuration

### Environment Variables
```bash
# Logging
RUST_LOG=info              # info, debug, trace
RUST_BACKTRACE=1           # Enable stack traces

# LLM Configuration
LLM_ENDPOINT=http://host.docker.internal:1234/v1/chat/completions
OPENAI_API_KEY=your_key    # For OpenAI
GOOGLE_API_KEY=your_key    # For Gemini
```

### Volume Mounts
```bash
# Required volumes
-v "/path/to/videos:/app/videos:ro"           # Video input directory
-v "./output:/app/output:rw"                  # Output directory
-v "$(which whisper-cli):/usr/local/bin/whisper-cli:ro"  # Whisper binary

# Optional volumes  
-v "./models:/app/models:ro"                  # Whisper models cache
-v "./config:/app/config:ro"                  # Configuration files
```

## GPU Deployment

### 1. Build GPU Container
```bash
docker build -f transcription-worker/Dockerfile.gpu -t bjj-transcription-worker:gpu .
```

### 2. Run with GPU Support
```bash
# Using Docker Compose
docker-compose --profile gpu up transcription-worker-gpu

# Or manually:
docker run --rm --gpus all \
  -v "/path/to/videos:/app/videos:ro" \
  -v "./output:/app/output:rw" \
  bjj-transcription-worker:gpu \
  --video-dir /app/videos --batch-size 5 --verbose
```

### 3. Performance Tuning
```bash
# High-performance batch processing
docker run --rm --gpus all \
  -v "/path/to/videos:/app/videos:ro" \
  -v "./output:/app/output:rw" \
  --memory=8g \
  --cpus=4 \
  bjj-transcription-worker:gpu \
  --video-dir /app/videos \
  --batch-size 10 \
  --verbose
```

## Processing Pipeline

The worker processes videos through these stages:
1. **Pending** → **AudioExtracted** (FFmpeg)
2. **AudioExtracted** → **Transcribed** (Whisper.cpp)
3. **Transcribed** → **LLMCorrected** (LMStudio/OpenAI/Gemini)
4. **LLMCorrected** → **SubtitlesGenerated** (.srt files)
5. **SubtitlesGenerated** → **Completed**

### Artifact Detection
The worker automatically detects existing artifacts and skips completed stages:
- `.wav` files (audio extracted)
- `.txt` files (transcribed)
- `_corrected.txt` files (LLM corrected)
- `.srt` files (subtitles generated)

## Production Deployment

### Docker Compose Production
```yaml
# docker-compose.prod.yml
services:
  transcription-worker:
    image: bjj-transcription-worker:gpu
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
    volumes:
      - /mnt/nas/videos:/app/videos:ro
      - /mnt/nas/output:/app/output:rw
    environment:
      - RUST_LOG=info
      - BATCH_SIZE=20
    restart: unless-stopped
    command: ["--video-dir", "/app/videos", "--batch-size", "20", "--mode", "continuous"]
```

### Kubernetes Deployment
```yaml
# k8s-deployment.yml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: bjj-transcription-worker
spec:
  replicas: 1
  selector:
    matchLabels:
      app: bjj-transcription-worker
  template:
    metadata:
      labels:
        app: bjj-transcription-worker
    spec:
      containers:
      - name: transcription-worker
        image: bjj-transcription-worker:gpu
        resources:
          limits:
            nvidia.com/gpu: 1
            memory: 8Gi
            cpu: 4
        volumeMounts:
        - name: videos
          mountPath: /app/videos
          readOnly: true
        - name: output  
          mountPath: /app/output
        args:
        - "--video-dir"
        - "/app/videos"
        - "--batch-size"
        - "15"
        - "--mode"
        - "continuous"
      volumes:
      - name: videos
        nfs:
          server: nas.local
          path: /videos
      - name: output
        nfs:
          server: nas.local  
          path: /output
```

## Monitoring and Logs

### View Logs
```bash
# Container logs
docker logs bjj-transcription-worker -f

# Structured logging
docker logs bjj-transcription-worker 2>&1 | jq '.'
```

### Health Checks
```bash
# Container health
docker inspect bjj-transcription-worker --format='{{.State.Health.Status}}'

# Process status
docker exec bjj-transcription-worker ps aux
```

### Performance Metrics
```bash
# GPU usage (if available)
nvidia-smi

# Container resource usage
docker stats bjj-transcription-worker

# Processing throughput
docker logs bjj-transcription-worker | grep "Processing complete"
```

## Troubleshooting

### Common Issues

1. **Whisper not found**
   ```bash
   # Mount host whisper binary
   -v "$(which whisper-cli):/usr/local/bin/whisper-cli:ro"
   ```

2. **LLM connection failed**
   ```bash
   # Test LMStudio connectivity
   curl http://localhost:1234/v1/models
   
   # Add host networking
   --add-host host.docker.internal:host-gateway
   ```

3. **Out of memory**
   ```bash
   # Reduce batch size
   --batch-size 1
   
   # Increase container memory
   --memory=8g
   ```

4. **GPU not detected**
   ```bash
   # Check NVIDIA runtime
   docker run --rm --gpus all nvidia/cuda:12.2-runtime-ubuntu22.04 nvidia-smi
   ```

### Debug Mode
```bash
# Enable debug logging
-e RUST_LOG=debug

# Enable backtraces
-e RUST_BACKTRACE=full

# Dry run mode
--dry-run
```

## Performance Benchmarks

### CPU vs GPU Performance
- **CPU (8-core)**: ~30 seconds per minute of video
- **GPU (RTX 4090)**: ~3 seconds per minute of video
- **Memory usage**: 2-4GB per worker
- **Throughput**: 100-500 videos per hour (depending on length)

### Scaling Guidelines
- **Single video**: 1 worker, batch-size 1
- **Small batch (10-50 videos)**: 1 worker, batch-size 5-10
- **Large batch (100+ videos)**: 2-4 workers, batch-size 10-20
- **Production**: Continuous mode with NAS/network storage

## Security Considerations

- Containers run as non-root user (`bjjapp`)
- Read-only video mounts
- No sensitive data in container images
- Network isolation for LLM API calls
- Regular security updates for base images

## Support

For issues and improvements:
1. Check logs for error messages
2. Verify all prerequisites are installed
3. Test with single video first
4. Use dry-run mode for debugging
5. Monitor resource usage during processing