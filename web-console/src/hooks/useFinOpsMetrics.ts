import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import {
  getFinOpsMetrics,
  subscribeToFinOpsMetrics,
  updateBudgetLimit,
  FinOpsMetrics,
} from '@/services/finopsApi';

export function useFinOpsMetrics() {
  const queryClient = useQueryClient();
  const [isConnected, setIsConnected] = useState(false);

  const query = useQuery({
    queryKey: ['finops-metrics'],
    queryFn: getFinOpsMetrics,
    refetchInterval: 30000, // Refresh every 30 seconds
    staleTime: 20000,
    gcTime: 5 * 60 * 1000,
  });

  const updateBudgetMutation = useMutation({
    mutationFn: (limit: number) => updateBudgetLimit(limit),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['finops-metrics'] });
    },
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    subscribeToFinOpsMetrics((metrics) => {
      queryClient.setQueryData<FinOpsMetrics>(['finops-metrics'], metrics);
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
  }, [queryClient]);

  return {
    data: query.data,
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    updateBudgetLimit: updateBudgetMutation.mutate,
    isUpdating: updateBudgetMutation.isPending,
    isConnected,
  };
}
