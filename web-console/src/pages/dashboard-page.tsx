import { Card } from "../components/ui/card";
import { KPICard } from "../components/ui/kpi-card";
import { ClusterTopologyMap } from "../components/ui/topology/cluster-topology-map";
import { SLAViolationTicker } from "../components/ui/ticker/sla-violation-ticker";
import { useKPIMetrics } from "../hooks/useKPIMetrics";
import { useClusterTopology } from "../hooks/useClusterTopology";
import { useSLAViolations } from "../hooks/useSLAViolations";

export function DashboardPage() {
  const {
    data: metrics,
    isLoading: loadingMetrics,
    isError: hasErrorMetrics,
    error: errorMetrics,
  } = useKPIMetrics();

  const {
    data: topology,
    isLoading: loadingTopology,
    isError: hasErrorTopology,
    error: errorTopology,
  } = useClusterTopology();

  const {
    data: slaData,
    isLoading: loadingSLA,
    isError: hasErrorSLA,
    error: errorSLA,
  } = useSLAViolations();

  const loading = loadingMetrics || loadingTopology || loadingSLA;
  const hasError = hasErrorMetrics || hasErrorTopology || hasErrorSLA;

  if (loading) {
    return (
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-nebula-text-primary">
            Dashboard
          </h1>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {Array.from({ length: 8 }).map((_, i) => (
            <Card key={i} className="p-6">
              <div className="animate-pulse space-y-3">
                <div className="h-4 bg-nebula-surface-secondary rounded w-3/4"></div>
                <div className="h-8 bg-nebula-surface-secondary rounded w-1/2"></div>
              </div>
            </Card>
          ))}
        </div>
        <Card className="p-6">
          <div className="animate-pulse">
            <div className="h-8 bg-nebula-surface-secondary rounded w-1/3 mb-4"></div>
            <div className="h-96 bg-nebula-surface-secondary rounded"></div>
          </div>
        </Card>
      </div>
    );
  }

  if (hasError || (!metrics && !topology)) {
    return (
      <div className="p-6 space-y-6">
        <h1 className="text-2xl font-bold text-nebula-text-primary">
          Dashboard
        </h1>
        <Card className="p-6 border-red-500/50 bg-red-500/10">
          <p className="text-red-400">
            Error al cargar métricas del dashboard:{" "}
            {errorMetrics?.message ||
              errorTopology?.message ||
              errorSLA?.message ||
              "Error desconocido"}
          </p>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-nebula-text-primary">
          Dashboard
        </h1>
      </div>

      <section aria-label="kpi-metrics" role="region">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {metrics ? (
            <>
              <KPICard
                title="Jobs en Cola"
                value={metrics.jobsInQueue}
                format="number"
              />

              <KPICard
                title="Jobs Ejecutándose"
                value={metrics.jobsRunning}
                format="number"
              />

              <KPICard
                title="Jobs Exitosos"
                value={metrics.jobsSucceeded}
                format="number"
              />

              <KPICard
                title="Jobs Fallidos"
                value={metrics.jobsFailed}
                format="number"
              />

              <KPICard
                title="Workers Activos"
                value={metrics.activeWorkers}
                format="number"
              />

              <KPICard
                title="CPU Promedio"
                value={metrics.cpuUsage}
                format="percentage"
              />

              <KPICard
                title="Uso de Memoria"
                value={metrics.memoryUsage}
                format="percentage"
              />

              <KPICard
                title="Tiempo de Ejecución"
                value={metrics.avgExecutionTime}
                format="number"
              />
            </>
          ) : (
            Array.from({ length: 8 }).map((_, i) => (
              <Card key={i} className="p-6">
                <div className="animate-pulse space-y-3">
                  <div className="h-4 bg-nebula-surface-secondary rounded w-3/4"></div>
                  <div className="h-8 bg-nebula-surface-secondary rounded w-1/2"></div>
                </div>
              </Card>
            ))
          )}
        </div>
      </section>

      {topology && (
        <section aria-label="cluster-topology" role="region">
          <ClusterTopologyMap nodes={topology.nodes} edges={topology.edges} />
        </section>
      )}

      {slaData?.violations && slaData.violations.length > 0 && (
        <section aria-label="sla-violations" role="region">
          <SLAViolationTicker violations={slaData.violations} />
        </section>
      )}
    </div>
  );
}
