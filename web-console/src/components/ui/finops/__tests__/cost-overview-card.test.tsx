import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CostOverviewCard } from '../cost-overview-card';

describe('CostOverviewCard', () => {
  const mockMetrics = {
    dailyCost: 125.50,
    monthlyCost: 3200.75,
    yearlyCost: 38409.00,
    projectedMonthlyCost: 3400.00,
    budgetLimit: 5000.00,
    budgetUsed: 3200.75,
    costPerJob: 12.50,
    costPerHour: 45.00,
    trend: 'up' as const,
    trendPercentage: 8.5,
  };

  it('renders the cost overview title', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText('Resumen de Costos')).toBeInTheDocument();
  });

  it('displays daily cost', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText('$125.50')).toBeInTheDocument();
    expect(screen.getByText(/costo diario/i)).toBeInTheDocument();
  });

  it('displays monthly cost', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText('$3,200.75')).toBeInTheDocument();
    expect(screen.getByText(/costo mensual/i)).toBeInTheDocument();
  });

  it('displays yearly cost', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText('$38,409.00')).toBeInTheDocument();
    expect(screen.getByText(/costo anual/i)).toBeInTheDocument();
  });

  it('shows budget usage percentage', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText(/64.0%/)).toBeInTheDocument();
  });

  it('displays cost per job', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText('$12.50')).toBeInTheDocument();
    expect(screen.getByText(/por job/i)).toBeInTheDocument();
  });

  it('displays cost per hour', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText('$45.00')).toBeInTheDocument();
    expect(screen.getByText(/por hora/i)).toBeInTheDocument();
  });

  it('shows trend indicator', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText(/↗ 8.5%/i)).toBeInTheDocument();
    expect(screen.getByText(/tendencia/i)).toBeInTheDocument();
  });

  it('displays projected monthly cost', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    expect(screen.getByText('$3,400.00')).toBeInTheDocument();
    expect(screen.getByText(/proyección mensual/i)).toBeInTheDocument();
  });

  it('shows budget progress bar', () => {
    render(<CostOverviewCard metrics={mockMetrics} />);
    const progressBar = document.querySelector('.budget-progress');
    expect(progressBar).toBeInTheDocument();
  });
});
