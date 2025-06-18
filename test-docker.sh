#!/bin/bash
# Test script for BJJ Transcription Worker Docker container

set -e

# Configuration
IMAGE_NAME="bjj-transcription-worker:latest"
TEST_VIDEO_DIR="/Users/rolandlechner/SW Development/Test Files"
OUTPUT_DIR="./docker-test-output"

echo "🧪 Testing BJJ Transcription Worker Docker container"
echo "📁 Test video directory: $TEST_VIDEO_DIR"
echo "📁 Output directory: $OUTPUT_DIR"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if image exists
if ! docker images "$IMAGE_NAME" --format "{{.Repository}}:{{.Tag}}" | grep -q "$IMAGE_NAME"; then
    echo "❌ Docker image $IMAGE_NAME not found. Please build it first:"
    echo "   ./build-docker.sh"
    exit 1
fi

echo "✅ Docker image found: $IMAGE_NAME"

# Test 1: Help command
echo ""
echo "🧪 Test 1: Help command"
docker run --rm "$IMAGE_NAME" --help

# Test 2: Dry run
echo ""
echo "🧪 Test 2: Dry run on test videos"
docker run --rm \
    -v "$TEST_VIDEO_DIR:/app/videos:ro" \
    -v "$OUTPUT_DIR:/app/output:rw" \
    "$IMAGE_NAME" \
    --video-dir /app/videos \
    --batch-size 1 \
    --dry-run \
    --verbose

# Test 3: Version info
echo ""
echo "🧪 Test 3: Check whisper-cli availability"
docker run --rm "$IMAGE_NAME" sh -c "whisper-cli --help | head -5"

# Test 4: Model files
echo ""
echo "🧪 Test 4: Check model files"
docker run --rm "$IMAGE_NAME" sh -c "ls -la /app/models/"

echo ""
echo "✅ All Docker tests passed!"
echo ""
echo "To run actual processing:"
echo "  docker run --rm \\"
echo "    -v \"$TEST_VIDEO_DIR:/app/videos:ro\" \\"
echo "    -v \"$OUTPUT_DIR:/app/output:rw\" \\"
echo "    --network host \\"
echo "    \"$IMAGE_NAME\" \\"
echo "    --video-dir /app/videos \\"
echo "    --batch-size 1 \\"
echo "    --verbose"