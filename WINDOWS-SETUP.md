# Windows 11 Setup Guide for BJJ Transcription Worker

This guide addresses Windows-specific setup and common Docker Desktop issues.

## Prerequisites

1. **Docker Desktop for Windows**
   - Download from: https://www.docker.com/products/docker-desktop/
   - Ensure WSL 2 backend is enabled
   - Switch to Linux containers (not Windows containers)

2. **WSL 2 (Windows Subsystem for Linux)**
   - Open PowerShell as Administrator:
   ```powershell
   wsl --install
   wsl --set-default-version 2
   ```

3. **Git for Windows**
   - Download from: https://git-scm.com/download/win
   - Use Git Bash for Unix-style commands

## Common Windows Docker Errors and Solutions

### Error: "The system cannot find the file specified"

This error typically occurs when:
1. Docker Desktop is not running
2. Docker is set to Windows containers instead of Linux
3. WSL 2 integration is not properly configured

**Solution:**
```powershell
# 1. Start Docker Desktop
# 2. Right-click Docker Desktop tray icon → "Switch to Linux containers"
# 3. Open Docker Desktop Settings → Resources → WSL Integration
#    Enable integration with your WSL 2 distros

# Test Docker
docker version

# If still having issues, reset Docker:
# Settings → Troubleshoot → Clean / Purge data
```

### Error: "DockerDesktopLinuxEngine"

This indicates Docker Desktop's Linux engine isn't properly initialized.

**Solution:**
```powershell
# Restart Docker Desktop service
net stop com.docker.service
net start com.docker.service

# Or restart via PowerShell (Admin)
Restart-Service Docker

# If that doesn't work, restart WSL
wsl --shutdown
# Then restart Docker Desktop
```

## Building on Windows

### Option 1: Using Windows Batch Script
```batch
# Clone the repository
git clone https://github.com/tigrerol/BJJ-Analyzer-Rust.git
cd BJJ-Analyzer-Rust

# Run Windows build script
build-docker-windows.bat
```

### Option 2: Using Git Bash
```bash
# In Git Bash terminal
./build-docker.sh
```

### Option 3: Direct Docker Command
```powershell
# In PowerShell or Command Prompt
docker build -f transcription-worker/Dockerfile.simple -t bjj-transcription-worker:simple .
```

## Windows-Specific Docker Compose

Create a `docker-compose.windows.yml`:

```yaml
version: '3.8'

services:
  transcription-worker:
    build:
      context: .
      dockerfile: transcription-worker/Dockerfile.simple
    image: bjj-transcription-worker:simple
    container_name: bjj-transcription-worker
    
    volumes:
      # Windows path format
      - ${VIDEO_DIR:-C:\Videos}:/app/videos:ro
      - ${OUTPUT_DIR:-.\output}:/app/output:rw
      
    environment:
      - RUST_LOG=${RUST_LOG:-info}
      - LLM_ENDPOINT=http://host.docker.internal:1234/v1/chat/completions
    
    # Windows-specific host networking
    extra_hosts:
      - "host.docker.internal:host-gateway"
```

## Running the Container on Windows

### PowerShell
```powershell
# Set environment variables
$env:VIDEO_DIR = "C:\Users\YourName\Videos"
$env:OUTPUT_DIR = ".\output"

# Run with Docker Compose
docker-compose -f docker-compose.windows.yml up

# Or run directly
docker run --rm `
  -v "${env:VIDEO_DIR}:/app/videos:ro" `
  -v "${env:OUTPUT_DIR}:/app/output:rw" `
  --add-host "host.docker.internal:host-gateway" `
  bjj-transcription-worker:simple `
  --video-dir /app/videos --batch-size 1
```

### Command Prompt
```batch
# Set environment variables
set VIDEO_DIR=C:\Users\YourName\Videos
set OUTPUT_DIR=.\output

# Run container
docker run --rm ^
  -v "%VIDEO_DIR%:/app/videos:ro" ^
  -v "%OUTPUT_DIR%:/app/output:rw" ^
  --add-host "host.docker.internal:host-gateway" ^
  bjj-transcription-worker:simple ^
  --video-dir /app/videos --batch-size 1
```

## Local Development on Windows (Without Docker)

If you want to run without Docker:

1. **Install Rust**
   ```powershell
   # Download and run rustup-init.exe from https://rustup.rs/
   # Or use winget:
   winget install Rustlang.Rust
   ```

2. **Install Dependencies**
   - FFmpeg: Download from https://ffmpeg.org/download.html
   - Add FFmpeg to PATH
   - Whisper.cpp: Build from source or use pre-built binaries

3. **Build and Run**
   ```powershell
   # Build
   cargo build --release --bin bjj-transcription-worker

   # Run
   .\target\release\bjj-transcription-worker.exe --video-dir "C:\Videos" --batch-size 1
   ```

## GPU Support on Windows

For NVIDIA GPU support:

1. **Install NVIDIA Container Toolkit**
   - Requires WSL 2 with GPU support
   - Install NVIDIA drivers in Windows (not WSL)
   - Follow: https://docs.nvidia.com/cuda/wsl-user-guide/

2. **Build GPU Container**
   ```powershell
   docker build -f transcription-worker/Dockerfile.gpu -t bjj-transcription-worker:gpu .
   ```

3. **Run with GPU**
   ```powershell
   docker run --rm --gpus all `
     -v "C:\Videos:/app/videos:ro" `
     bjj-transcription-worker:gpu
   ```

## Troubleshooting Windows Issues

### 1. Path Format Issues
- Use forward slashes in Docker volume mounts: `C:/Users/Name/Videos`
- Or escape backslashes: `C:\\Users\\Name\\Videos`
- In Git Bash, use Unix-style paths: `/c/Users/Name/Videos`

### 2. Line Ending Issues
```bash
# If you get errors about line endings
git config --global core.autocrlf input
git clone https://github.com/tigrerol/BJJ-Analyzer-Rust.git
```

### 3. Permission Issues
- Run Docker Desktop as Administrator
- Ensure your user is in the docker-users group
- Check Windows Defender/Antivirus exclusions

### 4. WSL 2 Memory Issues
Create `.wslconfig` in your Windows user folder:
```ini
[wsl2]
memory=8GB
processors=4
```

### 5. Docker Desktop Resources
- Docker Desktop → Settings → Resources
- Increase CPU and Memory allocation
- Enable WSL 2 based engine

## Quick Test

After setup, test with:
```powershell
# Test Docker
docker run hello-world

# Test our container
docker run --rm bjj-transcription-worker:simple --help
```

If you continue to have issues, please share:
1. Exact error message
2. Docker Desktop version
3. Whether WSL 2 is enabled
4. Output of `docker version`