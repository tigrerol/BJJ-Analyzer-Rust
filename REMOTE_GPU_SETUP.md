# Remote GPU Whisper Setup Guide

This guide shows how to set up and use the remote GPU Whisper server for high-performance transcription in the BJJ Video Analyzer.

## Overview

The remote GPU setup allows you to:
- **5-10x faster transcription** with GPU acceleration
- **Offload processing** to a dedicated GPU machine
- **Maintain local fallback** if remote server is unavailable
- **Use larger models** (large-v3) that require more GPU memory

## Quick Start

### 1. Start the Remote Server

On your GPU machine:

```bash
cd remote-whisper-server/
./deploy.sh start
```

The server will be available at `http://localhost:8080`

### 2. Configure the Rust Client

Copy the example configuration:

```bash
cp config/remote-gpu-example.toml config/bjj-analyzer.toml
```

Edit the endpoint if your server is on a different machine:

```toml
[transcription]
provider = "Remote"
api_endpoint = "http://192.168.1.100:8080"  # Replace with your server IP
```

### 3. Run the Analyzer

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- --video-dir "/path/to/videos" --output-dir "./output"
```

## Detailed Setup

### Server Setup (GPU Machine)

#### Prerequisites
- NVIDIA GPU with CUDA support
- Docker Desktop with GPU runtime enabled
- 8GB+ GPU memory for large models

#### 1. Enable GPU Support in Docker

**Windows (Docker Desktop):**
1. Go to Settings → Resources → WSL Integration
2. Enable GPU support
3. Restart Docker Desktop

**Linux:**
```bash
# Install nvidia-docker2
sudo apt-get update
sudo apt-get install -y nvidia-docker2
sudo systemctl restart docker

# Test GPU access
docker run --rm --gpus all nvidia/cuda:11.0-base nvidia-smi
```

#### 2. Deploy the Server

```bash
cd remote-whisper-server/

# Build and start
./deploy.sh start

# Check status
curl http://localhost:8080/health

# View logs
./deploy.sh logs
```

#### 3. Test the Server

```bash
# Health check
curl http://localhost:8080/health

# Available models
curl http://localhost:8080/models

# Test transcription (replace with actual audio file)
curl -X POST "http://localhost:8080/transcribe" \
  -F "audio=@test.wav" \
  -F "model=base" \
  -F "language=en"
```

### Client Setup (Local Machine)

#### 1. Configuration

Create or modify `config/bjj-analyzer.toml`:

```toml
[transcription]
provider = "Remote"
api_endpoint = "http://YOUR_GPU_SERVER:8080"
enable_fallback = true  # Important for reliability
connection_timeout = 30
model = "base"  # or "large-v3" for best quality
```

#### 2. Network Setup

**Same Machine:**
- Use `http://localhost:8080`

**Different Machine (Local Network):**
- Use `http://192.168.1.XXX:8080`
- Ensure port 8080 is open on GPU machine

**Remote Machine (VPN/Cloud):**
- Use appropriate IP/domain
- Consider HTTPS and authentication for production

### Advanced Configuration

#### Model Selection

| Model | Speed | Accuracy | GPU Memory | Use Case |
|-------|--------|----------|------------|----------|
| `tiny` | Fastest | Basic | 1GB | Quick tests |
| `base` | Fast | Good | 2GB | **Recommended** |
| `small` | Medium | Better | 3GB | Higher quality |
| `medium` | Slow | High | 4GB | Professional |
| `large-v3` | Slowest | Best | 6GB+ | Maximum quality |

#### Performance Tuning

**Server (docker-compose.yml):**
```yaml
environment:
  - WORKERS=1  # Usually 1 for GPU workloads
  - DEFAULT_MODEL=base  # Preload model
```

**Client (config/bjj-analyzer.toml):**
```toml
[transcription]
upload_chunk_size = 20971520  # 20MB for faster uploads
connection_timeout = 60  # Longer timeout for large files
max_retries = 5

[performance]
max_workers = 2  # Don't overwhelm remote server
```

## Workflow Examples

### Basic Usage

```bash
# Start remote server
cd remote-whisper-server && ./deploy.sh start

# Process videos
cargo run -- --video-dir "/path/to/videos"
```

### High-Quality Processing

```toml
# config/bjj-analyzer.toml
[transcription]
provider = "Remote"
model = "large-v3"  # Best quality
temperature = 0.0   # Deterministic
word_timestamps = true
```

### Batch Processing

```bash
# Process multiple directories
for dir in /videos/series*; do
  cargo run -- --video-dir "$dir" --output-dir "./output/$(basename $dir)"
done
```

## Troubleshooting

### Server Issues

**GPU Not Available:**
```bash
# Check NVIDIA driver
nvidia-smi

# Check Docker GPU access
docker run --rm --gpus all nvidia/cuda:11.0-base nvidia-smi
```

**Container Won't Start:**
```bash
# Check logs
./deploy.sh logs

# Rebuild
docker-compose down
docker-compose build --no-cache
./deploy.sh start
```

**Out of Memory:**
```bash
# Use smaller model
curl -X POST "localhost:8080/models/base/load"
curl -X DELETE "localhost:8080/models/large-v3"
```

### Client Issues

**Connection Failed:**
- Check server health: `curl http://SERVER:8080/health`
- Verify network connectivity
- Check firewall settings

**Slow Performance:**
- Increase upload chunk size
- Use wired network connection
- Check server GPU utilization

**Fallback to Local:**
- Install local whisper: `brew install whisper-cpp`
- Check fallback setting: `enable_fallback = true`

### Network Optimization

**Local Network:**
```toml
[transcription]
upload_chunk_size = 52428800  # 50MB for fast local network
connection_timeout = 120
```

**Remote/VPN:**
```toml
[transcription]
upload_chunk_size = 5242880   # 5MB for slower connections
connection_timeout = 300
max_retries = 5
```

## Production Deployment

### Security

**Docker Compose (Production):**
```yaml
services:
  whisper-gpu:
    restart: unless-stopped
    environment:
      - API_KEY=your-secure-key  # Add authentication
    ports:
      - "127.0.0.1:8080:8080"  # Bind to localhost only
```

**Reverse Proxy (Nginx):**
```nginx
server {
    listen 443 ssl;
    server_name whisper.yourdomain.com;
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        client_max_body_size 100M;
    }
}
```

### Monitoring

**Health Checks:**
```bash
# Add to crontab
*/5 * * * * curl -s http://localhost:8080/health || systemctl restart docker-compose
```

**Resource Monitoring:**
```bash
# GPU usage
nvidia-smi -l 1

# Docker stats
docker stats whisper-gpu-server
```

## Performance Comparison

| Setup | Processing Time (10min video) | GPU Usage | Notes |
|-------|------------------------------|-----------|-------|
| Local CPU (base) | ~8 minutes | 0% | Baseline |
| Local GPU (base) | ~3 minutes | 80% | If available |
| Remote GPU (base) | ~2 minutes | 90% | + Network overhead |
| Remote GPU (large-v3) | ~5 minutes | 95% | Best quality |

## Cost Analysis

**Local Processing:**
- CPU: High CPU usage, slower
- GPU: If available, good performance

**Remote Processing:**
- **Pros**: Dedicated GPU, better models, parallel capability
- **Cons**: Network dependency, server maintenance
- **Sweet Spot**: Process multiple videos in batch

The remote GPU setup is especially beneficial for:
- Processing large video collections
- Using high-quality models (large-v3)
- Teams sharing GPU resources
- Freeing up local resources for other work