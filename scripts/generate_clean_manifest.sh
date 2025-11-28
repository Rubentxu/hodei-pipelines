#!/bin/bash
################################################################################
# Generador de CODE_MANIFEST Limpio
#
# Versión mejorada del generador que excluye archivos de target/
# y directorios irrelevantes para tener un manifiesto más limpio.
#
# Uso:
#   ./generate_clean_manifest.sh [opciones] [archivo_salida]
#
# Opciones:
#   --ports-adapters, -pa    Solo procesar crates de adapters y ports (Infraestructura)
#
# Por defecto: CODE_MANIFEST.csv en el directorio raíz del proyecto
################################################################################

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Parseo de opciones
PORTS_ADAPTERS_ONLY=0
OUTPUT_FILE=""
SHIFTED_ARGS=()

while [[ $# -gt 0 ]]; do
    case $1 in
        --ports-adapters|-pa)
            PORTS_ADAPTERS_ONLY=1
            shift
            ;;
        -h|--help)
            echo "Uso: $0 [opciones] [archivo_salida]"
            echo ""
            echo "Opciones:"
            echo "  --ports-adapters, -pa    Solo procesar crates de adapters y ports (Infraestructura)"
            echo ""
            echo "Ejemplos:"
            echo "  $0                       # Generar CODE_MANIFEST.csv completo"
            echo "  $0 output.csv            # Generar archivo personalizado"
            echo "  $0 --ports-adapters      # Solo adapters y ports"
            echo "  $0 -pa infra.csv         # Solo adapters y ports, archivo personalizado"
            exit 0
            ;;
        *)
            # Si no es una opción, es el archivo de salida
            OUTPUT_FILE="$1"
            shift
            ;;
    esac
done

# Parámetros por defecto
OUTPUT_FILE="${OUTPUT_FILE:-CODE_MANIFEST.csv}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Resolver ruta absoluta solo si se pasó un argumento y no es una ruta absoluta
case "$OUTPUT_FILE" in
    /*) ;; # Ya es absoluta
    *) OUTPUT_FILE="$ROOT_DIR/$OUTPUT_FILE" ;;
esac

# Banner
echo -e "${BLUE}============================================================${NC}"
if [ $PORTS_ADAPTERS_ONLY -eq 1 ]; then
    echo -e "${BLUE}Generador de CODE_MANIFEST Limpio (Solo Ports & Adapters)${NC}"
else
    echo -e "${BLUE}Generador de CODE_MANIFEST Limpio (Sin target/ y tests/)${NC}"
fi
echo -e "${BLUE}============================================================${NC}"
echo -e "${YELLOW}Directorio raíz:${NC} $ROOT_DIR"
echo -e "${YELLOW}Archivo de salida:${NC} $OUTPUT_FILE"
if [ $PORTS_ADAPTERS_ONLY -eq 1 ]; then
    echo -e "${GREEN}Modo: Solo crates/adapters y crates/ports${NC}"
fi
echo ""

# Verificar directorio
if [ ! -d "$ROOT_DIR" ]; then
    echo -e "${RED}❌ Error: El directorio $ROOT_DIR no existe${NC}"
    exit 1
fi

# Crear temporal
TEMP_FILE=$(mktemp)
echo "Ruta Completa,Categoria,Procesado" > "$TEMP_FILE"

# Contadores
TOTAL=0
declare -A CATEGORIES

echo -e "${BLUE}🔍 Buscando archivos .rs (excluyendo target/ y tests/)...${NC}"

# Construir comando find dinámico basado en el modo
if [ $PORTS_ADAPTERS_ONLY -eq 1 ]; then
    echo -e "${GREEN}   Filtrando solo: crates/adapters y crates/ports${NC}"
    SEARCH_PATH="$ROOT_DIR/crates/adapters $ROOT_DIR/crates/ports"
else
    SEARCH_PATH="$ROOT_DIR"
fi

# Buscar archivos excluyendo target/ y otros directorios irrelevantes
while IFS= read -r -d '' file; do
    ((TOTAL++))

    REL_PATH="${file#$ROOT_DIR/}"

    # Categorizar
    CATEGORY=""
    if [[ "$file" == *"hwp-agent"* && "$file" == *"/src/"* ]]; then
        CATEGORY="HWP Agent - src/"
    elif [[ "$file" == *"hwp-agent"* && *"/tests/"* ]]; then
        CATEGORY="HWP Agent - tests"
    elif [[ "$file" == *"hwp-proto"* ]]; then
        CATEGORY="HWP Proto"
    elif [[ "$file" == *"e2e-tests"* ]]; then
        CATEGORY="E2E Tests"
    elif [[ "$file" == *"/tests/"* ]] || [[ "$file" == *"/test/"* ]]; then
        CATEGORY="Tests (unit/integration)"
    elif [[ "$file" == *"/examples/"* ]]; then
        CATEGORY="Examples"
    elif [[ "$file" == *"crates/core"* ]]; then
        CATEGORY="Core Domain"
    elif [[ "$file" == *"crates/modules"* ]]; then
        CATEGORY="Modules (Application)"
    elif [[ "$file" == *"crates/adapters"* ]]; then
        CATEGORY="Adapters (Infrastructure)"
    elif [[ "$file" == *"crates/ports"* ]]; then
        CATEGORY="Ports (Interfaces)"
    elif [[ "$file" == *"/server/src/"* ]]; then
        CATEGORY="Server"
    elif [[ "$file" == *"/build.rs"* ]]; then
        CATEGORY="Build scripts"
    elif [[ "$file" == *"code-manifest"* ]]; then
        CATEGORY="Manifest utility"
    else
        CATEGORY="Otros"
    fi

    # Escribir línea (solo 3 columnas)
    echo "\"$REL_PATH\",\"$CATEGORY\",\"\"" >> "$TEMP_FILE"

    # Stats
    CATEGORIES[$CATEGORY]=$((CATEGORIES[$CATEGORY] + 1))

    # Progreso
    if ((TOTAL % 30 == 0)); then
        echo -ne "${BLUE}   Procesados: $TOTAL${NC}\r"
    fi

done < <(find $SEARCH_PATH \
    -type f \
    -name "*.rs" \
    -not -path "*/target/*" \
    -not -path "*/.git/*" \
    -not -path "*/node_modules/*" \
    -not -path "*/.idea/*" \
    -not -path "*/.vscode/*" \
    -not -path "*/.claude/*" \
    -not -path "*/Cargo.lock" \
    -not -path "*/tests/*" \
    -not -path "*/test/*" \
    -not -name "*_test.rs" \
    -not -name "*_tests.rs" \
    -not -name "functional_tests.rs" \
    -not -name "integration_tests.rs" \
    -print0 2>/dev/null | sort -z)

# Mover archivo
if mv "$TEMP_FILE" "$OUTPUT_FILE" 2>/dev/null; then
    echo ""
    echo -e "${GREEN}✅ CODE_MANIFEST.csv generado exitosamente${NC}"
    echo ""
    echo -e "${YELLOW}📊 Estadísticas:${NC}"
    echo -e "   Total de archivos (limpio): ${GREEN}$TOTAL${NC}"
    echo ""
    echo -e "${YELLOW}📋 Resumen por categoría:${NC}"
    for cat in "${!CATEGORIES[@]}"; do
        echo -e "   ${BLUE}$cat:${NC} ${GREEN}${CATEGORIES[$cat]}${NC}"
    done
    echo ""
    echo -e "${YELLOW}📄 Archivo:${NC} $OUTPUT_FILE"
    echo ""
    echo -e "${YELLOW}💡 Uso:${NC}"
    echo "   1. Abre CODE_MANIFEST.csv en Excel/Google Sheets"
    echo "   2. Marca la columna 'Procesado' según avances"
    echo "   3. Usa filtros para organizar por categoría"
    echo ""
    echo -e "${GREEN}✨ Proceso completado${NC}"
else
    echo -e "${RED}❌ Error al generar el archivo${NC}"
    exit 1
fi
