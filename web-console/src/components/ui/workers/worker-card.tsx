import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { Button } from '@/components/ui/button';
import { Worker } from '@/types';
import { CpuIcon, HardDriveIcon, ActivityIcon, SettingsIcon } from 'lucide-react';

interface WorkerCardProps {
  worker: Worker;
  onStart?: (id: string) => void;
  onStop?: (id: string) => void;
  onRestart?: (id: string) => void;
  onConfigure?: (id: string) => void;
  className?: string;
}

const statusConfig = {
  healthy: {
    badge: 'success' as const,
    label: 'Saludable',
    icon: ActivityIcon,
  },
  failed: {
    badge: 'error' as const,
    label: 'Fallo',
    icon: ActivityIcon,
  },
  maintenance: {
    badge: 'warning' as const,
    label: 'Mantenimiento',
    icon: SettingsIcon,
  },
};

export function WorkerCard({
  worker,
  onStart,
  onStop,
  onRestart,
  onConfigure,
  className,
}: WorkerCardProps) {
  const config = statusConfig[worker.status];
  const StatusIcon = config.icon;

  const utilizationPercentage = worker.maxJobs > 0
    ? (worker.currentJobs / worker.maxJobs) * 100
    : 0;

  return (
    <Card className={cn('hover:bg-nebula-surface-secondary/50 transition-colors', className)}>
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <CardTitle className="text-lg text-nebula-text-primary mb-1">
              {worker.name}
            </CardTitle>
            <div className="flex items-center gap-2">
              <StatusBadge status={config.badge} label={config.label} />
              <span className="text-xs text-nebula-text-secondary">
                Pool: {worker.poolId}
              </span>
            </div>
          </div>
          <StatusIcon className="w-5 h-5 text-nebula-text-secondary" />
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-3">
          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="flex items-center gap-2 text-xs text-nebula-text-secondary mb-1">
              <CpuIcon className="w-3 h-3" />
              <span>Jobs Actuales</span>
            </div>
            <div className="text-lg font-semibold text-nebula-text-primary">
              {worker.currentJobs} / {worker.maxJobs}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="flex items-center gap-2 text-xs text-nebula-text-secondary mb-1">
              <ActivityIcon className="w-3 h-3" />
              <span>Utilización</span>
            </div>
            <div className="text-lg font-semibold text-nebula-text-primary">
              {utilizationPercentage.toFixed(0)}%
            </div>
          </div>
        </div>

        <div className="space-y-1">
          <div className="flex justify-between text-xs text-nebula-text-secondary">
            <span>Carga de trabajo</span>
            <span>{utilizationPercentage.toFixed(0)}%</span>
          </div>
          <div className="w-full bg-nebula-surface-secondary rounded-full h-2">
            <div
              className={cn(
                'h-2 rounded-full transition-all',
                utilizationPercentage > 80
                  ? 'bg-nebula-accent-red'
                  : utilizationPercentage > 60
                  ? 'bg-nebula-accent-yellow'
                  : 'bg-nebula-accent-green'
              )}
              style={{ width: `${utilizationPercentage}%` }}
            />
          </div>
        </div>

        <div className="flex gap-2 pt-2">
          {worker.status === 'healthy' && onStop && (
            <Button
              onClick={() => onStop(worker.id)}
              size="sm"
              variant="outline"
              className="flex-1 border-nebula-accent-red text-nebula-accent-red hover:bg-nebula-accent-red/20"
            >
              Detener
            </Button>
          )}
          {worker.status === 'failed' && onStart && (
            <Button
              onClick={() => onStart(worker.id)}
              size="sm"
              className="flex-1 bg-nebula-accent-green hover:bg-nebula-accent-green/80"
            >
              Iniciar
            </Button>
          )}
          {onRestart && (
            <Button
              onClick={() => onRestart(worker.id)}
              size="sm"
              variant="outline"
              className="border-nebula-accent-blue text-nebula-accent-blue hover:bg-nebula-accent-blue/20"
            >
              Reiniciar
            </Button>
          )}
          {onConfigure && (
            <Button
              onClick={() => onConfigure(worker.id)}
              size="sm"
              variant="outline"
              className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
            >
              <SettingsIcon className="w-4 h-4" />
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
