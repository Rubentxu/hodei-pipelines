import { Pipeline, PipelineTask } from '@/types';

export interface CreatePipelineRequest {
  name: string;
  description?: string;
  tasks: PipelineTask[];
  schedule?: {
    cron: string;
    timezone: string;
  };
  tags?: string[];
}

export interface UpdatePipelineRequest {
  name?: string;
  description?: string;
  tasks?: PipelineTask[];
  status?: 'active' | 'paused' | 'error' | 'inactive';
  schedule?: {
    cron: string;
    timezone: string;
  };
  tags?: string[];
}

export async function getPipelines(): Promise<Pipeline[]> {
  const response = await fetch('/api/v1/pipelines');

  if (!response.ok) {
    throw new Error('Failed to fetch pipelines');
  }

  return response.json();
}

export async function getPipeline(id: string): Promise<Pipeline> {
  const response = await fetch(`/api/v1/pipelines/${id}`);

  if (!response.ok) {
    throw new Error('Failed to fetch pipeline');
  }

  return response.json();
}

export async function createPipeline(
  data: CreatePipelineRequest
): Promise<Pipeline> {
  const response = await fetch('/api/v1/pipelines', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    throw new Error('Failed to create pipeline');
  }

  return response.json();
}

export async function updatePipeline(
  id: string,
  data: UpdatePipelineRequest
): Promise<Pipeline> {
  const response = await fetch(`/api/v1/pipelines/${id}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    throw new Error('Failed to update pipeline');
  }

  return response.json();
}

export async function deletePipeline(id: string): Promise<void> {
  const response = await fetch(`/api/v1/pipelines/${id}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    throw new Error('Failed to delete pipeline');
  }
}

export async function executePipeline(id: string): Promise<{ executionId: string }> {
  const response = await fetch(`/api/v1/pipelines/${id}/execute`, {
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error('Failed to execute pipeline');
  }

  return response.json();
}
