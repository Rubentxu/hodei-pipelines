# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy src to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release
RUN rm src/main.rs

# Copy source code
COPY src ./src
COPY ../../shared-types ./shared-types

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install ca-certificates
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Copy binary from builder
COPY --from=builder /app/target/release/scheduler /usr/local/bin/scheduler

# Set permissions
RUN chmod +x /usr/local/bin/scheduler

# Switch to app user
USER appuser

# Expose port
EXPOSE 8081

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# Set environment
ENV SCHEDULER_PORT=8081

# Run the application
CMD ["scheduler"]
