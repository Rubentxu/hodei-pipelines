-- ========================================
-- Script de inicialización de PostgreSQL
-- Crear extensiones y usuario para Hodei Pipelines
-- ========================================

-- Crear extensión para UUID
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Crear extensión para fechas/horarios
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Crear schema para la aplicación
CREATE SCHEMA IF NOT EXISTS hodei;

-- Configurar timezone por defecto
SET timezone = 'UTC';

-- Crear índices para mejor performance
-- (Estos índices se crearán cuando las tablas sean creadas por la aplicación)

-- Mensaje de confirmación
DO $$
BEGIN
    RAISE NOTICE 'Base de datos inicializada correctamente';
    RAISE NOTICE 'Extensions: uuid-ossp, pgcrypto';
    RAISE NOTICE 'Schema: hodei';
    RAISE NOTICE 'Timezone: UTC';
END $$;
