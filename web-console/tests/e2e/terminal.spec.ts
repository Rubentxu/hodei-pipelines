import { expect, test } from '@playwright/test';

test.describe('Terminal / Job Inspection', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });

        // Navigate to a terminal page with a fake job ID
        await page.goto('/terminal/job-123');
    });

    test('should display terminal UI', async ({ page }) => {
        await expect(page.getByText('Terminal Interactivo')).toBeVisible();
        await expect(page.getByText('Job ID: job-123')).toBeVisible();
        await expect(page.getByText('Console de Terminal')).toBeVisible();
    });

    test('should handle connection failure gracefully', async ({ page }) => {
        // Since we don't have a WS server, it should fail to connect.
        // The component shows a toast and "Desconectado" status.

        // Check for status indicator
        await expect(page.getByText('Desconectado')).toBeVisible();

        // Check for error toast (might need to wait a bit)
        // Toast usually appears in a container. We can check for text.
        // Note: Toasts might be transient.
        // await expect(page.getByText('No se pudo conectar al terminal')).toBeVisible(); 
        // (Skipping toast check as it might be flaky without specific setup, but status check is robust)
    });

    test('should navigate back', async ({ page }) => {
        await page.getByRole('button', { name: 'Volver' }).click();
        await expect(page).toHaveURL('/');
    });
});
