# Build stage
FROM rust:1-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and fetch dependencies for caching
COPY Cargo.toml Cargo.lock ./
# Dummy target so cargo accepts the manifest before real sources are copied
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo fetch --locked

# Copy source code
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations

# Build the application
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/brows3r /usr/local/bin/brows3r

# Copy templates and migrations
COPY templates ./templates
COPY migrations ./migrations

# Create a non-root user
RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app

USER appuser

EXPOSE 8000

CMD ["brows3r"]
