import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CostBreakdownChart } from '../cost-breakdown-chart';

vi.mock('echarts', () => ({
  init: vi.fn(() => ({
    setOption: vi.fn(),
    resize: vi.fn(),
    dispose: vi.fn(),
  })),
}));

describe('CostBreakdownChart', () => {
  const mockBreakdown = [
    { pipelineId: '1', pipelineName: 'Data Processing', cost: 1200, percentage: 37.5, jobCount: 45, avgDuration: 125 },
    { pipelineId: '2', pipelineName: 'ETL Pipeline', cost: 800, percentage: 25, jobCount: 30, avgDuration: 200 },
    { pipelineId: '3', pipelineName: 'Report Generation', cost: 600, percentage: 18.75, jobCount: 25, avgDuration: 90 },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the breakdown chart title', () => {
    render(<CostBreakdownChart breakdown={mockBreakdown} />);
    expect(screen.getByText('Desglose de Costos por Pipeline')).toBeInTheDocument();
  });

  it('displays all pipeline names', () => {
    render(<CostBreakdownChart breakdown={mockBreakdown} />);
    expect(screen.getByText('Data Processing')).toBeInTheDocument();
    expect(screen.getByText('ETL Pipeline')).toBeInTheDocument();
    expect(screen.getByText('Report Generation')).toBeInTheDocument();
  });

  it('displays cost for each pipeline', () => {
    render(<CostBreakdownChart breakdown={mockBreakdown} />);
    expect(screen.getByText('$1,200.00')).toBeInTheDocument();
    expect(screen.getByText('$800.00')).toBeInTheDocument();
    expect(screen.getByText('$600.00')).toBeInTheDocument();
  });

  it('displays percentage for each pipeline', () => {
    render(<CostBreakdownChart breakdown={mockBreakdown} />);
    expect(screen.getByText('37.5%')).toBeInTheDocument();
    expect(screen.getByText('25%')).toBeInTheDocument();
    expect(screen.getByText('18.75%')).toBeInTheDocument();
  });

  it('displays job count', () => {
    render(<CostBreakdownChart breakdown={mockBreakdown} />);
    expect(screen.getByText(/45 jobs/)).toBeInTheDocument();
    expect(screen.getByText(/30 jobs/)).toBeInTheDocument();
    expect(screen.getByText(/25 jobs/)).toBeInTheDocument();
  });

  it('displays average duration', () => {
    render(<CostBreakdownChart breakdown={mockBreakdown} />);
    expect(screen.getByText(/2m 5s/)).toBeInTheDocument();
    expect(screen.getByText(/3m 20s/)).toBeInTheDocument();
    expect(screen.getByText(/1m 30s/)).toBeInTheDocument();
  });

  it('handles empty breakdown', () => {
    render(<CostBreakdownChart breakdown={[]} />);
    expect(screen.getByText('Desglose de Costos por Pipeline')).toBeInTheDocument();
    expect(screen.queryByText('Data Processing')).not.toBeInTheDocument();
  });

  it('displays total cost', () => {
    render(<CostBreakdownChart breakdown={mockBreakdown} />);
    expect(screen.getByText('Total:')).toBeInTheDocument();
    expect(screen.getByText('$2,600.00')).toBeInTheDocument();
  });
});
