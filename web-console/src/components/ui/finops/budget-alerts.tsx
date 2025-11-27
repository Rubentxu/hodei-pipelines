import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { StatusBadge } from '@/components/ui/status-badge';
import { BudgetAlert } from '@/types';
import { AlertTriangleIcon, AlertCircleIcon, XIcon } from 'lucide-react';

interface BudgetAlertsProps {
  alerts: BudgetAlert[];
  onDismiss?: (id: string) => void;
  className?: string;
}

const alertConfig = {
  critical: {
    icon: AlertCircleIcon,
    badge: 'error' as const,
    label: 'Crítico',
    bgColor: 'bg-nebula-accent-red/10',
    borderColor: 'border-nebula-accent-red/50',
  },
  warning: {
    icon: AlertTriangleIcon,
    badge: 'warning' as const,
    label: 'Advertencia',
    bgColor: 'bg-nebula-accent-yellow/10',
    borderColor: 'border-nebula-accent-yellow/50',
  },
};

export function BudgetAlerts({ alerts, onDismiss, className }: BudgetAlertsProps) {
  if (alerts.length === 0) {
    return null;
  }

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary">
          Alertas de Presupuesto
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {alerts.map((alert) => {
          const config = alertConfig[alert.type];
          const AlertIcon = config.icon;

          return (
            <div
              key={alert.id}
              className={cn(
                'p-4 rounded-lg border',
                config.bgColor,
                config.borderColor
              )}
            >
              <div className="flex items-start gap-3">
                <AlertIcon className="w-5 h-5 text-nebula-accent-red mt-0.5" />
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <StatusBadge status={config.badge} label={config.label} />
                    <span className="text-xs text-nebula-text-secondary">
                      {new Date(alert.date).toLocaleDateString('es-ES', {
                        day: '2-digit',
                        month: 'short',
                        hour: '2-digit',
                        minute: '2-digit',
                      })}
                    </span>
                  </div>
                  <p className="text-sm text-nebula-text-primary mb-2">
                    {alert.message}
                  </p>
                  <div className="flex items-center gap-4 text-xs">
                    <span className="text-nebula-text-secondary">
                      Umbral: {alert.threshold}%
                    </span>
                    <span className="text-nebula-text-secondary">
                      Actual: {alert.current.toFixed(1)}%
                    </span>
                  </div>
                </div>
                {onDismiss && (
                  <Button
                    onClick={() => onDismiss(alert.id)}
                    variant="ghost"
                    size="sm"
                    className="text-nebula-text-secondary hover:text-nebula-accent-red hover:bg-nebula-accent-red/20"
                  >
                    <XIcon className="w-4 h-4" />
                  </Button>
                )}
              </div>
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
