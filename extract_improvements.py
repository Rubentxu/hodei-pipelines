#!/usr/bin/env python3
"""
Script simplificado para extraer puntos de mejora del análisis DDD.
"""

import re
import json
from collections import defaultdict

def extract_all_improvements(file_path: str):
    """Extrae todos los puntos de mejora usando un enfoque más simple."""

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Buscar todas las líneas de la tabla
    # El patrón busca líneas que comienzan con | número | `archivo` |
    table_lines = []
    for line in content.split('\n'):
        line = line.strip()
        if line.startswith('|') and '`' in line and '|' in line:
            table_lines.append(line)

    print(f"Encontradas {len(table_lines)} líneas de tabla")

    improvements = {
        'mejoras': [],
        'code_smells': [],
        'sospechosos': [],
        'otros': []
    }

    for line in table_lines:
        # Extraer la última columna (Comments)
        parts = line.split('|')
        if len(parts) >= 8:
            comments = parts[7].strip()

            # Extraer número y archivo
            num_match = re.search(r'^\| (\d+) \|', line)
            file_match = re.search(r'`([^`]+)`', line)

            num = num_match.group(1) if num_match else '0'
            file_path = file_match.group(1) if file_match else ''

            # Extraer capa y tipo DDD
            layer = parts[2].strip() if len(parts) > 2 else ''
            ddd_type = parts[3].strip() if len(parts) > 3 else ''

            # Buscar Mejora:
            mejora_match = re.search(r'Mejora: ([^\.]+\.?)', comments)
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

            # Buscar Code Smell:
            smell_match = re.search(r'Code Smell: ([^\.]+\.?)', comments)
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

            # Buscar Sospechoso:
            sospechoso_match = re.search(r'Sospechoso: ([^\.]+\.?)', comments)
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

    return improvements

def categorize_improvements(improvements):
    """Categoriza los puntos de mejora."""

    categories = defaultdict(list)

    # Categorizar mejoras
    for item in improvements['mejoras']:
        text = item['text'].lower() + ' ' + item['full_comment'].lower()

        if any(word in text for word in ['value object', 'entity', 'aggregate', 'domain', 'ddd']):
            categories['ddd_tactico'].append(item)
        elif any(word in text for word in ['arquitectura', 'capa', 'layer', 'separación', 'hexagonal']):
            categories['arquitectura'].append(item)
        elif any(word in text for word in ['seguridad', 'vulnerabilidad', 'autenticación']):
            categories['seguridad'].append(item)
        elif any(word in text for word in ['test', 'testing', 'coverage']):
            categories['testing'].append(item)
        elif any(word in text for word in ['performance', 'rendimiento', 'cache', 'memory']):
            categories['performance'].append(item)
        elif any(word in text for word in ['documentación', 'comentario', 'explicación']):
            categories['documentacion'].append(item)
        else:
            categories['calidad_codigo'].append(item)

    # Categorizar code smells
    for item in improvements['code_smells']:
        text = item['text'].lower() + ' ' + item['full_comment'].lower()

        if any(word in text for word in ['hardcoded', 'mock', 'placeholder']):
            categories['implementacion_mock'].append(item)
        elif any(word in text for word in ['unwrap', 'error', 'panic']):
            categories['manejo_errores'].append(item)
        elif any(word in text for word in ['duplicación', 'repetitivo']):
            categories['duplicacion'].append(item)
        elif any(word in text for word in ['acoplamiento', 'dependencia']):
            categories['acoplamiento'].append(item)
        else:
            categories['code_smell_general'].append(item)

    # Categorizar sospechosos
    for item in improvements['sospechosos']:
        text = item['text'].lower() + ' ' + item['full_comment'].lower()

        if any(word in text for word in ['value object', 'immutable', 'igualdad']):
            categories['ddd_tactico'].append(item)
        elif any(word in text for word in ['entity', 'aggregate']):
            categories['ddd_tactico'].append(item)
        else:
            categories['sospechoso_general'].append(item)

    return categories

def generate_report(improvements, categories):
    """Genera un reporte completo."""

    report = {
        'resumen': {
            'total_mejoras': len(improvements['mejoras']),
            'total_code_smells': len(improvements['code_smells']),
            'total_sospechosos': len(improvements['sospechosos']),
            'total_general': len(improvements['mejoras']) + len(improvements['code_smells']) + len(improvements['sospechosos'])
        },
        'categorias': {},
        'prioridades': {
            'alta': [],
            'media': [],
            'baja': []
        }
    }

    # Contar por categoría
    for category, items in categories.items():
        report['categorias'][category] = len(items)

    # Determinar prioridades
    all_items = []
    for category_items in categories.values():
        all_items.extend(category_items)

    for item in all_items:
        text = item['text'].lower() + ' ' + item.get('full_comment', '').lower()

        # Alta prioridad
        if any(word in text for word in ['bug', 'crítico', 'security', 'seguridad', 'vulnerabilidad',
                                        'production', 'unwrap', 'panic', 'hardcoded production']):
            report['prioridades']['alta'].append(item)
        # Media prioridad
        elif any(word in text for word in ['performance', 'rendimiento', 'cache', 'memory',
                                          'duplicación', 'acoplamiento', 'mock data']):
            report['prioridades']['media'].append(item)
        # Baja prioridad
        else:
            report['prioridades']['baja'].append(item)

    return report

def main():
    """Función principal."""

    file_path = '/home/rubentxu/Proyectos/rust/hodei-jobs/docs/analysis/ddd_tactical_analysis.md'

    print("Extrayendo puntos de mejora...")
    improvements = extract_all_improvements(file_path)

    print(f"\n=== RESULTADOS ===")
    print(f"Mejoras encontradas: {len(improvements['mejoras'])}")
    print(f"Code smells encontrados: {len(improvements['code_smells'])}")
    print(f"Elementos sospechosos: {len(improvements['sospechosos'])}")
    print(f"Total: {len(improvements['mejoras']) + len(improvements['code_smells']) + len(improvements['sospechosos'])}")

    print("\nCategorizando...")
    categories = categorize_improvements(improvements)

    print("\n=== DISTRIBUCIÓN POR CATEGORÍA ===")
    for category, items in categories.items():
        print(f"{category}: {len(items)} items")

    print("\nGenerando reporte...")
    report = generate_report(improvements, categories)

    # Guardar resultados
    output = {
        'improvements': improvements,
        'categories': dict(categories),
        'report': report
    }

    with open('/home/rubentxu/Proyectos/rust/hodei-jobs/technical_debt_analysis.json', 'w', encoding='utf-8') as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n=== RESUMEN FINAL ===")
    print(f"Total puntos identificados: {report['resumen']['total_general']}")
    print(f"Prioridad Alta: {len(report['prioridades']['alta'])}")
    print(f"Prioridad Media: {len(report['prioridades']['media'])}")
    print(f"Prioridad Baja: {len(report['prioridades']['baja'])}")

    print(f"\nResultados guardados en: technical_debt_analysis.json")

    # Mostrar algunos ejemplos de alta prioridad
    print(f"\n=== EJEMPLOS DE ALTA PRIORIDAD ===")
    for i, item in enumerate(report['prioridades']['alta'][:5]):
        print(f"{i+1}. [{item.get('num', '?')}] {item.get('file', '')}")
        print(f"   Tipo: {item.get('ddd_type', '')}")
        print(f"   Issue: {item.get('text', '')[:100]}...")
        print()

if __name__ == '__main__':
    main()