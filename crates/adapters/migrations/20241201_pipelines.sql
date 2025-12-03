-- =====================================================
-- Pipelines and Pipeline Steps Migration
-- Creates tables for pipeline definitions and step configurations
-- Migration: 20241201_pipelines
-- =====================================================

-- Pipelines table
CREATE TABLE IF NOT EXISTS pipelines (
    pipeline_id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NULL,
    status TEXT NOT NULL,
    variables JSONB NOT NULL DEFAULT '{}'::jsonb,
    workflow_definition JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tenant_id TEXT NULL
);

-- Pipeline steps table
CREATE TABLE IF NOT EXISTS pipeline_steps (
    step_id UUID PRIMARY KEY,
    pipeline_id UUID NOT NULL REFERENCES pipelines(pipeline_id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    job_spec JSONB NOT NULL,
    timeout_ms BIGINT NOT NULL DEFAULT 300000,
    depends_on JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for faster pipeline name lookups
CREATE INDEX IF NOT EXISTS idx_pipelines_name
ON pipelines(name);

-- Index for pipeline status queries
CREATE INDEX IF NOT EXISTS idx_pipelines_status
ON pipelines(status);

-- Index for tenant-based queries
CREATE INDEX IF NOT EXISTS idx_pipelines_tenant_id
ON pipelines(tenant_id);

-- Index for pipeline steps by pipeline_id
CREATE INDEX IF NOT EXISTS idx_pipeline_steps_pipeline_id
ON pipeline_steps(pipeline_id);

-- Index for step order queries
CREATE INDEX IF NOT EXISTS idx_pipeline_steps_created_at
ON pipeline_steps(created_at);

COMMENT ON TABLE pipelines IS 'Stores pipeline definitions and metadata';
COMMENT ON COLUMN pipelines.pipeline_id IS 'Unique identifier for the pipeline';
COMMENT ON COLUMN pipelines.name IS 'Human-readable name for the pipeline';
COMMENT ON COLUMN pipelines.status IS 'Current status (DRAFT, ACTIVE, ARCHIVED)';
COMMENT ON COLUMN pipelines.variables IS 'JSON variables for pipeline execution';
COMMENT ON COLUMN pipelines.workflow_definition IS 'JSON definition of the workflow';

COMMENT ON TABLE pipeline_steps IS 'Stores individual steps within a pipeline';
COMMENT ON COLUMN pipeline_steps.step_id IS 'Unique identifier for the step';
COMMENT ON COLUMN pipeline_steps.pipeline_id IS 'Reference to parent pipeline';
COMMENT ON COLUMN pipeline_steps.name IS 'Human-readable name for the step';
COMMENT ON COLUMN pipeline_steps.job_spec IS 'JSON specification for jobs executed by this step';
COMMENT ON COLUMN pipeline_steps.timeout_ms IS 'Maximum time in milliseconds before step times out';
COMMENT ON COLUMN pipeline_steps.depends_on IS 'JSON array of step IDs this step depends on';
