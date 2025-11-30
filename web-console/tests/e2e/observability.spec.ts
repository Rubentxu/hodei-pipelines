
import { expect, test } from '@playwright/test';

test.describe('Observability Dashboard', () => {
    test.beforeEach(async ({ page }) => {
        page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
        page.on('pageerror', err => console.log(`BROWSER ERROR: ${err.message}`));

        // Mock Metrics
        await page.route('/api/observability/metrics*', async (route) => {
            await route.fulfill({
                json: {
                    metrics: {
                        systemHealth: 98,
                        requestRate: 1500,
                        errorRate: 0.5,
                        avgResponseTime: 45,
                        cpuUsage: 45,
                        memoryUsage: 60,
                        diskUsage: 70,
                        networkIn: 5000000,
                        networkOut: 3000000
                    },
                    history: [
                        { timestamp: new Date().toISOString(), cpuUsage: 40, memoryUsage: 55, requestRate: 1400, errorRate: 0.4 },
                        { timestamp: new Date().toISOString(), cpuUsage: 45, memoryUsage: 60, requestRate: 1500, errorRate: 0.5 }
                    ]
                }
            });
        });

        // Mock Logs
        await page.route('/api/observability/logs*', async (route) => {
            await route.fulfill({
                json: {
                    logs: [
                        { id: 'log-1', timestamp: new Date().toISOString(), level: 'error', message: 'Database connection failed', service: 'db-service' },
                        { id: 'log-2', timestamp: new Date().toISOString(), level: 'info', message: 'User logged in', service: 'auth-service' }
                    ],
                    total: 2
                }
            });
        });

        // Mock Traces
        await page.route('/api/observability/traces*', async (route) => {
            await route.fulfill({
                json: {
                    traces: [
                        { id: 'trace-1', traceId: 'abc-123', serviceName: 'api-gateway', operationName: 'GET /users', duration: 150, startTime: new Date().toISOString(), status: 'ok' },
                        { id: 'trace-2', traceId: 'def-456', serviceName: 'auth-service', operationName: 'POST /login', duration: 500, startTime: new Date().toISOString(), status: 'error' }
                    ],
                    total: 2
                }
            });
        });

        // Mock Alerts
        await page.route('/api/observability/alerts*', async (route) => {
            await route.fulfill({
                json: {
                    alerts: [
                        { id: 'alert-1', ruleName: 'High Error Rate', severity: 'critical', message: 'Error rate > 5%', status: 'open', timestamp: new Date().toISOString() }
                    ],
                    total: 1
                }
            });
        });

        // Mock Alert Rules
        await page.route('/api/observability/alerts/rules*', async (route) => {
            await route.fulfill({
                json: [
                    { id: 'rule-1', name: 'High Error Rate', severity: 'critical', enabled: true }
                ]
            });
        });

        // Mock Services
        await page.route('/api/observability/services*', async (route) => {
            await route.fulfill({
                json: {
                    services: [
                        { id: 'svc-1', name: 'api-gateway', status: 'healthy', dependencies: [], metrics: { requestRate: 100, errorRate: 0, avgResponseTime: 20 } },
                        { id: 'svc-2', name: 'auth-service', status: 'degraded', dependencies: ['db-service'], metrics: { requestRate: 50, errorRate: 2, avgResponseTime: 100 } }
                    ]
                }
            });
        });

        await page.goto('/observability');
    });

    test('should display default metrics tab', async ({ page }) => {
        await expect(page.getByText('Advanced Observability')).toBeVisible();
        // Check for metrics content (assuming MetricsDashboard renders these)
        // Since we don't know exact UI of MetricsDashboard, we check for tab active state
        await expect(page.getByRole('button', { name: 'Metrics' })).toHaveClass(/text-blue-400/);
    });

    test('should navigate to logs tab', async ({ page }) => {
        await page.getByRole('button', { name: 'Logs' }).click();
        await expect(page.getByRole('button', { name: 'Logs' })).toHaveClass(/text-blue-400/);
        // Verify logs are displayed
        await expect(page.getByText('Database connection failed')).toBeVisible();
    });

    test('should navigate to traces tab', async ({ page }) => {
        await page.getByRole('button', { name: 'Traces' }).click();
        await expect(page.getByRole('button', { name: 'Traces' })).toHaveClass(/text-blue-400/);
        // Verify traces are displayed
        await expect(page.getByText('GET /users')).toBeVisible();
    });

    test('should navigate to alerts tab and toggle views', async ({ page }) => {
        await page.getByRole('button', { name: 'Alerts' }).click();
        await expect(page.getByRole('button', { name: 'Alerts' })).toHaveClass(/text-blue-400/);

        // Default view is "Alert Rules"
        await expect(page.getByRole('button', { name: 'Alert Rules' })).toHaveClass(/bg-blue-600/);
        // Assuming AlertRules component renders the list of rules
        // We mocked /api/observability/alerts/rules
        // We might need to check for rule name if visible.

        // Switch to Active Alerts
        await page.getByRole('button', { name: 'Active Alerts' }).click();
        await expect(page.getByRole('button', { name: 'Active Alerts' })).toHaveClass(/bg-blue-600/);
        await expect(page.getByText('High Error Rate')).toBeVisible();
        await expect(page.getByText('Error rate > 5%')).toBeVisible();
    });

    test('should navigate to services tab', async ({ page }) => {
        await page.getByRole('button', { name: 'Services' }).click();
        await expect(page.getByRole('button', { name: 'Services' })).toHaveClass(/text-blue-400/);
        // Verify services are displayed
        await expect(page.getByText('api-gateway')).toBeVisible();
        await expect(page.getByText('auth-service')).toBeVisible();
    });
});
