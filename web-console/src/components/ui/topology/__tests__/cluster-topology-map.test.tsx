import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { ClusterTopologyMap } from '../cluster-topology-map';

describe('ClusterTopologyMap', () => {
  const mockNodes = [
    { id: 'node-1', name: 'Master Node', type: 'master', status: 'healthy' },
    { id: 'node-2', name: 'Worker Node 1', type: 'worker', status: 'healthy' },
    { id: 'node-3', name: 'Worker Node 2', type: 'worker', status: 'warning' },
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
    expect(screen.getByText('Worker Node 1')).toBeInTheDocument();
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

    expect(screen.getByText('Master')).toBeInTheDocument();
    expect(screen.getAllByText('Worker').length).toBe(2);
  });

  it('shows connection count', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);
    expect(screen.getByText(/conexiones: 2/)).toBeInTheDocument();
  });

  it('renders the SVG visualization', () => {
    render(<ClusterTopologyMap nodes={mockNodes} edges={mockEdges} />);
    expect(screen.getByRole('img')).toBeInTheDocument();
  });
});
