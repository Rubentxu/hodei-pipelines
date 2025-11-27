import { http, HttpResponse } from 'msw';

// Observability API mock
export const handlers = [
  // Dashboard KPIs
  http.get('/api/observability/metrics/kpis', () => {
    return HttpResponse.json({
      activeJobs: 452,
      clusterHealth: 98,
      monthlyCost: 1240,
      queuePressure: 3,
    });
  }),

  // Pipeline list
  http.get('/api/pipelines', () => {
    return HttpResponse.json([
      {
        id: 'pipe-123',
        name: 'web-app-build',
        status: 'active',
        lastRun: '2025-11-27T14:32:00Z',
        tags: ['react', 'docker'],
      },
      {
        id: 'pipe-124',
        name: 'api-deploy',
        status: 'paused',
        lastRun: '2025-11-27T13:15:00Z',
        tags: ['nodejs', 'postgresql'],
      },
    ]);
  }),

  // Execution history
  http.get('/api/pipelines/:id/executions', () => {
    return HttpResponse.json([
      {
        id: 'exec-1247',
        status: 'success',
        duration: 323,
        cost: 0.42,
        startedAt: '2025-11-27T14:32:00Z',
        trigger: 'manual',
      },
    ]);
  }),
];
