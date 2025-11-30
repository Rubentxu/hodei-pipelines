import { expect, test } from '@playwright/test';

test.describe('Logs Explorer', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });

        // Mock Log Levels
        await page.route('/api/v1/logs/levels', async (route) => {
            await route.fulfill({ json: ['info', 'error', 'warn', 'debug'] });
        });

        // Mock Services
        await page.route('/api/v1/logs/services', async (route) => {
            await route.fulfill({ json: ['api-gateway', 'auth-service', 'worker-service'] });
        });

        // Mock Stats
        await page.route('/api/v1/logs/stats*', async (route) => {
            await route.fulfill({
                json: {
                    totalLogs: 1250,
                    errorRate: 0.02,
                    topServices: [{ service: 'api-gateway', count: 500 }],
                    topLevels: [{ level: 'info', count: 800 }]
                }
            });
        });

        // Mock Logs List
        await page.route('/api/v1/logs?*', async (route) => {
            const url = new URL(route.request().url());
            const level = url.searchParams.get('level');
            const service = url.searchParams.get('service');

            let logs = [
                {
                    id: 'log-1',
                    timestamp: new Date().toISOString(),
                    level: 'info',
                    service: 'api-gateway',
                    message: 'Incoming request GET /api/v1/workers',
                    traceId: 'trace-123',
                    spanId: 'span-456'
                },
                {
                    id: 'log-2',
                    timestamp: new Date().toISOString(),
                    level: 'error',
                    service: 'auth-service',
                    message: 'Failed to authenticate user',
                    traceId: 'trace-789',
                    spanId: 'span-012',
                    metadata: { userId: 'user-1' }
                }
            ];

            if (level) {
                logs = logs.filter(l => l.level === level);
            }
            if (service) {
                logs = logs.filter(l => l.service === service);
            }

            await route.fulfill({
                json: {
                    logs,
                    total: logs.length,
                    hasMore: false
                }
            });
        });

        // Mock Search
        await page.route('/api/v1/logs/search', async (route) => {
            await route.fulfill({
                json: {
                    logs: [
                        {
                            id: 'log-3',
                            timestamp: new Date().toISOString(),
                            level: 'warn',
                            service: 'worker-service',
                            message: 'Worker pool utilization high',
                            traceId: 'trace-333'
                        }
                    ],
                    total: 1,
                    searchQuery: 'high',
                    searchTime: 10
                }
            });
        });

        await page.goto('/logs');
    });

    test('should display initial stats and logs', async ({ page }) => {
        await expect(page.getByRole('heading', { name: 'Logs Explorer' })).toBeVisible();
        await expect(page.getByText('1250')).toBeVisible(); // Total logs
        await expect(page.getByText('2.0%')).toBeVisible(); // Error rate

        await expect(page.getByText('Incoming request GET /api/v1/workers')).toBeVisible();
        await expect(page.getByText('Failed to authenticate user')).toBeVisible();
    });

    test('should filter logs by level', async ({ page }) => {
        await page.getByLabel('Log Level').selectOption('error');

        // Should show error log
        await expect(page.getByText('Failed to authenticate user')).toBeVisible();
        // Should NOT show info log
        await expect(page.getByText('Incoming request GET /api/v1/workers')).not.toBeVisible();
    });

    test('should search logs', async ({ page }) => {
        await page.getByPlaceholder('Search logs...').fill('high');
        await page.getByRole('button', { name: 'Search' }).click();

        await expect(page.getByText('Worker pool utilization high')).toBeVisible();
    });
});
