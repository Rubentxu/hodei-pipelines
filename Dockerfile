# Multi-stage build for Hodei Jobs Platform
# Stage 1: Build the application
FROM rust:1.80 AS builder

# Install necessary tools
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/hodei-jobs

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates crates/

# Create dummy main to cache dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release
RUN rm src/main.rs

# Copy source code and build application
COPY . .
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r hodei && useradd -r -g hodei hodei

# Set working directory
WORKDIR /usr/src/hodei-jobs

# Copy binary from builder stage
COPY --from=builder /usr/src/hodei-jobs/target/release/hodei-jobs-api /usr/local/bin/hodei-jobs-api

# Copy configuration files
COPY config/ config/

# Set ownership
RUN chown -R hodei:hodei /usr/local/bin/hodei-jobs-api /usr/src/hodei-jobs

# Switch to non-root user
USER hodei

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command
CMD ["hodei-jobs-api"]
