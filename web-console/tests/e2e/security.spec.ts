import { expect, test } from '@playwright/test';

test.describe('Security Center', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });

        // Mock Security Metrics
        await page.route('/api/security/metrics', async (route) => {
            await route.fulfill({
                json: {
                    metrics: {
                        activeUsers: 150,
                        activeRoles: 5,
                        failedLogins: 12,
                        securityScore: 85
                    },
                    recentLogs: [
                        { id: 'log-1', action: 'LOGIN_FAILED', userId: 'user-1', timestamp: new Date().toISOString() },
                        { id: 'log-2', action: 'ROLE_UPDATED', userId: 'admin-1', timestamp: new Date().toISOString() }
                    ]
                }
            });
        });

        // Mock Users
        await page.route('/api/security/users', async (route) => {
            await route.fulfill({
                json: [
                    { id: 'user-1', username: 'alice', email: 'alice@example.com', role: 'admin', status: 'active' },
                    { id: 'user-2', username: 'bob', email: 'bob@example.com', role: 'user', status: 'inactive' }
                ]
            });
        });

        // Mock Roles
        await page.route('/api/security/roles', async (route) => {
            await route.fulfill({
                json: [
                    { id: 'role-1', name: 'Admin', permissions: ['all'] },
                    { id: 'role-2', name: 'User', permissions: ['read'] }
                ]
            });
        });

        // Mock Audit Logs
        await page.route('/api/security/audit-logs*', async (route) => {
            await route.fulfill({
                json: [
                    { id: 'log-1', action: 'LOGIN_FAILED', userId: 'user-1', timestamp: new Date().toISOString(), details: { ip: '192.168.1.1' } },
                    { id: 'log-2', action: 'ROLE_UPDATED', userId: 'admin-1', timestamp: new Date().toISOString(), details: { role: 'Admin' } }
                ]
            });
        });

        await page.goto('/security');
    });

    test('should display security overview', async ({ page }) => {
        await expect(page.getByText('Centro de Seguridad')).toBeVisible();

        // Check metrics (assuming SecurityOverview renders them)
        // We might need to check for specific text or numbers depending on the component implementation.
        // Let's assume the numbers are visible.
        await expect(page.getByText('150')).toBeVisible(); // Active Users
        await expect(page.getByText('85')).toBeVisible(); // Security Score
    });

    test('should navigate to users tab', async ({ page }) => {
        await page.getByRole('button', { name: 'Usuarios' }).click();

        await expect(page.getByText('alice@example.com')).toBeVisible();
        await expect(page.getByText('bob@example.com')).toBeVisible();
    });

    test('should navigate to roles tab', async ({ page }) => {
        await page.getByRole('button', { name: 'Roles' }).click();

        await expect(page.getByText('Admin', { exact: true })).toBeVisible();
        await expect(page.getByText('User', { exact: true })).toBeVisible();
    });

    test('should navigate to audit tab', async ({ page }) => {
        await page.getByRole('button', { name: 'Auditoría' }).click();

        await expect(page.getByText('LOGIN_FAILED')).toBeVisible();
        await expect(page.getByText('ROLE_UPDATED')).toBeVisible();
    });
});
