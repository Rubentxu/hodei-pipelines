import { expect, test } from '@playwright/test';

test.describe('Authentication', () => {
    test('should allow a user to login', async ({ page }) => {
        await page.goto('/login');

        // Check if we are on the login page

        await expect(page.getByRole('heading', { name: 'Welcome Back' })).toBeVisible();

        // Fill in credentials
        await page.fill('input[name="username"]', 'testuser');
        await page.fill('input[name="password"]', 'password123');

        // Submit form
        await page.click('button[type="submit"]');

        // Verify redirection to dashboard
        await expect(page).toHaveURL('/');
        await expect(page.getByText('Dashboard', { exact: true })).toBeVisible();
    });
});
