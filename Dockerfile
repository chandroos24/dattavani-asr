# Multi-stage Dockerfile for Dattavani ASR Rust

# Build stage
FROM rust:1.75-bookworm as builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ffmpeg \
    ca-certificates \
    curl \
    python3 \
    python3-pip \
    && pip3 install --no-cache-dir openai-whisper \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app appuser

# Create necessary directories
RUN mkdir -p /app/logs /tmp/dattavani_asr /tmp/dattavani_cache \
    && chown -R appuser:appuser /app /tmp/dattavani_asr /tmp/dattavani_cache

# Copy the binary from builder stage
COPY --from=builder /app/target/release/dattavani-asr /usr/local/bin/dattavani-asr
RUN chmod +x /usr/local/bin/dattavani-asr

# Switch to app user
USER appuser
WORKDIR /app

# Set environment variables
ENV RUST_LOG=info
ENV TEMP_DIR=/tmp/dattavani_asr
ENV CACHE_DIR=/tmp/dattavani_cache

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD dattavani-asr health-check || exit 1

# Default command
ENTRYPOINT ["dattavani-asr"]
CMD ["--help"]
