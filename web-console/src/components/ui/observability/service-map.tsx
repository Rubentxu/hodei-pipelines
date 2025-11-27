import { Card } from '../card';
import { useServiceMap } from '../../../hooks/useObservability';
import { StatusBadge } from '../status-badge';

export function ServiceMap() {
  const { data, isLoading } = useServiceMap();

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center justify-center h-[400px]">
          <div className="text-gray-400">Loading service map...</div>
        </div>
      </Card>
    );
  }

  const services = data?.services || [];

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'healthy':
        return 'bg-green-500';
      case 'degraded':
        return 'bg-yellow-500';
      case 'unhealthy':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold text-gray-100 mb-6">Service Dependency Map</h3>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {services.map((service) => (
          <div
            key={service.id}
            className="bg-gray-800 rounded-lg p-4 hover:bg-gray-750 transition-colors"
          >
            <div className="flex items-start justify-between mb-3">
              <div className="flex items-center gap-3">
                <div className={`w-3 h-3 rounded-full ${getStatusColor(service.status)}`} />
                <h4 className="text-gray-100 font-medium">{service.name}</h4>
              </div>
              <StatusBadge
                status={
                  service.status === 'healthy'
                    ? 'success'
                    : service.status === 'degraded'
                    ? 'warning'
                    : 'error'
                }
              />
            </div>

            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Request Rate:</span>
                <span className="text-gray-300">{service.metrics.requestRate.toFixed(2)} req/s</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Error Rate:</span>
                <span className={`${
                  service.metrics.errorRate > 5 ? 'text-red-400' : 'text-gray-300'
                }`}>
                  {service.metrics.errorRate.toFixed(2)}%
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Avg Response Time:</span>
                <span className="text-gray-300">{service.metrics.avgResponseTime.toFixed(2)}ms</span>
              </div>
            </div>

            {service.dependencies.length > 0 && (
              <div className="mt-3 pt-3 border-t border-gray-700">
                <div className="text-xs text-gray-500 mb-2">Dependencies:</div>
                <div className="flex flex-wrap gap-1">
                  {service.dependencies.map((dep) => (
                    <span
                      key={dep}
                      className="text-xs bg-gray-700 text-gray-300 px-2 py-1 rounded"
                    >
                      {dep}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </Card>
  );
}
