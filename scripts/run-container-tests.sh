#!/bin/bash
# =============================================================================
# Script para Ejecutar Tests con Contenedores de Forma Segura
# =============================================================================
#
# ESTE SCRIPT:
# - Ejecuta tests con contenedores PostgreSQL de forma SECUENCIAL
# - Monitorea y limita el uso de memoria
# - Limpia recursos automáticamente antes y después
# - Proporciona reportes detallados
#
# USO:
#   ./scripts/run-container-tests.sh                    # Ejecuta todos los tests con contenedores
#   ./scripts/run-container-tests.sh --test us_prod_02  # Ejecuta test específico
#   ./scripts/run-container-tests.sh --memory-limit 256 # Límite personalizado
#
# MEMORIA ESPERADA:
# - Sin este script: ~4GB (múltiples contenedores)
# - Con este script: ~512MB (contenedor único compartido)
# =============================================================================

set -euo pipefail  # Exit on error, undefined vars, pipe failures

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuración por defecto
MEMORY_LIMIT_MB=512
CPU_LIMIT=512
TEST_THREADS=1
CARGO_PROFILE="container-tests"
CLEANUP_BEFORE=true
MONITOR_MEMORY=true
LOG_LEVEL="info"

# Parsear argumentos
while [[ $# -gt 0 ]]; do
    case $1 in
        --test)
            SPECIFIC_TEST="$2"
            shift 2
            ;;
        --memory-limit)
            MEMORY_LIMIT_MB="$2"
            shift 2
            ;;
        --cpu-limit)
            CPU_LIMIT="$2"
            shift 2
            ;;
        --no-cleanup)
            CLEANUP_BEFORE=false
            shift
            ;;
        --no-monitor)
            MONITOR_MEMORY=false
            shift
            ;;
        --log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        --help|-h)
            echo -e "${CYAN}Uso: $0 [opciones]"
            echo ""
            echo "Opciones:"
            echo "  --test TEST_NAME        Ejecutar test específico"
            echo "  --memory-limit MB       Límite de memoria en MB (default: 512)"
            echo "  --cpu-limit LIMIT       Límite de CPU shares (default: 512)"
            echo "  --no-cleanup            No limpiar Docker antes de empezar"
            echo "  --no-monitor            No monitorear memoria"
            echo "  --log-level LEVEL       Nivel de log (trace, debug, info, warn, error)"
            echo "  --help, -h              Mostrar esta ayuda"
            exit 0
            ;;
        *)
            echo -e "${RED}❌ Opción desconocida: $1${NC}"
            exit 1
            ;;
    esac
done

# Directorio del proyecto
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

# Funciones de utilidad
print_header() {
    echo -e "\n${PURPLE}================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}================================${NC}\n"
}

print_step() {
    echo -e "${BLUE}▶ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Verificar dependencias
check_dependencies() {
    print_step "Verificando dependencias..."

    if ! command -v cargo &> /dev/null; then
        print_error "Cargo no está instalado"
        exit 1
    fi
    print_success "Cargo encontrado: $(cargo --version)"

    if ! command -v docker &> /dev/null; then
        print_error "Docker no está instalado"
        exit 1
    fi
    print_success "Docker encontrado: $(docker --version)"

    if ! systemctl is-active --quiet docker; then
        print_error "Docker daemon no está ejecutándose"
        exit 1
    fi
    print_success "Docker daemon está activo"
}

# Limpiar recursos de Docker
cleanup_docker() {
    if [ "$CLEANUP_BEFORE" = true ]; then
        print_step "Limpiando recursos de Docker..."

        # Mostrar estado inicial
        CONTAINERS_BEFORE=$(docker ps -aq | wc -l)
        IMAGES_BEFORE=$(docker images -aq | wc -l)

        if [ "$CONTAINERS_BEFORE" -gt 0 ]; then
            print_warning "Encontrados $CONTAINERS_BEFORE contenedores detenido(s)"
            docker container prune -f > /dev/null 2>&1 || true
        fi

        # Limpiar imágenes huérfanas (con cuidado)
        print_warning "Limpiando imágenes Docker huérfanas..."
        docker image prune -af > /dev/null 2>&1 || true

        CONTAINERS_AFTER=$(docker ps -aq | wc -l)
        print_success "Docker cleanup completado ($CONTAINERS_AFTER contenedores restantes)"
    fi
}

# Obtener uso de memoria
get_memory_usage() {
    if command -v free &> /dev/null; then
        free -m | awk '/^Mem:/ {print $3}' | tr -d ' '
    else
        echo "N/A"
    fi
}

# Monitorear memoria durante ejecución
monitor_memory() {
    if [ "$MONITOR_MEMORY" = false ]; then
        return
    fi

    print_step "Iniciando monitoreo de memoria..."
    LOG_FILE="$PROJECT_DIR/test-memory-monitor.log"

    {
        echo "=== Test Container Memory Monitor ==="
        echo "Fecha: $(date)"
        echo "Límite configurado: ${MEMORY_LIMIT_MB}MB"
        echo "Límite CPU: ${CPU_LIMIT} shares"
        echo ""

        # Monitoreo continuo
        while pgrep -f "cargo test" > /dev/null; do
            MEMORY=$(get_memory_usage)
            CONTAINERS=$(docker ps --format "{{.ID}}" | wc -l)
            TIMESTAMP=$(date '+%H:%M:%S')
            echo "[$TIMESTAMP] Memory: ${MEMORY}MB | Containers: $CONTAINERS"
            sleep 2
        done

        echo ""
        echo "=== Monitoreo finalizado ==="
        echo "Fecha: $(date)"
    } > "$LOG_FILE" 2>&1 &

    MONITOR_PID=$!
    echo "$MONITOR_PID" > "$PROJECT_DIR/test-monitor.pid"
    print_success "Monitoreo iniciado (PID: $MONITOR_PID, log: $LOG_FILE)"
}

# Detener monitoreo
stop_monitor() {
    if [ "$MONITOR_MEMORY" = false ]; then
        return
    fi

    if [ -f "$PROJECT_DIR/test-monitor.pid" ]; then
        MONITOR_PID=$(cat "$PROJECT_DIR/test-monitor.pid")
        if kill -0 "$MONITOR_PID" 2>/dev/null; then
            print_step "Deteniendo monitoreo (PID: $MONITOR_PID)..."
            kill "$MONITOR_PID" 2>/dev/null || true
            rm -f "$PROJECT_DIR/test-monitor.pid"
        fi
    fi
}

# Ejecutar tests
run_tests() {
    print_step "Preparando ejecución de tests..."

    # Variables de entorno para el pool de contenedores
    export TEST_PG_MEMORY_LIMIT_MB="$MEMORY_LIMIT_MB"
    export TEST_PG_CPU_LIMIT="$CPU_LIMIT"
    export TEST_POOL_SEQUENTIAL="true"

    # Comando cargo
    CARGO_CMD="cargo test"
    CARGO_CMD="$CARGO_CMD --profile $CARGO_PROFILE"
    CARGO_CMD="$CARGO_CMD -- --test-threads=$TEST_THREADS"
    CARGO_CMD="$CARGO_CMD --nocapture"

    if [ -n "${SPECIFIC_TEST:-}" ]; then
        CARGO_CMD="$CARGO_CMD --test $SPECIFIC_TEST"
    fi

    # Agregar filtros específicos para tests con contenedores
    CARGO_CMD="$CARGO_CMD postgres"
    CARGO_CMD="$CARGO_CMD pipeline"
    CARGO_CMD="$CARGO_CMD us_prod"

    print_step "Ejecutando tests:"
    echo -e "${CYAN}$CARGO_CMD${NC}"
    echo ""

    # Configurar el pool de contenedores antes de ejecutar
    export RUST_BACKTRACE=1
    export RUST_LOG=container_tests=$LOG_LEVEL

    # Ejecutar con timeout y manejo de señales
    timeout 1800 "$CARGO_CMD" || {
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 124 ]; then
            print_error "Tests timed out after 30 minutes"
        else
            print_error "Tests failed with exit code: $EXIT_CODE"
        fi
        return $EXIT_CODE
    }

    return 0
}

# Limpieza final
final_cleanup() {
    print_step "Realizando limpieza final..."

    # Mostrar estadísticas de Docker
    print_step "Estadísticas de Docker:"
    CONTAINERS=$(docker ps -a --format "{{.ID}} {{.Image}} {{.Status}}" | grep -E "(postgres|container)" || echo "No containers found")
    if [ -n "$CONTAINERS" ]; then
        echo "$CONTAINERS"
    fi

    # Detener monitoreo
    stop_monitor

    # Limpieza final agresiva
    print_step "Limpiando contenedores de tests..."
    docker ps -a --filter "label=com.docker.compose.project=hodei" -q | xargs -r docker rm -f 2>/dev/null || true

    # Limpiar volúmenes huérfanos
    print_step "Limpiando volúmenes huérfanos..."
    docker volume prune -f > /dev/null 2>&1 || true

    # Mostrar uso final de memoria
    FINAL_MEMORY=$(get_memory_usage)
    print_success "Memoria final: ${FINAL_MEMORY}MB"

    print_success "Limpieza final completada"
}

# Mostrar resumen
show_summary() {
    print_header "RESUMEN DE EJECUCIÓN"

    echo -e "${CYAN}Configuración:${NC}"
    echo "  - Límite de memoria: ${MEMORY_LIMIT_MB}MB"
    echo "  - Límite de CPU: ${CPU_LIMIT} shares"
    echo "  - Test threads: $TEST_THREADS (secuencial)"
    echo "  - Perfil: $CARGO_PROFILE"
    echo ""

    if [ -f "$PROJECT_DIR/test-memory-monitor.log" ]; then
        echo -e "${CYAN}Logs de monitoreo:${NC}"
        echo "  - Archivo: $PROJECT_DIR/test-memory-monitor.log"
        echo "  - Tamaño: $(du -h "$PROJECT_DIR/test-memory-monitor.log" | cut -f1)"
        echo ""

        # Mostrar picos de memoria
        PEAK_MEMORY=$(grep -oP 'Memory: \K\d+' "$PROJECT_DIR/test-memory-monitor.log" | sort -n -r | head -1)
        if [ -n "$PEAK_MEMORY" ] && [ "$PEAK_MEMORY" != "N/A" ]; then
            echo -e "${CYAN}Pico de memoria:${NC}"
            echo "  - Máximo observado: ${PEAK_MEMORY}MB"
            echo "  - Límite configurado: ${MEMORY_LIMIT_MB}MB"
            if [ "$PEAK_MEMORY" -lt "$MEMORY_LIMIT_MB" ]; then
                echo -e "${GREEN}  ✅ Dentro del límite${NC}"
            else
                echo -e "${YELLOW}  ⚠️  Sobre el límite${NC}"
            fi
        fi
    fi

    echo ""
    print_success "Tests ejecutados exitosamente"
    print_success "Recursos liberados automáticamente"
}

# Manejo de señales
trap 'echo -e "\n${YELLOW}⚠️  Interrumpido por usuario${NC}"; stop_monitor; final_cleanup; exit 130' INT
trap 'echo -e "\n${YELLOW}⚠️  Error en script${NC}"; stop_monitor; final_cleanup; exit 1' ERR

# MAIN
main() {
    print_header "🔧 RUN CONTAINER TESTS - SECURE MODE"

    echo -e "${CYAN}Iniciando ejecución segura de tests con contenedores${NC}"
    echo ""
    echo "Parámetros:"
    echo "  - Memoria límite: ${MEMORY_LIMIT_MB}MB"
    echo "  - CPU límite: ${CPU_LIMIT} shares"
    echo "  - Threads: $TEST_THREADS (secuencial)"
    echo "  - Cleanup antes: $CLEANUP_BEFORE"
    echo "  - Monitoreo: $MONITOR_MEMORY"
    echo ""

    # Verificar y preparar
    check_dependencies
    cleanup_docker

    # Iniciar monitoreo
    monitor_memory

    # Ejecutar tests
    START_TIME=$(date +%s)
    if run_tests; then
        END_TIME=$(date +%s)
        DURATION=$((END_TIME - START_TIME))

        print_success "Tests completados en ${DURATION}s"
    else
        EXIT_CODE=$?
        print_error "Tests fallaron"
        final_cleanup
        exit $EXIT_CODE
    fi

    # Cleanup y resumen
    final_cleanup
    show_summary

    print_header "✅ EJECUCIÓN COMPLETADA"
}

# Ejecutar
main "$@"
