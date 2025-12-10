-- =====================================================
-- RBAC Tables Migration
-- Creates tables for Role-Based Access Control
-- Migration: 20241130_rbac_tables
-- =====================================================

-- Roles table
CREATE TABLE IF NOT EXISTS rbac_roles (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Permissions table
CREATE TABLE IF NOT EXISTS rbac_permissions (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    resource VARCHAR(255) NOT NULL,
    action VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_resource_action UNIQUE (resource, action)
);

-- Role-Permissions junction table (many-to-many)
CREATE TABLE IF NOT EXISTS rbac_role_permissions (
    role_id UUID NOT NULL REFERENCES rbac_roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES rbac_permissions(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_by VARCHAR(255),
    PRIMARY KEY (role_id, permission_id)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_rbac_roles_name ON rbac_roles(name);
CREATE INDEX IF NOT EXISTS idx_rbac_permissions_resource ON rbac_permissions(resource);
CREATE INDEX IF NOT EXISTS idx_rbac_permissions_resource_action ON rbac_permissions(resource, action);
CREATE INDEX IF NOT EXISTS idx_rbac_role_permissions_role_id ON rbac_role_permissions(role_id);
CREATE INDEX IF NOT EXISTS idx_rbac_role_permissions_permission_id ON rbac_role_permissions(permission_id);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_rbac_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers to automatically update updated_at
CREATE TRIGGER update_rbac_roles_updated_at
    BEFORE UPDATE ON rbac_roles
    FOR EACH ROW
    EXECUTE FUNCTION update_rbac_updated_at();

CREATE TRIGGER update_rbac_permissions_updated_at
    BEFORE UPDATE ON rbac_permissions
    FOR EACH ROW
    EXECUTE FUNCTION update_rbac_updated_at();

-- =====================================================
-- Initial Data Seeding
-- =====================================================

-- Insert default permissions
INSERT INTO rbac_permissions (id, name, description, resource, action) VALUES
    (gen_random_uuid(), 'pipeline:read', 'Read pipeline information', 'pipeline', 'read'),
    (gen_random_uuid(), 'pipeline:create', 'Create new pipelines', 'pipeline', 'create'),
    (gen_random_uuid(), 'pipeline:update', 'Update existing pipelines', 'pipeline', 'update'),
    (gen_random_uuid(), 'pipeline:delete', 'Delete pipelines', 'pipeline', 'delete'),
    (gen_random_uuid(), 'pipeline:execute', 'Execute pipelines', 'pipeline', 'execute'),
    (gen_random_uuid(), 'execution:read', 'Read execution information', 'execution', 'read'),
    (gen_random_uuid(), 'execution:cancel', 'Cancel running executions', 'execution', 'cancel'),
    (gen_random_uuid(), 'execution:retry', 'Retry failed executions', 'execution', 'retry'),
    (gen_random_uuid(), 'worker:read', 'Read worker information', 'worker', 'read'),
    (gen_random_uuid(), 'worker:manage', 'Manage workers', 'worker', 'manage'),
    (gen_random_uuid(), 'metrics:read', 'Read metrics and dashboards', 'metrics', 'read'),
    (gen_random_uuid(), 'logs:read', 'Read execution logs', 'logs', 'read'),
    (gen_random_uuid(), 'admin:system', 'System administration', 'admin', 'system'),
    (gen_random_uuid(), 'admin:rbac', 'Manage RBAC (roles and permissions)', 'admin', 'rbac'),
    (gen_random_uuid(), 'budget:read', 'Read budget information', 'budget', 'read'),
    (gen_random_uuid(), 'budget:manage', 'Manage budget and alerts', 'budget', 'manage'),
    (gen_random_uuid(), 'security:read', 'Read security information', 'security', 'read'),
    (gen_random_uuid(), 'security:manage', 'Manage security settings', 'security', 'manage')
ON CONFLICT (name) DO NOTHING;

-- Insert default roles
DO $$
DECLARE
    admin_role_id UUID := gen_random_uuid();
    operator_role_id UUID := gen_random_uuid();
    viewer_role_id UUID := gen_random_uuid();
    worker_role_id UUID := gen_random_uuid();
    security_role_id UUID := gen_random_uuid();
    budget_manager_role_id UUID := gen_random_uuid();
BEGIN
    -- Admin role (all permissions)
    INSERT INTO rbac_roles (id, name, description) VALUES
        (admin_role_id, 'admin', 'System administrator with full access')
    ON CONFLICT (name) DO NOTHING;

    -- Operator role (manage pipelines and executions)
    INSERT INTO rbac_roles (id, name, description) VALUES
        (operator_role_id, 'operator', 'Pipeline operator with execution management capabilities')
    ON CONFLICT (name) DO NOTHING;

    -- Viewer role (read-only access)
    INSERT INTO rbac_roles (id, name, description) VALUES
        (viewer_role_id, 'viewer', 'Read-only access to pipelines, executions, and metrics')
    ON CONFLICT (name) DO NOTHING;

    -- Worker role (execute pipelines)
    INSERT INTO rbac_roles (id, name, description) VALUES
        (worker_role_id, 'worker', 'Can execute pipelines and read execution status')
    ON CONFLICT (name) DO NOTHING;

    -- Security manager role
    INSERT INTO rbac_roles (id, name, description) VALUES
        (security_role_id, 'security_manager', 'Manage security settings and vulnerability tracking')
    ON CONFLICT (name) DO NOTHING;

    -- Budget manager role
    INSERT INTO rbac_roles (id, name, description) VALUES
        (budget_manager_role_id, 'budget_manager', 'Manage budgets and cost optimization')
    ON CONFLICT (name) DO NOTHING;

    -- Assign permissions to roles
    -- Admin gets all permissions
    INSERT INTO rbac_role_permissions (role_id, permission_id)
    SELECT admin_role_id, id FROM rbac_permissions
    ON CONFLICT DO NOTHING;

    -- Operator permissions
    INSERT INTO rbac_role_permissions (role_id, permission_id)
    SELECT operator_role_id, id FROM rbac_permissions
    WHERE resource IN ('pipeline', 'execution', 'worker', 'logs')
    ON CONFLICT DO NOTHING;

    -- Viewer permissions (read-only)
    INSERT INTO rbac_role_permissions (role_id, permission_id)
    SELECT viewer_role_id, id FROM rbac_permissions
    WHERE action = 'read'
    ON CONFLICT DO NOTHING;

    -- Worker permissions
    INSERT INTO rbac_role_permissions (role_id, permission_id)
    SELECT worker_role_id, id FROM rbac_permissions
    WHERE (resource = 'pipeline' AND action IN ('read', 'execute'))
       OR (resource = 'execution' AND action = 'read')
    ON CONFLICT DO NOTHING;

    -- Security manager permissions
    INSERT INTO rbac_role_permissions (role_id, permission_id)
    SELECT security_role_id, id FROM rbac_permissions
    WHERE resource = 'security'
    ON CONFLICT DO NOTHING;

    -- Budget manager permissions
    INSERT INTO rbac_role_permissions (role_id, permission_id)
    SELECT budget_manager_role_id, id FROM rbac_permissions
    WHERE resource = 'budget'
    ON CONFLICT DO NOTHING;

END $$;

-- =====================================================
-- Views for easier queries
-- =====================================================

-- View to get roles with their permissions
CREATE OR REPLACE VIEW rbac_roles_with_permissions AS
SELECT
    r.id,
    r.name,
    r.description,
    r.created_at,
    r.updated_at,
    array_agg(p.id ORDER BY p.resource, p.action) as permission_ids,
    array_agg(p.name ORDER BY p.resource, p.action) as permission_names,
    array_agg(p.resource ORDER BY p.resource, p.action) as permission_resources,
    array_agg(p.action ORDER BY p.resource, p.action) as permission_actions
FROM rbac_roles r
LEFT JOIN rbac_role_permissions rp ON r.id = rp.role_id
LEFT JOIN rbac_permissions p ON rp.permission_id = p.id
GROUP BY r.id, r.name, r.description, r.created_at, r.updated_at;

-- View to get permissions with usage count
CREATE OR REPLACE VIEW rbac_permissions_usage AS
SELECT
    p.id,
    p.name,
    p.description,
    p.resource,
    p.action,
    p.created_at,
    p.updated_at,
    COALESCE(rp.usage_count, 0) as usage_count
FROM rbac_permissions p
LEFT JOIN (
    SELECT
        permission_id,
        COUNT(*) as usage_count
    FROM rbac_role_permissions
    GROUP BY permission_id
) rp ON p.id = rp.permission_id;

-- Comments for documentation
COMMENT ON TABLE rbac_roles IS 'Stores RBAC roles with descriptions';
COMMENT ON TABLE rbac_permissions IS 'Stores RBAC permissions (resource:action)';
COMMENT ON TABLE rbac_role_permissions IS 'Junction table linking roles to permissions';
COMMENT ON VIEW rbac_roles_with_permissions IS 'Roles with aggregated permission information';
COMMENT ON VIEW rbac_permissions_usage IS 'Permissions with their usage count in roles';

-- =====================================================
-- End of Migration
-- =====================================================
