import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { Button } from '@/components/ui/button';
import { AuditLog } from '@/types';
import { FileTextIcon, FilterIcon, DownloadIcon } from 'lucide-react';

interface AuditLogViewerProps {
  logs: AuditLog[];
  className?: string;
}

export function AuditLogViewer({ logs, className }: AuditLogViewerProps) {
  const handleExport = () => {
    console.log('Exporting audit logs...');
  };

  const handleFilter = () => {
    console.log('Opening filter dialog...');
  };

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-nebula-text-primary flex items-center gap-2">
            <FileTextIcon className="w-5 h-5" />
            Registro de Auditoría
          </CardTitle>
          <div className="flex gap-2">
            <Button
              onClick={handleFilter}
              variant="outline"
              size="sm"
              className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
            >
              <FilterIcon className="w-4 h-4 mr-2" />
              Filtrar
            </Button>
            <Button
              onClick={handleExport}
              variant="outline"
              size="sm"
              className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
            >
              <DownloadIcon className="w-4 h-4 mr-2" />
              Exportar
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {logs.map((log) => (
            <div
              key={log.id}
              className="flex items-center justify-between p-3 bg-nebula-surface-secondary rounded hover:bg-nebula-surface-secondary/80 transition-colors"
            >
              <div className="flex-1 grid grid-cols-5 gap-4 items-center">
                <div className="text-sm">
                  <div className="font-medium text-nebula-text-primary">
                    {log.userName}
                  </div>
                  <div className="text-xs text-nebula-text-secondary">
                    {log.userId}
                  </div>
                </div>

                <div>
                  <div className="text-sm font-medium text-nebula-text-primary">
                    {log.action}
                  </div>
                  <div className="text-xs text-nebula-text-secondary">
                    {log.resource}
                    {log.resourceId && ` #${log.resourceId}`}
                  </div>
                </div>

                <div className="text-sm">
                  <StatusBadge
                    status={log.status === 'success' ? 'success' : 'error'}
                    label={log.status === 'success' ? 'Éxito' : 'Fallo'}
                  />
                </div>

                <div className="text-sm">
                  <div className="text-nebula-text-secondary">
                    {log.ipAddress}
                  </div>
                  <div className="text-xs text-nebula-text-secondary truncate max-w-[150px]">
                    {log.userAgent}
                  </div>
                </div>

                <div className="text-right">
                  <div className="text-xs text-nebula-text-secondary">
                    {new Date(log.timestamp).toLocaleDateString('es-ES', {
                      day: '2-digit',
                      month: 'short',
                      hour: '2-digit',
                      minute: '2-digit',
                    })}
                  </div>
                </div>
              </div>
            </div>
          ))}

          {logs.length === 0 && (
            <div className="text-center py-8 text-nebula-text-secondary">
              No hay registros de auditoría disponibles
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
