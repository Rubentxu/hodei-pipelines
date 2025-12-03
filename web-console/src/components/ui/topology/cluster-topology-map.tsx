import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { cn } from '@/utils/cn';

interface Node {
  id: string;
  name: string;
  type: 'master' | 'worker' | 'storage' | 'control_plane';
  status: 'healthy' | 'warning' | 'critical';
  cpuUsage?: number;
  memoryUsage?: number;
}

interface Edge {
  source: string;
  target: string;
}

interface ClusterTopologyMapProps {
  nodes: Node[];
  edges: Edge[];
  className?: string;
}

const nodeTypeConfig = {
  master: {
    size: 60,
    color: 'fill-nebula-accent-blue',
    label: 'Master',
  },
  worker: {
    size: 45,
    color: 'fill-nebula-accent-purple',
    label: 'Worker',
  },
  storage: {
    size: 50,
    color: 'fill-nebula-accent-cyan',
    label: 'Storage',
  },
  control_plane: {
    size: 60,
    color: 'fill-nebula-accent-blue',
    label: 'Control Plane',
  },
};

const statusConfig = {
  healthy: {
    badge: 'success',
    stroke: 'stroke-nebula-accent-green',
    label: 'Saludable',
  },
  warning: {
    badge: 'warning',
    stroke: 'stroke-nebula-accent-yellow',
    label: 'Advertencia',
  },
  critical: {
    badge: 'error',
    stroke: 'stroke-nebula-accent-red',
    label: 'Crítico',
  },
};

export function ClusterTopologyMap({
  nodes,
  edges,
  className,
}: ClusterTopologyMapProps) {
  const width = 800;
  const height = 400;
  const centerX = width / 2;
  const centerY = height / 2;
  const radius = Math.min(width, height) / 3;

  const getNodePosition = (index: number, total: number) => {
    if (total === 1) {
      return { x: centerX, y: centerY };
    }

    const angle = (index * 2 * Math.PI) / total - Math.PI / 2;
    return {
      x: centerX + radius * Math.cos(angle),
      y: centerY + radius * Math.sin(angle),
    };
  };

  const masterNodes = nodes.filter(n => n.type === 'master' || n.type === 'control_plane');
  const workerNodes = nodes.filter(n => n.type === 'worker');

  const positionedNodes = nodes.map((node, index) => {
    const position = getNodePosition(index, nodes.length);
    return { ...node, ...position };
  });

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary">
          Topología del Cluster
        </CardTitle>
        <div className="flex items-center gap-4 text-sm text-nebula-text-secondary">
          <span>{nodes.length} nodos</span>
          <span>{edges.length} conexiones</span>
        </div>
      </CardHeader>
      <CardContent>
        <div className="relative">
          <svg
            role="img"
            width={width}
            height={height}
            className="w-full h-auto border border-nebula-surface-secondary rounded"
            viewBox={`0 0 ${width} ${height}`}
          >
            <rect
              width={width}
              height={height}
              fill="var(--nebula-surface-primary)"
            />

            <defs>
              <filter id="shadow" x="-50%" y="-50%" width="200%" height="200%">
                <feDropShadow
                  dx="0"
                  dy="2"
                  stdDeviation="3"
                  floodOpacity="0.3"
                />
              </filter>
            </defs>

            {edges.map((edge, index) => {
              const sourceNode = positionedNodes.find(n => n.id === edge.source);
              const targetNode = positionedNodes.find(n => n.id === edge.target);

              if (!sourceNode || !targetNode) return null;

              return (
                <line
                  key={index}
                  x1={sourceNode.x}
                  y1={sourceNode.y}
                  x2={targetNode.x}
                  y2={targetNode.y}
                  stroke="var(--nebula-surface-secondary)"
                  strokeWidth="2"
                  strokeDasharray={sourceNode.type === 'master' || sourceNode.type === 'control_plane' ? '5,5' : 'none'}
                  opacity="0.6"
                />
              );
            })}

            {positionedNodes.map((node) => {
              const nodeType = nodeTypeConfig[node.type];
              const status = statusConfig[node.status];
              const size = nodeType.size;
              const radius = size / 2;

              return (
                <g key={node.id} transform={`translate(${node.x}, ${node.y})`}>
                  <circle
                    r={radius}
                    className={nodeType.color}
                    filter="url(#shadow)"
                    stroke={status.stroke.replace('stroke-', 'var(--')}
                    strokeWidth="2"
                  />

                  <circle
                    r={radius - 8}
                    fill="var(--nebula-surface-primary)"
                  />

                  <text
                    x="0"
                    y="-5"
                    textAnchor="middle"
                    fill="var(--nebula-text-primary)"
                    fontSize="10"
                    fontWeight="bold"
                  >
                    {node.name}
                  </text>

                  <text
                    x="0"
                    y="8"
                    textAnchor="middle"
                    fill="var(--nebula-text-secondary)"
                    fontSize="8"
                  >
                    {nodeType.label}
                  </text>

                  {node.cpuUsage !== undefined && (
                    <text
                      x="0"
                      y="20"
                      textAnchor="middle"
                      fill="var(--nebula-text-secondary)"
                      fontSize="7"
                    >
                      CPU: {node.cpuUsage}%
                    </text>
                  )}
                </g>
              );
            })}
          </svg>
        </div>

        <div className="mt-4 flex flex-wrap gap-2">
          {['control_plane', 'worker', 'storage'].map((type) => {
            const config = nodeTypeConfig[type as keyof typeof nodeTypeConfig];
            return (
              <div key={type} className="flex items-center gap-2 text-xs">
                <div
                  className={`w-3 h-3 rounded-full ${config.color}`}
                />
                <span className="text-nebula-text-secondary">{config.label}</span>
              </div>
            );
          })}
        </div>

        <div className="mt-2 flex flex-wrap gap-2">
          {['healthy', 'warning', 'critical'].map((status) => {
            const config = statusConfig[status as keyof typeof statusConfig];
            return (
              <StatusBadge
                key={status}
                status={config.badge as any}
                label={config.label}
                className="text-xs"
              />
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
