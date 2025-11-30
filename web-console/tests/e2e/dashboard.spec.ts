import { expect, test } from '@playwright/test';

test.describe('Dashboard', () => {
    test.beforeEach(async ({ page }) => {
        // Mock KPI Metrics API (GET)
        await page.route('http://localhost:3005/api/observability/metrics', async route => {
            console.log('MOCK HIT:', route.request().url());
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                body: JSON.stringify({
                    jobsInQueue: 5,
                    jobsRunning: 2,
                    jobsSucceeded: 100,
                    jobsFailed: 3,
                    activeWorkers: 4,
                    cpuUsage: 45,
                    memoryUsage: 60,
                    avgExecutionTime: 120
                })
            });
            console.log('MOCK FULFILLED');
        });

        // Mock KPI Metrics Stream
        await page.route('/api/observability/metrics/stream', async route => {
            // Keep connection open or return a valid stream event
            await route.fulfill({
                status: 200,
                contentType: 'text/event-stream',
                body: 'data: {"jobsInQueue": 5}\n\n'
            });
        });

        // Mock Cluster Topology API
        await page.route('/api/observability/services', async route => {
            console.log('MOCK HIT: /api/observability/services');
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                json: {
                    services: [
                        { id: 'service-1', name: 'Service A', status: 'healthy', dependencies: [], metrics: { requestRate: 10, errorRate: 0, avgResponseTime: 50 } }
                    ]
                }
            });
        });

        // Mock SLA Violations API (GET)
        await page.route('/api/observability/sla/violations', async route => {
            console.log('MOCK HIT: /api/observability/sla/violations');
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                json: {
                    violations: [],
                    total: 0,
                    criticalCount: 0,
                    warningCount: 0
                }
            });
        });

        // Mock SLA Violations Stream
        await page.route('/api/observability/sla/violations/stream', async route => {
            await route.fulfill({
                status: 200,
                contentType: 'text/event-stream',
                body: 'data: {"violations": []}\n\n'
            });
        });

        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });

        await page.goto('/');
    });

    test('should display dashboard with KPI cards', async ({ page }) => {
        page.on('console', msg => console.log('CONSOLE:', msg.text()));
        page.on('request', request => console.log('>>', request.method(), request.url()));
        page.on('requestfailed', request => console.log('!!', request.method(), request.url(), request.failure()?.errorText));

        await expect(page.getByText('Dashboard', { exact: true })).toBeVisible();

        // Wait for loading to finish
        await expect(page.locator('.animate-pulse')).toHaveCount(0);

        // Check if error is visible
        const errorVisible = await page.getByText('Error al cargar').isVisible();
        if (errorVisible) {
            console.log('ERROR STATE DETECTED');
        }
        console.log('Dashboard Body HTML:', await page.locator('body').innerHTML());

        // Check for specific KPI cards
        await expect(page.getByText('Jobs en Cola')).toBeVisible();
        await expect(page.getByText('5')).toBeVisible(); // jobsInQueue value

        await expect(page.getByText('Jobs Ejecutándose')).toBeVisible();
        await expect(page.getByText('2')).toBeVisible(); // jobsRunning value
    });
});
