import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getSLAViolations, subscribeToSLAViolations, SLAViolation } from '@/services/slaApi';

export function useSLAViolations() {
  const queryClient = useQueryClient();
  const [isConnected, setIsConnected] = useState(false);

  const query = useQuery({
    queryKey: ['sla-violations'],
    queryFn: getSLAViolations,
    refetchInterval: 15000, // Refresh every 15 seconds
    staleTime: 10000,
    gcTime: 5 * 60 * 1000,
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    subscribeToSLAViolations((violations) => {
      queryClient.setQueryData<SLAViolation[]>(['sla-violations'], violations);
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
    ...query,
    isConnected,
  };
}
