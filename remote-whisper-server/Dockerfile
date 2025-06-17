# Multi-stage build for lightweight production image
FROM python:3.11-slim as builder

# Install system dependencies needed for building
RUN apt-get update && apt-get install -y \
    build-essential \
    ffmpeg \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create virtual environment
RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Copy requirements and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir --upgrade pip \
    && pip install --no-cache-dir -r requirements.txt

# Production stage
FROM python:3.11-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Copy virtual environment from builder
COPY --from=builder /opt/venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Create non-root user for security
RUN useradd --create-home --shell /bin/bash whisper
USER whisper
WORKDIR /home/whisper

# Copy application code
COPY --chown=whisper:whisper main.py .

# Create directories for models and temp files
RUN mkdir -p models temp

# Environment variables
ENV HOST=0.0.0.0
ENV PORT=8080
ENV WORKERS=1
ENV DEFAULT_MODEL=base

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD python -c "import requests; requests.get('http://localhost:8080/health')"

# Expose port
EXPOSE 8080

# Start server
CMD ["python", "main.py"]