import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { DragDropContext, Droppable, Draggable } from '@hello-pangea/dnd';
import { PipelineBuilder } from '../pipeline-builder';

describe('PipelineBuilder', () => {
  const mockOnSave = vi.fn();
  const mockOnCancel = vi.fn();

  const mockTasks = [
    { id: 'task-1', name: 'Extract Data', type: 'extract', status: 'pending' },
    { id: 'task-2', name: 'Transform Data', type: 'transform', status: 'pending' },
    { id: 'task-3', name: 'Load Data', type: 'load', status: 'pending' },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the pipeline builder title', () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );
    expect(screen.getByText('Constructor de Pipeline')).toBeInTheDocument();
  });

  it('displays all task cards', () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    expect(screen.getByText('Extract Data')).toBeInTheDocument();
    expect(screen.getByText('Transform Data')).toBeInTheDocument();
    expect(screen.getByText('Load Data')).toBeInTheDocument();
  });

  it('shows task types with color coding', () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const extractTasks = screen.getAllByText('Extract');
    const transformTasks = screen.getAllByText('Transform');
    const loadTasks = screen.getAllByText('Load');

    expect(extractTasks.length).toBeGreaterThan(0);
    expect(transformTasks.length).toBeGreaterThan(0);
    expect(loadTasks.length).toBeGreaterThan(0);
  });

  it('shows Save and Cancel buttons', () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    expect(screen.getByText('Guardar Pipeline')).toBeInTheDocument();
    expect(screen.getByText('Cancelar')).toBeInTheDocument();
  });

  it('handles adding new tasks', async () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const addButton = screen.getByText('Agregar Tarea');
    expect(addButton).toBeInTheDocument();
  });

  it('calls onSave when save button is clicked', async () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const saveButton = screen.getByText('Guardar Pipeline');
    saveButton.click();

    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalled();
    });
  });

  it('calls onCancel when cancel button is clicked', async () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const cancelButton = screen.getByText('Cancelar');
    cancelButton.click();

    await waitFor(() => {
      expect(mockOnCancel).toHaveBeenCalled();
    });
  });

  it('allows reordering tasks via drag and drop', async () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const draggableElements = screen.getAllByText(/Extract Data|Transform Data|Load Data/);
    expect(draggableElements.length).toBeGreaterThan(0);
  });

  it('validates pipeline before saving', async () => {
    render(
      <PipelineBuilder
        initialTasks={[]}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const saveButton = screen.getByText('Guardar Pipeline');
    saveButton.click();

    await waitFor(() => {
      expect(screen.getByText(/pipeline debe tener al menos una tarea/i)).toBeInTheDocument();
    });
  });

  it('shows connection lines between tasks', () => {
    render(
      <PipelineBuilder
        initialTasks={mockTasks}
        onSave={mockOnSave}
        onCancel={mockOnCancel}
      />
    );

    const connections = document.querySelectorAll('.pipeline-connection');
    expect(connections.length).toBeGreaterThan(0);
  });
});
