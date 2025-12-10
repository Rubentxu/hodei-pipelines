-- =====================================================
-- Workers Table Migration
-- Creates table for worker node registration and tracking
-- Migration: 20241201_workers
-- =====================================================

CREATE TABLE IF NOT EXISTS workers (
    worker_id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    current_jobs JSONB NOT NULL DEFAULT '[]'::jsonb,
    last_heartbeat TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tenant_id TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for faster status lookups
CREATE INDEX IF NOT EXISTS idx_workers_status
ON workers(status);

-- Index for finding stale workers
CREATE INDEX IF NOT EXISTS idx_workers_last_heartbeat
ON workers(last_heartbeat);

-- Index for worker metadata queries
CREATE INDEX IF NOT EXISTS idx_workers_tenant_id
ON workers(tenant_id);

COMMENT ON TABLE workers IS 'Stores worker node information and status';
COMMENT ON COLUMN workers.worker_id IS 'Unique identifier for the worker';
COMMENT ON COLUMN workers.name IS 'Human-readable name for the worker';
COMMENT ON COLUMN workers.status IS 'Current status (e.g., IDLE, BUSY, OFFLINE)';
COMMENT ON COLUMN workers.capabilities IS 'JSON describing worker capabilities (cpu, memory, etc)';
COMMENT ON COLUMN workers.metadata IS 'Additional metadata about the worker';
COMMENT ON COLUMN workers.current_jobs IS 'List of currently executing job IDs';
COMMENT ON COLUMN workers.last_heartbeat IS 'Last time the worker sent a heartbeat';
