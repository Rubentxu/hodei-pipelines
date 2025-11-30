import { expect, test } from '@playwright/test';

test.describe('DevTools Page', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });
    });

    test('should navigate to devtools page', async ({ page }) => {
        await page.goto('/devtools');
        await expect(page.getByText('Developer Tools')).toBeVisible();
        await expect(page.getByText('Utilities for debugging')).toBeVisible();
    });

    test('should switch tabs', async ({ page }) => {
        await page.goto('/devtools');

        // Default tab is API Inspector
        await expect(page.getByRole('heading', { name: 'API Inspector' })).toBeVisible();

        // Switch to State Viewer
        await page.getByRole('button', { name: 'State Viewer' }).click();
        await expect(page.getByRole('heading', { name: 'Global State' })).toBeVisible();

        // Switch to Performance
        await page.getByRole('button', { name: 'Performance' }).click();
        await expect(page.getByRole('heading', { name: 'Performance Metrics' })).toBeVisible();
    });

    test('should make api request', async ({ page }) => {
        await page.goto('/devtools');

        // Mock API response
        await page.route('**/api/health', async (route) => {
            await route.fulfill({
                status: 200,
                contentType: 'application/json',
                body: JSON.stringify({ status: 'ok' })
            });
        });

        // Click send request
        await page.getByRole('button', { name: 'Send Request' }).click();

        // Check response
        await expect(page.locator('pre')).toContainText('"status": "ok"');
    });
});
