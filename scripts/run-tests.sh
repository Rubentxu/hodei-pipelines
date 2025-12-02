#!/bin/bash
# Wrapper para ejecutar tests con cleanup automático

echo "🧪 Iniciando tests con cleanup automático..."
echo "================================================"
echo ""

# Capturar señales para cleanup
trap cleanup SIGINT SIGTERM EXIT

# Función de cleanup
cleanup() {
    echo ""
    echo "🧹 Limpiando recursos automáticamente..."

    # Eliminar contenedores PostgreSQL
    PG_CONTAINERS=$(docker ps -q --filter ancestor=postgres:16-alpine 2>/dev/null || true)
    if [ -n "$PG_CONTAINERS" ]; then
        echo "   📦 Eliminando contenedores PostgreSQL..."
        docker rm -f $PG_CONTAINERS 2>/dev/null || true
    fi

    # Eliminar contenedores ryuk
    RYUK_CONTAINERS=$(docker ps -q --filter name=ryuk 2>/dev/null || true)
    if [ -n "$RYUK_CONTAINERS" ]; then
        echo "   🧹 Eliminando contenedores Ryuk..."
        docker rm -f $RYUK_CONTAINERS 2>/dev/null || true
    fi

    # Limpiar contenedores no utilizados
    docker container prune -f 2>/dev/null || true

    echo "   ✅ Cleanup completado"
    echo ""
    echo "💾 Memoria actual:"
    free -h | grep "Mem:"
}

# Ejecutar tests
cargo test "$@"

# Guardar código de salida
TEST_EXIT_CODE=$?

# Cleanup solo si el test terminó normalmente (no Ctrl+C)
if [ $TEST_EXIT_CODE -ne 130 ]; then
    cleanup
fi

exit $TEST_EXIT_CODE
