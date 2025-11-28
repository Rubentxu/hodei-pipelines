#!/bin/bash
# =============================================================================
# Script de Instalación de Hodei Pipelines con Helm
# =============================================================================

set -e

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Función para logging
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Función para mostrar ayuda
show_help() {
    cat << EOF
Hodei Pipelines - Script de Instalación

USO:
    ./install.sh [OPCIONES]

OPCIONES:
    -e, --environment ENV        Entorno de instalación (dev|prod) [default: dev]
    -n, --namespace NS          Namespace de Kubernetes [default: hodei-dev o hodei-pipelines]
    -t, --tag TAG               Tag de la imagen Docker [default: latest]
    --dry-run                  Ejecutar en modo dry-run
    --upgrade                  Actualizar instalación existente
    --uninstall                Desinstalar Hodei
    -h, --help                 Mostrar esta ayuda

EJEMPLOS:
    # Instalar en desarrollo
    ./install.sh -e dev

    # Instalar en producción
    ./install.sh -e prod -t v0.1.0

    # Actualizar instalación
    ./install.sh --upgrade -t v0.1.1

    # Desinstalar
    ./install.sh --uninstall

EOF
}

# Verificar prerequisitos
check_prerequisites() {
    log_info "Verificando prerequisitos..."

    # Verificar kubectl
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl no está instalado"
        exit 1
    fi
    log_success "kubectl encontrado: $(kubectl version --client --short 2>/dev/null || kubectl version --client 2>/dev/null | head -1)"

    # Verificar helm
    if ! command -v helm &> /dev/null; then
        log_error "Helm no está instalado"
        exit 1
    fi
    log_success "Helm encontrado: $(helm version --short 2>/dev/null || helm version 2>/dev/null | head -1)"

    # Verificar conexión a Kubernetes
    if ! kubectl cluster-info &> /dev/null; then
        log_error "No se puede conectar a Kubernetes"
        exit 1
    fi
    log_success "Conexión a Kubernetes verificada"

    # Verificar acceso al cluster
    if ! kubectl auth can-i create deployment &> /dev/null; then
        log_warning "No tienes permisos para crear deployments en el cluster"
    fi
}

# Función para instalar
install_hodei() {
    local environment=$1
    local namespace=$2
    local tag=$3
    local dry_run=$4
    local upgrade=$5

    # Configurar valores según entorno
    local values_file="deployment/helm/values.yaml"
    case $environment in
        dev)
            namespace=${namespace:-hodei-dev}
            values_file="deployment/helm/values-dev.yaml"
            log_info "Instalando en modo DESARROLLO"
            log_warning "No se recomienda para producción"
            ;;
        prod)
            namespace=${namespace:-hodei-pipelines}
            values_file="deployment/helm/values-prod.yaml"
            log_info "Instalando en modo PRODUCCIÓN"
            ;;
        *)
            log_error "Entorno desconocido: $environment"
            exit 1
            ;;
    esac

    # Construir comando Helm
    local helm_cmd="helm install"
    if [ "$upgrade" = "true" ]; then
        helm_cmd="helm upgrade"
    fi

    local helm_args=""
    helm_args="$helm_args --namespace $namespace"
    helm_args="$helm_args --create-namespace"
    helm_args="$helm_args -f $values_file"
    helm_args="$helm_args --set hodei-server.image.tag=$tag"
    helm_args="$helm_args --set hwp-agent.image.tag=$tag"

    if [ "$dry_run" = "true" ]; then
        helm_args="$helm_args --dry-run"
        log_info "Ejecutando en modo DRY-RUN"
    fi

    # Añadir repos si es necesario
    log_info "Añadiendo repos de Helm..."
    helm repo add bitnami https://charts.bitnami.com/bitnami --force-update 2>/dev/null || true
    helm repo add nats https://nats-io.github.io/k8s/helm/charts --force-update 2>/dev/null || true
    helm repo add prometheus-community https://prometheus-community.github.io/helm-charts --force-update 2>/dev/null || true
    helm repo add grafana https://grafana.github.io/helm-charts --force-update 2>/dev/null || true
    helm repo update

    # Ejecutar instalación
    local release_name="hodei"
    log_info "Instalando Hodei Pipelines..."
    log_info "Namespace: $namespace"
    log_info "Values: $values_file"
    log_info "Tag: $tag"

    if eval "$helm_cmd $release_name deployment/helm $helm_args"; then
        log_success "Hodei instalado exitosamente"
    else
        log_error "Error durante la instalación"
        exit 1
    fi

    # Mostrar información post-instalación
    show_post_install_info $namespace
}

# Función para desinstalar
uninstall_hodei() {
    local namespace=$1

    log_warning "Desinstalando Hodei Pipelines..."
    read -p "⚠️  ¿Estás seguro? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_info "Desinstalando release 'hodei'..."
        helm uninstall hodei --namespace $namespace || true
        log_info "Eliminando namespace..."
        kubectl delete namespace $namespace --ignore-not-found=true || true
        log_success "Hodei desinstalado"
    else
        log_info "Desinstalación cancelada"
    fi
}

# Función para mostrar información post-instalación
show_post_install_info() {
    local namespace=$1

    echo ""
    echo "=========================================="
    echo "✅ Hodei Pipelines instalado exitosamente"
    echo "=========================================="
    echo ""
    echo "Namespace: $namespace"
    echo ""
    echo "📋 Comandos útiles:"
    echo ""
    echo "  Ver estado:"
    echo "    helm status hodei -n $namespace"
    echo ""
    echo "  Ver pods:"
    echo "    kubectl get pods -n $namespace"
    echo ""
    echo "  Ver logs del servidor:"
    echo "    kubectl logs -f deployment/hodei-server -n $namespace"
    echo ""
    echo "  Ver logs de workers:"
    echo "    kubectl logs -f deployment/hwp-agent -n $namespace"
    echo ""
    echo "  Ver servicios:"
    echo "    kubectl get svc -n $namespace"
    echo ""
    if [ -n "$(kubectl get ingress -n $namespace 2>/dev/null | grep hodei)" ]; then
        echo "  Ver ingress:"
        echo "    kubectl get ingress -n $namespace"
        echo ""
    fi
    echo "=========================================="
    echo ""
    echo "📚 Documentación completa en: deployment/README.md"
    echo ""
}

# Función principal
main() {
    # Parsear argumentos
    ENVIRONMENT="dev"
    NAMESPACE=""
    TAG="latest"
    DRY_RUN="false"
    UPGRADE="false"
    UNINSTALL="false"

    while [[ $# -gt 0 ]]; do
        case $1 in
            -e|--environment)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -n|--namespace)
                NAMESPACE="$2"
                shift 2
                ;;
            -t|--tag)
                TAG="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            --upgrade)
                UPGRADE="true"
                shift
                ;;
            --uninstall)
                UNINSTALL="true"
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Opción desconocida: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Verificar prerequisitos
    check_prerequisites

    # Ejecutar acción
    if [ "$UNINSTALL" = "true" ]; then
        local namespace=${NAMESPACE:-hodei-pipelines}
        uninstall_hodei $namespace
    else
        install_hodei $ENVIRONMENT $NAMESPACE $TAG $DRY_RUN $UPGRADE
    fi
}

# Ejecutar función principal
main "$@"
