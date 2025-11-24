# ========================================
# Makefile para Hodei Jobs Server
# Comandos comunes para Docker y desarrollo
# ========================================

.PHONY: help build build-all up up-workers up down logs clean test launch launch-full

# Colores para output
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[1;33m
BLUE=\033[0;34m
NC=\033[0m # No Color

# Variables
COMPOSE_FILE=docker-compose.yml
COMPOSE_ENV_FILE=.env
CONTAINER_NAME=hodei-server

# ========================================
# Comandos Principales
# ========================================
launch: build-all up-workers ## 🚀 Comando principal: Compilar y lanzar aplicación completa
	@echo ""
	@echo "$(GREEN)========================================$(NC)"
	@echo "$(GREEN)   ¡Hodei Jobs Server está listo!   $(NC)"
	@echo "$(GREEN)========================================$(NC)"
	@echo ""
	@echo "$(BLUE)🌐 Accesos:$(NC)"
	@echo "  - API REST:      http://localhost:8080"
	@echo "  - Swagger UI:    http://localhost:8080/api/docs"
	@echo "  - OpenAPI JSON:  http://localhost:8080/api/openapi.json"
	@echo "  - Health Check:  http://localhost:8080/health"
	@echo ""
	@echo "$(BLUE)⚙️  Workers activos:$(NC)"
	@echo "  - worker-01: 4 cores, 8GB RAM"
	@echo "  - worker-02: 4 cores, 8GB RAM"
	@echo "  - worker-03: 2 cores, 4GB RAM"
	@echo ""
	@echo "$(YELLOW)📝 Comandos útiles:$(NC)"
	@echo "  - make logs          (ver logs)"
	@echo "  - make health        (verificar estado)"
	@echo "  - make workers       (estado de workers)"
	@echo "  - ./test-api.sh      (probar API)"
	@echo ""

launch-full: build-all up-workers-with-monitoring ## 🚀 Launch completo con monitoreo (server + workers + Grafana + Prometheus)
	@echo ""
	@echo "$(GREEN)========================================$(NC)"
	@echo "$(GREEN)   ¡Stack completo iniciado!   $(NC)"
	@echo "$(GREEN)========================================$(NC)"
	@echo ""
	@echo "$(BLUE)🌐 Accesos:$(NC)"
	@echo "  - API REST:      http://localhost:8080"
	@echo "  - Swagger UI:    http://localhost:8080/api/docs"
	@echo "  - Prometheus:    http://localhost:9090"
	@echo "  - Grafana:       http://localhost:3000 (admin/grafana_admin_2024)"
	@echo ""

# ========================================
# Help
# ========================================
help: ## Mostrar ayuda
	@echo "$(BLUE)========================================$(NC)"
	@echo "$(BLUE)   Hodei Jobs Server - Comandos Docker   $(NC)"
	@echo "$(BLUE)========================================$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-35s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)🎯 COMANDOS PRINCIPALES:$(NC)"
	@echo "  $(YELLOW)make launch$(NC)              - Lanzar aplicación completa (server + 3 workers)"
	@echo "  $(YELLOW)make launch-full$(NC)         - Lanzar stack completo + monitoreo"
	@echo ""
	@echo "$(BLUE)📚 DOCUMENTACIÓN:$(NC)"
	@echo "  - README-DOCKER.md     - Guía completa de Docker"
	@echo "  - docs/WORKERS-DEPLOYMENT.md - Opciones para manejar workers"
	@echo ""

# ========================================
# Setup Inicial
# ========================================
init: ## Inicializar entorno (crear directorios, permisos, etc.)
	@echo "$(GREEN)Inicializando entorno...$(NC)"
	@mkdir -p data/{postgres,redis,prometheus,grafana,traefik}
	@mkdir -p logs
	@echo "$(GREEN)Directorios creados en ./data/$(NC)"
	@echo "$(YELLOW)Recuerda copiar .env a .env.local y configurar las variables$(NC)"
	@echo ""

build: ## Construir imagen del servidor
	@echo "$(BLUE)Construyendo imagen de Hodei Server...$(NC)"
	docker compose build $(COMPOSE_FILE) hodei-server
	@echo "$(GREEN)¡Build completado!$(NC)"

build-all: ## Construir servidor y workers
	@echo "$(BLUE)Construyendo imágenes completas...$(NC)"
	docker compose build hodei-server worker-01 worker-02 worker-03
	@echo "$(GREEN)¡Builds completados!$(NC)"

# ========================================
# Servicios
# ========================================
up: ## Levantar servicios core (PostgreSQL, Server)
	@echo "$(GREEN)Levantando servicios core...$(NC)"
	docker compose up -d postgres $(CONTAINER_NAME)
	@echo "$(GREEN)Servicios levantados$(NC)"
	@echo ""
	@echo "$(BLUE)Accesos:$(NC)"
	@echo "  - API REST: http://localhost:8080"
	@echo "  - Swagger UI: http://localhost:8080/api/docs"
	@echo "  - OpenAPI: http://localhost:8080/api/openapi.json"

up-full: ## Levantar stack completo (incluye monitoreo)
	@echo "$(GREEN)Levantando stack completo con monitoreo...$(NC)"
	docker compose --profile monitoring up -d
	@echo "$(GREEN)Stack completo levantado$(NC)"

up-proxy: ## Levantar con Traefik proxy
	@echo "$(GREEN)Levantando con Traefik reverse proxy...$(NC)"
	docker compose --profile proxy up -d
	@echo "$(GREEN)Proxy configurado$(NC)"

up-workers: ## Levantar server + 3 workers
	@echo "$(GREEN)Levantando server + workers...$(NC)"
	docker compose up -d postgres hodei-server worker-01 worker-02 worker-03
	@echo "$(GREEN)Server y workers levantados$(NC)"
	@echo ""
	@echo "$(BLUE)Accesos:$(NC)"
	@echo "  - API REST: http://localhost:8080"
	@echo "  - Swagger UI: http://localhost:8080/api/docs"
	@echo ""
	@echo "$(BLUE)Workers activos:$(NC)"
	@echo "  - worker-01: 4 cores, 8GB RAM"
	@echo "  - worker-02: 4 cores, 8GB RAM"
	@echo "  - worker-03: 2 cores, 4GB RAM"

up-workers-with-monitoring: ## Levantar stack completo + workers + monitoreo
	@echo "$(GREEN)Levantando stack completo con workers y monitoreo...$(NC)"
	docker compose --profile monitoring --profile workers up -d
	@echo "$(GREEN)Stack completo levantado$(NC)"

down: ## Detener todos los servicios
	@echo "$(YELLOW)Deteniendo servicios...$(NC)"
	docker compose down
	@echo "$(GREEN)Servicios detenidos$(NC)"

# ========================================
# Logs
# ========================================
logs: ## Ver logs del servidor en tiempo real
	docker compose logs -f $(CONTAINER_NAME)

logs-all: ## Ver logs de todos los servicios
	docker compose logs -f

logs-postgres: ## Ver logs de PostgreSQL
	docker compose logs -f postgres

logs-redis: ## Ver logs de Redis
	docker compose logs -f redis

# ========================================
# Estado y Monitoreo
# ========================================
ps: ## Ver estado de los contenedores
	@echo "$(BLUE)Estado de los contenedores:$(NC)"
	docker compose ps

stats: ## Ver uso de recursos
	@echo "$(BLUE)Estadísticas de contenedores:$(NC)"
	docker stats

health: ## Verificar health checks
	@echo "$(BLUE)Health checks:$(NC)"
	docker compose ps
	@echo ""
	@echo "$(BLUE)Testing health endpoints...$(NC)"
	@curl -s http://localhost:8080/health | jq '.' || echo "Server no disponible aún"

workers: ## Ver estado de los workers
	@echo "$(BLUE)Estado de los workers:$(NC)"
	@docker compose ps worker-01 worker-02 worker-03
	@echo ""
	@echo "$(BLUE)Workers en la red:$(NC)"
	@docker network inspect hodei-jobs_hodei-network --format '{{range .Containers}}{{.Name}}: {{.IPv4Address}}{{"\n"}}{{end}}' 2>/dev/null | grep worker || echo "No workers found in network"

# ========================================
# API Testing
# ========================================
test-health: ## Probar endpoint de health
	@echo "$(BLUE)Probando /health...$(NC)"
	curl -s http://localhost:8080/health | jq '.' || echo "$(RED)Error: Server no disponible$(NC)"

test-swagger: ## Abrir Swagger UI en el navegador
	@echo "$(BLUE)Abrindo Swagger UI...$(NC)"
	@if command -v xdg-open > /dev/null; then \
		xdg-open http://localhost:8080/api/docs; \
	elif command -v open > /dev/null; then \
		open http://localhost:8080/api/docs; \
	else \
		echo "Swagger UI: http://localhost:8080/api/docs"; \
	fi

# ========================================
# Database
# ========================================
db-shell: ## Conectar a PostgreSQL
	@echo "$(BLUE)Conectando a PostgreSQL...$(NC)"
	docker compose exec postgres psql -U hodei -d hodei_jobs

db-backup: ## Crear backup de la base de datos
	@echo "$(YELLOW)Creando backup...$(NC)"
	@mkdir -p backups
	docker compose exec postgres pg_dump -U hodei hodei_jobs > backups/hodei_$(shell date +%Y%m%d_%H%M%S).sql
	@echo "$(GREEN)Backup creado en backups/$(NC)"

# ========================================
# Desarrollo
# ========================================
dev: up ## Levantar servicios de desarrollo (dev profile)
	@echo "$(GREEN)Modo desarrollo activado$(NC)"

shell: ## Abrir shell en el contenedor del servidor
	docker compose exec $(CONTAINER_NAME) /bin/bash

restart: ## Reiniciar servidor
	@echo "$(YELLOW)Reiniciando servidor...$(NC)"
	docker compose restart $(CONTAINER_NAME)

rebuild: ## Rebuild completo y levantar
	@echo "$(YELLOW)Rebuilding servidor...$(NC)"
	docker compose build --no-cache $(CONTAINER_NAME)
	docker compose up -d $(CONTAINER_NAME)

# ========================================
# Limpieza
# ========================================
clean: down ## Detener servicios
	@echo "$(YELLOW)Limpiando contenedores y redes...$(NC)"
	docker compose down --remove-orphans

clean-volumes: down ## Detener y eliminar volúmenes (¡CUIDADO! Borra datos)
	@echo "$(RED)¡ATENCIÓN! Esto borrará todos los datos$(NC)"
	@read -p "¿Estás seguro? [y/N] " -r; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker compose down -v; \
		$(MAKE) clean; \
		echo "$(GREEN)Limpieza completa realizada$(NC)"; \
	else \
		echo "$(YELLOW)Cancelado$(NC)"; \
	fi

clean-images: ## Eliminar imágenes del proyecto
	@echo "$(YELLOW)Eliminando imágenes...$(NC)"
	docker compose down --rmi all
	docker rmi $$(docker images | grep hodei | awk '{print $$3}') 2>/dev/null || true

clean-all: clean clean-volumes clean-images ## Limpiar TODO (contenedores, imágenes, volúmenes)
	@echo "$(GREEN)Limpieza total completada$(NC)"

# ========================================
# Monitoreo
# ========================================
grafana: ## Abrir Grafana en el navegador
	@echo "$(BLUE)Grafana: http://localhost:3000 (admin/grafana_admin_2024)$(NC)"
	@if command -v xdg-open > /dev/null; then \
		xdg-open http://localhost:3000; \
	elif command -v open > /dev/null; then \
		open http://localhost:3000; \
	else \
		echo "Abra manualmente: http://localhost:3000"; \
	fi

prometheus: ## Abrir Prometheus en el navegador
	@echo "$(BLUE)Prometheus: http://localhost:9090$(NC)"
	@if command -v xdg-open > /dev/null; then \
		xdg-open http://localhost:9090; \
	elif command -v open > /dev/null; then \
		open http://localhost:9090; \
	else \
		echo "Abra manualmente: http://localhost:9090"; \
	fi

# ========================================
# Utilities
# ========================================
version: ## Mostrar versiones
	@echo "$(BLUE)Versión de Docker:$(NC)"
	@docker --version
	@echo "$(BLUE)Versión de Docker Compose:$(NC)"
	@docker compose version
	@echo "$(BLUE)Versión de Hodei Server:$(NC)"
	@docker compose exec $(CONTAINER_NAME) hodei-server --version 2>/dev/null || echo "Server no ejecutándose"

info: ## Mostrar información completa del sistema
	@echo "$(BLUE)========================================$(NC)"
	@echo "$(BLUE)   Hodei Jobs Server - Info del Sistema   $(NC)"
	@echo "$(BLUE)========================================$(NC)"
	@echo ""
	@echo "$(YELLOW)Servicios Activos:$(NC)"
	@docker compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"
	@echo ""
	@echo "$(YELLOW)Accesos:$(NC)"
	@echo "  - API REST:      http://localhost:8080"
	@echo "  - Swagger UI:    http://localhost:8080/api/docs"
	@echo "  - OpenAPI JSON:  http://localhost:8080/api/openapi.json"
	@echo "  - Prometheus:    http://localhost:9090"
	@echo "  - Grafana:       http://localhost:3000"
