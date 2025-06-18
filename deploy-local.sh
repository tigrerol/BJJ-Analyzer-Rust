#!/bin/bash
# Deploy BJJ Transcription Worker locally (no Docker required)

set -e

echo "🚀 Local BJJ Transcription Worker Deployment"

# Configuration
BINARY_PATH="./target/release/bjj-transcription-worker"
VIDEO_DIR="/Users/rolandlechner/SW Development/Test Files"
OUTPUT_DIR="./local-output"

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "🔨 Building release binary..."
    cargo build --release --bin bjj-transcription-worker
fi

echo "✅ Binary found: $BINARY_PATH"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check prerequisites
echo "🔍 Checking prerequisites..."

# Check LMStudio
if curl -s http://localhost:1234/v1/models > /dev/null; then
    echo "✅ LMStudio is running"
    LLM_STATUS="enabled"
else
    echo "⚠️  LMStudio not detected - LLM correction will be disabled"
    LLM_STATUS="disabled"
fi

# Check whisper-cli
if command -v whisper-cli > /dev/null; then
    echo "✅ whisper-cli found at: $(which whisper-cli)"
else
    echo "❌ whisper-cli not found. Please install whisper.cpp"
    exit 1
fi

# Check FFmpeg
if command -v ffmpeg > /dev/null; then
    echo "✅ FFmpeg found at: $(which ffmpeg)"
else
    echo "❌ FFmpeg not found. Please install FFmpeg"
    exit 1
fi

echo ""
echo "📋 Configuration:"
echo "   Binary: $BINARY_PATH"
echo "   Video directory: $VIDEO_DIR"
echo "   Output directory: $OUTPUT_DIR"
echo "   LLM correction: $LLM_STATUS"
echo ""

# Test 1: Dry run
echo "🧪 Test 1: Dry run (video detection)"
"$BINARY_PATH" \
    --video-dir "$VIDEO_DIR" \
    --batch-size 1 \
    --dry-run \
    --verbose

echo ""
echo "🧪 Test 2: Process one video batch"
echo "⚡ Starting transcription processing..."

# Run the actual processing
"$BINARY_PATH" \
    --video-dir "$VIDEO_DIR" \
    --batch-size 1 \
    --verbose &

WORKER_PID=$!
echo "📊 Worker started with PID: $WORKER_PID"

# Wait a bit and show progress
sleep 5
echo "⏱️  Processing in progress..."
echo "📁 Monitor output in: $VIDEO_DIR"
echo "🔍 Check logs above for detailed progress"
echo ""
echo "⏹️  To stop processing: kill $WORKER_PID"
echo "📈 To monitor: tail -f bjj_analyzer.log (if log file exists)"
echo ""
echo "✅ Local deployment successful!"
echo ""
echo "🎯 Next steps for GPU deployment:"
echo "  1. Copy entire project to GPU machine"
echo "  2. Install NVIDIA Container Toolkit"
echo "  3. Build GPU container: ./build-docker.sh gpu"
echo "  4. Run with GPU acceleration"

# Keep worker running
wait $WORKER_PID