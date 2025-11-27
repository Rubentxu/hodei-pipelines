import { cn } from '@/utils/cn';
import { Card, CardContent } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';

interface SLAViolation {
  id: string;
  message: string;
  severity: 'critical' | 'warning' | 'error';
  timestamp?: string;
}

interface SLAViolationTickerProps {
  violations: SLAViolation[];
  autoScroll?: boolean;
  maxVisible?: number;
  className?: string;
}

const severityStyles = {
  critical: 'border-l-nebula-accent-red bg-nebula-accent-red/5',
  warning: 'border-l-nebula-accent-yellow bg-nebula-accent-yellow/5',
  error: 'border-l-nebula-accent-red bg-nebula-accent-red/5',
};

const severityBadge = {
  critical: 'error' as const,
  warning: 'warning' as const,
  error: 'error' as const,
};

const severityLabel = {
  critical: 'Crítico',
  warning: 'Advertencia',
  error: 'Error',
};

export function SLAViolationTicker({
  violations,
  autoScroll = true,
  maxVisible = 10,
  className,
}: SLAViolationTickerProps) {
  const displayViolations = violations.slice(0, maxVisible);

  if (displayViolations.length === 0) {
    return null;
  }

  return (
    <Card className={cn('border-l-4', className)}>
      <CardContent className="p-3">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-sm font-semibold text-nebula-text-primary">
            Alertas SLA
          </h3>
          <span className="text-xs text-nebula-text-secondary">
            {violations.length} total
          </span>
        </div>

        <div className="relative overflow-hidden">
          <div
            aria-label="ticker-horizontal"
            className={cn(
              'flex gap-3',
              autoScroll && 'animate-scroll-x'
            )}
          >
            {displayViolations.map((violation) => (
              <div
                key={violation.id}
                className={cn(
                  'flex items-center gap-2 px-3 py-2 rounded border-l-4 whitespace-nowrap',
                  severityStyles[violation.severity]
                )}
              >
                <StatusBadge
                  status={severityBadge[violation.severity]}
                  label={severityLabel[violation.severity]}
                  className="text-xs"
                />
                <span className="text-sm text-nebula-text-primary">
                  {violation.message}
                </span>
                {violation.timestamp && (
                  <span className="text-xs text-nebula-text-secondary">
                    {violation.timestamp}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>

        {violations.length > maxVisible && (
          <div className="mt-2 text-right">
            <span className="text-xs text-nebula-text-secondary">
              +{violations.length - maxVisible} más
            </span>
          </div>
        )}
      </CardContent>

      <style>{`
        @keyframes scroll-x {
          0% {
            transform: translateX(100%);
          }
          100% {
            transform: translateX(-100%);
          }
        }
        .animate-scroll-x {
          animation: scroll-x 30s linear infinite;
        }
        @media (prefers-reduced-motion: reduce) {
          .animate-scroll-x {
            animation: none;
          }
        }
      `}</style>
    </Card>
  );
}
