import { expect, test } from '@playwright/test';

test.describe('Workers Management', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Workers API
        await page.route('/api/workers', async (route) => {
            await route.fulfill({
                json: [
                    { id: 'worker-1', name: 'Worker 1', status: 'healthy', cpu: 45, memory: 60, version: '1.0.0' },
                    { id: 'worker-2', name: 'Worker 2', status: 'failed', cpu: 0, memory: 0, version: '1.0.0' },
                    { id: 'worker-3', name: 'Worker 3', status: 'maintenance', cpu: 10, memory: 20, version: '1.0.0' },
                ],
            });
        });

        // Mock Worker Pools API
        await page.route('/api/worker-pools', async (route) => {
            await route.fulfill({
                json: [
                    { id: 'pool-1', name: 'General Purpose', replicas: 3, minReplicas: 1, maxReplicas: 10, status: 'active', type: 'static', currentWorkers: 3, maxWorkers: 10, minWorkers: 1, scalingPolicy: { cpuThreshold: 80, memoryThreshold: 80, queueDepthThreshold: 100 } },
                    { id: 'pool-2', name: 'GPU Pool', replicas: 1, minReplicas: 0, maxReplicas: 5, status: 'scaling', type: 'dynamic', currentWorkers: 1, maxWorkers: 5, minWorkers: 0, scalingPolicy: { cpuThreshold: 80, memoryThreshold: 80, queueDepthThreshold: 100 } },
                ],
            });
        });

        // Mock Actions
        await page.route('/api/workers/*/start', async (route) => {
            await route.fulfill({ status: 200 });
        });
        await page.route('/api/workers/*/stop', async (route) => {
            await route.fulfill({ status: 200 });
        });
        await page.route('/api/workers/*/restart', async (route) => {
            await route.fulfill({ status: 200 });
        });
        await page.route('/api/worker-pools/*/scale', async (route) => {
            await route.fulfill({
                json: { id: 'pool-1', replicas: 4 }, // Mocked response
            });
        });

        await page.goto('/workers');
    });

    test('should list workers and display statistics', async ({ page }) => {
        await expect(page.getByText('Workers', { exact: true }).first()).toBeVisible();

        // Check statistics cards
        await expect(page.getByText('Activos')).toBeVisible();
        await expect(page.getByText('1', { exact: true })).toBeVisible(); // 1 healthy worker
        await expect(page.getByText('Fallidos')).toBeVisible();
        await expect(page.getByText('1', { exact: true })).toBeVisible(); // 1 failed worker
        await expect(page.getByText('Total')).toBeVisible();
        await expect(page.getByText('3', { exact: true })).toBeVisible(); // 3 total workers

        // Check worker cards
        await expect(page.getByText('Worker 1')).toBeVisible();
        await expect(page.getByText('Worker 2')).toBeVisible();
        await expect(page.getByText('Worker 3')).toBeVisible();
    });

    test('should allow managing workers', async ({ page }) => {
        // Worker 1 is healthy, should have Stop and Restart buttons
        const worker1Card = page.locator('.card', { hasText: 'Worker 1' }); // Generic selector, might need adjustment based on rendered HTML
        // Actually, looking at WorkerCard, it uses Shadcn Card.
        // We can scope by text.

        // Stop Worker 1
        await page.locator('.rounded-lg', { hasText: 'Worker 1' }).getByRole('button', { name: 'Detener' }).click();
        // Verify API call was made (Playwright routes handle this, but we could assert request if needed)

        // Worker 2 is failed, should have Start button
        await page.locator('.rounded-lg', { hasText: 'Worker 2' }).getByRole('button', { name: 'Iniciar' }).click();
    });

    test('should list worker pools', async ({ page }) => {
        // Switch to Pools tab
        await page.getByRole('button', { name: 'Pools' }).click();

        await expect(page.getByText('General Purpose')).toBeVisible();
        await expect(page.getByText('GPU Pool')).toBeVisible();
    });

    test('should allow scaling a worker pool', async ({ page }) => {
        await page.getByRole('button', { name: 'Pools' }).click();

        // General Purpose pool is static (implied by having scale buttons if type is static)
        // We need to ensure our mock data reflects a static pool.
        // In workers.spec.ts mock: { id: 'pool-1', name: 'General Purpose', ... } 
        // We need to add type: 'static' to the mock for pool-1 to see the buttons.

        await page.locator('.rounded-lg', { hasText: 'General Purpose' }).getByRole('button', { name: '+1' }).click();
    });
});
