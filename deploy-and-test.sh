#!/bin/bash
# Deploy and test BJJ Transcription Worker locally

set -e

echo "🚀 Deploying BJJ Transcription Worker locally"

# Configuration
IMAGE_NAME="bjj-transcription-worker:simple"
VIDEO_DIR="/Users/rolandlechner/SW Development/Test Files"
OUTPUT_DIR="./docker-output"
CONTAINER_NAME="bjj-transcription-test"

# Check if image exists
if ! docker images "$IMAGE_NAME" --format "{{.Repository}}:{{.Tag}}" | grep -q "$IMAGE_NAME"; then
    echo "❌ Docker image $IMAGE_NAME not found. Building..."
    docker build -f transcription-worker/Dockerfile.simple -t "$IMAGE_NAME" .
fi

echo "✅ Docker image found: $IMAGE_NAME"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if LMStudio is running
echo "🔍 Checking LMStudio connectivity..."
if curl -s http://localhost:1234/v1/models > /dev/null; then
    echo "✅ LMStudio is running"
else
    echo "⚠️  LMStudio not detected at localhost:1234"
    echo "   LLM correction will be disabled"
fi

# Check if whisper-cli is available on host
echo "🔍 Checking whisper-cli availability..."
if command -v whisper-cli > /dev/null; then
    echo "✅ whisper-cli found at: $(which whisper-cli)"
else
    echo "❌ whisper-cli not found on host system"
    echo "   Please install whisper.cpp first"
    exit 1
fi

# Stop any existing container
echo "🧹 Cleaning up existing containers..."
docker rm -f "$CONTAINER_NAME" 2>/dev/null || true

echo ""
echo "🧪 Test 1: Dry run to check video detection"
docker run --rm \
    --name "${CONTAINER_NAME}-dryrun" \
    -v "$VIDEO_DIR:/app/videos:ro" \
    -v "$OUTPUT_DIR:/app/output:rw" \
    -v "$(which whisper-cli):/usr/local/bin/whisper-cli:ro" \
    -v "$PWD/models:/app/models:ro" \
    --add-host host.docker.internal:host-gateway \
    "$IMAGE_NAME" \
    --video-dir /app/videos \
    --batch-size 1 \
    --dry-run \
    --verbose

echo ""
echo "🧪 Test 2: Process one video (with transcription)"
docker run --rm \
    --name "${CONTAINER_NAME}-process" \
    -v "$VIDEO_DIR:/app/videos:ro" \
    -v "$OUTPUT_DIR:/app/output:rw" \
    -v "$(which whisper-cli):/usr/local/bin/whisper-cli:ro" \
    -v "$PWD/models:/app/models:ro" \
    --add-host host.docker.internal:host-gateway \
    "$IMAGE_NAME" \
    --video-dir /app/videos \
    --batch-size 1 \
    --verbose

echo ""
echo "✅ Local deployment test complete!"
echo "📁 Output directory: $OUTPUT_DIR"
echo ""
echo "📊 Results:"
echo "   Docker image: $IMAGE_NAME"
echo "   Video directory: $VIDEO_DIR"
echo "   Output directory: $OUTPUT_DIR"
echo ""
echo "🎉 Ready for GPU deployment!"
echo ""
echo "To run on GPU machine:"
echo "  1. Copy this project to GPU machine"
echo "  2. Build GPU container: docker build -f transcription-worker/Dockerfile.gpu -t bjj-transcription-worker:gpu ."
echo "  3. Run with GPU: docker run --gpus all bjj-transcription-worker:gpu ..."