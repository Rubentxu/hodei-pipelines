import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import {
  getAuditLogs,
  getSecurityMetrics,
  subscribeToAuditLogs,
  AuditLog,
  SecurityMetricsResponse
} from '@/services/securityApi';

export function useSecurityMetrics() {
  const query = useQuery({
    queryKey: ['security', 'metrics'],
    queryFn: getSecurityMetrics,
    refetchInterval: 60000, // Refresh every minute
    staleTime: 30000,
  });

  return {
    data: query.data,
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
  };
}

export function useAuditLogs(startDate?: string, endDate?: string) {
  const queryClient = useQueryClient();
  const [isConnected, setIsConnected] = useState(false);

  const query = useQuery({
    queryKey: ['security', 'audit-logs', startDate, endDate],
    queryFn: () => getAuditLogs(startDate, endDate),
    staleTime: 10000,
    refetchInterval: 30000,
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    subscribeToAuditLogs((logs) => {
      queryClient.setQueryData<AuditLog[]>(['security', 'audit-logs', startDate, endDate], logs);
      setIsConnected(true);
    }).then((unsubscribeFn) => {
      cleanup = unsubscribeFn;
    });

    const connectionTimeout = setTimeout(() => {
      setIsConnected(false);
    }, 30000);

    return () => {
      cleanup?.();
      clearTimeout(connectionTimeout);
    };
  }, [queryClient, startDate, endDate]);

  return {
    logs: query.data || [],
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    isConnected,
  };
}
