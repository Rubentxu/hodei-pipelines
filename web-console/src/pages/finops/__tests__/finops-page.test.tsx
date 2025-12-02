import * as useFinOpsMetricsHook from '@/hooks/useFinOpsMetrics';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FinOpsPage } from '../finops-page';

// Mock subcomponents
vi.mock('@/components/ui/finops/cost-overview-card', () => ({
    CostOverviewCard: () => <div data-testid="cost-overview-card">Cost Overview</div>,
}));
vi.mock('@/components/ui/finops/cost-breakdown-chart', () => ({
    CostBreakdownChart: () => <div data-testid="cost-breakdown-chart">Cost Breakdown</div>,
}));
vi.mock('@/components/ui/finops/cost-history-chart', () => ({
    CostHistoryChart: () => <div data-testid="cost-history-chart">Cost History</div>,
}));
vi.mock('@/components/ui/finops/budget-alerts', () => ({
    BudgetAlerts: () => <div data-testid="budget-alerts">Budget Alerts</div>,
}));

// Mock hook
vi.mock('@/hooks/useFinOpsMetrics', () => ({
    useFinOpsMetrics: vi.fn(),
}));

describe('FinOpsPage', () => {
    const mockUseFinOpsMetrics = useFinOpsMetricsHook.useFinOpsMetrics as any;

    const mockData = {
        metrics: {
            budgetLimit: 1000,
            budgetUsed: 500,
            projectedMonthlyCost: 1100,
        },
        history: [],
        breakdown: [],
        alerts: [],
        lastUpdated: new Date().toISOString(),
    };

    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('renders loading state', () => {
        mockUseFinOpsMetrics.mockReturnValue({
            isLoading: true,
            isError: false,
            data: null,
        });

        render(<FinOpsPage />);
        // Check for loading skeleton elements or structure
        expect(screen.getByText('FinOps Dashboard')).toBeInTheDocument();
        // We can check for a specific class that indicates loading, e.g., animate-pulse
        const skeleton = document.querySelector('.animate-pulse');
        expect(skeleton).toBeInTheDocument();
    });

    it('renders error state', () => {
        mockUseFinOpsMetrics.mockReturnValue({
            isLoading: false,
            isError: true,
            error: { message: 'Failed to fetch' },
            data: null,
        });

        render(<FinOpsPage />);
        expect(screen.getByText(/Error al cargar métricas FinOps/)).toBeInTheDocument();
        expect(screen.getByText(/Failed to fetch/)).toBeInTheDocument();
    });

    it('renders dashboard with data', () => {
        mockUseFinOpsMetrics.mockReturnValue({
            isLoading: false,
            isError: false,
            data: mockData,
        });

        render(<FinOpsPage />);
        expect(screen.getByText('FinOps Dashboard')).toBeInTheDocument();
        expect(screen.getByTestId('cost-overview-card')).toBeInTheDocument();
        expect(screen.getByTestId('cost-breakdown-chart')).toBeInTheDocument();
        expect(screen.getByTestId('cost-history-chart')).toBeInTheDocument();
        expect(screen.getByTestId('budget-alerts')).toBeInTheDocument();

        // Check specific data rendering
        expect(screen.getByText(/\$1[.,]?000/)).toBeInTheDocument(); // Budget Limit
        expect(screen.getByText(/50[.,]0%/)).toBeInTheDocument(); // Budget Used %
        expect(screen.getByText(/\$1[.,]?100/)).toBeInTheDocument(); // Projected
    });

    it('handles period selection', () => {
        mockUseFinOpsMetrics.mockReturnValue({
            isLoading: false,
            isError: false,
            data: mockData,
        });

        render(<FinOpsPage />);

        const periodButton = screen.getByText('Últimos 90 días');
        fireEvent.click(periodButton);

        // Since state is local, we can verify the button style changes or just that it's clickable
        // In a real integration test we would check if it triggers a refetch if that logic existed
        expect(periodButton).toHaveClass('bg-nebula-accent-blue');
    });
});
