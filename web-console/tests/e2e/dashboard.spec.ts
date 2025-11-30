import { expect, test } from '@playwright/test';

test.describe('Dashboard', () => {
    test.beforeEach(async ({ page }) => {
        // Mock KPI Metrics API
        await page.route('**/api/metrics/kpi', async route => {
            await route.fulfill({
                json: {
                    jobsInQueue: 5,
                    jobsRunning: 2,
                    jobsSucceeded: 100,
                    jobsFailed: 3,
                    activeWorkers: 4,
                    cpuUsage: 45,
                    memoryUsage: 60,
                    avgExecutionTime: 120
                }
            });
        });

        // Mock Cluster Topology API
        await page.route('**/api/topology', async route => {
            await route.fulfill({
                json: {
                    nodes: [
                        { id: 'node-1', type: 'master', status: 'ready' },
                        { id: 'node-2', type: 'worker', status: 'ready' }
                    ],
                    edges: [
                        { source: 'node-1', target: 'node-2' }
                    ]
                }
            });
        });

        // Mock SLA Violations API
        await page.route('**/api/sla/violations', async route => {
            await route.fulfill({
                json: {
                    violations: []
                }
            });
        });

        // Login first
        await page.goto('/login');
        await page.fill('input[name="username"]', 'testuser');
        await page.fill('input[name="password"]', 'password123');
        await page.click('button[type="submit"]');
    });

    test('should display dashboard with KPI cards', async ({ page }) => {
        await expect(page.getByText('Dashboard', { exact: true })).toBeVisible();

        // Check for specific KPI cards
        await expect(page.getByText('Jobs en Cola')).toBeVisible();
        await expect(page.getByText('5')).toBeVisible(); // jobsInQueue value

        await expect(page.getByText('Jobs Ejecutándose')).toBeVisible();
        await expect(page.getByText('2')).toBeVisible(); // jobsRunning value
    });
});
