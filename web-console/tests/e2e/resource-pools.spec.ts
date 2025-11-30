import { expect, test } from '@playwright/test';

test.describe('Resource Pools Management', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });

        // Mock Resource Pools API
        await page.route('/api/v1/worker-pools', async (route) => {
            if (route.request().method() === 'GET') {
                await route.fulfill({
                    json: [
                        {
                            id: 'pool-1',
                            name: 'Production Cluster',
                            poolType: 'Kubernetes',
                            providerName: 'AWS EKS',
                            minSize: 3,
                            maxSize: 10,
                            defaultResources: { cpu: 2, memory: 4096 },
                            tags: { env: 'prod', region: 'us-east-1' }
                        },
                        {
                            id: 'pool-2',
                            name: 'Dev Cluster',
                            poolType: 'Docker',
                            providerName: 'Local Docker',
                            minSize: 1,
                            maxSize: 3,
                            defaultResources: { cpu: 1, memory: 2048 },
                            tags: { env: 'dev' }
                        }
                    ],
                });
            } else if (route.request().method() === 'POST') {
                await route.fulfill({
                    status: 201,
                    json: { id: 'pool-3', name: 'New Pool', ...route.request().postDataJSON() }
                });
            }
        });

        // Mock Pool Status API
        await page.route('/api/v1/worker-pools/*/status', async (route) => {
            const url = route.request().url();
            if (url.includes('pool-1')) {
                await route.fulfill({
                    json: {
                        name: 'Production Cluster',
                        poolType: 'Kubernetes',
                        totalCapacity: 100,
                        availableCapacity: 20,
                        activeWorkers: 8,
                        pendingRequests: 2
                    }
                });
            } else {
                await route.fulfill({
                    json: {
                        name: 'Dev Cluster',
                        poolType: 'Docker',
                        totalCapacity: 30,
                        availableCapacity: 25,
                        activeWorkers: 1,
                        pendingRequests: 0
                    }
                });
            }
        });

        await page.goto('/resource-pools');
    });

    test('should list resource pools with status', async ({ page }) => {
        await expect(page.getByText('Resource Pools', { exact: true })).toBeVisible();

        // Check for pool names
        await expect(page.getByText('Production Cluster')).toBeVisible();
        await expect(page.getByText('Dev Cluster')).toBeVisible();

        // Check for status badges/info (using text content from mock status)
        await expect(page.getByText('AWS EKS')).toBeVisible();
        await expect(page.getByText('Kubernetes')).toBeVisible();

        // Check utilization (calculated from mock status)
        // Pool 1: 80 used / 100 total = 80%
        await expect(page.getByText('80%')).toBeVisible();
    });

    test('should navigate to create pool form', async ({ page }) => {
        await page.getByRole('button', { name: 'Create Pool' }).first().click();
        await expect(page).toHaveURL('/resource-pools/new');
        // Assuming the form has a header or title
        // We haven't checked ResourcePoolForm yet, but usually it has a title.
        // Let's just check URL for now.
    });

    test('should navigate to pool details', async ({ page }) => {
        // Click "View Details" for the first pool
        await page.locator('li').filter({ hasText: 'Production Cluster' }).getByRole('button', { name: 'View Details' }).click();
        await expect(page).toHaveURL('/resource-pools/pool-1');
    });
});
