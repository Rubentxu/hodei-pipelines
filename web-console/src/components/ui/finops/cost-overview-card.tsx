import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { TrendingUpIcon, TrendingDownIcon, DollarSignIcon } from 'lucide-react';
import { CostMetrics } from '@/types';

interface CostOverviewCardProps {
  metrics: CostMetrics;
  className?: string;
}

const trendIcons = {
  up: TrendingUpIcon,
  down: TrendingDownIcon,
  stable: DollarSignIcon,
};

const trendColors = {
  up: 'text-nebula-accent-red',
  down: 'text-nebula-accent-green',
  stable: 'text-nebula-text-secondary',
};

export function CostOverviewCard({ metrics, className }: CostOverviewCardProps) {
  const TrendIcon = trendIcons[metrics.trend];
  const budgetPercentage = (metrics.budgetUsed / metrics.budgetLimit) * 100;

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
    }).format(value);
  };

  const formatPercentage = (value: number) => {
    return `${value.toFixed(1)}%`;
  };

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary flex items-center gap-2">
          <DollarSignIcon className="w-5 h-5" />
          Resumen de Costos
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="text-xs text-nebula-text-secondary mb-1">Costo Diario</div>
            <div className="text-xl font-bold text-nebula-text-primary">
              {formatCurrency(metrics.dailyCost)}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="text-xs text-nebula-text-secondary mb-1">Costo Mensual</div>
            <div className="text-xl font-bold text-nebula-text-primary">
              {formatCurrency(metrics.monthlyCost)}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="text-xs text-nebula-text-secondary mb-1">Costo Anual</div>
            <div className="text-xl font-bold text-nebula-text-primary">
              {formatCurrency(metrics.yearlyCost)}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="text-xs text-nebula-text-secondary mb-1">Proyección Mensual</div>
            <div className="text-xl font-bold text-nebula-text-primary">
              {formatCurrency(metrics.projectedMonthlyCost)}
            </div>
          </div>
        </div>

        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span className="text-nebula-text-secondary">Presupuesto Utilizado</span>
            <span className="text-nebula-text-primary font-medium">
              {formatCurrency(metrics.budgetUsed)} / {formatCurrency(metrics.budgetLimit)}
              {' '}({formatPercentage(budgetPercentage)})
            </span>
          </div>
          <div className="w-full bg-nebula-surface-secondary rounded-full h-2.5">
            <div
              className={cn(
                'h-2.5 rounded-full transition-all',
                budgetPercentage > 90
                  ? 'bg-nebula-accent-red'
                  : budgetPercentage > 75
                  ? 'bg-nebula-accent-yellow'
                  : 'bg-nebula-accent-green'
              )}
              style={{ width: `${Math.min(budgetPercentage, 100)}%` }}
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="text-xs text-nebula-text-secondary mb-1">Costo por Job</div>
            <div className="text-lg font-semibold text-nebula-text-primary">
              {formatCurrency(metrics.costPerJob)}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-3">
            <div className="text-xs text-nebula-text-secondary mb-1">Costo por Hora</div>
            <div className="text-lg font-semibold text-nebula-text-primary">
              {formatCurrency(metrics.costPerHour)}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2 pt-2 border-t border-nebula-surface-secondary">
          <TrendIcon
            className={cn('w-4 h-4', trendColors[metrics.trend])}
          />
          <span className="text-sm text-nebula-text-secondary">
            Tendencia: {metrics.trend === 'up' ? '↗' : metrics.trend === 'down' ? '↘' : '→'}{' '}
            {formatPercentage(Math.abs(metrics.trendPercentage))}
          </span>
        </div>
      </CardContent>
    </Card>
  );
}
