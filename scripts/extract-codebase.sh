#!/bin/bash

# =============================================================================
# Extract Codebase Script
# =============================================================================
# Description: Extracts all source code from 'crates' and 'server' directories
#              into a single consolidated text file for analysis.
#              Excludes test files and test directories.
#
# Usage: ./scripts/extract-codebase.sh [opciones] [output_file]
#
# Opciones:
#   --ports-adapters, -pa    Solo procesar crates de adapters y ports (Infraestructura)
#
# Arguments:
#   output_file    (optional) Path to the output file
#                  Default: ./codebase_complete.txt
#
# Output Format:
#   - Each file is preceded by a header with relative and absolute paths
#   - Files are separated by blank lines
#   - Only includes .rs, .toml, and .md files
#
# Example:
#   ./scripts/extract-codebase.sh
#   ./scripts/extract-codebase.sh /tmp/my_codebase.txt
#   ./scripts/extract-codebase.sh --ports-adapters
#   ./scripts/extract-codebase.sh -pa infra.txt
#
# =============================================================================

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PORTS_ADAPTERS_ONLY=0
OUTPUT_FILE=""
TEMP_DIR="/tmp/code_extract_$$"

# Parseo de opciones
while [[ $# -gt 0 ]]; do
    case $1 in
        --ports-adapters|-pa)
            PORTS_ADAPTERS_ONLY=1
            shift
            ;;
        -h|--help)
            echo "Uso: $0 [opciones] [output_file]"
            echo ""
            echo "Opciones:"
            echo "  --ports-adapters, -pa    Solo procesar crates de adapters y ports (Infraestructura)"
            echo ""
            echo "Ejemplos:"
            echo "  $0                       # Extraer todo el codebase"
            echo "  $0 output.txt            # Extraer a archivo personalizado"
            echo "  $0 --ports-adapters      # Solo adapters y ports"
            echo "  $0 -pa infra.txt         # Solo adapters y ports, archivo personalizado"
            exit 0
            ;;
        *)
            OUTPUT_FILE="$1"
            shift
            ;;
    esac
done

# Parámetros por defecto
OUTPUT_FILE="${OUTPUT_FILE:-${PROJECT_ROOT}/codebase_complete.txt}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored messages
info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to process files
process_files() {
    local dir=$1
    local prefix=$2
    
    if [ ! -d "$dir" ]; then
        warn "Directory not found: $dir"
        return
    fi
    
    info "Processing directory: $dir"
    
    # Find all .rs, .toml, and .md files, sort them, and process
    # Exclude test files and test directories
    find "$dir" -type f \( -name "*.rs" -o -name "*.toml" -o -name "*.md" \) \
        -not -path "*/tests/*" \
        -not -path "*/test/*" \
        -not -name "*_test.rs" \
        -not -name "*_tests.rs" \
        -not -name "functional_tests.rs" \
        -not -name "integration_tests.rs" \
        | sort | while read -r file; do
        # Calculate relative path
        rel_path="${file#./}"
        
        # Write header
        echo "================================================" >> "$OUTPUT_FILE"
        echo "Archivo: $rel_path" >> "$OUTPUT_FILE"
        echo "Ruta completa: $(pwd)/$rel_path" >> "$OUTPUT_FILE"
        echo "================================================" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        
        # Write file content
        cat "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    done
}

# Main execution
main() {
    info "Starting codebase extraction..."
    info "Output file: $OUTPUT_FILE"

    if [ $PORTS_ADAPTERS_ONLY -eq 1 ]; then
        info "Mode: Only adapters and ports crates"
    fi

    # Change to project root
    cd "$PROJECT_ROOT"

    # Create output file with header
    if [ $PORTS_ADAPTERS_ONLY -eq 1 ]; then
        echo "=== EXTRACCIÓN DE CÓDIGO: Solo adapters y ports ===" > "$OUTPUT_FILE"
    else
        echo "=== EXTRACCIÓN DE CÓDIGO: crates y server ===" > "$OUTPUT_FILE"
    fi
    echo "Fecha: $(date)" >> "$OUTPUT_FILE"
    echo "Proyecto: Hodei Jobs" >> "$OUTPUT_FILE"
    echo "Directorio raíz: $(pwd)" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    # Process directories
    if [ $PORTS_ADAPTERS_ONLY -eq 1 ]; then
        # Solo adapters y ports
        process_files "crates/adapters" "adapters"
        process_files "crates/ports" "ports"
    else
        # Todo el codebase
        process_files "crates" "crates"
        process_files "server" "server"
    fi
    
    # Generate statistics
    local total_files=$(grep -c "^Archivo:" "$OUTPUT_FILE" 2>/dev/null || echo "0")
    local total_lines=$(wc -l < "$OUTPUT_FILE" 2>/dev/null || echo "0")
    local file_size=$(du -h "$OUTPUT_FILE" 2>/dev/null | cut -f1 || echo "unknown")
    
    # Cleanup
    rm -rf "$TEMP_DIR"
    
    # Print summary
    info "Extraction complete!"
    info "Output file: $OUTPUT_FILE"
    info "Total files: $total_files"
    info "Total lines: $total_lines"
    info "File size: $file_size"
    info ""
    info "File types included:"
    info "  - Rust files (.rs): $(grep -c '^Archivo:.*\.rs$' "$OUTPUT_FILE" 2>/dev/null || echo "0")"
    info "  - Config files (.toml): $(grep -c '^Archivo:.*\.toml$' "$OUTPUT_FILE" 2>/dev/null || echo "0")"
    info "  - Markdown files (.md): $(grep -c '^Archivo:.*\.md$' "$OUTPUT_FILE" 2>/dev/null || echo "0")"
}

# Run main function
main
