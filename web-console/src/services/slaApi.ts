export interface SLAViolation {
  id: string;
  message: string;
  severity: 'critical' | 'warning' | 'error';
  timestamp?: string;
}

export interface SLAViolationsResponse {
  violations: SLAViolation[];
  total: number;
  criticalCount: number;
  warningCount: number;
}

export async function getSLAViolations(): Promise<SLAViolationsResponse> {
  const response = await fetch('/api/observability/sla/violations');

  if (!response.ok) {
    throw new Error('Failed to fetch SLA violations');
  }

  return response.json();
}

export async function subscribeToSLAViolations(
  callback: (violations: SLAViolation[]) => void
): Promise<() => void> {
  const eventSource = new EventSource('/api/observability/sla/violations/stream');

  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      callback(data.violations || []);
    } catch (error) {
      console.error('Failed to parse SLA violations data:', error);
    }
  };

  eventSource.onerror = (error) => {
    console.error('EventSource error:', error);
  };

  return () => {
    eventSource.close();
  };
}
