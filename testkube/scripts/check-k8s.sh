#!/bin/bash
# Kubernetes Health Check and Setup Script

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Kubernetes Health Check & Setup                   ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check kubectl
echo -e "${BLUE}[1/5]${NC} Verificando kubectl..."
if command -v kubectl &> /dev/null; then
    K8S_VERSION=$(kubectl version --client -o json 2>/dev/null | grep -o '"gitVersion":"[^"]*' | head -1 | cut -d'"' -f4 || echo "unknown")
    echo -e "${GREEN}✅ kubectl está instalado (v${K8S_VERSION})${NC}"
else
    echo -e "${RED}❌ kubectl NO está instalado${NC}"
    echo "   Instalar desde: https://kubernetes.io/docs/tasks/tools/"
    exit 1
fi
echo ""

# Check cluster connection
echo -e "${BLUE}[2/5]${NC} Verificando conexión a cluster..."
CURRENT_CONTEXT=$(kubectl config current-context 2>/dev/null || echo "")
if [ -z "$CURRENT_CONTEXT" ]; then
    echo -e "${YELLOW}⚠️  No hay contexto configurado${NC}"
    CONTEXT_SET=false
else
    echo -e "   Contexto actual: ${CURRENT_CONTEXT}"
    if kubectl cluster-info &> /dev/null; then
        echo -e "${GREEN}✅ Cluster conectado${NC}"
        CLUSTER_OK=true
    else
        echo -e "${RED}❌ No se puede conectar al cluster${NC}"
        CLUSTER_OK=false
    fi
fi
echo ""

# Check for running clusters
echo -e "${BLUE}[3/5]${NC} Verificando clusters locales..."

echo -e "   ${BLUE}a) CRC/OpenShift:${NC}"
if command -v crc &> /dev/null; then
    CRC_STATUS=$(crc status 2>/dev/null | grep "CRC VM:" | awk '{print $3}')
    if [ "$CRC_STATUS" = "Running" ]; then
        echo -e "      ${GREEN}✅ CRC está ejecutándose${NC}"
    else
        echo -e "      ${YELLOW}⚠️  CRC está detenido${NC}"
        echo -e "         Para iniciar: ${BLUE}crc start${NC}"
    fi
else
    echo -e "      ${RED}❌ CRC no instalado${NC}"
fi

echo -e "   ${BLUE}b) Minikube:${NC}"
if command -v minikube &> /dev/null; then
    if minikube status &> /dev/null; then
        echo -e "      ${GREEN}✅ Minikube está ejecutándose${NC}"
    else
        echo -e "      ${YELLOW}⚠️  Minikube está detenido${NC}"
        echo -e "         Para iniciar: ${BLUE}minikube start${NC}"
    fi
else
    echo -e "      ${RED}❌ Minikube no instalado${NC}"
fi

echo -e "   ${BLUE}c) kind:${NC}"
if command -v kind &> /dev/null; then
    KIND_CLUSTERS=$(kind get clusters 2>/dev/null | wc -l)
    if [ "$KIND_CLUSTERS" -gt 0 ]; then
        echo -e "      ${GREEN}✅ kind clusters: $(kind get clusters 2>/dev/null | tr '\n' ', ')${NC}"
    else
        echo -e "      ${YELLOW}⚠️  No hay clusters kind${NC}"
    fi
else
    echo -e "      ${RED}❌ kind no instalado${NC}"
fi

echo -e "   ${BLUE}d) k3d:${NC}"
if command -v k3d &> /dev/null; then
    K3D_CLUSTERS=$(k3d cluster list 2>/dev/null | grep -v "NAME" | wc -l)
    if [ "$K3D_CLUSTERS" -gt 0 ]; then
        echo -e "      ${GREEN}✅ k3d clusters ejecutándose${NC}"
    else
        echo -e "      ${YELLOW}⚠️  No hay clusters k3d${NC}"
    fi
else
    echo -e "      ${RED}❌ k3d no instalado${NC}"
fi
echo ""

# Check namespaces and pods
echo -e "${BLUE}[4/5]${NC} Verificando estado del cluster..."
if [ "$CLUSTER_OK" = true ]; then
    NS_COUNT=$(kubectl get ns 2>/dev/null | tail -n +2 | wc -l)
    echo -e "   Namespaces: ${GREEN}${NS_COUNT}${NC}"
    
    POD_COUNT=$(kubectl get pods --all-namespaces 2>/dev/null | tail -n +2 | wc -l)
    echo -e "   Total pods: ${GREEN}${POD_COUNT}${NC}"
    
    RUNNING_PODS=$(kubectl get pods --all-namespaces --field-selector=status.phase=Running 2>/dev/null | tail -n +2 | wc -l)
    echo -e "   Pods ejecutándose: ${GREEN}${RUNNING_PODS}${NC}"
else
    echo -e "${RED}   Cluster no accesible - no se puede verificar${NC}"
fi
echo ""

# Recommendations
echo -e "${BLUE}[5/5]${NC} Recomendaciones..."
echo ""
if [ "$CLUSTER_OK" != true ]; then
    echo -e "${YELLOW}Para iniciar un cluster, elige una opción:${NC}"
    echo ""
    echo -e "${GREEN}Opción 1 - CRC (recomendado si ya está instalado):${NC}"
    echo "  $ crc start"
    echo "  $ crc status  # Esperar hasta que esté 'Running'"
    echo "  $ kubectl config use-context crc-developer"
    echo ""
    echo -e "${GREEN}Opción 2 - Minikube:${NC}"
    echo "  $ minikube start"
    echo "  $ minikube status"
    echo ""
    echo -e "${GREEN}Opción 3 - kind:${NC}"
    echo "  $ kind create cluster --name hodei"
    echo "  $ kubectl cluster-info --context kind-hodei"
    echo ""
    echo -e "${GREEN}Opción 4 - k3d:${NC}"
    echo "  $ k3d cluster create hodei"
    echo "  $ kubectl cluster-info --context k3d-hodei"
else
    echo -e "${GREEN}✅ Cluster está funcionando correctamente!${NC}"
    echo ""
    echo "Puedes proceder con:"
    echo "  - Desplegar Hodei Pipelines: ./deployment/scripts/install.sh"
    echo "  - Instalar TestKube: ./testkube/scripts/setup-testkube.sh"
    echo "  - Ejecutar tests: ./testkube/scripts/quick-test.sh"
fi
echo ""

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                     ✅ Verificación completa                 ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
