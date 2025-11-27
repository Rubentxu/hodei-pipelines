import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import {
  getWorkers,
  getWorker,
  startWorker,
  stopWorker,
  restartWorker,
  subscribeToWorkers,
  Worker,
} from '@/services/workersApi';

export function useWorkers() {
  const queryClient = useQueryClient();
  const [isConnected, setIsConnected] = useState(false);

  const query = useQuery({
    queryKey: ['workers'],
    queryFn: getWorkers,
    refetchInterval: 10000, // Refresh every 10 seconds
    staleTime: 5000,
    gcTime: 5 * 60 * 1000,
  });

  const startMutation = useMutation({
    mutationFn: (id: string) => startWorker(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workers'] });
    },
  });

  const stopMutation = useMutation({
    mutationFn: (id: string) => stopWorker(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workers'] });
    },
  });

  const restartMutation = useMutation({
    mutationFn: (id: string) => restartWorker(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workers'] });
    },
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    subscribeToWorkers((workers) => {
      queryClient.setQueryData<Worker[]>(['workers'], workers);
      setIsConnected(true);
    }).then((unsubscribeFn) => {
      cleanup = unsubscribeFn;
    });

    const connectionTimeout = setTimeout(() => {
      setIsConnected(false);
    }, 15000);

    return () => {
      cleanup?.();
      clearTimeout(connectionTimeout);
    };
  }, [queryClient]);

  return {
    workers: query.data || [],
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    startWorker: startMutation.mutate,
    stopWorker: stopMutation.mutate,
    restartWorker: restartMutation.mutate,
    isStarting: startMutation.isPending,
    isStopping: stopMutation.isPending,
    isRestarting: restartMutation.isPending,
    isConnected,
  };
}

export function useWorker(id: string) {
  return useQuery({
    queryKey: ['workers', id],
    queryFn: () => getWorker(id),
    enabled: !!id,
  });
}
