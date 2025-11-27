import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { Button } from '@/components/ui/button';
import { WorkerPool } from '@/types';
import { UsersIcon, TrendingUpIcon, TrendingDownIcon } from 'lucide-react';

interface WorkerPoolCardProps {
  pool: WorkerPool;
  onScaleUp?: (id: string) => void;
  onScaleDown?: (id: string) => void;
  onConfigure?: (id: string) => void;
  className?: string;
}

const poolTypeConfig = {
  static: {
    label: 'Estático',
    badge: 'pending' as const,
  },
  dynamic: {
    label: 'Dinámico',
    badge: 'running' as const,
  },
};

export function WorkerPoolCard({
  pool,
  onScaleUp,
  onScaleDown,
  onConfigure,
  className,
}: WorkerPoolCardProps) {
  const config = poolTypeConfig[pool.type];
  const utilizationPercentage = pool.maxWorkers > 0
    ? (pool.currentWorkers / pool.maxWorkers) * 100
    : 0;

  const canScaleUp = pool.currentWorkers < pool.maxWorkers;
  const canScaleDown = pool.currentWorkers > pool.minWorkers;

  return (
    <Card className={cn('hover:bg-nebula-surface-secondary/50 transition-colors', className)}>
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <CardTitle className="text-lg text-nebula-text-primary mb-1">
              {pool.name}
            </CardTitle>
            <div className="flex items-center gap-2">
              <StatusBadge status={config.badge} label={config.label} />
              <span className="text-xs text-nebula-text-secondary">
                {pool.type === 'dynamic' ? 'Auto-scaling' : 'Fixed'}
              </span>
            </div>
          </div>
          <UsersIcon className="w-5 h-5 text-nebula-text-secondary" />
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="bg-nebula-surface-secondary rounded p-3">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-nebula-text-secondary">Workers Activos</span>
            <span className="text-sm font-semibold text-nebula-text-primary">
              {pool.currentWorkers} / {pool.maxWorkers}
            </span>
          </div>
          <div className="w-full bg-nebula-surface-primary rounded-full h-2">
            <div
              className={cn(
                'h-2 rounded-full transition-all',
                utilizationPercentage > 90
                  ? 'bg-nebula-accent-red'
                  : utilizationPercentage > 70
                  ? 'bg-nebula-accent-yellow'
                  : 'bg-nebula-accent-green'
              )}
              style={{ width: `${utilizationPercentage}%` }}
            />
          </div>
        </div>

        {pool.type === 'dynamic' && (
          <div className="grid grid-cols-3 gap-2 text-xs">
            <div className="bg-nebula-surface-secondary rounded p-2">
              <div className="text-nebula-text-secondary">Mín</div>
              <div className="text-nebula-text-primary font-semibold">{pool.minWorkers}</div>
            </div>
            <div className="bg-nebula-surface-secondary rounded p-2">
              <div className="text-nebula-text-secondary">Máx</div>
              <div className="text-nebula-text-primary font-semibold">{pool.maxWorkers}</div>
            </div>
            <div className="bg-nebula-surface-secondary rounded p-2">
              <div className="text-nebula-text-secondary">Actual</div>
              <div className="text-nebula-text-primary font-semibold">{pool.currentWorkers}</div>
            </div>
          </div>
        )}

        <div className="space-y-2">
          <div className="text-xs text-nebula-text-secondary font-medium">
            Política de Scaling
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="bg-nebula-surface-secondary rounded p-2">
              <div className="text-nebula-text-secondary">CPU</div>
              <div className="text-nebula-text-primary">{pool.scalingPolicy.cpuThreshold}%</div>
            </div>
            <div className="bg-nebula-surface-secondary rounded p-2">
              <div className="text-nebula-text-secondary">Memoria</div>
              <div className="text-nebula-text-primary">{pool.scalingPolicy.memoryThreshold}%</div>
            </div>
            <div className="bg-nebula-surface-secondary rounded p-2">
              <div className="text-nebula-text-secondary">Cola</div>
              <div className="text-nebula-text-primary">{pool.scalingPolicy.queueDepthThreshold}</div>
            </div>
          </div>
        </div>

        <div className="flex gap-2 pt-2">
          {pool.type === 'static' && (
            <>
              {canScaleDown && onScaleDown && (
                <Button
                  onClick={() => onScaleDown(pool.id)}
                  size="sm"
                  variant="outline"
                  className="flex-1 border-nebula-accent-red text-nebula-accent-red hover:bg-nebula-accent-red/20"
                >
                  <TrendingDownIcon className="w-4 h-4 mr-1" />
                  -1
                </Button>
              )}
              {canScaleUp && onScaleUp && (
                <Button
                  onClick={() => onScaleUp(pool.id)}
                  size="sm"
                  className="flex-1 bg-nebula-accent-green hover:bg-nebula-accent-green/80"
                >
                  <TrendingUpIcon className="w-4 h-4 mr-1" />
                  +1
                </Button>
              )}
            </>
          )}
          {pool.type === 'dynamic' && (
            <div className="flex-1 text-center text-xs text-nebula-text-secondary">
              Auto-scaling habilitado
            </div>
          )}
          {onConfigure && (
            <Button
              onClick={() => onConfigure(pool.id)}
              size="sm"
              variant="outline"
              className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
            >
              Configurar
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
