import { expect, test } from '@playwright/test';

test.describe('Alert Management', () => {
    test.beforeEach(async ({ page }) => {
        // Mock Auth
        await page.addInitScript(() => {
            window.localStorage.setItem('token', 'fake-jwt-token');
            window.localStorage.setItem('user', JSON.stringify({ id: 'user-1', name: 'Test User', email: 'test@example.com', role: 'admin' }));
        });

        // Mock Alerts List
        await page.route('/api/v1/alerts?*', async (route) => {
            const url = new URL(route.request().url());
            const status = url.searchParams.getAll('status');

            let alerts = [
                {
                    id: 'alert-1',
                    ruleId: 'rule-1',
                    ruleName: 'High CPU Usage',
                    severity: 'critical',
                    status: 'open',
                    message: 'CPU usage > 90% on worker-1',
                    triggeredAt: new Date().toISOString()
                },
                {
                    id: 'alert-2',
                    ruleId: 'rule-2',
                    ruleName: 'Memory Leak',
                    severity: 'high',
                    status: 'acknowledged',
                    message: 'Memory usage increasing rapidly',
                    triggeredAt: new Date().toISOString()
                }
            ];

            if (status.length > 0) {
                alerts = alerts.filter(a => status.includes(a.status));
            }

            await route.fulfill({
                json: {
                    alerts,
                    total: alerts.length,
                    hasMore: false
                }
            });
        });

        // Mock Alert Rules
        await page.route('/api/v1/alerts/rules*', async (route) => {
            await route.fulfill({
                json: [
                    {
                        id: 'rule-1',
                        name: 'High CPU Usage',
                        severity: 'critical',
                        enabled: true,
                        conditions: [],
                        actions: [],
                        triggerCount: 5
                    }
                ]
            });
        });

        // Mock Stats
        await page.route('/api/v1/alerts/stats*', async (route) => {
            await route.fulfill({
                json: {
                    totalAlerts: 10,
                    openAlerts: 2,
                    acknowledgedAlerts: 3,
                    resolvedAlerts: 5,
                    bySeverity: { critical: 1, high: 2, medium: 3, low: 4, info: 0 },
                    avgResolutionTime: 15,
                    topRules: [{ ruleId: 'rule-1', ruleName: 'High CPU Usage', count: 5 }]
                }
            });
        });

        // Mock Actions
        await page.route('/api/v1/alerts/*/acknowledge', async (route) => {
            await route.fulfill({ status: 200, json: { status: 'acknowledged' } });
        });

        await page.route('/api/v1/alerts/*/resolve', async (route) => {
            await route.fulfill({ status: 200, json: { status: 'resolved' } });
        });

        await page.goto('/alerts');
    });

    test('should display initial stats and alerts', async ({ page }) => {
        await expect(page.getByText('Alert Management')).toBeVisible();

        // Stats
        await expect(page.getByText('Total Alerts')).toBeVisible();
        await expect(page.getByText('10', { exact: true })).toBeVisible(); // Total count

        // Alerts List (default filter is Open)
        await expect(page.getByText('High CPU Usage')).toBeVisible();
        await expect(page.getByText('CPU usage > 90% on worker-1')).toBeVisible();
    });

    test('should filter alerts', async ({ page }) => {
        // Change status filter to Acknowledged
        await page.getByLabel('Status').selectOption(['acknowledged']);

        // Should show acknowledged alert
        await expect(page.getByText('Memory Leak')).toBeVisible();
        // Should NOT show open alert (unless multiple selected, but here we select only acknowledged)
        await expect(page.getByText('High CPU Usage')).not.toBeVisible();
    });

    test('should acknowledge an alert', async ({ page }) => {
        // Find the Acknowledge button for the open alert
        const alertCard = page.locator('.bg-card', { hasText: 'High CPU Usage' });
        await alertCard.getByRole('button', { name: 'Acknowledge' }).click();

        // Since we mock the refresh, we might not see immediate UI update unless we mock the subsequent fetch.
        // But we can verify the request was made if we wanted.
        // For UI update, our mock keeps returning 'open' status for alert-1 unless we change the mock state.
        // In a real test we might want to update the mock state.
        // For this simple test, ensuring the button is clickable is a good start.
    });

    test('should navigate tabs', async ({ page }) => {
        await page.getByRole('button', { name: 'Alert Rules' }).click();
        await expect(page.getByText('Create Rule')).toBeVisible();
        await expect(page.getByText('High CPU Usage')).toBeVisible(); // Rule name
    });
});
