#!/bin/bash

# ========================================
# Script para iniciar Worker Bare Metal
# Uso: ./start-worker.sh [worker-name]
# ========================================

set -e

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuración por defecto
WORKER_NAME=${1:-worker-bare-metal-$(hostname)}
SERVER_URL=${SERVER_URL:-"http://localhost:8080"}
GRPC_SERVER_URL=${GRPC_SERVER_URL:-"http://localhost:50051"}
WORKER_CPU_CORES=${WORKER_CPU_CORES:-$(nproc)}
WORKER_MEMORY_GB=${WORKER_MEMORY_GB:-$(free -g | awk '/^Mem:/{print $2}')}
HWP_LOG_DIR=${HWP_LOG_DIR:-"/var/log/hwp-agent"}

# Función para imprimir mensajes
print_header() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Banner
print_header "HWP Worker - Bare Metal Startup"

# Verificar prerrequisitos
print_header "Verificando Prerrequisitos"

# Verificar que hwp-agent esté instalado
if [ ! -f "/usr/local/bin/hwp-agent" ]; then
    print_error "hwp-agent no encontrado en /usr/local/bin/hwp-agent"
    print_header "Instalando hwp-agent..."

    # Detectar arquitectura
    ARCH=$(uname -m)
    DOWNLOAD_URL=""

    case $ARCH in
        x86_64)
            DOWNLOAD_URL="https://github.com/Rubentxu/hodei-jobs/releases/latest/download/hwp-agent-linux-amd64"
            ;;
        aarch64|arm64)
            DOWNLOAD_URL="https://github.com/Rubentxu/hodei-jobs/releases/latest/download/hwp-agent-linux-arm64"
            ;;
        *)
            print_error "Arquitectura no soportada: $ARCH"
            print_info "Por favor compila hwp-agent desde el código fuente"
            exit 1
            ;;
    esac

    # Descargar
    curl -L -o /tmp/hwp-agent "$DOWNLOAD_URL"
    chmod +x /tmp/hwp-agent
    sudo mv /tmp/hwp-agent /usr/local/bin/hwp-agent
    print_success "hwp-agent instalado"
fi

print_success "hwp-agent encontrado"

# Verificar conectividad al servidor
print_header "Verificando Conectividad"

if curl -s -f "$SERVER_URL/health" > /dev/null 2>&1; then
    print_success "Servidor disponible en $SERVER_URL"
else
    print_warning "Servidor no responde en $SERVER_URL"
    print_info "Verificando en puerto alternativo..."

    ALT_URL="http://localhost:8080"
    if curl -s -f "$ALT_URL/health" > /dev/null 2>&1; then
        SERVER_URL="$ALT_URL"
        GRPC_SERVER_URL="http://localhost:50051"
        print_success "Usando servidor local"
    else
        print_error "No se pudo conectar al servidor"
        print_info "Asegúrate de que el servidor esté ejecutándose:"
        print_info "  docker compose up -d hodei-server"
        print_info "  O configura SERVER_URL correctamente"
        print_info ""
        read -p "¿Continuar de todos modos? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

# Crear usuario si no existe
print_header "Configurando Usuario"
if ! id -u hwp-agent > /dev/null 2>&1; then
    sudo useradd -r -s /bin/false hwp-agent
    print_success "Usuario hwp-agent creado"
else
    print_success "Usuario hwp-agent existe"
fi

# Crear directorio de logs
print_header "Configurando Directorios"
sudo mkdir -p "$HWP_LOG_DIR"
sudo chown hwp-agent:hwp-agent "$HWP_LOG_DIR"
print_success "Directorio de logs configurado: $HWP_LOG_DIR"

# Configurar worker
print_header "Configuración del Worker"
echo "Worker Name: $WORKER_NAME"
echo "CPU Cores: $WORKER_CPU_CORES"
echo "Memory (GB): $WORKER_MEMORY_GB"
echo "Server URL: $SERVER_URL"
echo "gRPC Server URL: $GRPC_SERVER_URL"
echo "Log Directory: $HWP_LOG_DIR"
echo ""

# Registrar worker en el servidor
print_header "Registrando Worker en el Servidor"

REGISTER_DATA=$(cat <<EOF
{
    "name": "$WORKER_NAME",
    "cpu_cores": $WORKER_CPU_CORES,
    "memory_gb": $WORKER_MEMORY_GB
}
EOF
)

if curl -s -X POST "$SERVER_URL/api/v1/workers" \
    -H "Content-Type: application/json" \
    -d "$REGISTER_DATA" > /dev/null 2>&1; then
    print_success "Worker registrado exitosamente"
else
    print_warning "Error al registrar worker (puede que ya esté registrado)"
fi

# Iniciar hwp-agent
print_header "Iniciando HWP Agent"

export WORKER_ID="$WORKER_NAME"
export WORKER_NAME="$WORKER_NAME"
export SERVER_URL="$SERVER_URL"
export GRPC_SERVER_URL="$GRPC_SERVER_URL"
export WORKER_CPU_CORES="$WORKER_CPU_CORES"
export WORKER_MEMORY_GB="$WORKER_MEMORY_GB"
export HWP_LOG_DIR="$HWP_LOG_DIR"
export RUST_LOG="${RUST_LOG:-info}"

print_info "Ejecutando hwp-agent..."

# Ejecutar en foreground para testing
if [ "${2:-}" = "--foreground" ]; then
    sudo -u hwp-agent -E /usr/local/bin/hwp-agent
else
    # Ejecutar en background con systemd (si está disponible)
    if command -v systemctl &> /dev/null; then
        print_header "Creando systemd service"

        SERVICE_FILE="/etc/systemd/system/hwp-worker@.service"
        sudo tee "$SERVICE_FILE" > /dev/null <<EOF
[Unit]
Description=HWP Worker Agent (%i)
After=network.target

[Service]
Type=simple
User=hwp-agent
Group=hwp-agent
WorkingDirectory=/app
ExecStart=/usr/local/bin/hwp-agent \
  --server-url=$SERVER_URL \
  --grpc-server-url=$GRPC_SERVER_URL \
  --worker-id=%i
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=HWP_LOG_DIR=$HWP_LOG_DIR

# Security hardening
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ReadWritePaths=$HWP_LOG_DIR
ProtectHome=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes

[Install]
WantedBy=multi-user.target
EOF

        sudo systemctl daemon-reload
        sudo systemctl enable "hwp-worker@$WORKER_NAME"
        sudo systemctl start "hwp-worker@$WORKER_NAME"

        print_success "Worker iniciado como servicio systemd"
        print_info "Para ver logs: sudo journalctl -u hwp-worker@$WORKER_NAME -f"
    else
        # Ejecutar en background sin systemd
        print_warning "systemd no disponible, ejecutando en background"
        nohup sudo -u hwp-agent -E /usr/local/bin/hwp-agent \
            > "$HWP_LOG_DIR/worker.log" 2>&1 &
        WORKER_PID=$!
        echo "$WORKER_PID" > /tmp/hwp-worker.pid
        print_success "Worker iniciado con PID: $WORKER_PID"
        print_info "Log file: $HWP_LOG_DIR/worker.log"
    fi
fi

# Resumen final
print_header "¡Worker Iniciado!"
echo ""
echo "Worker ID: $WORKER_NAME"
echo "Status: Running"
echo "Log: $HWP_LOG_DIR/worker.log"
echo ""
print_success "Para verificar estado:"
echo "  curl $SERVER_URL/api/v1/workers"
echo ""
print_success "Para detener el worker:"
if command -v systemctl &> /dev/null; then
    echo "  sudo systemctl stop hwp-worker@$WORKER_NAME"
else
    echo "  kill \$(cat /tmp/hwp-worker.pid)"
fi
echo ""
