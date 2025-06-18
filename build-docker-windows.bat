@echo off
REM Build script for BJJ Transcription Worker Docker container on Windows
REM Requires Docker Desktop to be running

echo Building BJJ Transcription Worker Docker container on Windows...
echo.

REM Check if Docker is running
docker version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Docker is not running or not installed.
    echo Please start Docker Desktop and try again.
    exit /b 1
)

REM Configuration
set IMAGE_NAME=bjj-transcription-worker
set TAG=latest
set FULL_IMAGE_NAME=%IMAGE_NAME%:%TAG%

echo Building image: %FULL_IMAGE_NAME%
echo.

REM Build the Docker image
docker build -f transcription-worker/Dockerfile.simple -t %FULL_IMAGE_NAME% .

if %ERRORLEVEL% neq 0 (
    echo.
    echo ERROR: Docker build failed.
    echo Common issues on Windows:
    echo   1. Make sure Docker Desktop is running
    echo   2. Switch to Linux containers (right-click Docker Desktop tray icon)
    echo   3. Check Docker Desktop settings - Resources - WSL Integration
    echo   4. Try: docker system prune -a (to clean up space)
    exit /b 1
)

echo.
echo Build complete: %FULL_IMAGE_NAME%

REM Show image size
echo.
echo Image size:
docker images %IMAGE_NAME% --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"

REM Test the container
echo.
echo Testing container...
docker run --rm %FULL_IMAGE_NAME% --help

echo.
echo Docker build and test successful!
echo.
echo To run the container:
echo   docker run --rm -v "C:\path\to\videos:/app/videos" %FULL_IMAGE_NAME% --video-dir /app/videos
echo.
echo Or use docker-compose:
echo   set VIDEO_DIR=C:\path\to\videos
echo   docker-compose up transcription-worker