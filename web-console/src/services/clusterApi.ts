export interface ClusterNode {
  id: string;
  name: string;
  type: 'master' | 'worker' | 'storage';
  status: 'healthy' | 'warning' | 'critical';
  cpuUsage?: number;
  memoryUsage?: number;
}

export interface ClusterEdge {
  source: string;
  target: string;
}

export interface ClusterTopology {
  nodes: ClusterNode[];
  edges: ClusterEdge[];
}

export async function getClusterTopology(): Promise<ClusterTopology> {
  const response = await fetch('/api/observability/topology');

  if (!response.ok) {
    throw new Error('Failed to fetch cluster topology');
  }

  return response.json();
}

export async function subscribeToTopology(
  callback: (topology: ClusterTopology) => void
): Promise<() => void> {
  const eventSource = new EventSource('/api/observability/topology/stream');

  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      callback(data);
    } catch (error) {
      console.error('Failed to parse topology data:', error);
    }
  };

  eventSource.onerror = (error) => {
    console.error('EventSource error:', error);
  };

  return () => {
    eventSource.close();
  };
}
