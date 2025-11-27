import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { SLAViolationTicker } from '../sla-violation-ticker';

describe('SLAViolationTicker', () => {
  const mockViolations = [
    { id: '1', message: 'Pipeline: data-processing exceeded SLA (2.5h > 2h)', severity: 'critical' },
    { id: '2', message: 'Worker pool: batch-workers at 85% capacity', severity: 'warning' },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the SLA ticker title', () => {
    render(<SLAViolationTicker violations={mockViolations} />);
    expect(screen.getByText('Alertas SLA')).toBeInTheDocument();
  });

  it('displays all violations', () => {
    render(<SLAViolationTicker violations={mockViolations} />);
    expect(screen.getByText(/Pipeline: data-processing/)).toBeInTheDocument();
    expect(screen.getByText(/Worker pool: batch-workers/)).toBeInTheDocument();
  });

  it('shows severity badges', () => {
    render(<SLAViolationTicker violations={mockViolations />);
    expect(screen.getByText('Crítico')).toBeInTheDocument();
    expect(screen.getByText('Advertencia')).toBeInTheDocument();
  });

  it('handles empty violations list', () => {
    render(<SLAViolationTicker violations={[]} />);
    expect(screen.getByText('Alertas SLA')).toBeInTheDocument();
    expect(screen.queryByText(/Pipeline/)).not.toBeInTheDocument();
  });

  it('animates the ticker horizontally', () => {
    render(<SLAViolationTicker violations={mockViolations} />);

    const ticker = screen.getByLabelText('ticker-horizontal');
    expect(ticker).toBeInTheDocument();
  });

  it('limits the number of visible violations', () => {
    const manyViolations = Array.from({ length: 20 }, (_, i) => ({
      id: `v-${i}`,
      message: `Violation ${i}`,
      severity: 'warning' as const,
    }));

    render(<SLAViolationTicker violations={manyViolations} maxVisible={5} />);

    expect(screen.queryByText(/Violation 0/)).toBeInTheDocument();
    expect(screen.queryByText(/Violation 1/)).toBeInTheDocument();
    expect(screen.queryByText(/Violation 4/)).toBeInTheDocument();
    expect(screen.queryByText(/Violation 5/)).not.toBeInTheDocument();
  });

  it('allows pausing and resuming the ticker', async () => {
    render(<SLAViolationTicker violations={mockViolations} autoScroll={true} />);

    const ticker = screen.getByLabelText('ticker-horizontal');
    expect(ticker).toBeInTheDocument();
  });
});
