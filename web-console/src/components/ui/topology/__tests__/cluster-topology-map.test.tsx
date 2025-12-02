import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ClusterTopologyMap } from '../cluster-topology-map';

describe('ClusterTopologyMap', () => {
  const mockNodes = [
    {
      id: 'node-1',
      name: 'Master Node',
      type: 'master' as const,
      status: 'healthy' as const,
      cpuUsage: 45,
      memoryUsage: 60,
    },
    {
      id: 'node-2',
      name: 'Worker 1',
      type: 'worker' as const,
      status: 'warning' as const,
      cpuUsage: 80,
      memoryUsage: 75,
    },
    {
      id: 'node-3',
      name: 'Worker Node 2',
      type: 'worker' as const,
      status: 'warning' as const,
      cpuUsage: 20,
      memoryUsage: 30,
    },
  ];

  const mockEdges = [
    { source: 'node-1', target: 'node-2' },
    { source: 'node-1', target: 'node-3' },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the topology map title', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);
    expect(screen.getByText('Topología del Cluster')).toBeInTheDocument();
  });

  it('displays all nodes', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);

    expect(screen.getByText('Master Node')).toBeInTheDocument();
    expect(screen.getByText('Worker 1')).toBeInTheDocument();
    expect(screen.getByText('Worker Node 2')).toBeInTheDocument();
  });

  it('shows node status badges', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);

    const healthyBadge = screen.getByText('Saludable');
    const warningBadge = screen.getByText('Advertencia');

    expect(healthyBadge).toBeInTheDocument();
    expect(warningBadge).toBeInTheDocument();
  });

  it('handles empty nodes gracefully', () => {
    render(<ClusterTopologyMap nodes={[]} edges={[]} />);
    expect(screen.getByText('Topología del Cluster')).toBeInTheDocument();
  });

  it('displays node type labels', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);

    expect(screen.getAllByText('Master').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Worker').length).toBeGreaterThan(0);
  });

  it('shows connection count', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);
    expect(screen.getByText(/2 conexiones/)).toBeInTheDocument();
  });

  it('renders the SVG visualization', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);
    expect(screen.getByRole('img')).toBeInTheDocument();
  });
});
