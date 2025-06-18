#!/bin/bash
# Build script for BJJ Transcription Worker Docker container

set -e

echo "🐳 Building BJJ Transcription Worker Docker container..."

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Build arguments
IMAGE_NAME="bjj-transcription-worker"
TAG="${1:-latest}"
FULL_IMAGE_NAME="${IMAGE_NAME}:${TAG}"

echo "📦 Building image: $FULL_IMAGE_NAME"

# Build the Docker image
docker build \
  -f transcription-worker/Dockerfile \
  -t "$FULL_IMAGE_NAME" \
  .

echo "✅ Build complete: $FULL_IMAGE_NAME"

# Show image size
echo "📊 Image size:"
docker images "$IMAGE_NAME" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"

# Test the container
echo "🧪 Testing container..."
docker run --rm "$FULL_IMAGE_NAME" --help

echo "🎉 Docker build and test successful!"
echo ""
echo "To run the container:"
echo "  docker run --rm -v /path/to/videos:/app/videos $FULL_IMAGE_NAME --video-dir /app/videos"
echo ""
echo "Or use docker-compose:"
echo "  VIDEO_DIR=/path/to/videos docker-compose up transcription-worker"