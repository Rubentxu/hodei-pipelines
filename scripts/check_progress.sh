#!/bin/bash
################################################################################
# Script de Verificación de Progreso del CODE_MANIFEST
#
# Lee el archivo CODE_MANIFEST.csv y genera un reporte de progreso
# mostrando qué archivos han sido marcados como procesados.
#
# Uso:
#   ./check_progress.sh [archivo_csv]
#
################################################################################

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Parámetros
MANIFEST_FILE="${1:-CODE_MANIFEST.csv}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_FILE="$SCRIPT_DIR/../$MANIFEST_FILE"

# Banner
echo -e "${BLUE}============================================================${NC}"
echo -e "${BLUE}Reporte de Progreso del Manifiesto de Código${NC}"
echo -e "${BLUE}============================================================${NC}"
echo ""

# Verificar archivo
if [ ! -f "$MANIFEST_FILE" ]; then
    echo -e "${RED}❌ Error: No se encontró el archivo $MANIFEST_FILE${NC}"
    echo ""
    echo "Para generar el manifiesto, ejecuta:"
    echo "  ./scripts/generate_clean_manifest.sh"
    exit 1
fi

echo -e "${YELLOW}Archivo:${NC} $MANIFEST_FILE"
echo ""

# Leer CSV y procesar
TOTAL=0
PROCESSED=0
declare -A CATEGORY_TOTAL
declare -A CATEGORY_PROCESSED

# Saltar header
while IFS=',' read -r path category processed; do
    # Limpiar comillas
    path="${path%\"}"
    path="${path#\"}"
    category="${category%\"}"
    category="${category#\"}"
    processed="${processed%\"}"
    processed="${processed#\"}"

    # Saltar header
    if [[ "$path" == "Ruta Completa" ]]; then
        continue
    fi

    ((TOTAL++))

    # Contar por categoría
    if [ -z "${CATEGORY_TOTAL[$category]+isset}" ]; then
        CATEGORY_TOTAL[$category]=0
        CATEGORY_PROCESSED[$category]=0
    fi
    CATEGORY_TOTAL[$category]=$((CATEGORY_TOTAL[$category] + 1))

    # Verificar si está procesado (procesado != "")
    if [ "$processed" != "" ] && [ "$processed" != '""' ]; then
        ((PROCESSED++))
        CATEGORY_PROCESSED[$category]=$((CATEGORY_PROCESSED[$category] + 1))
    fi

done < "$MANIFEST_FILE"

# Calcular porcentaje
PERCENTAGE=$(awk "BEGIN {printf \"%.1f\", ($PROCESSED/$TOTAL)*100}")

# Mostrar resumen general
echo -e "${YELLOW}📊 Resumen General:${NC}"
echo -e "   Total de archivos: ${BLUE}$TOTAL${NC}"
echo -e "   Procesados: ${GREEN}$PROCESSED${NC}"
echo -e "   Pendientes: ${YELLOW}$((TOTAL - PROCESSED))${NC}"
echo -e "   Progreso: ${GREEN}$PERCENTAGE%${NC}"
echo ""

# Mostrar barra de progreso
BAR_LENGTH=50
FILLED=$((PROCESSED * BAR_LENGTH / TOTAL))
EMPTY=$((BAR_LENGTH - FILLED))

echo -e "${YELLOW}📈 Barra de Progreso:${NC}"
printf "   ["
printf "%${FILLED}s" | tr ' ' '█'
printf "%${EMPTY}s" | tr ' ' '░'
echo -e "] ${GREEN}${PERCENTAGE}%${NC}"
echo ""
echo ""

# Mostrar progreso por categoría
echo -e "${YELLOW}📋 Progreso por Categoría:${NC}"
echo ""

# Headers
printf "   %-35s %10s %10s %10s\n" "Categoría" "Total" "Procesado" "Porcentaje"
printf "   %-35s %10s %10s %10s\n" "-----------------------------------" "----------" "----------" "----------"

# Mostrar cada categoría
for category in "${!CATEGORY_TOTAL[@]}"; do
    cat_total=${CATEGORY_TOTAL[$category]}
    cat_processed=${CATEGORY_PROCESSED[$category]}
    cat_pending=$((cat_total - cat_processed))

    # Calcular porcentaje por categoría
    cat_percentage=$(awk "BEGIN {printf \"%.0f\", ($cat_processed/$cat_total)*100}")

    # Color según porcentaje
    if [ "$cat_percentage" -eq 100 ]; then
        color=$GREEN
    elif [ "$cat_percentage" -ge 50 ]; then
        color=$YELLOW
    else
        color=$RED
    fi

    printf "   %-35s ${BLUE}%10d${NC} ${color}%10d${NC} %10.0f%%\n" \
        "$category" \
        "$cat_total" \
        "$cat_processed" \
        "$cat_percentage"
done

echo ""

# Lista de archivos procesados
if [ $PROCESSED -gt 0 ]; then
    echo -e "${YELLOW}✅ Archivos Procesados:${NC}"
    echo ""

    # Leer nuevamente y mostrar procesados
    while IFS=',' read -r path category processed; do
        # Limpiar comillas
        path="${path%\"}"
        path="${path#\"}"
        category="${category%\"}"
        category="${category#\"}"
        processed="${processed%\"}"
        processed="${processed#\"}"

        # Saltar header
        if [[ "$path" == "Ruta Completa" ]]; then
            continue
        fi

        # Mostrar si está procesado
        if [ "$processed" != "" ] && [ "$processed" != '""' ]; then
            echo -e "   ${GREEN}✓${NC} $path"
        fi

    done < "$MANIFEST_FILE"

    echo ""
fi

# Lista de archivos pendientes (solo primeras 20)
if [ $((TOTAL - PROCESSED)) -gt 0 ]; then
    PENDING=$((TOTAL - PROCESSED))
    echo -e "${YELLOW}⏳ Archivos Pendientes${NC}"

    if [ "$PENDING" -gt 20 ]; then
        echo -e "   (Mostrando primeras 20 de $PENDING pendientes)"
    fi

    echo ""

    count=0
    while IFS=',' read -r path category processed; do
        # Limpiar comillas
        path="${path%\"}"
        path="${path#\"}"
        processed="${processed%\"}"
        processed="${processed#\"}"

        # Saltar header y procesados
        if [[ "$path" == "Ruta Completa" ]] || { [ "$processed" != "" ] && [ "$processed" != '""' ]; }; then
            continue
        fi

        # Mostrar archivo pendiente
        echo -e "   ${YELLOW}○${NC} $path"
        count=$((count + 1))

        # Limitar a 20
        if [ $count -ge 20 ]; then
            break
        fi

    done < "$MANIFEST_FILE"

    if [ "$PENDING" -gt 20 ]; then
        echo ""
        echo -e "   ... y ${BLUE}$((PENDING - 20))${NC} más"
    fi

    echo ""
fi

# Consejos
echo -e "${YELLOW}💡 Consejos:${NC}"
echo "   1. Abre CODE_MANIFEST.csv en Excel/Google Sheets para marcar archivos"
echo "   2. Ejecuta este script nuevamente para ver el progreso actualizado"
echo "   3. Usa filtros en Excel para organizar por categoría"
echo ""

# Footer
echo -e "${GREEN}✨ Reporte completado${NC}"
