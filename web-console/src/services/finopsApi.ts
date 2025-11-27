import { FinOpsMetrics } from '@/types';

export async function getFinOpsMetrics(): Promise<FinOpsMetrics> {
  const response = await fetch('/api/finops/metrics');

  if (!response.ok) {
    throw new Error('Failed to fetch FinOps metrics');
  }

  return response.json();
}

export async function getCostBreakdown(
  startDate: string,
  endDate: string
): Promise<FinOpsMetrics['breakdown']> {
  const response = await fetch(
    `/api/finops/breakdown?startDate=${startDate}&endDate=${endDate}`
  );

  if (!response.ok) {
    throw new Error('Failed to fetch cost breakdown');
  }

  return response.json();
}

export async function getCostHistory(
  period: '7d' | '30d' | '90d' | '1y'
): Promise<FinOpsMetrics['history']> {
  const response = await fetch(`/api/finops/history?period=${period}`);

  if (!response.ok) {
    throw new Error('Failed to fetch cost history');
  }

  return response.json();
}

export async function updateBudgetLimit(
  limit: number
): Promise<{ success: boolean }> {
  const response = await fetch('/api/finops/budget', {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ limit }),
  });

  if (!response.ok) {
    throw new Error('Failed to update budget limit');
  }

  return response.json();
}

export async function subscribeToFinOpsMetrics(
  callback: (metrics: FinOpsMetrics) => void
): Promise<() => void> {
  const eventSource = new EventSource('/api/finops/metrics/stream');

  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      callback(data);
    } catch (error) {
      console.error('Failed to parse FinOps metrics data:', error);
    }
  };

  eventSource.onerror = (error) => {
    console.error('EventSource error:', error);
  };

  return () => {
    eventSource.close();
  };
}
