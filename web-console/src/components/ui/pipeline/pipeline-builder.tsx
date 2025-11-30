import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { PipelineTask } from '@/types';
import { cn } from '@/utils/cn';
import { DragDropContext, Draggable, Droppable, DropResult } from '@hello-pangea/dnd';
import { useState } from 'react';

interface PipelineBuilderProps {
  initialTasks?: PipelineTask[];
  onSave: (tasks: PipelineTask[]) => void;
  onCancel: () => void;
  className?: string;
}

const taskTypeConfig = {
  extract: {
    color: 'border-l-nebula-accent-blue bg-nebula-accent-blue/10',
    badge: 'success' as const,
    label: 'Extract',
  },
  transform: {
    color: 'border-l-nebula-accent-purple bg-nebula-accent-purple/10',
    badge: 'running' as const,
    label: 'Transform',
  },
  load: {
    color: 'border-l-nebula-accent-cyan bg-nebula-accent-cyan/10',
    badge: 'pending' as const,
    label: 'Load',
  },
  validate: {
    color: 'border-l-nebula-accent-yellow bg-nebula-accent-yellow/10',
    badge: 'warning' as const,
    label: 'Validate',
  },
  notify: {
    color: 'border-l-nebula-accent-green bg-nebula-accent-green/10',
    badge: 'success' as const,
    label: 'Notify',
  },
};

const taskTypeOptions = [
  { value: 'extract', label: 'Extract', description: 'Extraer datos de origen' },
  { value: 'transform', label: 'Transform', description: 'Transformar datos' },
  { value: 'load', label: 'Load', description: 'Cargar datos a destino' },
  { value: 'validate', label: 'Validate', description: 'Validar datos' },
  { value: 'notify', label: 'Notify', description: 'Enviar notificaciones' },
];

export function PipelineBuilder({
  initialTasks = [],
  onSave,
  onCancel,
  className,
}: PipelineBuilderProps) {
  const [tasks, setTasks] = useState<PipelineTask[]>(
    initialTasks.length > 0
      ? initialTasks
      : [
        {
          id: `task-${Date.now()}`,
          name: 'Nueva Tarea',
          type: 'extract',
          status: 'pending',
          position: 0,
        },
      ]
  );

  const [error, setError] = useState<string | null>(null);

  const handleDragEnd = (result: DropResult) => {
    const { source, destination } = result;

    if (!destination) {
      return;
    }

    if (source.index === destination.index) {
      return;
    }

    const newTasks = Array.from(tasks);
    const [reorderedTask] = newTasks.splice(source.index, 1);
    newTasks.splice(destination.index, 0, reorderedTask);

    const updatedTasks = newTasks.map((task, index) => ({
      ...task,
      position: index,
    }));

    setTasks(updatedTasks);
  };

  const addTask = () => {
    const newTask: PipelineTask = {
      id: `task-${Date.now()}`,
      name: `Nueva Tarea ${tasks.length + 1}`,
      type: 'extract',
      status: 'pending',
      position: tasks.length,
    };
    setTasks([...tasks, newTask]);
    setError(null);
  };

  const removeTask = (taskId: string) => {
    setTasks(tasks.filter((t) => t.id !== taskId));
  };

  const updateTask = (taskId: string, updates: Partial<PipelineTask>) => {
    setTasks(
      tasks.map((task) =>
        task.id === taskId ? { ...task, ...updates } : task
      )
    );
  };

  const handleSave = () => {
    if (tasks.length === 0) {
      setError('El pipeline debe tener al menos una tarea');
      return;
    }
    setError(null);
    onSave(tasks);
  };

  return (
    <div className={cn('space-y-6', className)}>
      <Card>
        <CardHeader>
          <CardTitle className="text-nebula-text-primary">
            Constructor de Pipeline
          </CardTitle>
          <p className="text-sm text-nebula-text-secondary">
            Arrastra y suelta las tareas para construir tu pipeline
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex justify-between items-center">
            <h3 className="text-lg font-semibold text-nebula-text-primary">
              Tareas del Pipeline
            </h3>
            <Button onClick={addTask} className="bg-nebula-accent-blue hover:bg-nebula-accent-blue/80">
              Agregar Tarea
            </Button>
          </div>

          <DragDropContext onDragEnd={handleDragEnd}>
            <Droppable droppableId="pipeline-tasks">
              {(provided, snapshot) => (
                <div
                  {...provided.droppableProps}
                  ref={provided.innerRef}
                  className={cn(
                    'space-y-2 p-4 rounded-lg border-2 border-dashed',
                    snapshot.isDraggingOver
                      ? 'border-nebula-accent-blue bg-nebula-accent-blue/5'
                      : 'border-nebula-surface-secondary'
                  )}
                >
                  {tasks.map((task, index) => {
                    const taskConfig = taskTypeConfig[task.type];

                    return (
                      <Draggable
                        key={task.id}
                        draggableId={task.id}
                        index={index}
                      >
                        {(provided, snapshot) => (
                          <div
                            ref={provided.innerRef}
                            {...provided.draggableProps}
                            {...provided.dragHandleProps}
                            className={cn(
                              'rounded-lg border-l-4 p-4 bg-nebula-surface-secondary',
                              taskConfig.color,
                              snapshot.isDragging && 'shadow-lg transform rotate-2'
                            )}
                          >
                            <div className="flex items-center justify-between">
                              <div className="flex-1">
                                <div className="flex items-center gap-2 mb-2">
                                  <StatusBadge
                                    status={taskConfig.badge}
                                    label={taskConfig.label}
                                  />
                                  <span className="text-xs text-nebula-text-secondary">
                                    Posición {index + 1}
                                  </span>
                                </div>
                                <input
                                  type="text"
                                  value={task.name}
                                  onChange={(e) =>
                                    updateTask(task.id, { name: e.target.value })
                                  }
                                  className="bg-transparent text-nebula-text-primary font-semibold text-lg w-full border-none outline-none"
                                />
                                <select
                                  value={task.type}
                                  onChange={(e) =>
                                    updateTask(task.id, {
                                      type: e.target.value as PipelineTask['type'],
                                    })
                                  }
                                  className="mt-2 bg-nebula-surface-primary text-nebula-text-primary text-sm rounded px-2 py-1"
                                >
                                  {taskTypeOptions.map((option) => (
                                    <option key={option.value} value={option.value}>
                                      {option.label} - {option.description}
                                    </option>
                                  ))}
                                </select>
                              </div>
                              <Button
                                onClick={() => removeTask(task.id)}
                                className="text-nebula-accent-red hover:bg-nebula-accent-red/20"
                                variant="ghost"
                                size="sm"
                              >
                                Eliminar
                              </Button>
                            </div>
                          </div>
                        )}
                      </Draggable>
                    );
                  })}
                  {provided.placeholder}
                </div>
              )}
            </Droppable>
          </DragDropContext>

          {tasks.length > 1 && (
            <div className="mt-4">
              <svg className="w-full h-8" viewBox="0 0 100 20">
                {Array.from({ length: tasks.length - 1 }).map((_, i) => (
                  <line
                    key={i}
                    x1="0"
                    y1="10"
                    x2="100"
                    y2="10"
                    stroke="var(--nebula-surface-secondary)"
                    strokeWidth="2"
                    strokeDasharray="5,5"
                    className="pipeline-connection"
                  />
                ))}
              </svg>
            </div>
          )}

          {error && (
            <div className="text-red-500 text-sm p-2 bg-red-500/10 rounded">
              {error}
            </div>
          )}

          <div className="flex gap-2 pt-4 border-t border-nebula-surface-secondary">
            <Button
              onClick={handleSave}
              className="bg-nebula-accent-green hover:bg-nebula-accent-green/80"
            >
              Guardar Pipeline
            </Button>
            <Button
              onClick={onCancel}
              variant="outline"
              className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
            >
              Cancelar
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
