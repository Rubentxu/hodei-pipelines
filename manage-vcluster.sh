#!/bin/bash
# vcluster Management Script

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

show_usage() {
    cat << EOF
vcluster Management Script

USAGE:
    ./manage-vcluster.sh [COMMAND]

COMMANDS:
    create         Crear un nuevo vcluster
    list           Listar vclusters
    connect        Conectar a un vcluster
    delete         Eliminar un vcluster
    status         Ver estado de vclusters
    logs           Ver logs de un vcluster
    setup-testkube Instalar TestKube en vcluster
    help           Mostrar esta ayuda

EXAMPLES:
    ./manage-vcluster.sh create
    ./manage-vcluster.sh list
    ./manage-vcluster.sh connect hodei-vcluster
    ./manage-vcluster.sh delete hodei-vcluster
    ./manage-vcluster.sh setup-testkube

EOF
}

create_vcluster() {
    echo -e "${BLUE}Crear vcluster${NC}"
    echo ""
    
    read -p "Nombre del vcluster [hodei-vcluster]: " NAME
    NAME=${NAME:-hodei-vcluster}
    
    read -p "Namespace [vcluster-$NAME]: " NAMESPACE
    NAMESPACE=${NAMESPACE:-vcluster-$NAME}
    
    echo ""
    echo "Creando vcluster '$NAME' en namespace '$NAMESPACE'..."
    
    vcluster create "$NAME" \
        --namespace "$NAMESPACE" \
        --expose \
        --sync-back \
        --sync-back-end-server
    
    echo ""
    echo -e "${GREEN}✅ vcluster creado${NC}"
    
    # Connect
    echo ""
    echo "Conectando..."
    vcluster connect "$NAME" --update-current
    
    echo ""
    echo -e "${GREEN}¡Listo!${NC}"
    kubectl cluster-info
}

list_vclusters() {
    echo -e "${BLUE}vclusters:${NC}"
    echo ""
    
    if command -v vcluster &> /dev/null; then
        vcluster list
    else
        echo -e "${RED}vcluster no está instalado${NC}"
        echo "Instalar desde: https://www.vcluster.com/docs/deploy/basic"
    fi
}

connect_vcluster() {
    if [ -z "$1" ]; then
        echo -e "${YELLOW}Conectar a vcluster${NC}"
        echo ""
        list_vclusters
        echo ""
        read -p "Nombre del vcluster: " NAME
        
        if [ -z "$NAME" ]; then
            echo -e "${RED}Nombre requerido${NC}"
            exit 1
        fi
    else
        NAME="$1"
    fi
    
    echo ""
    echo "Conectando a '$NAME'..."
    vcluster connect "$NAME" --update-current
    
    echo ""
    echo -e "${GREEN}✅ Conectado${NC}"
    kubectl cluster-info
}

delete_vcluster() {
    if [ -z "$1" ]; then
        echo -e "${YELLOW}Eliminar vcluster${NC}"
        echo ""
        list_vclusters
        echo ""
        read -p "Nombre del vcluster a eliminar: " NAME
        
        if [ -z "$NAME" ]; then
            echo -e "${RED}Nombre requerido${NC}"
            exit 1
        fi
    else
        NAME="$1"
    fi
    
    echo ""
    read -p "¿Eliminar vcluster '$NAME'? (y/n): " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Eliminando..."
        vcluster delete "$NAME"
        echo -e "${GREEN}✅ Eliminado${NC}"
    else
        echo "Cancelado"
    fi
}

status_vclusters() {
    echo -e "${BLUE}Estado de vclusters${NC}"
    echo ""
    
    if ! command -v vcluster &> /dev/null; then
        echo -e "${RED}vcluster no está instalado${NC}"
        exit 1
    fi
    
    vcluster list
    
    echo ""
    echo "Clusters base disponibles:"
    kubectl config get-contexts | grep -v "^$" | head -10
}

logs_vcluster() {
    if [ -z "$1" ]; then
        echo -e "${YELLOW}Ver logs de vcluster${NC}"
        echo ""
        list_vclusters
        echo ""
        read -p "Nombre del vcluster: " NAME
        
        if [ -z "$NAME" ]; then
            echo -e "${RED}Nombre requerido${NC}"
            exit 1
        fi
    else
        NAME="$1"
    fi
    
    echo ""
    echo "Logs de '$NAME':"
    echo "Presiona Ctrl+C para salir"
    echo ""
    
    vcluster logs "$NAME" -f
}

setup_testkube() {
    echo -e "${BLUE}Setup TestKube en vcluster${NC}"
    echo ""
    
    if [ -z "$1" ]; then
        read -p "Nombre del vcluster: " NAME
        
        if [ -z "$NAME" ]; then
            echo -e "${RED}Nombre requerido${NC}"
            exit 1
        fi
    else
        NAME="$1"
    fi
    
    # Connect first
    echo "Conectando a vcluster..."
    vcluster connect "$NAME" --update-current
    
    echo ""
    echo "Ejecutando setup de TestKube..."
    ./testkube/scripts/setup-vcluster-testkube.sh
}

# Main
case "${1:-}" in
    create)
        create_vcluster
        ;;
    list)
        list_vclusters
        ;;
    connect)
        connect_vcluster "$2"
        ;;
    delete)
        delete_vcluster "$2"
        ;;
    status)
        status_vclusters
        ;;
    logs)
        logs_vcluster "$2"
        ;;
    setup-testkube)
        setup_testkube "$2"
        ;;
    help|--help|-h)
        show_usage
        ;;
    *)
        show_usage
        exit 1
        ;;
esac
