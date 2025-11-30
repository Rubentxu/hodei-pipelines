-- =====================================================
-- Metrics Time Series Tables Migration
-- Creates tables for persisting metrics in TimescaleDB
-- Migration: 20241201_metrics_timeseries
-- =====================================================

-- Enable TimescaleDB extension (if available)
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Metrics table (uses TimescaleDB hypertable for time-series data)
CREATE TABLE IF NOT EXISTS metrics_timeseries (
    time TIMESTAMPTZ NOT NULL,
    metric_name VARCHAR(255) NOT NULL,
    metric_type VARCHAR(50) NOT NULL, -- counter, gauge, histogram, summary
    value DOUBLE PRECISION NOT NULL,
    labels JSONB NOT NULL DEFAULT '{}'::jsonb,
    tenant_id VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (time, metric_name, labels)
);

-- Create hypertable for time-series data
-- Only create if TimescaleDB is available
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        SELECT create_hypertable('metrics_timeseries', 'time', if_not_exists => TRUE);
    END IF;
END $$;

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_metrics_timeseries_name ON metrics_timeseries(metric_name);
CREATE INDEX IF NOT EXISTS idx_metrics_timeseries_time ON metrics_timeseries(time DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_timeseries_tenant ON metrics_timeseries(tenant_id);
CREATE INDEX IF NOT EXISTS idx_metrics_timeseries_name_time ON metrics_timeseries(metric_name, time DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_timeseries_labels ON metrics_timeseries USING GIN(labels);

-- Continuous aggregate for hourly metrics
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', time) AS hour,
    metric_name,
    metric_type,
    tenant_id,
    labels,
    AVG(value) as avg_value,
    MIN(value) as min_value,
    MAX(value) as max_value,
    COUNT(*) as sample_count
FROM metrics_timeseries
GROUP BY hour, metric_name, metric_type, tenant_id, labels
WITH NO DATA;

-- Continuous aggregate for daily metrics
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_daily
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', time) AS day,
    metric_name,
    metric_type,
    tenant_id,
    labels,
    AVG(value) as avg_value,
    MIN(value) as min_value,
    MAX(value) as max_value,
    COUNT(*) as sample_count
FROM metrics_timeseries
GROUP BY day, metric_name, metric_type, tenant_id, labels
WITH NO DATA;

-- Function to refresh continuous aggregates
CREATE OR REPLACE FUNCTION refresh_metrics_aggregates()
RETURNS void AS $$
BEGIN
    -- Refresh hourly aggregates
    CALL refresh_continuous_aggregate('metrics_hourly', NOW() - INTERVAL '1 day', NOW());

    -- Refresh daily aggregates
    CALL refresh_continuous_aggregate('metrics_daily', NOW() - INTERVAL '7 days', NOW());
END;
$$ LANGUAGE plpgsql;

-- Policy to automatically drop old data (keep 90 days)
SELECT add_retention_policy('metrics_timeseries', INTERVAL '90 days');

-- Policy to automatically refresh aggregates every 5 minutes
SELECT add_continuous_aggregate_policy('metrics_hourly',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '5 minutes');

SELECT add_continuous_aggregate_policy('metrics_daily',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 hour');

-- =====================================================
-- Functions for metrics operations
-- =====================================================

-- Function to insert a metric point
CREATE OR REPLACE FUNCTION insert_metric(
    p_time TIMESTAMPTZ,
    p_metric_name VARCHAR(255),
    p_metric_type VARCHAR(50),
    p_value DOUBLE PRECISION,
    p_labels JSONB DEFAULT '{}'::jsonb,
    p_tenant_id VARCHAR(255) DEFAULT NULL
)
RETURNS void AS $$
BEGIN
    INSERT INTO metrics_timeseries (
        time, metric_name, metric_type, value, labels, tenant_id
    ) VALUES (
        p_time, p_metric_name, p_metric_type, p_value, p_labels, p_tenant_id
    );
END;
$$ LANGUAGE plpgsql;

-- Function to get metric history
CREATE OR REPLACE FUNCTION get_metric_history(
    p_metric_name VARCHAR(255),
    p_start_time TIMESTAMPTZ,
    p_end_time TIMESTAMPTZ DEFAULT NOW(),
    p_tenant_id VARCHAR(255) DEFAULT NULL,
    p_labels JSONB DEFAULT NULL
)
RETURNS TABLE (
    time TIMESTAMPTZ,
    metric_name VARCHAR(255),
    metric_type VARCHAR(50),
    value DOUBLE PRECISION,
    labels JSONB,
    tenant_id VARCHAR(255)
) AS $$
BEGIN
    IF p_labels IS NULL THEN
        RETURN QUERY
        SELECT
            m.time,
            m.metric_name,
            m.metric_type,
            m.value,
            m.labels,
            m.tenant_id
        FROM metrics_timeseries m
        WHERE m.metric_name = p_metric_name
            AND m.time BETWEEN p_start_time AND p_end_time
            AND (p_tenant_id IS NULL OR m.tenant_id = p_tenant_id)
        ORDER BY m.time DESC;
    ELSE
        -- Filter by labels using jsonbcontains
        RETURN QUERY
        SELECT
            m.time,
            m.metric_name,
            m.metric_type,
            m.value,
            m.labels,
            m.tenant_id
        FROM metrics_timeseries m
        WHERE m.metric_name = p_metric_name
            AND m.time BETWEEN p_start_time AND p_end_time
            AND (p_tenant_id IS NULL OR m.tenant_id = p_tenant_id)
            AND m.labels @> p_labels
        ORDER BY m.time DESC;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Function to get aggregated metrics
CREATE OR REPLACE FUNCTION get_aggregated_metrics(
    p_metric_name VARCHAR(255),
    p_bucket_interval VARCHAR(50), -- '1 hour', '1 day', etc.
    p_start_time TIMESTAMPTZ,
    p_end_time TIMESTAMPTZ DEFAULT NOW(),
    p_tenant_id VARCHAR(255) DEFAULT NULL
)
RETURNS TABLE (
    bucket TIMESTAMPTZ,
    metric_name VARCHAR(255),
    avg_value DOUBLE PRECISION,
    min_value DOUBLE PRECISION,
    max_value DOUBLE PRECISION,
    sample_count BIGINT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        time_bucket(p_bucket_interval::interval, m.time) AS bucket,
        m.metric_name,
        AVG(m.value) as avg_value,
        MIN(m.value) as min_value,
        MAX(m.value) as max_value,
        COUNT(*) as sample_count
    FROM metrics_timeseries m
    WHERE m.metric_name = p_metric_name
        AND m.time BETWEEN p_start_time AND p_end_time
        AND (p_tenant_id IS NULL OR m.tenant_id = p_tenant_id)
    GROUP BY bucket, m.metric_name
    ORDER BY bucket DESC;
END;
$$ LANGUAGE plpgsql;

-- Function to get metric statistics
CREATE OR REPLACE FUNCTION get_metric_stats(
    p_metric_name VARCHAR(255),
    p_start_time TIMESTAMPTZ,
    p_end_time TIMESTAMPTZ DEFAULT NOW(),
    p_tenant_id VARCHAR(255) DEFAULT NULL
)
RETURNS TABLE (
    metric_name VARCHAR(255),
    count BIGINT,
    avg_value DOUBLE PRECISION,
    min_value DOUBLE PRECISION,
    max_value DOUBLE PRECISION,
    p50 DOUBLE PRECISION,
    p90 DOUBLE PRECISION,
    p95 DOUBLE PRECISION,
    p99 DOUBLE PRECISION
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        m.metric_name,
        COUNT(*) as count,
        AVG(m.value) as avg_value,
        MIN(m.value) as min_value,
        MAX(m.value) as max_value,
        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY m.value) as p50,
        PERCENTILE_CONT(0.9) WITHIN GROUP (ORDER BY m.value) as p90,
        PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY m.value) as p95,
        PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY m.value) as p99
    FROM metrics_timeseries m
    WHERE m.metric_name = p_metric_name
        AND m.time BETWEEN p_start_time AND p_end_time
        AND (p_tenant_id IS NULL OR m.tenant_id = p_tenant_id)
    GROUP BY m.metric_name;
END;
$$ LANGUAGE plpgsql;

-- Function to delete old metrics
CREATE OR REPLACE FUNCTION delete_old_metrics(
    p_before_time TIMESTAMPTZ
)
RETURNS BIGINT AS $$
DECLARE
    deleted_count BIGINT;
BEGIN
    DELETE FROM metrics_timeseries
    WHERE time < p_before_time;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- =====================================================
-- Views for easy querying
-- =====================================================

-- View for latest metrics
CREATE OR REPLACE VIEW metrics_latest AS
SELECT DISTINCT ON (metric_name, labels, tenant_id)
    time,
    metric_name,
    metric_type,
    value,
    labels,
    tenant_id
FROM metrics_timeseries
ORDER BY metric_name, labels, tenant_id, time DESC;

-- View for top metrics by value
CREATE OR REPLACE VIEW metrics_top_values AS
SELECT
    metric_name,
    AVG(value) as avg_value,
    MAX(value) as max_value,
    COUNT(*) as sample_count,
    tenant_id
FROM metrics_timeseries
WHERE time > NOW() - INTERVAL '1 hour'
GROUP BY metric_name, tenant_id
ORDER BY max_value DESC;

-- Comments for documentation
COMMENT ON TABLE metrics_timeseries IS 'Time-series metrics storage using TimescaleDB';
COMMENT ON MATERIALIZED VIEW metrics_hourly IS 'Hourly aggregated metrics';
COMMENT ON MATERIALIZED VIEW metrics_daily IS 'Daily aggregated metrics';
COMMENT ON FUNCTION insert_metric IS 'Insert a single metric point';
COMMENT ON FUNCTION get_metric_history IS 'Get metric history with optional filtering';
COMMENT ON FUNCTION get_aggregated_metrics IS 'Get aggregated metrics over time buckets';
COMMENT ON FUNCTION get_metric_stats IS 'Get statistical aggregations for a metric';
COMMENT ON FUNCTION delete_old_metrics IS 'Delete metrics older than specified time';

-- =====================================================
-- End of Migration
-- =====================================================
