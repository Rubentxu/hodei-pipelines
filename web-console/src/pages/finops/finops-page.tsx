import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { CostOverviewCard } from '@/components/ui/finops/cost-overview-card';
import { CostBreakdownChart } from '@/components/ui/finops/cost-breakdown-chart';
import { CostHistoryChart } from '@/components/ui/finops/cost-history-chart';
import { BudgetAlerts } from '@/components/ui/finops/budget-alerts';
import { useFinOpsMetrics } from '@/hooks/useFinOpsMetrics';
import { DollarSignIcon, DownloadIcon, SettingsIcon, TrendingUpIcon } from 'lucide-react';

type PeriodType = '7d' | '30d' | '90d' | '1y';

export function FinOpsPage() {
  const { data, isLoading, isError, error } = useFinOpsMetrics();
  const [selectedPeriod, setSelectedPeriod] = useState<PeriodType>('30d');

  if (isLoading) {
    return (
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-nebula-text-primary flex items-center gap-2">
            <DollarSignIcon className="w-6 h-6" />
            FinOps Dashboard
          </h1>
        </div>
        <div className="animate-pulse space-y-4">
          <Card className="p-6">
            <div className="h-8 bg-nebula-surface-secondary rounded w-1/3 mb-4"></div>
            <div className="grid grid-cols-4 gap-4">
              {Array.from({ length: 4 }).map((_, i) => (
                <div key={i} className="h-24 bg-nebula-surface-secondary rounded"></div>
              ))}
            </div>
          </Card>
          <Card className="p-6">
            <div className="h-64 bg-nebula-surface-secondary rounded"></div>
          </Card>
          <div className="grid md:grid-cols-2 gap-4">
            <Card className="p-6">
              <div className="h-64 bg-nebula-surface-secondary rounded"></div>
            </Card>
            <Card className="p-6">
              <div className="h-64 bg-nebula-surface-secondary rounded"></div>
            </Card>
          </div>
        </div>
      </div>
    );
  }

  if (isError || !data) {
    return (
      <div className="p-6 space-y-6">
        <h1 className="text-2xl font-bold text-nebula-text-primary flex items-center gap-2">
          <DollarSignIcon className="w-6 h-6" />
          FinOps Dashboard
        </h1>
        <Card className="p-6 border-red-500/50 bg-red-500/10">
          <p className="text-red-400">
            Error al cargar métricas FinOps: {error?.message || 'Error desconocido'}
          </p>
        </Card>
      </div>
    );
  }

  const handleExportReport = () => {
    console.log('Exporting FinOps report...');
  };

  const handleDismissAlert = (id: string) => {
    console.log('Dismissing alert:', id);
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-nebula-text-primary flex items-center gap-2">
            <DollarSignIcon className="w-6 h-6" />
            FinOps Dashboard
          </h1>
          <p className="text-sm text-nebula-text-secondary mt-1">
            Monitoreo y análisis de costos de infraestructura
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            onClick={handleExportReport}
            variant="outline"
            className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
          >
            <DownloadIcon className="w-4 h-4 mr-2" />
            Exportar Reporte
          </Button>
          <Button
            variant="outline"
            className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
          >
            <SettingsIcon className="w-4 h-4 mr-2" />
            Configuración
          </Button>
        </div>
      </div>

      <CostOverviewCard metrics={data.metrics} />

      <div className="grid md:grid-cols-3 gap-4">
        <Card className="p-4 bg-nebula-surface-secondary">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-nebula-text-secondary">Presupuesto Mensual</p>
              <p className="text-2xl font-bold text-nebula-text-primary">
                ${data.metrics.budgetLimit.toLocaleString()}
              </p>
            </div>
            <TrendingUpIcon className="w-8 h-8 text-nebula-text-secondary" />
          </div>
        </Card>

        <Card className="p-4 bg-nebula-surface-secondary">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-nebula-text-secondary">Utilizado</p>
              <p className="text-2xl font-bold text-nebula-accent-blue">
                {((data.metrics.budgetUsed / data.metrics.budgetLimit) * 100).toFixed(1)}%
              </p>
            </div>
            <TrendingUpIcon className="w-8 h-8 text-nebula-text-secondary" />
          </div>
        </Card>

        <Card className="p-4 bg-nebula-surface-secondary">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-nebula-text-secondary">Proyección</p>
              <p className="text-2xl font-bold text-nebula-text-primary">
                ${data.metrics.projectedMonthlyCost.toLocaleString()}
              </p>
            </div>
            <TrendingUpIcon className="w-8 h-8 text-nebula-text-secondary" />
          </div>
        </Card>
      </div>

      <div className="flex gap-2">
        {(['7d', '30d', '90d', '1y'] as PeriodType[]).map((period) => (
          <Button
            key={period}
            onClick={() => setSelectedPeriod(period)}
            variant={selectedPeriod === period ? 'default' : 'outline'}
            className={
              selectedPeriod === period
                ? 'bg-nebula-accent-blue hover:bg-nebula-accent-blue/80'
                : 'border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary'
            }
          >
            {period === '7d' && 'Últimos 7 días'}
            {period === '30d' && 'Últimos 30 días'}
            {period === '90d' && 'Últimos 90 días'}
            {period === '1y' && 'Último año'}
          </Button>
        ))}
      </div>

      <CostHistoryChart history={data.history} />

      <div className="grid md:grid-cols-2 gap-4">
        <CostBreakdownChart breakdown={data.breakdown} />
        <BudgetAlerts
          alerts={data.alerts}
          onDismiss={handleDismissAlert}
        />
      </div>

      <div className="text-xs text-nebula-text-secondary text-center">
        Última actualización: {new Date(data.lastUpdated).toLocaleString('es-ES')}
      </div>
    </div>
  );
}
