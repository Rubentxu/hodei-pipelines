-- =====================================================
-- Pipeline Executions and Step Executions Migration
-- Creates tables for tracking pipeline execution history
-- Migration: 20241201_pipeline_executions
-- =====================================================

-- Pipeline executions table
CREATE TABLE IF NOT EXISTS pipeline_executions (
    execution_id UUID PRIMARY KEY,
    pipeline_id UUID NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NULL,
    variables JSONB NOT NULL DEFAULT '{}'::jsonb,
    tenant_id TEXT NULL,
    correlation_id TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Step executions table
CREATE TABLE IF NOT EXISTS step_executions (
    step_execution_id UUID PRIMARY KEY,
    execution_id UUID NOT NULL REFERENCES pipeline_executions(execution_id) ON DELETE CASCADE,
    step_id UUID NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    retry_count INT NOT NULL DEFAULT 0,
    error_message TEXT NULL,
    logs TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for execution lookups by pipeline
CREATE INDEX IF NOT EXISTS idx_pipeline_executions_pipeline_id
ON pipeline_executions(pipeline_id);

-- Index for execution status queries
CREATE INDEX IF NOT EXISTS idx_pipeline_executions_status
ON pipeline_executions(status);

-- Index for execution status queries
CREATE INDEX IF NOT EXISTS idx_pipeline_executions_tenant_id
ON pipeline_executions(tenant_id);

-- Index for correlation tracking
CREATE INDEX IF NOT EXISTS idx_pipeline_executions_correlation_id
ON pipeline_executions(correlation_id);

-- Index for execution creation time
CREATE INDEX IF NOT EXISTS idx_pipeline_executions_created_at
ON pipeline_executions(created_at);

-- Index for step execution lookups by execution
CREATE INDEX IF NOT EXISTS idx_step_executions_execution_id
ON step_executions(execution_id);

-- Index for step execution status queries
CREATE INDEX IF NOT EXISTS idx_step_executions_status
ON step_executions(status);

-- Index for retry tracking
CREATE INDEX IF NOT EXISTS idx_step_executions_retry_count
ON step_executions(retry_count);

-- Index for step execution time
CREATE INDEX IF NOT EXISTS idx_step_executions_started_at
ON step_executions(started_at);

COMMENT ON TABLE pipeline_executions IS 'Stores individual pipeline execution instances';
COMMENT ON COLUMN pipeline_executions.execution_id IS 'Unique identifier for the execution';
COMMENT ON COLUMN pipeline_executions.pipeline_id IS 'Reference to the executed pipeline';
COMMENT ON COLUMN pipeline_executions.status IS 'Current status (PENDING, RUNNING, COMPLETED, FAILED, CANCELLED)';
COMMENT ON COLUMN pipeline_executions.variables IS 'JSON variables used during execution';
COMMENT ON COLUMN pipeline_executions.correlation_id IS 'Correlation ID for distributed tracing';

COMMENT ON TABLE step_executions IS 'Stores individual step execution instances within a pipeline execution';
COMMENT ON COLUMN step_executions.step_execution_id IS 'Unique identifier for the step execution';
COMMENT ON COLUMN step_executions.execution_id IS 'Reference to parent execution';
COMMENT ON COLUMN step_executions.step_id IS 'Reference to the pipeline step definition';
COMMENT ON COLUMN step_executions.status IS 'Current status (PENDING, RUNNING, COMPLETED, FAILED, SKIPPED)';
COMMENT ON COLUMN step_executions.retry_count IS 'Number of times this step has been retried';
COMMENT ON COLUMN step_executions.error_message IS 'Error message if the step failed';
COMMENT ON COLUMN step_executions.logs IS 'Log output from the step execution';
