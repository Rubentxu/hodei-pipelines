import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getWorkerPools, scaleWorkerPool, updateWorkerPool, WorkerPool } from '@/services/workersApi';

export function useWorkerPools() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['worker-pools'],
    queryFn: getWorkerPools,
    refetchInterval: 15000,
    staleTime: 10000,
    gcTime: 5 * 60 * 1000,
  });

  const scaleMutation = useMutation({
    mutationFn: ({ id, action }: { id: string; action: 'up' | 'down' }) =>
      scaleWorkerPool(id, action),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['worker-pools'] });
      queryClient.invalidateQueries({ queryKey: ['workers'] });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, updates }: { id: string; updates: Partial<WorkerPool> }) =>
      updateWorkerPool(id, updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['worker-pools'] });
    },
  });

  return {
    pools: query.data || [],
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    scalePool: scaleMutation.mutate,
    updatePool: updateMutation.mutate,
    isScaling: scaleMutation.isPending,
    isUpdating: updateMutation.isPending,
  };
}
