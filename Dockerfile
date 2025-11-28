# Multi-stage build for optimized production image
# Stage 1: Build the application
FROM rust:1.83-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY bin ./bin

# Build dependencies only (for caching)
RUN cargo build --release --locked

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/serv /app/serv

# Copy migrations for runtime execution
COPY crates/mms-db/migrations /app/migrations

# Set ownership
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/bin/sh", "-c", "wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1"]

# Set environment defaults
ENV RUST_LOG=info \
    ENV=production

# Run the binary
CMD ["/app/serv"]
