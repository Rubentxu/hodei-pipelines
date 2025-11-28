#!/bin/bash
# Quick Fix for Kubernetes - Start a cluster automatically

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Kubernetes Quick Fix - Auto Setup                  ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if cluster is already running
if kubectl cluster-info &> /dev/null; then
    echo -e "${GREEN}✅ Kubernetes cluster ya está ejecutándose${NC}"
    kubectl cluster-info
    echo ""
    echo -e "${GREEN}¡Listo! Puedes usar todos los scripts:${NC}"
    echo "  ./testkube/scripts/check-k8s.sh"
    echo "  ./deployment/scripts/install.sh"
    exit 0
fi

echo -e "${YELLOW}No se detectó un cluster Kubernetes ejecutándose${NC}"
echo ""
echo -e "${BLUE}Selecciona una opción para iniciar un cluster:${NC}"
echo ""
echo "1) vcluster       - ⭐ RECOMENDADO - Cluster virtual ultra-rápido (30s)"
echo "2) Minikube       - VM tradicional (2-5 min)"
echo "3) CRC (OpenShift) - Completo (5-10 min), ya instalado"
echo "4) kind           - Cluster local (1-2 min)"
echo "5) k3d            - Ultra ligero (30s)"
echo "6) Solo verificar - No iniciar nada, solo revisar"
echo ""

read -p "Opción [1-6]: " -n 1 -r
echo ""
echo ""

case $REPLY in
    1)
        echo -e "${BLUE}=== Configurando vcluster ===${NC}"

        # Check if vcluster is installed
        if ! command -v vcluster &> /dev/null; then
            echo -e "${YELLOW}Instalando vcluster...${NC}"
            ./install-vcluster.sh
            exit 0
        fi

        # Check base cluster
        if ! kubectl cluster-info &> /dev/null; then
            echo -e "${YELLOW}No hay cluster base. Creando con kind...${NC}"
            if ! command -v kind &> /dev/null; then
                echo -e "${RED}❌ kind no está instalado${NC}"
                echo "   Instalar kind o usar otra opción"
                exit 1
            fi
            kind create cluster --name base-cluster
            echo -e "${GREEN}✅ Cluster base creado${NC}"
        fi

        echo ""
        echo "Configurando vcluster + TestKube automáticamente..."
        ./testkube/scripts/install-and-setup-vcluster.sh
        ;;
    2)
        echo -e "${BLUE}=== Iniciando Minikube ===${NC}"
        if ! command -v minikube &> /dev/null; then
            echo -e "${RED}❌ Minikube no está instalado${NC}"
            echo "   Instalar desde: https://minikube.sigs.k8s.io/docs/install/"
            exit 1
        fi
        
        echo "Iniciando Minikube..."
        minikube start
        
        echo ""
        echo -e "${BLUE}Verificando estado...${NC}"
        minikube status
        
        echo ""
        echo -e "${GREEN}✅ Minikube iniciado${NC}"
        kubectl cluster-info
        ;;

    3)
        echo -e "${BLUE}=== Iniciando CRC (OpenShift) ===${NC}"
        if ! command -v crc &> /dev/null; then
            echo -e "${RED}❌ CRC no está instalado${NC}"
            echo "   Instalar desde: https://crc.dev/"
            exit 1
        fi

        echo "Iniciando CRC (esto puede tomar varios minutos)..."
        crc start

        echo ""
        echo -e "${BLUE}Configurando kubectl...${NC}"
        kubectl config use-context crc-developer

        echo ""
        echo -e "${GREEN}✅ CRC iniciado${NC}"
        kubectl cluster-info
        ;;

    4)
        echo -e "${BLUE}=== Creando cluster con kind ===${NC}"
        if ! command -v kind &> /dev/null; then
            echo -e "${RED}❌ kind no está instalado${NC}"
            echo "   Instalar desde: https://kind.sigs.k8s.io/docs/user/quick-start/#installation"
            exit 1
        fi

        echo "Creando cluster 'hodei'..."
        kind create cluster --name hodei

        echo ""
        echo -e "${GREEN}✅ Cluster kind creado${NC}"
        kubectl cluster-info --context kind-hodei
        ;;

    5)
        echo -e "${BLUE}=== Creando cluster con k3d ===${NC}"
        if ! command -v k3d &> /dev/null; then
            echo -e "${RED}❌ k3d no está instalado${NC}"
            echo "   Instalar desde: https://k3d.io/#installation"
            exit 1
        fi

        echo "Creando cluster 'hodei'..."
        k3d cluster create hodei

        echo ""
        echo -e "${GREEN}✅ Cluster k3d creado${NC}"
        kubectl cluster-info --context k3d-hodei
        ;;

    6)
        echo -e "${BLUE}=== Verificando configuración ===${NC}"
        echo "Contextos disponibles:"
        kubectl config get-contexts
        echo ""
        echo "Clusters configurados:"
        kubectl config view | grep "server:"
        echo ""
        echo -e "${YELLOW}No se inició ningún cluster${NC}"
        ;;
        
    *)
        echo -e "${RED}Opción inválida${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                    ✅ Setup completado                       ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Final verification
if kubectl cluster-info &> /dev/null; then
    echo -e "${GREEN}🎉 ¡Kubernetes está funcionando!${NC}"
    echo ""
    echo "Próximos pasos:"
    echo "  1. Verificar estado completo:"
    echo "     ./testkube/scripts/check-k8s.sh"
    echo ""
    echo "  2. Desplegar Hodei Pipelines:"
    echo "     ./deployment/scripts/install.sh -e dev"
    echo ""
    echo "  3. Instalar TestKube:"
    echo "     ./testkube/scripts/setup-testkube.sh"
    echo ""
    echo "  4. Ejecutar tests:"
    echo "     ./testkube/scripts/quick-test.sh"
else
    echo -e "${YELLOW}⚠️  No se pudo conectar al cluster${NC}"
    echo "   Ejecuta de nuevo: ./testkube/scripts/check-k8s.sh"
fi
