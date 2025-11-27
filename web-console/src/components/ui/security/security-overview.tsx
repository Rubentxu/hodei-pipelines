import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { ShieldIcon, AlertTriangleIcon, UsersIcon, CheckCircleIcon } from 'lucide-react';
import { SecurityMetrics } from '@/types';

interface SecurityOverviewProps {
  metrics: SecurityMetrics;
  className?: string;
}

const severityColors = {
  critical: 'text-nebula-accent-red',
  high: 'text-nebula-accent-yellow',
  medium: 'text-nebula-accent-blue',
  low: 'text-nebula-accent-green',
};

export function SecurityOverview({ metrics, className }: SecurityOverviewProps) {
  const getSecurityScoreColor = (score: number) => {
    if (score >= 90) return 'text-nebula-accent-green';
    if (score >= 70) return 'text-nebula-accent-yellow';
    return 'text-nebula-accent-red';
  };

  const getSecurityScoreBadge = (score: number) => {
    if (score >= 90) return 'success' as const;
    if (score >= 70) return 'warning' as const;
    return 'error' as const;
  };

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary flex items-center gap-2">
          <ShieldIcon className="w-5 h-5" />
          Resumen de Seguridad
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-nebula-surface-secondary rounded p-4">
            <div className="flex items-center gap-2 mb-2">
              <UsersIcon className="w-4 h-4 text-nebula-accent-blue" />
              <span className="text-xs text-nebula-text-secondary">Usuarios Activos</span>
            </div>
            <div className="text-2xl font-bold text-nebula-text-primary">
              {metrics.activeUsers}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-4">
            <div className="flex items-center gap-2 mb-2">
              <AlertTriangleIcon className="w-4 h-4 text-nebula-accent-red" />
              <span className="text-xs text-nebula-text-secondary">Login Fallidos</span>
            </div>
            <div className="text-2xl font-bold text-nebula-accent-red">
              {metrics.failedLogins}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-4">
            <div className="flex items-center gap-2 mb-2">
              <CheckCircleIcon className="w-4 h-4 text-nebula-accent-yellow" />
              <span className="text-xs text-nebula-text-secondary">Pendientes</span>
            </div>
            <div className="text-2xl font-bold text-nebula-accent-yellow">
              {metrics.pendingApprovals}
            </div>
          </div>

          <div className="bg-nebula-surface-secondary rounded p-4">
            <div className="flex items-center gap-2 mb-2">
              <ShieldIcon className="w-4 h-4 text-nebula-text-secondary" />
              <span className="text-xs text-nebula-text-secondary">Score</span>
            </div>
            <div className="flex items-center gap-2">
              <div className={`text-2xl font-bold ${getSecurityScoreColor(metrics.securityScore)}`}>
                {metrics.securityScore}
              </div>
              <StatusBadge
                status={getSecurityScoreBadge(metrics.securityScore)}
                label={`${metrics.securityScore >= 90 ? 'Seguro' : metrics.securityScore >= 70 ? 'Medio' : 'Riesgo'}`}
              />
            </div>
          </div>
        </div>

        <div className="space-y-3">
          <h4 className="text-sm font-semibold text-nebula-text-primary">
            Vulnerabilidades
          </h4>
          <div className="grid grid-cols-4 gap-3">
            <div className="bg-nebula-surface-secondary rounded p-3">
              <div className="text-xs text-nebula-text-secondary mb-1">Críticas</div>
              <div className="text-xl font-bold text-nebula-accent-red">
                {metrics.vulnerabilitiesCritical}
              </div>
            </div>
            <div className="bg-nebula-surface-secondary rounded p-3">
              <div className="text-xs text-nebula-text-secondary mb-1">Altas</div>
              <div className="text-xl font-bold text-nebula-accent-yellow">
                {metrics.vulnerabilitiesHigh}
              </div>
            </div>
            <div className="bg-nebula-surface-secondary rounded p-3">
              <div className="text-xs text-nebula-text-secondary mb-1">Medias</div>
              <div className="text-xl font-bold text-nebula-accent-blue">
                {metrics.vulnerabilitiesMedium}
              </div>
            </div>
            <div className="bg-nebula-surface-secondary rounded p-3">
              <div className="text-xs text-nebula-text-secondary mb-1">Bajas</div>
              <div className="text-xl font-bold text-nebula-accent-green">
                {metrics.vulnerabilitiesLow}
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
