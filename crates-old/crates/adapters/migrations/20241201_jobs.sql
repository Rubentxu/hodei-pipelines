-- =====================================================
-- Jobs Table Migration
-- Creates table for job queue and execution tracking
-- Migration: 20241201_jobs
-- =====================================================

CREATE TABLE IF NOT EXISTS jobs (
    job_id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NULL,
    spec JSONB NOT NULL,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    tenant_id TEXT NULL,
    result JSONB NULL
);

-- Add missing columns if they don't exist (for backward compatibility)
DO $$
BEGIN
    -- Add state column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'jobs' AND column_name = 'state') THEN
        ALTER TABLE jobs ADD COLUMN state TEXT NOT NULL DEFAULT 'PENDING';
    END IF;

    -- Add started_at column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'jobs' AND column_name = 'started_at') THEN
        ALTER TABLE jobs ADD COLUMN started_at TIMESTAMPTZ NULL;
    END IF;

    -- Add completed_at column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'jobs' AND column_name = 'completed_at') THEN
        ALTER TABLE jobs ADD COLUMN completed_at TIMESTAMPTZ NULL;
    END IF;

    -- Add tenant_id column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'jobs' AND column_name = 'tenant_id') THEN
        ALTER TABLE jobs ADD COLUMN tenant_id TEXT NULL;
    END IF;

    -- Add result column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'jobs' AND column_name = 'result') THEN
        ALTER TABLE jobs ADD COLUMN result JSONB NULL;
    END IF;
END $$;

-- Index for faster state lookups
CREATE INDEX IF NOT EXISTS idx_jobs_state
ON jobs(state);

-- Index for finding pending jobs
CREATE INDEX IF NOT EXISTS idx_jobs_pending
ON jobs(state, created_at)
WHERE state = 'PENDING';

-- Index for finding running jobs
CREATE INDEX IF NOT EXISTS idx_jobs_running
ON jobs(state, created_at)
WHERE state = 'RUNNING';

-- Index for tenant-based queries
CREATE INDEX IF NOT EXISTS idx_jobs_tenant_id
ON jobs(tenant_id);

-- Index for job creation time
CREATE INDEX IF NOT EXISTS idx_jobs_created_at
ON jobs(created_at);

COMMENT ON TABLE jobs IS 'Stores job definitions and execution state';
COMMENT ON COLUMN jobs.job_id IS 'Unique identifier for the job';
COMMENT ON COLUMN jobs.name IS 'Human-readable name for the job';
COMMENT ON COLUMN jobs.description IS 'Optional description of the job';
COMMENT ON COLUMN jobs.spec IS 'JSON specification for the job';
COMMENT ON COLUMN jobs.state IS 'Current state (PENDING, RUNNING, COMPLETED, FAILED, CANCELLED)';
COMMENT ON COLUMN jobs.started_at IS 'Time when job execution started';
COMMENT ON COLUMN jobs.completed_at IS 'Time when job execution completed';
COMMENT ON COLUMN jobs.result IS 'JSON result of job execution';
