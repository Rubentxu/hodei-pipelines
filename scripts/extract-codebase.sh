#!/bin/bash

# =============================================================================
# Extract Codebase Script
# =============================================================================
# Description: Extracts all source code from 'crates' and 'server' directories
#              into a single consolidated text file for analysis.
#
# Usage: ./scripts/extract-codebase.sh [output_file]
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
#
# =============================================================================

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_FILE="${1:-${PROJECT_ROOT}/codebase_complete.txt}"
TEMP_DIR="/tmp/code_extract_$$"

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
    find "$dir" -type f \( -name "*.rs" -o -name "*.toml" -o -name "*.md" \) | sort | while read -r file; do
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
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Create output file with header
    echo "=== EXTRACCIÓN DE CÓDIGO: crates y server ===" > "$OUTPUT_FILE"
    echo "Fecha: $(date)" >> "$OUTPUT_FILE"
    echo "Proyecto: Hodei Jobs" >> "$OUTPUT_FILE"
    echo "Directorio raíz: $(pwd)" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    
    # Process directories
    process_files "crates" "crates"
    process_files "server" "server"
    
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
