#!/bin/bash
# Complete vcluster + TestKube Setup

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        vcluster + TestKube - Setup Completo (TODO EN UNO)      ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Install vcluster
echo -e "${BLUE}[1/5]${NC} Instalando vcluster..."

if ! command -v vcluster &> /dev/null; then
    echo "Descargando e instalando vcluster..."
    
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    if [ "$ARCH" = "x86_64" ]; then
        ARCH="amd64"
    elif [ "$ARCH" = "aarch64" ]; then
        ARCH="arm64"
    fi
    
    LATEST_VERSION=$(curl -s https://api.github.com/repos/loft-sh/vcluster/releases/latest | grep -o '"tag_name": "v[^"]*"' | cut -d'"' -f4)
    DOWNLOAD_URL="https://github.com/loft-sh/vcluster/releases/download/${LATEST_VERSION}/vcluster-${OS}-${ARCH}"
    
    curl -L -o vcluster "$DOWNLOAD_URL"
    chmod +x vcluster
    sudo mv vcluster /usr/local/bin/vcluster
    
    echo -e "${GREEN}✅ vcluster instalado${NC}"
else
    echo -e "${GREEN}✅ vcluster ya está instalado${NC}"
fi
echo ""

# Step 2: Ensure base cluster
echo -e "${BLUE}[2/5]${NC} Verificando cluster base..."

if ! kubectl cluster-info &> /dev/null; then
    echo -e "${YELLOW}⚠️  No hay cluster base${NC}"
    echo ""
    echo "Creando cluster base con kind..."
    kind create cluster --name base-cluster
    kubectl cluster-info --context kind-base-cluster
fi

echo -e "${GREEN}✅ Cluster base disponible${NC}"
echo ""

# Step 3: Create vcluster
echo -e "${BLUE}[3/5]${NC} Creando vcluster..."

VCLUSTER_NAME="hodei-vcluster"
NAMESPACE="vcluster-${VCLUSTER_NAME}"

if vcluster list 2>/dev/null | grep -q "$VCLUSTER_NAME"; then
    echo -e "${YELLOW}vcluster '$VCLUSTER_NAME' ya existe${NC}"
    read -p "¿Usar el existente? (Y/n): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Eliminando vcluster existente..."
        vcluster delete "$VCLUSTER_NAME"
        sleep 3
        vcluster create "$VCLUSTER_NAME" \
            --namespace "$NAMESPACE" \
            --expose \
            --sync-back \
            --sync-back-end-server
    fi
else
    echo "Creando vcluster '$VCLUSTER_NAME'..."
    vcluster create "$VCLUSTER_NAME" \
        --namespace "$NAMESPACE" \
        --expose \
        --sync-back \
        --sync-back-end-server
fi

echo "Esperando a que esté listo..."
sleep 10

echo -e "${GREEN}✅ vcluster creado y ejecutándose${NC}"
echo ""

# Step 4: Connect and install TestKube
echo -e "${BLUE}[4/5]${NC] Conectando y configurando kubectl..."

vcluster connect "$VCLUSTER_NAME" --update-current

echo -e "${GREEN}✅ Conectado al vcluster${NC}"
kubectl cluster-info
echo ""

# Step 5: Install TestKube
echo -e "${BLUE}[5/5]${NC} Instalando TestKube..."

TESTKUBE_NS="testkube"

echo "Creando namespace..."
kubectl create namespace "$TESTKUBE_NS" --dry-run=client -o yaml | kubectl apply -f -

echo "Instalando TestKube..."
helm repo add testkube https://kubeshop.github.io/helm-charts --force-update 2>/dev/null || true
helm repo update

helm upgrade --install testkube testkube/testkube \
    --namespace "$TESTKUBE_NS" \
    --create-namespace \
    --set testkube-dashboard.enabled=true \
    --set testkube-api-server.service.type=ClusterIP \
    --wait --timeout=300s

echo -e "${GREEN}✅ TestKube instalado${NC}"
echo ""

# Deploy tests
echo "Desplegando tests..."
if [ -d "testkube/tests" ]; then
    kubectl apply -f testkube/tests/ -n "$TESTKUBE_NS" 2>/dev/null || true
    kubectl apply -f testkube/test-suites/ -n "$TESTKUBE_NS" 2>/dev/null || true
    echo -e "${GREEN}✅ Tests desplegados${NC}"
fi
echo ""

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                ✅ ¡Setup completado!                         ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}🎉 ¡Todo listo!${NC}"
echo ""
echo "📊 Información:"
echo "   vcluster: $VCLUSTER_NAME"
echo "   TestKube namespace: $TESTKUBE_NS"
echo "   Contexto kubectl: vcluster-$VCLUSTER_NAME"
echo ""

echo -e "${GREEN}🚀 Comandos para usar:${NC}"
echo ""
echo "1. Dashboard TestKube:"
echo "   kubectl port-forward svc/testkube-dashboard 8088:8088 -n $TESTKUBE_NS"
echo "   # Abrir: http://localhost:8088"
echo ""
echo "2. Ver tests disponibles:"
echo "   kubectl get tests -n $TESTKUBE_NS"
echo ""
echo "3. Ejecutar smoke tests:"
echo "   ./testkube/scripts/run-tests.sh --suite hodei-smoke-tests"
echo ""
echo "4. Ver estado del vcluster:"
echo "   vcluster list"
echo ""
echo "5. Ver logs:"
echo "   vcluster logs $VCLUSTER_NAME"
echo ""

echo -e "${GREEN}📝 Script para gestionar vcluster:${NC}"
echo "   ./manage-vcluster.sh"
echo ""
