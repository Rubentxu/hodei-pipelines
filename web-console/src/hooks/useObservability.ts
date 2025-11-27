import { useQuery, useInfiniteQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { observabilityApi } from '../services/observabilityApi';
import { LogEntry } from '../types';

export function useLogs(filters?: {
  level?: string;
  service?: string;
  traceId?: string;
  search?: string;
}) {
  const queryClient = useQueryClient();

  const query = useInfiniteQuery({
    queryKey: ['logs', filters],
    queryFn: async ({ pageParam = 0 }) => {
      const result = await observabilityApi.getLogs({
        ...filters,
        limit: 50,
        offset: pageParam,
      });
      return result;
    },
    getNextPageParam: (lastPage, allPages) => {
      const totalLoaded = allPages.reduce((sum, page) => sum + page.logs.length, 0);
      return totalLoaded < lastPage.total ? totalLoaded : undefined;
    },
    refetchInterval: 5000, // Auto-refetch every 5 seconds
  });

  // Setup real-time log streaming
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    eventSourceRef.current = observabilityApi.streamLogs((newLog: LogEntry) => {
      queryClient.setQueryData(['logs', filters], (oldData: any) => {
        if (!oldData) return oldData;

        const newPages = [...oldData.pages];
        newPages[0] = {
          ...newPages[0],
          logs: [newLog, ...newPages[0].logs],
        };

        return {
          ...oldData,
          pages: newPages,
        };
      });
    });

    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
    };
  }, [queryClient, filters]);

  return query;
}

export function useTraces(filters?: {
  service?: string;
  status?: string;
}) {
  return useInfiniteQuery({
    queryKey: ['traces', filters],
    queryFn: async ({ pageParam = 0 }) => {
      const result = await observabilityApi.getTraces({
        ...filters,
        limit: 20,
        offset: pageParam,
      });
      return result;
    },
    getNextPageParam: (lastPage, allPages) => {
      const totalLoaded = allPages.reduce((sum, page) => sum + page.traces.length, 0);
      return totalLoaded < lastPage.total ? totalLoaded : undefined;
    },
    refetchInterval: 10000,
  });
}

export function useTrace(traceId: string) {
  return useQuery({
    queryKey: ['trace', traceId],
    queryFn: () => observabilityApi.getTraceById(traceId),
    enabled: !!traceId,
  });
}

export function useObservabilityMetrics() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['observability-metrics'],
    queryFn: () => observabilityApi.getMetrics(),
    refetchInterval: 5000, // Update every 5 seconds
  });

  // Setup real-time metrics streaming
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    eventSourceRef.current = observabilityApi.streamMetrics((metrics) => {
      queryClient.setQueryData(['observability-metrics'], metrics);
    });

    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
    };
  }, [queryClient]);

  return query;
}

export function useAlertRules() {
  return useQuery({
    queryKey: ['alert-rules'],
    queryFn: () => observabilityApi.getAlertRules(),
    refetchInterval: 30000,
  });
}

export function useCreateAlertRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (rule: Parameters<typeof observabilityApi.createAlertRule>[0]) =>
      observabilityApi.createAlertRule(rule),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alert-rules'] });
    },
  });
}

export function useUpdateAlertRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ ruleId, updates }: { ruleId: string; updates: any }) =>
      observabilityApi.updateAlertRule(ruleId, updates),
    onSuccess: (data) => {
      queryClient.setQueryData(['alert-rules'], (oldData: any) => {
        if (!oldData) return oldData;
        return oldData.map((rule: any) =>
          rule.id === data.id ? data : rule
        );
      });
    },
  });
}

export function useDeleteAlertRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (ruleId: string) => observabilityApi.deleteAlertRule(ruleId),
    onSuccess: (_, ruleId) => {
      queryClient.setQueryData(['alert-rules'], (oldData: any) => {
        if (!oldData) return oldData;
        return oldData.filter((rule: any) => rule.id !== ruleId);
      });
    },
  });
}

export function useAlerts(filters?: {
  severity?: string;
  acknowledged?: boolean;
}) {
  return useInfiniteQuery({
    queryKey: ['alerts', filters],
    queryFn: async ({ pageParam = 0 }) => {
      const result = await observabilityApi.getAlerts({
        ...filters,
        limit: 30,
        offset: pageParam,
      });
      return result;
    },
    getNextPageParam: (lastPage, allPages) => {
      const totalLoaded = allPages.reduce((sum, page) => sum + page.alerts.length, 0);
      return totalLoaded < lastPage.total ? totalLoaded : undefined;
    },
    refetchInterval: 10000,
  });
}

export function useAcknowledgeAlert() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ alertId, userId }: { alertId: string; userId: string }) =>
      observabilityApi.acknowledgeAlert(alertId, userId),
    onSuccess: (data) => {
      queryClient.setQueryData(['alerts'], (oldData: any) => {
        if (!oldData) return oldData;

        const newPages = oldData.pages.map((page: any) => ({
          ...page,
          alerts: page.alerts.map((alert: any) =>
            alert.id === data.id ? data : alert
          ),
        }));

        return {
          ...oldData,
          pages: newPages,
        };
      });
    },
  });
}

export function useServiceMap() {
  return useQuery({
    queryKey: ['service-map'],
    queryFn: () => observabilityApi.getServiceMap(),
    refetchInterval: 30000,
  });
}
