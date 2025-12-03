#!/usr/bin/env python3
"""
Script para extraer puntos de mejora - versión mejorada.
"""

import re
import json
from collections import defaultdict

def extract_improvements(file_path: str):
    """Extrae puntos de mejora usando regex más flexible."""

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Buscar todas las líneas que parecen ser de la tabla
    # Patrón: | número | `archivo` | capa | tipo | descripción | reglas | invariantes | comentarios |
    pattern = r'^\|\s*(\d+)\s*\|\s*`([^`]+)`\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|\s*(.+?)\s*\|\s*$'

    improvements = {
        'mejoras': [],
        'code_smells': [],
        'sospechosos': [],
        'bugs': []
    }

    lines = content.split('\n')
    for line in lines:
        line = line.strip()
        if not line.startswith('|'):
            continue

        # Intentar con regex
        match = re.match(pattern, line)
        if match:
            num, file_path, layer, ddd_type, description, business_rules, invariants, comments = match.groups()

            # Limpiar los campos
            num = num.strip()
            file_path = file_path.strip()
            layer = layer.strip()
            ddd_type = ddd_type.strip()
            comments = comments.strip()

            # Buscar patrones en los comentarios
            # Mejora: (puede tener **negrita**)
            mejora_match = re.search(r'\*\*Mejora:\*\*\s*([^\.]+\.?)', comments)
            if not mejora_match:
                mejora_match = re.search(r'Mejora:\s*([^\.]+\.?)', comments)

            if mejora_match:
                mejora = mejora_match.group(1).strip()
                improvements['mejoras'].append({
                    'num': num,
                    'file': file_path,
                    'layer': layer,
                    'ddd_type': ddd_type,
                    'text': mejora,
                    'full_comment': comments
                })

            # Code Smell:
            smell_match = re.search(r'\*\*Code Smell:\*\*\s*([^\.]+\.?)', comments)
            if not smell_match:
                smell_match = re.search(r'Code Smell:\s*([^\.]+\.?)', comments)

            if smell_match:
                smell = smell_match.group(1).strip()
                improvements['code_smells'].append({
                    'num': num,
                    'file': file_path,
                    'layer': layer,
                    'ddd_type': ddd_type,
                    'text': smell,
                    'full_comment': comments
                })

            # Sospechoso:
            sospechoso_match = re.search(r'\*\*Sospechoso:\*\*\s*([^\.]+\.?)', comments)
            if not sospechoso_match:
                sospechoso_match = re.search(r'Sospechoso:\s*([^\.]+\.?)', comments)

            if sospechoso_match:
                sospechoso = sospechoso_match.group(1).strip()
                improvements['sospechosos'].append({
                    'num': num,
                    'file': file_path,
                    'layer': layer,
                    'ddd_type': ddd_type,
                    'text': sospechoso,
                    'full_comment': comments
                })

            # Buscar bugs críticos
            if 'BUG' in comments.upper() or 'Crítico' in comments or 'bug' in comments.lower():
                improvements['bugs'].append({
                    'num': num,
                    'file': file_path,
                    'layer': layer,
                    'ddd_type': ddd_type,
                    'text': 'BUG identificado',
                    'full_comment': comments
                })

    return improvements

def main():
    """Función principal."""

    file_path = '/home/rubentxu/Proyectos/rust/hodei-jobs/docs/analysis/ddd_tactical_analysis.md'

    print("Extrayendo puntos de mejora (v2)...")
    improvements = extract_improvements(file_path)

    print(f"\n=== RESULTADOS ===")
    print(f"Mejoras encontradas: {len(improvements['mejoras'])}")
    print(f"Code smells encontrados: {len(improvements['code_smells'])}")
    print(f"Elementos sospechosos: {len(improvements['sospechosos'])}")
    print(f"BUGs identificados: {len(improvements['bugs'])}")
    total = (len(improvements['mejoras']) + len(improvements['code_smells']) +
             len(improvements['sospechosos']) + len(improvements['bugs']))
    print(f"Total: {total}")

    # Mostrar algunos ejemplos
    if improvements['mejoras']:
        print(f"\n=== EJEMPLOS DE MEJORAS ===")
        for i, mejora in enumerate(improvements['mejoras'][:3]):
            print(f"{i+1}. [{mejora['num']}] {mejora['file']}")
            print(f"   Mejora: {mejora['text'][:80]}...")
            print()

    if improvements['code_smells']:
        print(f"\n=== EJEMPLOS DE CODE SMELLS ===")
        for i, smell in enumerate(improvements['code_smells'][:3]):
            print(f"{i+1}. [{smell['num']}] {smell['file']}")
            print(f"   Smell: {smell['text'][:80]}...")
            print()

    if improvements['bugs']:
        print(f"\n=== BUGS CRÍTICOS ===")
        for i, bug in enumerate(improvements['bugs']):
            print(f"{i+1}. [{bug['num']}] {bug['file']}")
            print(f"   Comentario: {bug['full_comment'][:100]}...")
            print()

    # Guardar resultados
    with open('/home/rubentxu/Proyectos/rust/hodei-jobs/improvements_extracted.json', 'w', encoding='utf-8') as f:
        json.dump(improvements, f, indent=2, ensure_ascii=False)

    print(f"Resultados guardados en: improvements_extracted.json")

if __name__ == '__main__':
    main()