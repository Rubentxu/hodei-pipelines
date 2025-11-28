#!/bin/bash
# Automatic Kubernetes Setup Script

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║              Kubernetes Automatic Setup                      ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to check if cluster is accessible
check_cluster() {
    if kubectl cluster-info &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Function to configure kubectl for CRC
configure_crc() {
    echo -e "${BLUE}Configurando kubectl para CRC...${NC}"
    
    if ! command -v crc &> /dev/null; then
        echo -e "${YELLOW}CRC no está instalado${NC}"
        return 1
    fi
    
    # Check CRC status
    CRC_STATUS=$(crc status 2>/dev/null | grep "CRC VM:" | awk '{print $3}')
    
    if [ "$CRC_STATUS" != "Running" ]; then
        echo -e "${YELLOW}CRC no está ejecutándose${NC}"
        echo ""
        read -p "¿Deseas iniciar CRC ahora? (y/n): " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${BLUE}Iniciando CRC... (esto puede tomar varios minutos)${NC}"
            crc start
            
            # Wait for CRC to be ready
            echo -e "${BLUE}Esperando a que CRC esté listo...${NC}"
            for i in {1..60}; do
                if crc status 2>/dev/null | grep -q "Running"; then
                    echo -e "${GREEN}✅ CRC está ejecutándose${NC}"
                    break
                fi
                echo -n "."
                sleep 5
            done
            echo ""
        else
            echo -e "${YELLOW}Cancelado por el usuario${NC}"
            return 1
        fi
    fi
    
    # Configure kubectl context
    echo -e "${BLUE}Configurando contexto kubectl...${NC}"
    
    if kubectl config use-context crc-developer &> /dev/null; then
        echo -e "${GREEN}✅ Contexto configurado: crc-developer${NC}"
    else
        echo -e "${YELLOW}⚠️  No se pudo configurar crc-developer, intentando crc-admin...${NC}"
        kubectl config use-context crc-admin &> /dev/null && \
            echo -e "${GREEN}✅ Contexto configurado: crc-admin${NC}" || \
            echo -e "${RED}❌ No se pudo configurar ningún contexto CRC${NC}"
    fi
    
    # Test connection
    if check_cluster; then
        echo -e "${GREEN}✅ Conexión al cluster exitosa${NC}"
        return 0
    else
        echo -e "${RED}❌ No se puede conectar al cluster CRC${NC}"
        return 1
    fi
}

# Function to configure minikube
configure_minikube() {
    echo -e "${BLUE}Configurando Minikube...${NC}"
    
    if ! command -v minikube &> /dev/null; then
        echo -e "${YELLOW}Minikube no está instalado${NC}"
        return 1
    fi
    
    # Start minikube
    if ! minikube status &> /dev/null; then
        echo -e "${BLUE}Iniciando Minikube...${NC}"
        minikube start
        
        # Wait for minikube to be ready
        echo -e "${BLUE}Esperando a que Minikube esté listo...${NC}"
        minikube status
    fi
    
    # Configure kubectl context
    kubectl config use-context minikube
    
    if check_cluster; then
        echo -e "${GREEN}✅ Minikube configurado y funcionando${NC}"
        return 0
    else
        echo -e "${RED}❌ No se puede conectar a Minikube${NC}"
        return 1
    fi
}

# Main setup
echo -e "${BLUE}Selecciona el cluster a configurar:${NC}"
echo ""
echo "1) CRC (OpenShift) - Recomendado si ya está instalado"
echo "2) Minikube - Para clusters locales simples"
echo "3) Solo verificar configuración actual"
echo ""

read -p "Opción [1-3]: " -n 1 -r
echo ""
echo ""

case $REPLY in
    1)
        configure_crc
        ;;
    2)
        configure_minikube
        ;;
    3)
        echo -e "${BLUE}Verificando configuración actual...${NC}"
        kubectl config current-context 2>/dev/null || echo "   No hay contexto configurado"
        kubectl cluster-info 2>/dev/null && echo -e "${GREEN}✅ Cluster accesible${NC}" || echo -e "${YELLOW}⚠️  Cluster no accesible${NC}"
        ;;
    *)
        echo -e "${RED}Opción inválida${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                        ✅ Setup completo                     ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Final verification
echo -e "${BLUE}Verificación final:${NC}"
CURRENT_CONTEXT=$(kubectl config current-context 2>/dev/null)
if [ -n "$CURRENT_CONTEXT" ]; then
    echo -e "  Contexto: ${GREEN}${CURRENT_CONTEXT}${NC}"
fi

if check_cluster; then
    echo -e "  Cluster: ${GREEN}Conectado${NC}"
    kubectl version --short 2>/dev/null | grep "Server Version" | sed 's/^/  /'
    echo ""
    echo -e "${GREEN}¡Listo para usar! Puedes proceder con:${NC}"
    echo "  - ./testkube/scripts/check-k8s.sh"
    echo "  - ./deployment/scripts/install.sh"
else
    echo -e "  Cluster: ${RED}No conectado${NC}"
    echo ""
    echo -e "${YELLOW}Ejecuta de nuevo el script de verificación:${NC}"
    echo "  ./testkube/scripts/check-k8s.sh"
fi
