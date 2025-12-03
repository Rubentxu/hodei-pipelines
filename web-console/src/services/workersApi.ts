import { Worker, WorkerPool } from '@/types';

export async function getWorkers(): Promise<Worker[]> {
  const response = await fetch('/api/workers');

  if (!response.ok) {
    throw new Error('Failed to fetch workers');
  }

  return response.json();
}

export async function getWorker(id: string): Promise<Worker> {
  const response = await fetch(`/api/workers/${id}`);

  if (!response.ok) {
    throw new Error('Failed to fetch worker');
  }

  return response.json();
}

export async function startWorker(id: string): Promise<void> {
  const response = await fetch(`/api/workers/${id}/start`, {
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error('Failed to start worker');
  }
}

export async function stopWorker(id: string): Promise<void> {
  const response = await fetch(`/api/workers/${id}/stop`, {
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error('Failed to stop worker');
  }
}

export async function restartWorker(id: string): Promise<void> {
  const response = await fetch(`/api/workers/${id}/restart`, {
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error('Failed to restart worker');
  }
}

export async function getWorkerPools(): Promise<WorkerPool[]> {
  const response = await fetch('/api/v1/resource-pools');

  if (!response.ok) {
    throw new Error('Failed to fetch worker pools');
  }

  return response.json();
}

export async function getWorkerPool(id: string): Promise<WorkerPool> {
  const response = await fetch(`/api/v1/resource-pools/${id}`);

  if (!response.ok) {
    throw new Error('Failed to fetch worker pool');
  }

  return response.json();
}

export async function scaleWorkerPool(
  id: string,
  action: 'up' | 'down'
): Promise<WorkerPool> {
  const response = await fetch(`/api/v1/resource-pools/${id}/scale`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ action }),
  });

  if (!response.ok) {
    throw new Error('Failed to scale worker pool');
  }

  return response.json();
}

export async function updateWorkerPool(
  id: string,
  updates: Partial<WorkerPool>
): Promise<WorkerPool> {
  const response = await fetch(`/api/v1/resource-pools/${id}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(updates),
  });

  if (!response.ok) {
    throw new Error('Failed to update worker pool');
  }

  return response.json();
}

export async function subscribeToWorkers(
  callback: (workers: Worker[]) => void
): Promise<() => void> {
  const eventSource = new EventSource('/api/workers/stream');

  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      callback(data);
    } catch (error) {
      console.error('Failed to parse workers data:', error);
    }
  };

  eventSource.onerror = (error) => {
    console.error('EventSource error:', error);
  };

  return () => {
    eventSource.close();
  };
}
