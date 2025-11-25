# ========================================
# Dockerfile para Hodei Pipelines Server
# Arquitectura: Multi-stage build para optimización
# ========================================

# Stage 1: Build
FROM rust:1.90 as builder

# Instalar dependencias del sistema necesarias para compilar
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Configurar el workspace como directorio de trabajo
WORKDIR /app

# Copiar primero los archivos de configuración para cachear dependencias
COPY Cargo.toml ./
COPY crates/shared-types/Cargo.toml ./crates/shared-types/
COPY crates/core/Cargo.toml ./crates/core/
COPY crates/ports/Cargo.toml ./crates/ports/
COPY crates/modules/Cargo.toml ./crates/modules/
COPY crates/adapters/Cargo.toml ./crates/adapters/
COPY crates/hwp-proto/Cargo.toml ./crates/hwp-proto/
COPY server/Cargo.toml ./server/

# Crear estructura de directorios src vacía para build inicial
RUN mkdir -p crates/shared-types/src crates/core/src crates/ports/src crates/modules/src crates/adapters/src crates/hwp-proto/src server/src && \
    touch crates/shared-types/src/lib.rs crates/core/src/lib.rs crates/ports/src/lib.rs crates/modules/src/lib.rs crates/adapters/src/lib.rs crates/hwp-proto/src/lib.rs server/src/main.rs

# Pre-build para cachear dependencias (ignore errores)
RUN cargo check --package hodei-server 2>&1 || true

# Copiar todo el código fuente
COPY . .

# Compilar la aplicación en modo release
RUN cargo build --release --package hodei-server

# ========================================
# Stage 2: Runtime
# ========================================
FROM debian:testing-slim

# Instalar ca-certificates para HTTPS
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Crear usuario no-root para seguridad
RUN useradd -r -s /bin/false hodei

# Configurar directorio de trabajo
WORKDIR /app

# Copiar el binario compilado desde el stage de build
COPY --from=builder /app/target/release/hodei-server /usr/local/bin/hodei-server

# Copiar configuración de ejemplo (si existe)
# COPY monitoring/prometheus/prometheus.yml /app/config/

# Cambiar permisos del binario
RUN chmod +x /usr/local/bin/hodei-server && \
    chown -R hodei:hodei /app

# Cambiar al usuario no-root
USER hodei

# Exponer puertos
# 8080: HTTP API
# 50051: gRPC
EXPOSE 8080 50051

# Healthcheck para verificar que el servicio está funcionando
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Variables de entorno por defecto
ENV HODEI_PORT=8080
ENV HODEI_GRPC_PORT=50051
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Comando por defecto
CMD ["hodei-server"]
