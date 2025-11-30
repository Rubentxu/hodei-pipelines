import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as useObservabilityHook from '../../../hooks/useObservability';
import { ObservabilityPage } from '../observability-page';

// Mock subcomponents
vi.mock('../../../components/ui/observability/metrics-dashboard', () => ({
    MetricsDashboard: () => <div data-testid="metrics-dashboard">Metrics Dashboard</div>,
}));
vi.mock('../../../components/ui/observability/log-viewer', () => ({
    LogViewer: () => <div data-testid="log-viewer">Log Viewer</div>,
}));
vi.mock('../../../components/ui/observability/trace-viewer', () => ({
    TraceViewer: () => <div data-testid="trace-viewer">Trace Viewer</div>,
}));
vi.mock('../../../components/ui/observability/alert-rules', () => ({
    AlertRules: () => <div data-testid="alert-rules">Alert Rules</div>,
}));
vi.mock('../../../components/ui/observability/service-map', () => ({
    ServiceMap: () => <div data-testid="service-map">Service Map</div>,
}));

// Mock hooks
vi.mock('../../../hooks/useObservability', () => ({
    useAlerts: vi.fn(),
    useAcknowledgeAlert: vi.fn(),
}));

describe('ObservabilityPage', () => {
    const mockUseAlerts = useObservabilityHook.useAlerts as any;
    const mockUseAcknowledgeAlert = useObservabilityHook.useAcknowledgeAlert as any;
    const mockAcknowledgeMutation = {
        mutateAsync: vi.fn(),
        isPending: false,
    };

    beforeEach(() => {
        vi.clearAllMocks();
        mockUseAcknowledgeAlert.mockReturnValue(mockAcknowledgeMutation);
        mockUseAlerts.mockReturnValue({
            data: {
                pages: [
                    {
                        alerts: [
                            {
                                id: '1',
                                ruleName: 'High CPU',
                                message: 'CPU usage > 90%',
                                severity: 'critical',
                                timestamp: new Date().toISOString(),
                            },
                        ],
                    },
                ],
            },
            isLoading: false,
            fetchNextPage: vi.fn(),
            hasNextPage: false,
        });
    });

    it('renders default tab (Metrics)', () => {
        render(<ObservabilityPage />);
        expect(screen.getByText('Advanced Observability')).toBeInTheDocument();
        expect(screen.getByTestId('metrics-dashboard')).toBeInTheDocument();
    });

    it('navigates between tabs', () => {
        render(<ObservabilityPage />);

        // Switch to Logs
        fireEvent.click(screen.getByText('Logs'));
        expect(screen.getByTestId('log-viewer')).toBeInTheDocument();
        expect(screen.queryByTestId('metrics-dashboard')).not.toBeInTheDocument();

        // Switch to Traces
        fireEvent.click(screen.getByText('Traces'));
        expect(screen.getByTestId('trace-viewer')).toBeInTheDocument();

        // Switch to Services
        fireEvent.click(screen.getByText('Services'));
        expect(screen.getByTestId('service-map')).toBeInTheDocument();
    });

    it('renders Alerts tab and interacts with alerts', async () => {
        render(<ObservabilityPage />);

        // Switch to Alerts
        fireEvent.click(screen.getByText('Alerts'));

        // Default view is Alert Rules
        expect(screen.getByTestId('alert-rules')).toBeInTheDocument();

        // Switch to Active Alerts
        fireEvent.click(screen.getByText('Active Alerts'));

        // Check if alert is rendered
        expect(screen.getByText('High CPU')).toBeInTheDocument();
        expect(screen.getByText('CPU usage > 90%')).toBeInTheDocument();

        // Acknowledge alert
        const acknowledgeButton = screen.getByText('Acknowledge');
        fireEvent.click(acknowledgeButton);

        await waitFor(() => {
            expect(mockAcknowledgeMutation.mutateAsync).toHaveBeenCalledWith({
                alertId: '1',
                userId: 'current-user-id',
            });
        });
    });

    it('renders loading state for active alerts', () => {
        mockUseAlerts.mockReturnValue({
            isLoading: true,
            data: null,
        });

        render(<ObservabilityPage />);
        fireEvent.click(screen.getByText('Alerts'));
        fireEvent.click(screen.getByText('Active Alerts'));

        expect(screen.getByText('Loading alerts...')).toBeInTheDocument();
    });

    it('renders empty state for active alerts', () => {
        mockUseAlerts.mockReturnValue({
            isLoading: false,
            data: { pages: [{ alerts: [] }] },
        });

        render(<ObservabilityPage />);
        fireEvent.click(screen.getByText('Alerts'));
        fireEvent.click(screen.getByText('Active Alerts'));

        expect(screen.getByText('No active alerts')).toBeInTheDocument();
    });
});
