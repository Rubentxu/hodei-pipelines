#!/bin/bash
# Setup TestKube with vcluster

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        vcluster + TestKube - Setup Completo                   ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check vcluster
echo -e "${BLUE}[1/4]${NC} Verificando vcluster..."
if ! command -v vcluster &> /dev/null; then
    echo -e "${RED}❌ vcluster no está instalado${NC}"
    echo ""
    echo "Instalar vcluster:"
    echo "  1. Ejecutar: ./install-vcluster.sh"
    echo "  2. O instalar manualmente: https://www.vcluster.com/docs/deploy/basic"
    exit 1
fi

VCLUSTERS=$(vcluster list 2>/dev/null | grep -v "^NAME" | wc -l)
echo -e "${GREEN}✅ vcluster disponible${NC}"
echo "   vclusters ejecutándose: $VCLUSTERS"
echo ""

# Check if any vcluster is running
if [ "$VCLUSTERS" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  No hay vclusters ejecutándose${NC}"
    echo ""
    echo "Ejecuta primero:"
    echo "  ./install-vcluster.sh"
    echo ""
    read -p "¿Deseas ejecutar el instalador ahora? (y/n): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        ./install-vcluster.sh
    else
        echo -e "${YELLOW}Cancelado${NC}"
        exit 1
    fi
fi

# Select vcluster
echo -e "${BLUE}[2/4]${NC} Seleccionar vcluster..."
echo ""
vcluster list
echo ""

if [ "$VCLUSTERS" -eq 1 ]; then
    SELECTED_VCLUSTER=$(vcluster list 2>/dev/null | grep -v "^NAME" | awk '{print $1}' | head -1)
    echo -e "${GREEN}Usando vcluster: $SELECTED_VCLUSTER${NC}"
else
    echo "Selecciona un vcluster:"
    read -p "Nombre del vcluster: " SELECTED_VCLUSTER
    
    if ! vcluster list 2>/dev/null | grep -q "$SELECTED_VCLUSTER"; then
        echo -e "${RED}❌ vcluster '$SELECTED_VCLUSTER' no encontrado${NC}"
        exit 1
    fi
fi
echo ""

# Connect to vcluster
echo -e "${BLUE}[3/4]${NC] Conectando a vcluster..."

echo "Conectando a vcluster '$SELECTED_VCLUSTER'..."
vcluster connect "$SELECTED_VCLUSTER" --update-current

if kubectl cluster-info &> /dev/null; then
    echo -e "${GREEN}✅ Conectado al vcluster${NC}"
    kubectl get nodes 2>/dev/null && echo "   Nodes disponibles" || echo "   (No hay nodes, es normal para vcluster)"
else
    echo -e "${RED}❌ No se pudo conectar al vcluster${NC}"
    exit 1
fi
echo ""

# Install TestKube
echo -e "${BLUE}[4/4]${NC} Instalando TestKube..."

NAMESPACE="testkube"

echo "Creando namespace '$NAMESPACE'..."
kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

echo ""
echo "Instalando TestKube..."
helm repo add testkube https://kubeshop.github.io/helm-charts --force-update 2>/dev/null || true
helm repo update

helm upgrade --install testkube testkube/testkube \
    --namespace "$NAMESPACE" \
    --create-namespace \
    --set testkube-dashboard.enabled=true \
    --set testkube-api-server.service.type=ClusterIP \
    --wait --timeout=300s

echo ""
echo -e "${GREEN}✅ TestKube instalado${NC}"

# Wait for TestKube to be ready
echo "Esperando a que TestKube esté listo..."
for i in {1..60}; do
    if kubectl wait --for=condition=available --timeout=5s deployment/testkube-dashboard -n "$NAMESPACE" 2>/dev/null; then
        echo -e "${GREEN}✅ TestKube dashboard está listo${NC}"
        break
    fi
    echo -n "."
    sleep 2
done
echo ""

# Deploy tests
echo -e "${BLUE}Desplegando tests de TestKube...${NC}"
if [ -d "testkube/tests" ]; then
    kubectl apply -f testkube/tests/ -n "$NAMESPACE" 2>/dev/null || echo "   (No hay tests para desplegar)"
    kubectl apply -f testkube/test-suites/ -n "$NAMESPACE" 2>/dev/null || echo "   (No hay test suites para desplegar)"
    echo -e "${GREEN}✅ Tests desplegados${NC}"
else
    echo -e "${YELLOW}⚠️  Directorio testkube/ no encontrado${NC}"
fi
echo ""

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║               ✅ vcluster + TestKube listo                     ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}🎉 ¡Todo configurado correctamente!${NC}"
echo ""
echo "📊 Información:"
echo "   vcluster: $SELECTED_VCLUSTER"
echo "   TestKube namespace: $NAMESPACE"
echo ""
echo -e "${GREEN}🚀 Comandos útiles:${NC}"
echo ""
echo "1. Acceder al dashboard TestKube:"
echo "   kubectl port-forward svc/testkube-dashboard 8088:8088 -n $NAMESPACE"
echo "   # Abrir: http://localhost:8088"
echo ""
echo "2. Ver tests:"
echo "   kubectl get tests -n $NAMESPACE"
echo ""
echo "3. Ejecutar tests:"
echo "   ./testkube/scripts/run-tests.sh --suite hodei-smoke-tests"
echo ""
echo "4. Ver logs del vcluster:"
echo "   vcluster logs $SELECTED_VCLUSTER"
echo ""
echo "5. Ver estado del vcluster:"
echo "   vcluster list"
echo ""

echo -e "${BLUE}📚 Siguiente paso recomendado:${NC}"
echo "   ./testkube/scripts/run-tests.sh --suite hodei-smoke-tests"
echo ""
