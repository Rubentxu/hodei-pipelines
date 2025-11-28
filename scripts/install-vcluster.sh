#!/bin/bash
# Install and Setup vcluster

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║              Vcluster Installation & Setup                    ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if kubectl is available
if ! kubectl cluster-info &> /dev/null; then
    echo -e "${YELLOW}⚠️  No se detectó un cluster Kubernetes base${NC}"
    echo ""
    echo -e "${BLUE}Para usar vcluster, necesitas un cluster Kubernetes base:${NC}"
    echo ""
    echo "Opciones:"
    echo "  1. Usar un cluster cloud (GKE, EKS, AKS)"
    echo "  2. Usar kind/k3d como cluster base"
    echo "  3. Usar un cluster existente"
    echo ""
    echo -e "${YELLOW}Si no tienes cluster, puedes crear uno rápidamente con:${NC}"
    echo "  kind create cluster --name base-cluster"
    echo "  # O"
    echo "  k3d cluster create base-cluster"
    echo ""
    read -p "¿Ya tienes un cluster Kubernetes? (y/n): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Creando cluster base con kind...${NC}"
        kind create cluster --name base-cluster
        kubectl cluster-info --context kind-base-cluster
    fi
fi

# Install vcluster
echo -e "${BLUE}[1/3]${NC} Verificando vcluster..."

if command -v vcluster &> /dev/null; then
    VCLUSTER_VERSION=$(vcluster version 2>/dev/null | head -1)
    echo -e "${GREEN}✅ vcluster ya está instalado${NC}"
    echo "   Versión: $VCLUSTER_VERSION"
else
    echo -e "${YELLOW}vcluster no está instalado${NC}"
    echo -e "${BLUE}Instalando vcluster...${NC}"
    
    # Detect OS
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    if [ "$ARCH" = "x86_64" ]; then
        ARCH="amd64"
    elif [ "$ARCH" = "aarch64" ]; then
        ARCH="arm64"
    fi
    
    # Get latest version
    LATEST_VERSION=$(curl -s https://api.github.com/repos/loft-sh/vcluster/releases/latest | grep -o '"tag_name": "v[^"]*"' | cut -d'"' -f4)
    
    if [ -z "$LATEST_VERSION" ]; then
        echo -e "${RED}❌ No se pudo obtener la versión de vcluster${NC}"
        echo "   Instalar manualmente desde: https://www.vcluster.com/docs/deploy/basic"
        exit 1
    fi
    
    # Download
    DOWNLOAD_URL="https://github.com/loft-sh/vcluster/releases/download/${LATEST_VERSION}/vcluster-${OS}-${ARCH}"
    
    echo "Descargando desde: $DOWNLOAD_URL"
    curl -L -o vcluster "$DOWNLOAD_URL"
    chmod +x vcluster
    
    # Move to /usr/local/bin
    sudo mv vcluster /usr/local/bin/vcluster
    
    echo -e "${GREEN}✅ vcluster instalado correctamente${NC}"
    vcluster version
fi
echo ""

# Create vcluster
echo -e "${BLUE}[2/3]${NC} Creando vcluster..."

VCLUSTER_NAME="hodei-vcluster"
NAMESPACE="vcluster-${VCLUSTER_NAME}"

echo "Creando vcluster '$VCLUSTER_NAME' en namespace '$NAMESPACE'..."
echo ""

# Check if vcluster already exists
if vcluster list 2>/dev/null | grep -q "$VCLUSTER_NAME"; then
    echo -e "${YELLOW}⚠️  vcluster '$VCLUSTER_NAME' ya existe${NC}"
    read -p "¿Deseas recrearlo? (y/n): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Eliminando vcluster existente...${NC}"
        vcluster delete "$VCLUSTER_NAME" --namespace "$NAMESPACE" 2>/dev/null || true
        sleep 2
    else
        echo -e "${YELLOW}Manteniendo vcluster existente${NC}"
    fi
fi

# Create vcluster if it doesn't exist or was recreated
if ! vcluster list 2>/dev/null | grep -q "$VCLUSTER_NAME"; then
    echo -e "${BLUE}Iniciando vcluster...${NC}"
    vcluster create "$VCLUSTER_NAME" \
        --namespace "$NAMESPACE" \
        --expose \
        --sync-back \
        --sync-back-end-server
    
    echo ""
    echo -e "${BLUE}Esperando a que vcluster esté listo...${NC}"
    sleep 10
    
    # Wait for vcluster to be ready
    for i in {1..30}; do
        if vcluster list 2>/dev/null | grep "$VCLUSTER_NAME" | grep -q "running\|Ready"; then
            echo -e "${GREEN}✅ vcluster está ejecutándose${NC}"
            break
        fi
        echo -n "."
        sleep 2
    done
    echo ""
fi
echo ""

# Setup kubectl context
echo -e "${BLUE}[3/3]${NC} Configurando contexto kubectl..."

CONTEXT_NAME="vcluster-${VCLUSTER_NAME}"

# Try to connect to vcluster
echo "Configurando contexto '$CONTEXT_NAME'..."
vcluster connect "$VCLUSTER_NAME" --namespace "$NAMESPACE" --update-current

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Contexto kubectl configurado${NC}"
    
    # Test connection
    echo ""
    echo "Probando conexión..."
    kubectl cluster-info
    kubectl get nodes 2>/dev/null || echo "   (No hay nodos aún, es normal)"
    
    echo ""
    echo -e "${GREEN}🎉 vcluster configurado correctamente!${NC}"
else
    echo -e "${RED}❌ Error al configurar vcluster${NC}"
    exit 1
fi

echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                   ✅ Setup completo                          ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Final instructions
echo -e "${GREEN}📊 Información del vcluster:${NC}"
echo "   Nombre: $VCLUSTER_NAME"
echo "   Namespace: $NAMESPACE"
echo "   Contexto: $CONTEXT_NAME"
echo ""

echo -e "${GREEN}🚀 Próximos pasos:${NC}"
echo ""
echo "1. Verificar estado:"
echo "   ./testkube/scripts/check-k8s.sh"
echo ""
echo "2. Desplegar Hodei Pipelines:"
echo "   ./deployment/scripts/install.sh -e dev"
echo ""
echo "3. Instalar TestKube:"
echo "   ./testkube/scripts/setup-testkube.sh"
echo ""
echo "4. Ejecutar tests:"
echo "   ./testkube/scripts/quick-test.sh"
echo ""

echo -e "${BLUE}Comandos útiles:${NC}"
echo "   Ver vclusters: vcluster list"
echo "   Conectar: vcluster connect $VCLUSTER_NAME"
echo "   Eliminar: vcluster delete $VCLUSTER_NAME"
echo "   Ver logs: vcluster logs $VCLUSTER_NAME"
echo ""

echo -e "${GREEN}¡Listo para usar!${NC}"
