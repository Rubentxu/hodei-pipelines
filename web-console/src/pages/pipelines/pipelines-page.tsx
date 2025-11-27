import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { StatusBadge } from '@/components/ui/status-badge';
import { PipelineBuilder } from '@/components/ui/pipeline/pipeline-builder';
import { usePipelines } from '@/hooks/usePipelines';
import { PipelineTask } from '@/types';
import { PlusIcon, PlayIcon, TrashIcon } from 'lucide-react';

export function PipelinesPage() {
  const navigate = useNavigate();
  const { pipelines, isLoading, isError, error, deletePipeline, executePipeline } = usePipelines();
  const [showBuilder, setShowBuilder] = useState(false);

  const handleSavePipeline = (tasks: PipelineTask[]) => {
    console.log('Pipeline saved with tasks:', tasks);
    setShowBuilder(false);
  };

  const handleCancel = () => {
    setShowBuilder(false);
  };

  if (isLoading) {
    return (
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-nebula-text-primary">Pipelines</h1>
        </div>
        <div className="animate-pulse space-y-4">
          {Array.from({ length: 5 }).map((_, i) => (
            <Card key={i} className="p-6">
              <div className="h-6 bg-nebula-surface-secondary rounded w-1/3 mb-2"></div>
              <div className="h-4 bg-nebula-surface-secondary rounded w-2/3"></div>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="p-6 space-y-6">
        <h1 className="text-2xl font-bold text-nebula-text-primary">Pipelines</h1>
        <Card className="p-6 border-red-500/50 bg-red-500/10">
          <p className="text-red-400">Error al cargar pipelines: {error?.message}</p>
        </Card>
      </div>
    );
  }

  if (showBuilder) {
    return (
      <div className="p-6 space-y-6">
        <PipelineBuilder
          onSave={handleSavePipeline}
          onCancel={handleCancel}
        />
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-nebula-text-primary">Pipelines</h1>
        <Button
          onClick={() => setShowBuilder(true)}
          className="bg-nebula-accent-blue hover:bg-nebula-accent-blue/80"
        >
          <PlusIcon className="w-4 h-4 mr-2" />
          Nuevo Pipeline
        </Button>
      </div>

      <div className="grid gap-4">
        {pipelines.length === 0 ? (
          <Card className="p-12 text-center">
            <p className="text-nebula-text-secondary mb-4">
              No hay pipelines creados
            </p>
            <Button
              onClick={() => setShowBuilder(true)}
              className="bg-nebula-accent-blue hover:bg-nebula-accent-blue/80"
            >
              <PlusIcon className="w-4 h-4 mr-2" />
              Crear tu primer pipeline
            </Button>
          </Card>
        ) : (
          pipelines.map((pipeline) => (
            <Card key={pipeline.id} className="p-6 hover:bg-nebula-surface-secondary/50 transition-colors">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <h3 className="text-lg font-semibold text-nebula-text-primary">
                      {pipeline.name}
                    </h3>
                    <StatusBadge
                      status={
                        pipeline.status === 'active' ? 'success' :
                        pipeline.status === 'paused' ? 'pending' :
                        pipeline.status === 'error' ? 'error' : 'warning'
                      }
                      label={pipeline.status}
                    />
                  </div>
                  {pipeline.description && (
                    <p className="text-sm text-nebula-text-secondary mb-2">
                      {pipeline.description}
                    </p>
                  )}
                  <div className="flex items-center gap-4 text-xs text-nebula-text-secondary">
                    <span>Última ejecución: {pipeline.lastRun}</span>
                    {pipeline.tasks && (
                      <span>{pipeline.tasks.length} tareas</span>
                    )}
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button
                    onClick={() => executePipeline(pipeline.id)}
                    size="sm"
                    className="bg-nebula-accent-green hover:bg-nebula-accent-green/80"
                  >
                    <PlayIcon className="w-4 h-4 mr-1" />
                    Ejecutar
                  </Button>
                  <Button
                    onClick={() => navigate(`/pipelines/${pipeline.id}`)}
                    variant="outline"
                    size="sm"
                  >
                    Ver Detalle
                  </Button>
                  <Button
                    onClick={() => deletePipeline(pipeline.id)}
                    size="sm"
                    variant="ghost"
                    className="text-nebula-accent-red hover:bg-nebula-accent-red/20"
                  >
                    <TrashIcon className="w-4 h-4" />
                  </Button>
                </div>
              </div>
            </Card>
          ))
        )}
      </div>
    </div>
  );
}
