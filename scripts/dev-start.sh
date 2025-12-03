#!/bin/bash

# Unset DOCKER_HOST to use Docker Desktop socket by default
unset DOCKER_HOST

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker no está instalado${NC}"
    echo "   Instala Docker desde: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker daemon is running
if ! docker info &> /dev/null; then
    echo -e "${RED}❌ Docker daemon no está ejecutándose${NC}"
    echo "   Inicia Docker y vuelve a intentarlo"
    exit 1
fi

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m'

# Configuration
COMPOSE_FILE="docker-compose.dev.yml"
SERVER_PORT=8080
GRPC_PORT=50051
WEB_PORT=3005
POSTGRES_PORT=5432

# PIDs
SERVER_PID=""
AGENT_PID=""
WEB_PID=""

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                                                            ║${NC}"
echo -e "${BLUE}║    🚀 HODEI PIPELINES - DESARROLLO COMPLETO                ║${NC}"
echo -e "${BLUE}║                                                            ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if help requested
if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    echo "Uso: $0 [OPCIONES]"
    echo ""
    echo "Opciones:"
    echo "  --full       Levanta servicios + backend + web (por defecto)"
    echo "  --services   Solo servicios de terceros (PostgreSQL)"
    echo "  --backend    Solo backend (requiere servicios activos)"
    echo "  --web        Solo web console (requiere backend activo)"
    echo "  --stop       Detiene todos los servicios"
    echo "  --help       Muestra esta ayuda"
    echo ""
    exit 0
fi

# Function to check if port is in use
check_port() {
    local port=$1
    local service=$2
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  Puerto $port está en uso por $service${NC}"
        return 1
    fi
    return 0
}

# Function to check if service is running
check_service() {
    local service=$1
    if docker compose -f $COMPOSE_FILE ps $service | grep -q "Up"; then
        return 0
    fi
    return 1
}

# Function to cleanup on exit
cleanup_on_exit() {
    echo ""
    echo -e "${YELLOW}🛑 Deteniendo servicios...${NC}"

    if [ -n "$SERVER_PID" ]; then
        echo "  - Deteniendo backend (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
    fi

    if [ -n "$AGENT_PID" ]; then
        echo "  - Deteniendo agent (PID: $AGENT_PID)..."
        kill $AGENT_PID 2>/dev/null || true
    fi

    if [ -n "$WEB_PID" ]; then
        echo "  - Deteniendo web console (PID: $WEB_PID)..."
        kill $WEB_PID 2>/dev/null || true
    fi

    # Force kill ports if still open
    fuser -k -9 $SERVER_PORT/tcp 2>/dev/null || true
    fuser -k -9 $GRPC_PORT/tcp 2>/dev/null || true
    fuser -k -9 $WEB_PORT/tcp 2>/dev/null || true

    echo -e "${GREEN}✅ Todos los servicios detenidos${NC}"
    exit 0
}

# Function to initialize database schema
init_database_schema() {
    echo -e "${BLUE}🗄️  Aplicando migraciones de base de datos...${NC}"

    # Define migration files in order
    local migrations=(
        "crates/adapters/migrations/20241130_rbac_tables.sql"
        "crates/adapters/migrations/20241201_pipelines.sql"
        "crates/adapters/migrations/20241201_jobs.sql"
        "crates/adapters/migrations/20241201_pipeline_executions.sql"
        "crates/adapters/migrations/20241201_workers.sql"
        "crates/adapters/migrations/20241201_compressed_logs.sql"
        "crates/adapters/migrations/20241201_metrics_timeseries.sql"
    )

    # Check if migration directory exists
    if [ ! -d "crates/adapters/migrations" ]; then
        echo -e "${YELLOW}  ⚠️  Directorio de migraciones no encontrado${NC}"
        return 0
    fi

    # Detect if TimescaleDB is available
    local has_timescaledb=false
    echo -e "${BLUE}  🔍 Verificando extensiones de PostgreSQL...${NC}"
    if env -u DOCKER_HOST docker compose -f $COMPOSE_FILE exec -T postgres psql -U hodei -d hodei_jobs -tAc "SELECT 1 FROM pg_extension WHERE extname = 'timescaledb'" | grep -q "1"; then
        has_timescaledb=true
        echo -e "${GREEN}    ✅ TimescaleDB detectado${NC}"
    else
        echo -e "${YELLOW}    ⚠️  TimescaleDB no disponible${NC}"
    fi

    # Apply each migration file
    for migration in "${migrations[@]}"; do
        if [ -f "$migration" ]; then
            # Skip TimescaleDB migration if not available
            if [[ "$migration" == *"metrics_timeseries"* ]] && [ "$has_timescaledb" = false ]; then
                echo -e "${YELLOW}  ⏭️  Omitiendo $(basename "$migration") (requiere TimescaleDB)${NC}"
                continue
            fi

            echo -e "${BLUE}  📋 Aplicando $(basename "$migration")...${NC}"
            env -u DOCKER_HOST docker compose -f $COMPOSE_FILE exec -T postgres psql -U hodei -d hodei_jobs < "$migration" > /dev/null 2>&1 || echo "  ℹ️  Migración ya aplicada o ignorada"
        else
            echo -e "${YELLOW}  ⚠️  $migration no encontrado${NC}"
        fi
    done

    echo -e "${GREEN}  ✅ Migraciones aplicadas${NC}"
    echo -e "${BLUE}  ℹ️  Las tablas dinámicas se crearán al iniciar el backend${NC}"
}

# Function to cleanup on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}🛑 Deteniendo servicios...${NC}"

    if [ -n "$SERVER_PID" ]; then
        echo "  - Deteniendo backend (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
    fi

    if [ -n "$AGENT_PID" ]; then
        echo "  - Deteniendo agent (PID: $AGENT_PID)..."
        kill $AGENT_PID 2>/dev/null || true
    fi

    if [ -n "$WEB_PID" ]; then
        echo "  - Deteniendo web console (PID: $WEB_PID)..."
        kill $WEB_PID 2>/dev/null || true
    fi

    # Force kill ports if still open
    fuser -k -9 $SERVER_PORT/tcp 2>/dev/null || true
    fuser -k -9 $GRPC_PORT/tcp 2>/dev/null || true
    fuser -k -9 $WEB_PORT/tcp 2>/dev/null || true

    echo -e "${GREEN}✅ Todos los servicios detenidos${NC}"
    exit 0
}

# Trap Ctrl+C
trap cleanup INT TERM

# Stop command
if [[ "$1" == "--stop" ]]; then
    echo -e "${YELLOW}🛑 Deteniendo todos los servicios...${NC}"
    docker compose -f $COMPOSE_FILE down
    pkill -f "hodei-pipelines-server" || true
    pkill -f "hwp-pipelines-agent" || true
    pkill -f "vite" || true
    echo -e "${GREEN}✅ Servicios detenidos${NC}"
    exit 0
fi

# Services only
if [[ "$1" == "--services" ]]; then
    echo -e "${GREEN}🚀 Configurando servicios de terceros...${NC}"

    # Check if already running
    if check_service postgres; then
        echo -e "${GREEN}✅ PostgreSQL ya está ejecutándose${NC}"
    else
        echo -e "${BLUE}📦 Iniciando PostgreSQL...${NC}"
        docker compose -f $COMPOSE_FILE up -d postgres
        sleep 5

        # Wait for PostgreSQL to be healthy
        echo -e "${BLUE}  ⏳ Esperando a que PostgreSQL esté listo...${NC}"
        for i in {1..30}; do
            if docker compose -f $COMPOSE_FILE exec -T postgres pg_isready -U hodei > /dev/null 2>&1; then
                echo -e "${GREEN}  ✅ PostgreSQL listo${NC}"
                break
            fi
            if [ $i -eq 30 ]; then
                echo -e "${RED}  ❌ Timeout esperando PostgreSQL${NC}"
                exit 1
            fi
            sleep 1
        done
    fi

    # Initialize database schema
    init_database_schema

    echo ""
    echo -e "${GREEN}✅ Servicios listos:${NC}"
    echo "  🗄️  PostgreSQL: localhost:5432"
    echo "  📦 Credenciales: hodei / hodei_password_2024 / hodei_jobs"
    echo ""
    echo -e "${BLUE}Para iniciar backend: $0 --backend${NC}"
    echo -e "${BLUE}Para iniciar web: $0 --web${NC}"
    exit 0
fi

# Backend only
if [[ "$1" == "--backend" ]]; then
    echo -e "${GREEN}🚀 Iniciando backend...${NC}"

    # Check if PostgreSQL is running
    if ! check_service postgres; then
        echo -e "${RED}❌ PostgreSQL no está ejecutándose.${NC}"
        echo -e "${YELLOW}   Inicia los servicios primero: $0 --services${NC}"
        exit 1
    fi

    # Check if backend is already running
    if curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Backend ya está ejecutándose${NC}"
        echo "  🌐 Server API:  http://localhost:8080"
        echo "  🔧 gRPC API:    localhost:50051"
        echo ""
        echo -e "${BLUE}Para iniciar web: $0 --web${NC}"
        echo ""
        echo -e "${YELLOW}Presiona Ctrl+C para detener${NC}"
        wait
        exit 0
    fi

    # Check ports and start if free
    if ! check_port $SERVER_PORT "hodei-pipelines-server"; then
        echo -e "${YELLOW}⚠️  Puerto $SERVER_PORT en uso, esperando...${NC}"
        sleep 2
    fi

    echo -e "${BLUE}🔨 Compilando backend...${NC}"
    cd /home/rubentxu/Proyectos/rust/hodei-jobs
    cargo build -p hodei-pipelines-server -p hwp-pipelines-agent 2>&1 | grep -E "(Compiling|Finished)" || true

    echo -e "${BLUE}🌐 Iniciando Hodei Server...${NC}"
    export HODEI_TOKEN=test-dev-token-2024
    export HODEI_DB_URL="postgresql://hodei:hodei_password_2024@localhost:5432/hodei_jobs"
    export RUST_LOG=info
    cargo run -p hodei-pipelines-server > server.log 2>&1 &
    SERVER_PID=$!
    echo "  PID: $SERVER_PID"

    sleep 3

    echo -e "${BLUE}🤖 Iniciando HWP Agent...${NC}"
    export WORKER_ID=worker-dev-01
    export SERVER_URL=http://localhost:8080
    export GRPC_SERVER_URL=http://localhost:50051
    cargo run -p hwp-pipelines-agent > agent.log 2>&1 &
    AGENT_PID=$!
    echo "  PID: $AGENT_PID"

    echo -e "${BLUE}  ⏳ Verificando que el backend responda...${NC}"
    for i in {1..30}; do
        if curl -s http://localhost:$SERVER_PORT/api/health > /dev/null 2>&1; then
            echo -e "${GREEN}  ✅ Backend responde correctamente${NC}"
            break
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}  ❌ Backend no responde en /api/health después de 30s${NC}"
            echo -e "${YELLOW}   Revisa los logs: tail -f server.log${NC}"
            exit 1
        fi
        sleep 1
    done

    echo ""
    echo -e "${GREEN}✅ Backend iniciado y funcionando:${NC}"
    echo "  🌐 Server API:  http://localhost:8080"
    echo "  🔧 gRPC API:    localhost:50051"
    echo "  🗄️  PostgreSQL: localhost:5432"
    echo ""
    echo -e "${GREEN}🧪 Prueba la API:${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${YELLOW}📖 Documentación:${NC}"
    echo -e "  🌐  Swagger UI:     ${BLUE}http://localhost:8080/docs${NC}"
    echo ""
    echo -e "${YELLOW}🔍 Endpoints:${NC}"
    echo -e "  ${BLUE}GET${NC}    http://localhost:8080/api/health         - Estado"
    echo -e "  ${BLUE}GET${NC}    http://localhost:8080/api/v1/pipelines  - Pipelines"
    echo ""
    echo -e "${GRAY}curl -s http://localhost:8080/api/health | jq${NC}"
    echo ""
    echo -e "${BLUE}Para iniciar web: $0 --web${NC}"
    echo ""
    echo -e "${YELLOW}Presiona Ctrl+C para detener${NC}"
    wait
fi

# Web only
if [[ "$1" == "--web" ]]; then
    echo -e "${GREEN}🚀 Iniciando Web Console...${NC}"

    # Check if server is running
    if ! curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
        echo -e "${RED}❌ Backend no está ejecutándose.${NC}"
        echo -e "${YELLOW}   Inicia el backend primero: $0 --backend${NC}"
        exit 1
    fi

    # Check if web is already running
    if curl -s http://localhost:5173 > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Web Console ya está ejecutándose${NC}"
        echo "  🌐 URL: http://localhost:$WEB_PORT"
        echo "  🔗 API: http://localhost:8080"
        echo ""
        echo -e "${YELLOW}Presiona Ctrl+C para detener${NC}"
        wait
        exit 0
    fi

    cd web-console

    echo -e "${BLUE}📦 Instalando/Verificando dependencias...${NC}"
    npm install --silent > /dev/null 2>&1 || echo "Dependencias ya instaladas"

    echo -e "${BLUE}🎨 Iniciando Web Console...${NC}"
    npm run dev > ../web.log 2>&1 &
    WEB_PID=$!

    sleep 5

    echo ""
    echo -e "${GREEN}✅ Web Console iniciado:${NC}"
    echo "  🌐 URL: http://localhost:$WEB_PORT"
    echo "  🔗 API: http://localhost:8080"
    echo ""
    echo -e "${YELLOW}Presiona Ctrl+C para detener${NC}"
    wait
fi

# Full setup (default)
if [[ "$1" == "--full" || -z "$1" ]]; then
    echo -e "${GREEN}🚀 Iniciando/entornando entorno completo de desarrollo...${NC}"
    echo ""

    # Step 1: Services
    echo -e "${BLUE}📋 Paso 1/3: Configurando servicios de terceros...${NC}"

    # Idempotent: Check if PostgreSQL is already running
    if check_service postgres; then
        echo -e "${GREEN}  ✅ PostgreSQL ya está ejecutándose${NC}"
    else
        docker compose -f $COMPOSE_FILE up -d
        sleep 5

        # Wait for PostgreSQL to be healthy
        echo -e "${BLUE}  ⏳ Esperando a que PostgreSQL esté listo...${NC}"
        for i in {1..30}; do
            if docker compose -f $COMPOSE_FILE exec -T postgres pg_isready -U hodei > /dev/null 2>&1; then
                echo -e "${GREEN}  ✅ PostgreSQL listo${NC}"
                break
            fi
            if [ $i -eq 30 ]; then
                echo -e "${RED}  ❌ Timeout esperando PostgreSQL${NC}"
                exit 1
            fi
            sleep 1
        done
    fi

    # Initialize database schema
    init_database_schema

    # Step 2: Backend
    echo -e "${BLUE}📋 Paso 2/3: Configurando backend...${NC}"

    # Idempotent: Check if backend is already running
    if curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ Backend ya está ejecutándose${NC}"
    else
        cd /home/rubentxu/Proyectos/rust/hodei-jobs
        echo -e "${BLUE}  🔨 Compilando backend...${NC}"
        cargo build -p hodei-pipelines-server -p hwp-pipelines-agent 2>&1 | grep -E "(Compiling|Finished)" || true

        export HODEI_TOKEN=test-dev-token-2024
        export HODEI_DB_URL="postgresql://hodei:hodei_password_2024@localhost:5432/hodei_jobs"
        export RUST_LOG=info

        cargo run -p hodei-pipelines-server > server.log 2>&1 &
        SERVER_PID=$!

        sleep 3

        export WORKER_ID=worker-dev-01
        export SERVER_URL=http://localhost:8080
        export GRPC_SERVER_URL=http://localhost:50051
        cargo run -p hwp-pipelines-agent > agent.log 2>&1 &
        AGENT_PID=$!

        sleep 3

        # Wait for backend to be ready
        echo -e "${BLUE}  ⏳ Esperando a que el backend esté listo...${NC}"
        for i in {1..30}; do
            if curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
                echo -e "${GREEN}  ✅ Backend listo${NC}"
                break
            fi
            if [ $i -eq 30 ]; then
                echo -e "${RED}  ❌ Backend no responde en /api/health después de 30s${NC}"
                echo -e "${YELLOW}   Revisa los logs: tail -f server.log${NC}"
                exit 1
            fi
            sleep 1
        done
    fi

    # Step 3: Web
    echo -e "${BLUE}📋 Paso 3/3: Configurando Web Console...${NC}"

    # Idempotent: Check if web is already running
    if curl -s http://localhost:5173 > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ Web Console ya está ejecutándose${NC}"
    else
        cd web-console
        echo -e "${BLUE}  📦 Verificando dependencias npm...${NC}"
        npm install --silent > /dev/null 2>&1 || echo "  ℹ️  Instalando dependencias...${NC}" && npm install > /dev/null 2>&1

        echo -e "${BLUE}  🎨 Iniciando Web Console...${NC}"
        npm run dev > ../web.log 2>&1 &
        WEB_PID=$!

        sleep 5

        # Wait for web to be ready
        echo -e "${BLUE}  ⏳ Esperando a que Web Console esté listo...${NC}"
        for i in {1..30}; do
            if curl -s http://localhost:5173 > /dev/null 2>&1; then
                echo -e "${GREEN}  ✅ Web Console listo${NC}"
                break
            fi
            if [ $i -eq 30 ]; then
                echo -e "${YELLOW}  ⚠️  Web Console puede necesitar más tiempo${NC}"
            fi
            sleep 1
        done
    fi

    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║                                                            ║${NC}"
    echo -e "${BLUE}║              ✅ ENTORNO LISTO PARA USAR                    ║${NC}"
    echo -e "${BLUE}║                                                            ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${GREEN}📊 Servicios:${NC}"
    echo -e "  🌐  Web Console:   ${BLUE}http://localhost:$WEB_PORT${NC}"
    echo -e "  🔧  API Server:    ${BLUE}http://localhost:$SERVER_PORT${NC}"
    echo -e "  🗄️  PostgreSQL:    ${BLUE}localhost:$POSTGRES_PORT${NC}"
    echo -e "  📡  gRPC:          ${BLUE}localhost:$GRPC_PORT${NC}"
    echo ""

    echo -e "${GREEN}🧪 Prueba la API:${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${YELLOW}📖 Documentación Interactiva:${NC}"
    echo -e "  🌐  Swagger UI:     ${BLUE}http://localhost:$SERVER_PORT/docs${NC}"
    echo -e "  📄  OpenAPI Spec:   ${BLUE}http://localhost:$SERVER_PORT/openapi.json${NC}"
    echo ""
    echo -e "${YELLOW}🔍 Endpoints para Probar:${NC}"
    echo -e "  ${BLUE}GET${NC}    http://localhost:$SERVER_PORT/api/health          - Estado del servidor"
    echo -e "  ${BLUE}GET${NC}    http://localhost:$SERVER_PORT/api/v1/pipelines    - Listar pipelines"
    echo -e "  ${BLUE}GET${NC}    http://localhost:$SERVER_PORT/api/v1/jobs         - Listar jobs"
    echo -e "  ${BLUE}GET${NC}    http://localhost:$SERVER_PORT/api/v1/workers      - Listar workers"
    echo ""
    echo -e "${YELLOW}💻 Ejemplos con curl:${NC}"
    echo -e "${GRAY}# Verificar estado del servidor${NC}"
    echo -e "${CYAN}curl -s http://localhost:$SERVER_PORT/api/health | jq${NC}"
    echo ""
    echo -e "${GRAY}# Listar pipelines${NC}"
    echo -e "${CYAN}curl -s http://localhost:$SERVER_PORT/api/v1/pipelines | jq${NC}"
    echo ""
    echo -e "${GRAY}# Listar jobs${NC}"
    echo -e "${CYAN}curl -s http://localhost:$SERVER_PORT/api/v1/jobs | jq${NC}"
    echo ""
    echo -e "${GRAY}# Listar workers${NC}"
    echo -e "${CYAN}curl -s http://localhost:$SERVER_PORT/api/v1/workers | jq${NC}"
    echo ""

    echo -e "${YELLOW}🎯 Próximos Pasos:${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "  1. 📖 Abre ${BLUE}http://localhost:$SERVER_PORT/docs${NC} en tu navegador"
    echo -e "  2. 🧪 Prueba endpoints desde Swagger UI"
    echo -e "  3. 🌐 Accede a la Web Console: ${BLUE}http://localhost:$WEB_PORT${NC}"
    echo -e "  4. 📊 Monitorea logs: ${CYAN}make dev-logs${NC}"
    echo ""
    echo -e "${GREEN}📝 Logs:${NC}"
    echo "  Backend:  tail -f /home/rubentxu/Proyectos/rust/hodei-jobs/server.log"
    echo "  Agent:    tail -f /home/rubentxu/Proyectos/rust/hodei-jobs/agent.log"
    echo "  Web:      tail -f /home/rubentxu/Proyectos/rust/hodei-jobs/web.log"
    echo "  PostgreSQL: docker compose -f docker-compose.dev.yml logs -f postgres"
    echo ""
    echo -e "${YELLOW}💡 Presiona Ctrl+C para detener todo${NC}"
    echo ""

    wait
fi
