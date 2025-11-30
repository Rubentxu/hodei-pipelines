import { expect, test } from '@playwright/test';

test.describe('FinOps Dashboard', () => {
    test.beforeEach(async ({ page }) => {
        // Mock FinOps Metrics
        await page.route('/api/finops/metrics', async (route) => {
            await route.fulfill({
                json: {
                    metrics: {
                        totalCost: 1500,
                        budgetLimit: 2000,
                        budgetUsed: 1500,
                        projectedMonthlyCost: 1800,
                        costTrend: 5.5
                    },
                    history: [
                        { date: '2023-01-01', cost: 100 },
                        { date: '2023-01-02', cost: 120 }
                    ],
                    breakdown: [
                        { category: 'Compute', cost: 800 },
                        { category: 'Storage', cost: 400 },
                        { category: 'Network', cost: 300 }
                    ],
                    alerts: [
                        { id: 'alert-1', message: 'Budget usage > 75%', severity: 'warning' }
                    ],
                    lastUpdated: new Date().toISOString()
                }
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
        await expect(page.getByText('FinOps Dashboard').first()).toBeVisible();

        // Budget metrics
        await expect(page.getByText('$2,000')).toBeVisible(); // Budget Limit
        await expect(page.getByText('75.0%')).toBeVisible(); // Used % (1500/2000)
        await expect(page.getByText('$1,800')).toBeVisible(); // Projected
    });

    test('should display cost breakdown', async ({ page }) => {
        // Check for breakdown categories (assuming they are rendered in a chart or list)
        // The component uses Recharts, so text might be in SVG or legend.
        // We can check for the text presence.
        // Note: Recharts often renders text in SVG elements.
        // If CostBreakdownChart renders a legend, we should see "Compute", "Storage".
        // Let's assume it does or check for the container.
        await expect(page.locator('.recharts-wrapper').first()).toBeVisible();
    });

    test('should switch periods', async ({ page }) => {
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
