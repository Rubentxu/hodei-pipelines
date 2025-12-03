#!/usr/bin/env python3
"""
Script para analizar y categorizar puntos de mejora del análisis DDD.
Extrae todos los comentarios de mejora, code smells y elementos sospechosos.
"""

import re
from collections import defaultdict, Counter
from typing import List, Dict, Tuple
import json

def extract_improvements(file_path: str) -> Dict[str, List[Dict]]:
    """Extrae todos los puntos de mejora del documento de análisis."""

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Patrón para extraer líneas de la tabla
    table_pattern = r'^\| (\d+) \| `([^`]+)` \| ([^|]+) \| ([^|]+) \| ([^|]+) \| ([^|]+) \| ([^|]+) \| ([^|]+) \|$'

    improvements = {
        'mejoras': [],
        'code_smells': [],
        'sospechosos': [],
        'categorias': defaultdict(list)
    }

    lines = content.split('\n')
    for line in lines:
        match = re.match(table_pattern, line.strip())
        if match:
            num, file_path, layer, ddd_type, description, business_rules, invariants, comments = match.groups()

            # Extraer puntos de mejora de los comentarios
            if 'Mejora:' in comments:
                mejora_match = re.search(r'Mejora: ([^\.]+\.?)', comments)
                if mejora_match:
                    mejora = mejora_match.group(1).strip()
                    improvements['mejoras'].append({
                        'num': int(num),
                        'file': file_path,
                        'layer': layer.strip(),
                        'ddd_type': ddd_type.strip(),
                        'mejora': mejora,
                        'full_comment': comments
                    })

            # Extraer code smells
            if 'Code Smell:' in comments:
                smell_match = re.search(r'Code Smell: ([^\.]+\.?)', comments)
                if smell_match:
                    smell = smell_match.group(1).strip()
                    improvements['code_smells'].append({
                        'num': int(num),
                        'file': file_path,
                        'layer': layer.strip(),
                        'ddd_type': ddd_type.strip(),
                        'smell': smell,
                        'full_comment': comments
                    })

            # Extraer elementos sospechosos
            if 'Sospechoso:' in comments:
                sospechoso_match = re.search(r'Sospechoso: ([^\.]+\.?)', comments)
                if sospechoso_match:
                    sospechoso = sospechoso_match.group(1).strip()
                    improvements['sospechosos'].append({
                        'num': int(num),
                        'file': file_path,
                        'layer': layer.strip(),
                        'ddd_type': ddd_type.strip(),
                        'sospechoso': sospechoso,
                        'full_comment': comments
                    })

    return improvements

def categorize_improvements(improvements: Dict) -> Dict[str, List]:
    """Categoriza los puntos de mejora por tipo y prioridad."""

    categories = {
        'arquitectura': [],
        'ddd_tactico': [],
        'calidad_codigo': [],
        'performance': [],
        'seguridad': [],
        'testing': [],
        'documentacion': [],
        'infraestructura': []
    }

    # Palabras clave para cada categoría
    keywords = {
        'arquitectura': ['arquitectura', 'capa', 'layer', 'separación', 'acoplamiento', 'responsabilidad',
                        'hexagonal', 'ports', 'adapters', 'dependencia', 'inyección'],
        'ddd_tactico': ['value object', 'entity', 'aggregate', 'domain service', 'repository',
                       'factory', 'immutable', 'invariante', 'validación', 'regla negocio'],
        'calidad_codigo': ['duplicación', 'hardcoded', 'mock', 'placeholder', 'unwrap', 'error handling',
                          'refactor', 'complexidad', 'smell', 'patrón'],
        'performance': ['cache', 'memoización', 'optimización', 'rendimiento', 'eficiencia', 'memory'],
        'seguridad': ['seguridad', 'vulnerabilidad', 'autenticación', 'autorización', 'RBAC', 'validación'],
        'testing': ['test', 'coverage', 'unit test', 'integration test', 'mock', 'fixture'],
        'documentacion': ['documentación', 'comentario', 'explicación', 'README', 'docstring'],
        'infraestructura': ['database', 'postgresql', 'nats', 'prometheus', 'grafana', 'configuración']
    }

    # Categorizar mejoras
    for mejora in improvements['mejoras']:
        text = mejora['mejora'].lower() + ' ' + mejora['full_comment'].lower()
        assigned = False

        for category, cat_keywords in keywords.items():
            for keyword in cat_keywords:
                if keyword in text:
                    categories[category].append({
                        'type': 'mejora',
                        'data': mejora,
                        'priority': determine_priority(mejora)
                    })
                    assigned = True
                    break
            if assigned:
                break

        if not assigned:
            categories['calidad_codigo'].append({
                'type': 'mejora',
                'data': mejora,
                'priority': determine_priority(mejora)
            })

    # Categorizar code smells
    for smell in improvements['code_smells']:
        text = smell['smell'].lower() + ' ' + smell['full_comment'].lower()
        assigned = False

        for category, cat_keywords in keywords.items():
            for keyword in cat_keywords:
                if keyword in text:
                    categories[category].append({
                        'type': 'code_smell',
                        'data': smell,
                        'priority': determine_priority(smell)
                    })
                    assigned = True
                    break
            if assigned:
                break

        if not assigned:
            categories['calidad_codigo'].append({
                'type': 'code_smell',
                'data': smell,
                'priority': determine_priority(smell)
            })

    # Categorizar sospechosos
    for sospechoso in improvements['sospechosos']:
        text = sospechoso['sospechoso'].lower() + ' ' + sospechoso['full_comment'].lower()
        assigned = False

        for category, cat_keywords in keywords.items():
            for keyword in cat_keywords:
                if keyword in text:
                    categories[category].append({
                        'type': 'sospechoso',
                        'data': sospechoso,
                        'priority': determine_priority(sospechoso)
                    })
                    assigned = True
                    break
            if assigned:
                break

        if not assigned:
            categories['ddd_tactico'].append({
                'type': 'sospechoso',
                'data': sospechoso,
                'priority': determine_priority(sospechoso)
            })

    return categories

def determine_priority(item: Dict) -> str:
    """Determina la prioridad basada en el contenido."""
    text = (item.get('mejora', '') + item.get('smell', '') + item.get('sospechoso', '')).lower()
    full_comment = item.get('full_comment', '').lower()

    # Palabras clave de alta prioridad
    high_priority = ['security', 'seguridad', 'vulnerabilidad', 'error handling', 'unwrap',
                    'hardcoded', 'mock data', 'production', 'violación', 'acoplamiento']

    # Palabras clave de media prioridad
    medium_priority = ['performance', 'rendimiento', 'cache', 'duplicación', 'refactor',
                      'validación', 'immutable', 'value object', 'entity']

    for keyword in high_priority:
        if keyword in text or keyword in full_comment:
            return 'alta'

    for keyword in medium_priority:
        if keyword in text or keyword in full_comment:
            return 'media'

    return 'baja'

def generate_statistics(improvements: Dict, categories: Dict) -> Dict:
    """Genera estadísticas del análisis."""

    stats = {
        'totales': {
            'mejoras': len(improvements['mejoras']),
            'code_smells': len(improvements['code_smells']),
            'sospechosos': len(improvements['sospechosos']),
            'total': len(improvements['mejoras']) + len(improvements['code_smells']) + len(improvements['sospechosos'])
        },
        'por_categoria': {},
        'por_prioridad': defaultdict(int),
        'por_capa': defaultdict(lambda: defaultdict(int))
    }

    # Estadísticas por categoría
    for category, items in categories.items():
        stats['por_categoria'][category] = len(items)
        for item in items:
            stats['por_prioridad'][item['priority']] += 1

            # Estadísticas por capa
            layer = item['data']['layer']
            stats['por_capa'][layer][category] = stats['por_capa'][layer].get(category, 0) + 1

    return stats

def main():
    """Función principal."""

    analysis_file = '/home/rubentxu/Proyectos/rust/hodei-jobs/docs/analysis/ddd_tactical_analysis.md'

    print("Extrayendo puntos de mejora del análisis DDD...")
    improvements = extract_improvements(analysis_file)

    print(f"Mejoras encontradas: {len(improvements['mejoras'])}")
    print(f"Code smells encontrados: {len(improvements['code_smells'])}")
    print(f"Elementos sospechosos: {len(improvements['sospechosos'])}")
    print(f"Total: {len(improvements['mejoras']) + len(improvements['code_smells']) + len(improvements['sospechosos'])}")

    print("\nCategorizando puntos de mejora...")
    categories = categorize_improvements(improvements)

    print("\nGenerando estadísticas...")
    stats = generate_statistics(improvements, categories)

    # Guardar resultados
    output = {
        'improvements': improvements,
        'categories': categories,
        'statistics': stats
    }

    with open('/home/rubentxu/Proyectos/rust/hodei-jobs/improvements_analysis.json', 'w', encoding='utf-8') as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print("\n=== RESUMEN DE ANÁLISIS ===")
    print(f"Total de puntos identificados: {stats['totales']['total']}")
    print(f"- Mejoras: {stats['totales']['mejoras']}")
    print(f"- Code Smells: {stats['totales']['code_smells']}")
    print(f"- Elementos sospechosos: {stats['totales']['sospechosos']}")

    print("\nDistribución por categoría:")
    for category, count in stats['por_categoria'].items():
        print(f"  - {category}: {count}")

    print("\nDistribución por prioridad:")
    for priority, count in stats['por_prioridad'].items():
        print(f"  - {priority}: {count}")

    print(f"\nResultados guardados en: improvements_analysis.json")

if __name__ == '__main__':
    main()