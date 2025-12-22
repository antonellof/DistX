# vectX Vector Database - Dockerfile
# Multi-stage build for minimal final image size

# ============================================
# Stage 1: Builder
# ============================================
FROM rust:1.83-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    liblmdb-dev \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    unzip \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy only Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY lib/core/Cargo.toml lib/core/
COPY lib/storage/Cargo.toml lib/storage/
COPY lib/api/Cargo.toml lib/api/

# Create dummy source files to build dependencies
RUN mkdir -p src lib/core/src lib/storage/src lib/api/src lib/api/proto && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    echo "pub fn dummy() {}" > lib/core/src/lib.rs && \
    echo "pub fn dummy() {}" > lib/storage/src/lib.rs && \
    echo "pub fn dummy() {}" > lib/api/src/lib.rs

# Copy proto files and build script
COPY lib/api/proto lib/api/proto/
COPY lib/api/build.rs lib/api/

# Build dependencies only (cached layer)
RUN cargo build --release 2>/dev/null || true

# Download and extract web UI (Qdrant Web UI - API compatible)
COPY tools/ tools/
RUN mkdir -p /static && STATIC_DIR=/static ./tools/sync-web-ui.sh

# Remove dummy sources and built artifacts for our crates
RUN rm -rf src lib/core/src lib/storage/src lib/api/src && \
    rm -rf target/release/vectx target/release/deps/vectx* && \
    rm -rf target/release/deps/libvectx* && \
    rm -rf target/release/.fingerprint/vectx* && \
    rm -rf target/release/.fingerprint/vectx_core* && \
    rm -rf target/release/.fingerprint/vectx_storage* && \
    rm -rf target/release/.fingerprint/vectx_api*

# Copy actual source code
COPY src src/
COPY lib lib/

# Touch source files to ensure rebuild and build the actual application
# Enable AVX2 and FMA for x86_64 SIMD performance
RUN find src lib -name "*.rs" -exec touch {} \; && \
    RUSTFLAGS="-C target-feature=+avx2,+fma -C opt-level=3" cargo build --release --bin vectx

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    liblmdb0 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash vectx

# Create data and static directories
RUN mkdir -p /qdrant/storage /qdrant/static && \
    chown -R vectx:vectx /qdrant

WORKDIR /qdrant

# Copy binary from builder
COPY --from=builder /build/target/release/vectx /usr/local/bin/vectx

# Copy web UI static files
COPY --from=builder --chown=vectx:vectx /static /qdrant/static

# Switch to non-root user
USER vectx

# Expose ports
# 6333 - REST API (Qdrant-compatible)
# 6334 - gRPC API
EXPOSE 6333 6334

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:6333/healthz || exit 1

# Default command
CMD ["vectx", "--data-dir", "/qdrant/storage", "--http-port", "6333", "--grpc-port", "6334"]

# Labels
LABEL org.opencontainers.image.title="vectX" \
      org.opencontainers.image.description="Fast in-memory vector database with Qdrant API compatibility" \
      org.opencontainers.image.url="https://github.com/antonellof/vectX" \
      org.opencontainers.image.source="https://github.com/antonellof/vectX" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"
