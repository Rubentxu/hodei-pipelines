# ========================================
# Dockerfile para HWP Agent (Worker)
# ========================================

# Stage 1: Build
FROM rust:1.90 as builder

WORKDIR /app

# Instalar dependencias
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Copiar dependencias de workspace
COPY Cargo.toml ./
COPY crates/hwp-proto/Cargo.toml ./crates/hwp-proto/
COPY crates/shared-types/Cargo.toml ./crates/shared-types/
COPY crates/core/Cargo.toml ./crates/core/
COPY crates/ports/Cargo.toml ./crates/ports/

# Crear estructura mínima para build inicial
RUN mkdir -p crates/hwp-proto/src crates/shared-types/src crates/core/src crates/ports/src && \
    touch crates/hwp-proto/src/lib.rs crates/shared-types/src/lib.rs crates/core/src/lib.rs crates/ports/src/lib.rs

# Pre-build para cachear dependencias (ignore errores)
RUN cargo check --package hwp-agent 2>&1 || true

# Copiar todo el código fuente
COPY . .
RUN cargo build --release --package hwp-agent

# ========================================
# Stage 2: Runtime
# ========================================
FROM debian:testing-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -r -s /bin/false hwp-agent && \
    mkdir -p /app /var/log/hwp-agent && \
    chown -R hwp-agent:hwp-agent /app /var/log/hwp-agent

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/hwp-agent /usr/local/bin/hwp-agent
RUN chmod +x /usr/local/bin/hwp-agent

# Switch to non-root user
USER hwp-agent

# Expose gRPC port
EXPOSE 50052

# Healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:50052/health 2>/dev/null || exit 1

# Default environment variables
ENV WORKER_ID=auto
ENV SERVER_URL=http://hodei-server:8080
ENV GRPC_SERVER_URL=http://hodei-server:50051
ENV RUST_LOG=info
ENV HWP_LOG_DIR=/var/log/hwp-agent

# Command to start the worker
CMD ["hwp-agent"]
