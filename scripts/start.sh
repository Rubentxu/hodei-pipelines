#!/bin/bash

# ========================================
# Script de inicio rápido para Hodei Jobs
# Levanta la aplicación con Docker Compose
# ========================================

set -e

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Función para imprimir mensajes
print_message() {
    echo -e "${1}${2}${NC}"
}

# Banner
print_message $BLUE "========================================"
print_message $BLUE "   Hodei Jobs Server - Quick Start"
print_message $BLUE "========================================"
echo ""

# Verificar prerrequisitos
print_message $YELLOW "Verificando prerrequisitos..."

if ! command -v docker &> /dev/null; then
    print_message $RED "✗ Docker no está instalado"
    exit 1
fi

if ! command -v docker compose &> /dev/null; then
    print_message $RED "✗ Docker Compose no está instalado"
    exit 1
fi

print_message $GREEN "✓ Docker instalado"
print_message $GREEN "✓ Docker Compose instalado"
echo ""

# Crear directorios necesarios
print_message $YELLOW "Creando directorios..."
mkdir -p data/{postgres,redis,prometheus,grafana,traefik}
mkdir -p logs backups
print_message $GREEN "✓ Directorios creados"
echo ""

# Verificar .env
if [ ! -f .env ]; then
    print_message $YELLOW "Creando archivo .env desde plantilla..."
    cp .env .env.local 2>/dev/null || true
    print_message $GREEN "✓ Archivo .env configurado"
else
    print_message $GREEN "✓ Archivo .env encontrado"
fi
echo ""

# Preguntar qué servicios levantar
print_message $BLUE "Selecciona qué servicios quieres levantar:"
echo "1) Solo servicios core (PostgreSQL, Redis, Server)"
echo "2) Stack completo con monitoreo (Prometheus, Grafana)"
echo "3) Con Traefik reverse proxy"
echo "4) Solo desarrollo (dev profile)"
echo ""
read -p "Opción [1-4]: " option

case $option in
    1)
        print_message $GREEN "Levantando servicios core..."
        docker compose up -d postgres redis hodei-server
        ;;
    2)
        print_message $GREEN "Levantando stack completo con monitoreo..."
        docker compose --profile monitoring up -d
        ;;
    3)
        print_message $GREEN "Levantando con Traefik proxy..."
        docker compose --profile proxy up -d
        ;;
    4)
        print_message $GREEN "Levantando en modo desarrollo..."
        docker compose --profile dev up -d
        ;;
    *)
        print_message $YELLOW "Opción inválida, levantando servicios core..."
        docker compose up -d postgres redis hodei-server
        ;;
esac

echo ""
print_message $BLUE "========================================"
print_message $GREEN "✓ ¡Servicios levantados!"
print_message $BLUE "========================================"
echo ""

# Esperar a que el servidor esté listo
print_message $YELLOW "Esperando a que el servidor esté listo..."
sleep 5

# Verificar health
print_message $BLUE "Verificando estado..."
if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    print_message $GREEN "✓ Servidor ejecutándose correctamente"
else
    print_message $YELLOW "⚠ El servidor aún se está iniciando..."
    sleep 10
fi

echo ""
print_message $BLUE "🌐 Accesos:"
print_message $NC "  - API REST:      http://localhost:8080"
print_message $NC "  - Swagger UI:    http://localhost:8080/api/docs"
print_message $NC "  - OpenAPI JSON:  http://localhost:8080/api/openapi.json"
print_message $NC "  - Health Check:  http://localhost:8080/health"

if [[ $option == "2" ]]; then
    echo ""
    print_message $BLUE "📊 Monitoreo:"
    print_message $NC "  - Prometheus:    http://localhost:9090"
    print_message $NC "  - Grafana:       http://localhost:3000 (admin/grafana_admin_2024)"
fi

if [[ $option == "3" ]]; then
    echo ""
    print_message $BLUE "🔀 Proxy:"
    print_message $NC "  - Traefik:       http://localhost:8080"
    print_message $NC "  - API via Proxy: http://api.hodei.local"
fi

echo ""
print_message $YELLOW "Para ver logs: docker compose logs -f hodei-server"
print_message $YELLOW "Para detener:   docker compose down"
echo ""
