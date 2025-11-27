# Scripts de Generación de Manifiesto de Código

## 📋 Descripción

Este directorio contiene scripts para generar un manifiesto de todos los archivos Rust (.rs) en el proyecto, útil para trackear qué archivos han sido analizados, procesados o revisados.

## 🚀 Scripts Disponibles

### 1. generate_clean_manifest.sh

**Script Bash - RECOMENDADO**

Genera un archivo CSV limpio con todos los archivos .rs del proyecto, excluyendo directorios irrelevantes como `target/`, `.git/`, etc.

#### Características:
- ✅ Sin dependencias externas (solo bash)
- ✅ Rápido y eficiente
- ✅ Excluye automáticamente: target/, .git/, node_modules/, .idea/, .vscode/
- ✅ Categoriza automáticamente los archivos
- ✅ Incluye estadísticas de tamaño y fechas
- ✅ Salida: `CODE_MANIFEST.csv` en la raíz del proyecto

#### Uso:
```bash
# Generar CODE_MANIFEST.csv
./scripts/generate_clean_manifest.sh

# Ver el archivo generado
cat CODE_MANIFEST.csv

# En Excel: abrir CODE_MANIFEST.csv
# Marcar la columna 'Procesado' según avances
```

#### Categorías Generadas:
- **Core Domain** - Archivos del dominio central (crates/core/)
- **Modules (Application)** - Capa de aplicación (crates/modules/)
- **Adapters (Infrastructure)** - Adapters (crates/adapters/)
- **Ports (Interfaces)** - Interfaces (crates/ports/)
- **Server** - Servidor (server/)
- **HWP Agent** - HWP Agent (crates/hwp-agent/)
- **HWP Proto** - Protocol buffers
- **E2E Tests** - Tests end-to-end
- **Tests** - Tests unitarios/integración
- **Examples** - Ejemplos
- **Build scripts** - Scripts de construcción
- **Otros** - Otros archivos

### 2. check_progress.sh

**Script de Verificación de Progreso**

Lee el archivo CODE_MANIFEST.csv y genera un reporte visual del progreso de análisis.

#### Uso:
```bash
# Verificar progreso actual
./scripts/check_progress.sh

# Ver progreso de un archivo específico
./scripts/check_progress.sh mi_manifiesto.csv
```

#### Salida:
- Resumen general (total, procesados, porcentaje)
- Barra de progreso visual
- Progreso por categoría
- Lista de archivos procesados
- Lista de archivos pendientes

## 📊 Formato del CODE_MANIFEST.csv

El archivo CSV generado tiene las siguientes columnas:

| # | Columna | Descripción |
|---|---------|-------------|
| 1 | **Ruta Completa** | Ruta relativa del archivo desde la raíz |
| 2 | **Nombre Archivo** | Nombre del archivo .rs |
| 3 | **Categoría** | Categoría del archivo según su ubicación |
| 4 | **Tamaño (KB)** | Tamaño del archivo en kilobytes |
| 5 | **Última Modificación** | Fecha y hora de última modificación |
| 6 | **Procesado** | Campo vacío para marcar si fue procesado |

### Ejemplo de CSV:
```csv
Ruta Completa,Nombre Archivo,Categoria,Tamaño (KB),Ultima Modificacion,Procesado
"crates/core/src/job.rs","job.rs","Core Domain","15","2025-11-27 10:30:15",""
"crates/modules/src/orchestrator.rs","orchestrator.rs","Modules (Application)","8","2025-11-27 09:15:42",""
...
```

## 💡 Flujo de Trabajo Típico

### Paso 1: Generar Manifiesto
```bash
cd /home/rubentxu/Proyectos/rust/hodei-jobs
./scripts/generate_clean_manifest.sh
```

### Paso 2: Abrir en Excel/Google Sheets
```bash
# En Linux
libreoffice CODE_MANIFEST.csv

# En macOS
open -a "Microsoft Excel" CODE_MANIFEST.csv

# O simplemente abrir con Google Sheets
```

### Paso 3: Procesar Archivos
1. Abre CODE_MANIFEST.csv en Excel
2. Habilita filtros: Datos → Filtro automático
3. Marca la columna "Procesado" según avances:
   - Escribir "✓" o "TRUE" o cualquier texto
   - O usar checkboxes (en Excel .xlsx)
4. Usa filtros por categoría para organizar el trabajo

### Paso 4: Verificar Progreso
```bash
# Ver progreso actual
./scripts/check_progress.sh
```

### Paso 5: Repetir
```bash
# Regenerar manifiesto (si se agregaron nuevos archivos)
./scripts/generate_clean_manifest.sh
```

## 🎯 Casos de Uso

### Análisis DDD Táctico:
```bash
./scripts/generate_clean_manifest.sh
# Abre CODE_MANIFEST.csv
# Marca archivos conforme los analizas
# Verifica progreso con check_progress.sh
```

### Auditoría de Código:
```bash
# Generar con nombre específico
./scripts/generate_clean_manifest.sh auditoria_$(date +%Y%m%d).csv
```

### Revisión por Pares:
```bash
# Generar manifiesto
./scripts/generate_clean_manifest.sh

# Compartir CODE_MANIFEST.csv con el equipo
# Cada miembro marca los archivos que revisó

# Verificar progreso
./scripts/check_progress.sh
```

## 📈 Ejemplo de Reporte de Progreso

```
============================================================
Reporte de Progreso del Manifiesto de Código
============================================================

Archivo: /home/rubentxu/Proyectos/rust/hodei-jobs/CODE_MANIFEST.csv

📊 Resumen General:
   Total de archivos: 180
   Procesados: 45
   Pendientes: 135
   Progreso: 25.0%

📈 Barra de Progreso:
   [█████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 25.0%

📋 Progreso por Categoría:
   Categoría                          Total   Procesado   Porcentaje
   ----------------------------------  ------  ----------  ----------
   Core Domain                               26           8         31%
   Modules (Application)                     28           7         25%
   Adapters (Infrastructure)                 28           6         21%
   Server                                    33           5         15%
   HWP Agent - src/                          20           3         15%
   ...
```

## 🛠️ Instalación y Requisitos

### Requisitos Mínimos:
- Bash 4.0+
- Linux/macOS/Windows (con WSL/Git Bash)

### Sin dependencias adicionales:
```bash
# No necesitas instalar nada más
./scripts/generate_clean_manifest.sh
```

## 📝 Formato de Marcar Archivos

### En CSV (Excel/LibreOffice):
```csv
Procesado
""          # No procesado
"✓"         # Procesado
"TRUE"      # Procesado
"DONE"      # Procesado
```

### Filtros Útiles en Excel:
- Mostrar solo procesados: Filtro → "Procesado" ≠ ""
- Mostrar solo pendientes: Filtro → "Procesado" = ""
- Por categoría: Filtro → "Categoría" = "Core Domain"

## 🔧 Personalización

### Modificar categorías:
```bash
# Editar generate_clean_manifest.sh
# Buscar la sección "Categorizar" y modificar
```

### Agregar exclusiones:
```bash
# En generate_clean_manifest.sh, línea ~55
# Agregar más exclusiones:
-not -path "*/generated/*" \
-not -path "*/vendor/*" \
```

## 🐛 Solución de Problemas

### Error: "Permission denied"
```bash
# Hacer el script ejecutable
chmod +x scripts/generate_clean_manifest.sh
chmod +x scripts/check_progress.sh
```

### No se encuentra el archivo CSV
```bash
# Regenerar manifiesto
./scripts/generate_clean_manifest.sh
```

### Ver progreso con archivo personalizado
```bash
./scripts/check_progress.sh mi_manifiesto_personalizado.csv
```

## 📚 Referencias

- CSV format: [RFC 4180](https://tools.ietf.org/html/rfc4180)
- Excel filtering: [Microsoft Excel Help](https://support.microsoft.com/excel)

---

## 📋 Resumen Rápido

```bash
# 1. Generar manifiesto
./scripts/generate_clean_manifest.sh

# 2. Abrir en Excel y marcar archivos procesados
# (columna "Procesado" al final)

# 3. Verificar progreso
./scripts/check_progress.sh

# 4. Repetir según sea necesario
```

**Archivos principales:**
- `generate_clean_manifest.sh` - Generar CSV
- `check_progress.sh` - Verificar progreso
- `CODE_MANIFEST.csv` - Manifiesto generado

---

**Generado:** 2025-11-27
**Scripts:** generate_clean_manifest.sh, check_progress.sh
**Compatibilidad:** Linux, macOS, Windows (WSL/Git Bash)
