import { expect, test } from '@playwright/test';

test.describe('FinOps Dashboard', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });

        // Mock FinOps Metrics
        await page.route('/api/finops/metrics', async (route) => {
            await route.fulfill({
                json: {
                    metrics: {
                        dailyCost: 150.50,
                        monthlyCost: 4500.00,
                        yearlyCost: 54000.00,
                        projectedMonthlyCost: 4800.00,
                        budgetLimit: 5000.00,
                        budgetUsed: 4500.0,
                        costPerJob: 0.45,
                        costPerHour: 6.25,
                        trend: 'up',
                        trendPercentage: 5.2
                    },
                    history: [
                        { date: '2023-01-01', cost: 140, jobCount: 100 },
                        { date: '2023-01-02', cost: 160, jobCount: 120 },
                        { date: '2023-01-03', cost: 150, jobCount: 110 }
                    ],
                    breakdown: [
                        { pipelineId: 'p1', pipelineName: 'Data Ingestion', cost: 800, jobCount: 10, avgDuration: 120, percentage: 53.3 },
                        { pipelineId: 'p2', pipelineName: 'Model Training', cost: 400, jobCount: 5, avgDuration: 300, percentage: 26.7 },
                        { pipelineId: 'p3', pipelineName: 'Reporting', cost: 300, jobCount: 8, avgDuration: 60, percentage: 20.0 }
                    ],
                    alerts: [
                        { id: 'alert-1', type: 'warning', threshold: 75, current: 75.5, message: 'Budget usage > 75%', date: new Date().toISOString() }
                    ],
                    lastUpdated: new Date().toISOString()
                }
            });
        });

        // Mock the stream endpoint
        await page.route('/api/finops/metrics/stream', async (route) => {
            await route.fulfill({
                status: 200,
                headers: {
                    'Content-Type': 'text/event-stream',
                    'Cache-Control': 'no-cache',
                    'Connection': 'keep-alive',
                },
                body: '', // Keep connection open
            });
        });

        // Mock History for period change
        await page.route('/api/finops/history*', async (route) => {
            await route.fulfill({
                json: [
                    { date: '2023-01-01', cost: 100 },
                    { date: '2023-01-02', cost: 120 },
                    { date: '2023-01-03', cost: 110 }
                ]
            });
        });

        await page.goto('/finops');
    });

    test('should display initial metrics', async ({ page }) => {
        // Wait for loading to finish
        await expect(page.locator('.animate-pulse')).not.toBeVisible();
        await expect(page.getByText('FinOps Dashboard').first()).toBeVisible();

        // Budget metrics
        await expect(page.getByText('$4,500.00', { exact: true })).toBeVisible(); // Monthly cost
        await expect(page.getByText('90.0%', { exact: true })).toBeVisible(); // Budget used
    });

    test('should display cost breakdown', async ({ page }) => {
        // Wait for loading to finish
        await expect(page.locator('.animate-pulse')).not.toBeVisible();

        // Check for card title
        await expect(page.getByText('Desglose de Costos por Pipeline')).toBeVisible();

        // Check for pipeline names in the list below the chart
        await expect(page.getByText('Data Ingestion')).toBeVisible();
        await expect(page.getByText('Model Training')).toBeVisible();
    });

    test('should switch periods', async ({ page }) => {
        // Wait for loading to finish
        await expect(page.locator('.animate-pulse')).not.toBeVisible();
        await page.getByRole('button', { name: 'Últimos 7 días' }).click();
        // Verify the button becomes active (class check or aria-pressed if implemented)
        // The component uses 'variant="default"' for active, which usually means a specific class.
        // We can check if the request was made to /api/finops/history?period=7d
        // But since we mocked it, we can just ensure no error occurred and UI is stable.
        await expect(page.getByRole('button', { name: 'Últimos 7 días' })).toHaveClass(/bg-nebula-accent-blue/);
    });

    test('should handle export', async ({ page }) => {
        // Mock console.log to verify export click
        const consoleLogs: string[] = [];
        page.on('console', msg => consoleLogs.push(msg.text()));

        await page.getByRole('button', { name: 'Exportar Reporte' }).click();

        expect(consoleLogs).toContain('Exporting FinOps report...');
    });
});
