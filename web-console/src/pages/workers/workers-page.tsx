import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { WorkerCard } from '@/components/ui/workers/worker-card';
import { WorkerPoolCard } from '@/components/ui/workers/worker-pool-card';
import { useWorkers } from '@/hooks/useWorkers';
import { useWorkerPools } from '@/hooks/useWorkerPools';
import { UsersIcon, ActivityIcon, SettingsIcon } from 'lucide-react';

type TabType = 'workers' | 'pools';

export function WorkersPage() {
  const [activeTab, setActiveTab] = useState<TabType>('workers');
  const { workers, isLoading: workersLoading, startWorker, stopWorker, restartWorker } = useWorkers();
  const { pools, isLoading: poolsLoading, scalePool } = useWorkerPools();

  const activeWorkers = workers.filter(w => w.status === 'healthy');
  const failedWorkers = workers.filter(w => w.status === 'failed');
  const maintenanceWorkers = workers.filter(w => w.status === 'maintenance');

  const renderWorkersTab = () => {
    if (workersLoading) {
      return (
        <div className="animate-pulse space-y-4">
          {Array.from({ length: 5 }).map((_, i) => (
            <Card key={i} className="p-6">
              <div className="h-6 bg-nebula-surface-secondary rounded w-1/3 mb-2"></div>
              <div className="h-4 bg-nebula-surface-secondary rounded w-2/3"></div>
            </Card>
          ))}
        </div>
      );
    }

    if (workers.length === 0) {
      return (
        <Card className="p-12 text-center">
          <UsersIcon className="w-12 h-12 mx-auto text-nebula-text-secondary mb-4" />
          <p className="text-nebula-text-secondary mb-4">No hay workers registrados</p>
        </Card>
      );
    }

    return (
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {workers.map((worker) => (
          <WorkerCard
            key={worker.id}
            worker={worker}
            onStart={startWorker}
            onStop={stopWorker}
            onRestart={restartWorker}
          />
        ))}
      </div>
    );
  };

  const renderPoolsTab = () => {
    if (poolsLoading) {
      return (
        <div className="animate-pulse space-y-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <Card key={i} className="p-6">
              <div className="h-6 bg-nebula-surface-secondary rounded w-1/3 mb-2"></div>
              <div className="h-4 bg-nebula-surface-secondary rounded w-2/3"></div>
            </Card>
          ))}
        </div>
      );
    }

    if (pools.length === 0) {
      return (
        <Card className="p-12 text-center">
          <ActivityIcon className="w-12 h-12 mx-auto text-nebula-text-secondary mb-4" />
          <p className="text-nebula-text-secondary mb-4">No hay worker pools configurados</p>
        </Card>
      );
    }

    return (
      <div className="grid gap-4 md:grid-cols-2">
        {pools.map((pool) => (
          <WorkerPoolCard
            key={pool.id}
            pool={pool}
            onScaleUp={(id) => scalePool({ id, action: 'up' })}
            onScaleDown={(id) => scalePool({ id, action: 'down' })}
          />
        ))}
      </div>
    );
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-nebula-text-primary">Workers</h1>
          <p className="text-sm text-nebula-text-secondary mt-1">
            Gestiona workers y worker pools
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="p-4 bg-nebula-surface-secondary">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-nebula-text-secondary">Activos</p>
              <p className="text-2xl font-bold text-nebula-accent-green">
                {activeWorkers.length}
              </p>
            </div>
            <ActivityIcon className="w-8 h-8 text-nebula-accent-green" />
          </div>
        </Card>

        <Card className="p-4 bg-nebula-surface-secondary">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-nebula-text-secondary">Fallidos</p>
              <p className="text-2xl font-bold text-nebula-accent-red">
                {failedWorkers.length}
              </p>
            </div>
            <SettingsIcon className="w-8 h-8 text-nebula-accent-red" />
          </div>
        </Card>

        <Card className="p-4 bg-nebula-surface-secondary">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-nebula-text-secondary">Total</p>
              <p className="text-2xl font-bold text-nebula-text-primary">
                {workers.length}
              </p>
            </div>
            <UsersIcon className="w-8 h-8 text-nebula-text-secondary" />
          </div>
        </Card>
      </div>

      <Card>
        <CardHeader className="pb-3">
          <div className="flex gap-2">
            <Button
              onClick={() => setActiveTab('workers')}
              variant={activeTab === 'workers' ? 'default' : 'outline'}
              className={
                activeTab === 'workers'
                  ? 'bg-nebula-accent-blue hover:bg-nebula-accent-blue/80'
                  : 'border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary'
              }
            >
              <UsersIcon className="w-4 h-4 mr-2" />
              Workers ({workers.length})
            </Button>
            <Button
              onClick={() => setActiveTab('pools')}
              variant={activeTab === 'pools' ? 'default' : 'outline'}
              className={
                activeTab === 'pools'
                  ? 'bg-nebula-accent-blue hover:bg-nebula-accent-blue/80'
                  : 'border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary'
              }
            >
              <ActivityIcon className="w-4 h-4 mr-2" />
              Pools ({pools.length})
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {activeTab === 'workers' ? renderWorkersTab() : renderPoolsTab()}
        </CardContent>
      </Card>
    </div>
  );
}
