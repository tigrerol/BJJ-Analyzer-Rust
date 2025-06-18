# Multi-stage Docker build for BJJ Transcription Worker
# Optimized for GPU support and minimal size

# Build stage
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ffmpeg \
    wget \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy all source code
COPY shared/ shared/
COPY transcription-worker/ transcription-worker/

# Build the transcription worker in release mode
RUN cargo build --release --bin bjj-transcription-worker

# Runtime stage - minimal image with dependencies
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    ffmpeg \
    wget \
    curl \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Build whisper.cpp from source (more reliable than pre-built binaries)
WORKDIR /tmp
RUN git clone https://github.com/ggerganov/whisper.cpp.git \
    && cd whisper.cpp \
    && make -j$(nproc) \
    && cp main /usr/local/bin/whisper-cli \
    && chmod +x /usr/local/bin/whisper-cli \
    && cd .. \
    && rm -rf whisper.cpp

# Create models directory and download base model
RUN mkdir -p /app/models
WORKDIR /app/models
RUN wget -O ggml-base.bin https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

# Create app user for security
RUN groupadd -r bjjapp && useradd -r -g bjjapp bjjapp

# Copy built binary from builder stage
COPY --from=builder /usr/src/app/target/release/bjj-transcription-worker /usr/local/bin/

# Create necessary directories
RUN mkdir -p /app/videos /app/output /app/models \
    && chown -R bjjapp:bjjapp /app

# Switch to app user
USER bjjapp
WORKDIR /app

# Set environment variables
ENV RUST_LOG=info
ENV WHISPER_MODEL_PATH=/app/models/ggml-base.bin

# Health check - test if binary works
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD /usr/local/bin/bjj-transcription-worker --help > /dev/null || exit 1

# Default command - help text
CMD ["/usr/local/bin/bjj-transcription-worker", "--help"]