#!/bin/bash
# =============================================================================
# Script de Verificación del Estado de Hodei Pipelines
# =============================================================================

set -e

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

NAMESPACE=${NAMESPACE:-hodei-jobs}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[⚠]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Verificar pods
check_pods() {
    log_info "Verificando pods en namespace $NAMESPACE..."

    local pods=$(kubectl get pods -n $NAMESPACE -o jsonpath='{.items[*].metadata.name}')
    local pod_count=$(echo $pods | wc -w)

    if [ $pod_count -eq 0 ]; then
        log_error "No hay pods en el namespace $NAMESPACE"
        return 1
    fi

    log_success "Encontrados $pod_count pods"

    # Verificar cada pod
    kubectl get pods -n $NAMESPACE

    # Verificar estado de pods
    local running_pods=$(kubectl get pods -n $NAMESPACE --field-selector=status.phase=Running -o jsonpath='{.items[*].metadata.name}' | wc -w)
    local total_pods=$(kubectl get pods -n $NAMESPACE -o jsonpath='{.items[*].metadata.name}' | wc -w)

    if [ "$running_pods" -eq "$total_pods" ]; then
        log_success "Todos los pods están ejecutándose ($running_pods/$total_pods)"
    else
        log_warning "Algunos pods no están ejecutándose ($running_pods/$total_pods)"
        kubectl get pods -n $NAMESPACE | grep -v Running
    fi
}

# Verificar servicios
check_services() {
    log_info "Verificando servicios..."

    local services=$(kubectl get svc -n $NAMESPACE -o jsonpath='{.items[*].metadata.name}')
    local service_count=$(echo $services | wc -w)

    log_success "Encontrados $service_count servicios"

    kubectl get svc -n $NAMESPACE
}

# Verificar PVCs
check_pvcs() {
    log_info "Verificando volúmenes persistentes..."

    local pvcs=$(kubectl get pvc -n $NAMESPACE -o jsonpath='{.items[*].metadata.name}')
    local pvc_count=$(echo $pvcs | wc -w)

    if [ $pvc_count -eq 0 ]; then
        log_warning "No hay PVCs configurados"
        return 0
    fi

    log_success "Encontrados $pvc_count PVCs"

    kubectl get pvc -n $NAMESPACE

    # Verificar estado de PVCs
    local bound_pvcs=$(kubectl get pvc -n $NAMESPACE --field-selector=status.phase=Bound -o jsonpath='{.items[*].metadata.name}' | wc -w)

    if [ "$bound_pvcs" -eq "$pvc_count" ]; then
        log_success "Todos los PVCs están bound"
    else
        log_warning "Algunos PVCs no están bound ($bound_pvcs/$pvc_count)"
    fi
}

# Verificar ingress
check_ingress() {
    log_info "Verificando ingress..."

    local ingress_count=$(kubectl get ingress -n $NAMESPACE 2>/dev/null | tail -n +2 | wc -l)

    if [ $ingress_count -eq 0 ]; then
        log_info "No hay ingress configurado"
        return 0
    fi

    log_success "Encontrados $ingress_count ingress"

    kubectl get ingress -n $NAMESPACE
}

# Verificar health checks
check_health() {
    log_info "Verificando health checks..."

    local server_pod=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=server -o jsonpath='{.items[0].metadata.name}')

    if [ -z "$server_pod" ]; then
        log_error "No se encontró pod del servidor"
        return 1
    fi

    log_info "Ejecutando health check en pod $server_pod..."

    if kubectl exec -n $NAMESPACE $server_pod -- curl -f http://localhost:8080/health &>/dev/null; then
        log_success "Health check passed"
    else
        log_error "Health check failed"
        kubectl logs -n $NAMESPACE $server_pod --tail=20
        return 1
    fi
}

# Verificar métricas
check_metrics() {
    log_info "Verificando endpoint de métricas..."

    local server_pod=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=server -o jsonpath='{.items[0].metadata.name}')

    if [ -z "$server_pod" ]; then
        log_error "No se encontró pod del servidor"
        return 1
    fi

    if kubectl exec -n $NAMESPACE $server_pod -- curl -f http://localhost:9091/metrics &>/dev/null; then
        log_success "Endpoint de métricas disponible"
    else
        log_warning "Endpoint de métricas no disponible"
    fi
}

# Verificar base de datos
check_database() {
    log_info "Verificando conexión a PostgreSQL..."

    local pg_pod=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/name=postgresql -o jsonpath='{.items[0].metadata.name}')

    if [ -z "$pg_pod" ]; then
        log_info "PostgreSQL no encontrado (puede estar en otro namespace)"
        return 0
    fi

    if kubectl exec -n $NAMESPACE $pg_pod -- pg_isready -U hodei &>/dev/null; then
        log_success "PostgreSQL está disponible"
    else
        log_error "PostgreSQL no está disponible"
        kubectl logs -n $NAMESPACE $pg_pod --tail=20
        return 1
    fi
}

# Verificar NATS
check_nats() {
    log_info "Verificando NATS..."

    local nats_pod=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/name=nats -o jsonpath='{.items[0].metadata.name}')

    if [ -z "$nats_pod" ]; then
        log_info "NATS no encontrado (puede estar en otro namespace)"
        return 0
    fi

    if kubectl exec -n $NAMESPACE $nats_pod -- nats-server --version &>/dev/null; then
        log_success "NATS está disponible"
    else
        log_error "NATS no está disponible"
        kubectl logs -n $NAMESPACE $nats_pod --tail=20
        return 1
    fi
}

# Verificar HPA
check_hpa() {
    log_info "Verificando HorizontalPodAutoscaler..."

    local hpa_count=$(kubectl get hpa -n $NAMESPACE 2>/dev/null | tail -n +2 | wc -l)

    if [ $hpa_count -eq 0 ]; then
        log_info "No hay HPA configurado"
        return 0
    fi

    log_success "Encontrados $hpa_count HPA"

    kubectl get hpa -n $NAMESPACE
}

# Verificar Prometheus
check_prometheus() {
    log_info "Verificando Prometheus..."

    local monitoring_ns="monitoring"

    if ! kubectl get namespace $monitoring_ns &>/dev/null; then
        log_info "Namespace de monitoreo no encontrado"
        return 0
    fi

    local prometheus_pod=$(kubectl get pods -n $monitoring_ns -l app.kubernetes.io/name=prometheus -o jsonpath='{.items[0].metadata.name}')

    if [ -z "$prometheus_pod" ]; then
        log_info "Prometheus no encontrado"
        return 0
    fi

    log_success "Prometheus está disponible"

    # Verificar ServiceMonitor
    local sm_count=$(kubectl get servicemonitor -n $NAMESPACE 2>/dev/null | tail -n +2 | wc -l)
    if [ $sm_count -gt 0 ]; then
        log_success "Encontrados $sm_count ServiceMonitor"
        kubectl get servicemonitor -n $NAMESPACE
    fi
}

# Verificar eventos
check_events() {
    log_info "Verificando eventos recientes..."

    local event_count=$(kubectl get events -n $NAMESPACE --field-selector reason!=Pulled,reason!=Pulling --sort-by='.lastTimestamp' 2>/dev/null | tail -n +2 | wc -l)

    if [ $event_count -gt 0 ]; then
        log_warning "Se encontraron $event_count eventos recientes"
        kubectl get events -n $NAMESPACE --field-selector reason!=Pulled,reason!=Pulling --sort-by='.lastTimestamp' | tail -20
    else
        log_success "No hay eventos recientes"
    fi
}

# Función principal
main() {
    echo "=============================================="
    echo "  Verificación de Hodei Pipelines"
    echo "  Namespace: $NAMESPACE"
    echo "=============================================="
    echo ""

    check_pods
    echo ""

    check_services
    echo ""

    check_pvcs
    echo ""

    check_ingress
    echo ""

    check_health
    echo ""

    check_metrics
    echo ""

    check_database
    echo ""

    check_nats
    echo ""

    check_hpa
    echo ""

    check_prometheus
    echo ""

    check_events
    echo ""

    echo "=============================================="
    echo "✅ Verificación completada"
    echo "=============================================="
}

main "$@"
