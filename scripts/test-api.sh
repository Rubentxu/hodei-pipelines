#!/bin/bash

# ========================================
# API Testing Script para Hodei Jobs
# Probar endpoints de la API
# ========================================

set -e

# Configuración
BASE_URL="http://localhost:8080"
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_header() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Verificar que el servidor esté disponible
check_server() {
    print_header "Verificando Conexión al Servidor"

    if curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
        print_success "Servidor disponible en $BASE_URL"
        return 0
    else
        print_warning "Servidor no disponible en $BASE_URL"
        print_info "Asegúrate de que el servidor esté ejecutándose:"
        print_info "  docker compose up -d hodei-server"
        return 1
    fi
}

# Test 1: Health Check
test_health() {
    print_header "Test 1: Health Check"

    response=$(curl -s "$BASE_URL/health")
    echo "$response" | jq '.' || echo "$response"

    if echo "$response" | grep -q "healthy"; then
        print_success "Health check pasó"
        return 0
    else
        print_warning "Health check falló"
        return 1
    fi
}

# Test 2: Crear un Job
test_create_job() {
    print_header "Test 2: Crear Job"

    job_data='{
        "name": "test-job-'"$(date +%s)"'",
        "image": "ubuntu:latest",
        "command": ["echo", "Hello from Hodei!"],
        "resources": {
            "cpu_m": 1000,
            "memory_mb": 512
        },
        "timeout_ms": 30000,
        "retries": 2,
        "env": {
            "ENVIRONMENT": "test"
        },
        "secret_refs": []
    }'

    print_info "Enviando request para crear job..."
    response=$(curl -s -X POST "$BASE_URL/api/v1/jobs" \
        -H "Content-Type: application/json" \
        -d "$job_data")

    echo "$response" | jq '.' || echo "$response"

    job_id=$(echo "$response" | jq -r '.id // empty')

    if [ -n "$job_id" ] && [ "$job_id" != "null" ]; then
        print_success "Job creado exitosamente (ID: $job_id)"
        echo "$job_id" > /tmp/test_job_id
        return 0
    else
        print_warning "Error al crear job"
        return 1
    fi
}

# Test 3: Obtener Job
test_get_job() {
    print_header "Test 3: Obtener Job"

    if [ ! -f /tmp/test_job_id ]; then
        print_warning "No se encontró ID de job. Ejecuta test_create_job primero."
        return 1
    fi

    job_id=$(cat /tmp/test_job_id)
    print_info "Obteniendo job con ID: $job_id"

    response=$(curl -s "$BASE_URL/api/v1/jobs/$job_id")
    echo "$response" | jq '.' || echo "$response"

    if echo "$response" | grep -q "$job_id"; then
        print_success "Job obtenido exitosamente"
        return 0
    else
        print_warning "Error al obtener job"
        return 1
    fi
}

# Test 4: Cancelar Job
test_cancel_job() {
    print_header "Test 4: Cancelar Job"

    if [ ! -f /tmp/test_job_id ]; then
        print_warning "No se encontró ID de job. Ejecuta test_create_job primero."
        return 1
    fi

    job_id=$(cat /tmp/test_job_id)
    print_info "Cancelando job con ID: $job_id"

    response=$(curl -s -X POST "$BASE_URL/api/v1/jobs/$job_id/cancel")
    echo "$response" | jq '.' || echo "$response"

    if echo "$response" | grep -q "successfully"; then
        print_success "Job cancelado exitosamente"
        return 0
    else
        print_warning "Error al cancelar job"
        return 1
    fi
}

# Test 5: Registrar Worker
test_register_worker() {
    print_header "Test 5: Registrar Worker"

    worker_data='{
        "name": "test-worker-'"$(date +%s)"'",
        "cpu_cores": 4,
        "memory_gb": 8
    }'

    print_info "Registrando worker..."
    response=$(curl -s -X POST "$BASE_URL/api/v1/workers" \
        -H "Content-Type: application/json" \
        -d "$worker_data")

    echo "$response" | jq '.' || echo "$response"

    if echo "$response" | grep -q "successfully"; then
        print_success "Worker registrado exitosamente"
        return 0
    else
        print_warning "Error al registrar worker"
        return 1
    fi
}

# Test 6: Worker Heartbeat
test_worker_heartbeat() {
    print_header "Test 6: Worker Heartbeat"

    # Usar un UUID de ejemplo para el worker
    worker_id="123e4567-e89b-12d3-a456-426614174000"
    print_info "Enviando heartbeat para worker: $worker_id"

    response=$(curl -s -X POST "$BASE_URL/api/v1/workers/$worker_id/heartbeat")
    echo "$response" | jq '.' || echo "$response"

    if echo "$response" | grep -q "successfully"; then
        print_success "Heartbeat enviado exitosamente"
        return 0
    else
        print_warning "Error al enviar heartbeat"
        return 1
    fi
}

# Test 7: Obtener Métricas
test_metrics() {
    print_header "Test 7: Obtener Métricas"

    print_info "Obteniendo métricas de Prometheus..."
    response=$(curl -s "$BASE_URL/api/v1/metrics")

    if [ -n "$response" ]; then
        print_success "Métricas obtenidas"
        print_info "Longitud de respuesta: $(echo -n "$response" | wc -c) bytes"
        return 0
    else
        print_warning "No se pudieron obtener métricas"
        return 1
    fi
}

# Test 8: Probar OpenAPI
test_openapi() {
    print_header "Test 8: OpenAPI Specification"

    print_info "Obteniendo especificación OpenAPI..."
    response=$(curl -s "$BASE_URL/api/openapi.json")

    version=$(echo "$response" | jq -r '.openapi // empty')
    title=$(echo "$response" | jq -r '.info.title // empty')

    if [ "$version" = "3.0.0" ]; then
        print_success "OpenAPI v3.0.0 disponible"
        print_info "Título: $title"
        return 0
    else
        print_warning "Error en la especificación OpenAPI"
        return 1
    fi
}

# Menú principal
show_menu() {
    print_header "API Testing - Hodei Jobs"

    echo "Selecciona qué test ejecutar:"
    echo "1)  Health Check"
    echo "2)  Crear Job"
    echo "3)  Obtener Job"
    echo "4)  Cancelar Job"
    echo "5)  Registrar Worker"
    echo "6)  Worker Heartbeat"
    echo "7)  Obtener Métricas"
    echo "8)  OpenAPI Spec"
    echo "9)  Ejecutar todos los tests"
    echo "0)  Salir"
    echo ""
    read -p "Opción: " choice
}

# Ejecutar tests individuales
run_single_test() {
    case $1 in
        1) test_health ;;
        2) test_create_job ;;
        3) test_get_job ;;
        4) test_cancel_job ;;
        5) test_register_worker ;;
        6) test_worker_heartbeat ;;
        7) test_metrics ;;
        8) test_openapi ;;
        *) echo "Opción inválida" ;;
    esac
}

# Ejecutar todos los tests
run_all_tests() {
    print_header "Ejecutando Todos los Tests"

    local passed=0
    local failed=0

    tests=("test_health" "test_create_job" "test_get_job" "test_cancel_job" "test_register_worker" "test_worker_heartbeat" "test_metrics" "test_openapi")

    for test in "${tests[@]}"; do
        echo ""
        if $test; then
            ((passed++))
        else
            ((failed++))
        fi
    done

    print_header "Resultados Finales"
    echo -e "${GREEN}Tests pasados: $passed${NC}"
    echo -e "${YELLOW}Tests fallidos: $failed${NC}"
    echo -e "${BLUE}Total: $((passed + failed))${NC}"

    if [ $failed -eq 0 ]; then
        print_success "¡Todos los tests pasaron!"
    fi
}

# Main
if ! check_server; then
    exit 1
fi

if [ $# -eq 0 ]; then
    # Modo interactivo
    while true; do
        show_menu
        case $choice in
            0) echo "¡Hasta luego!"; break ;;
            9) run_all_tests ;;
            *) if [ -n "$choice" ] && [ "$choice" -ge 1 ] && [ "$choice" -le 8 ]; then
                   run_single_test $choice
               else
                   echo "Opción inválida"
               fi ;;
        esac
        echo ""
        read -p "Presiona Enter para continuar..."
    done
else
    # Modo directo (pasando número de test como parámetro)
    run_single_test $1
fi
