#!/bin/bash

# Remote Whisper GPU Server Deployment Script
set -e

echo "🚀 Building Remote Whisper GPU Server..."

# Build the Docker image
echo "📦 Building Docker image..."
docker build -t whisper-gpu-server:latest .

echo "✅ Docker image built successfully!"

# Check if we should start the service
if [ "$1" = "start" ] || [ "$1" = "run" ]; then
    echo "🌟 Starting Whisper GPU Server..."
    
    # Stop any existing container
    docker-compose down 2>/dev/null || true
    
    # Start the service
    docker-compose up -d
    
    echo "⏳ Waiting for server to start..."
    sleep 10
    
    # Test the server
    echo "🔍 Testing server health..."
    if curl -s http://localhost:8080/health | grep -q "healthy"; then
        echo "✅ Server is running and healthy!"
        echo "🌐 Server URL: http://localhost:8080"
        echo "📚 API Documentation: http://localhost:8080/docs"
        echo "🏥 Health Check: http://localhost:8080/health"
    else
        echo "❌ Server health check failed"
        echo "📋 Checking logs..."
        docker-compose logs
        exit 1
    fi
    
elif [ "$1" = "stop" ]; then
    echo "🛑 Stopping Whisper GPU Server..."
    docker-compose down
    echo "✅ Server stopped"
    
elif [ "$1" = "logs" ]; then
    echo "📋 Server logs:"
    docker-compose logs -f
    
else
    echo "✅ Build complete!"
    echo ""
    echo "📋 Available commands:"
    echo "  ./deploy.sh start   - Build and start the server"
    echo "  ./deploy.sh stop    - Stop the server"
    echo "  ./deploy.sh logs    - View server logs"
    echo ""
    echo "🌐 Manual start: docker-compose up -d"
fi