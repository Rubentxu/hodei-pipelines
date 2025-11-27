import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { getClusterTopology, subscribeToTopology, ClusterTopology } from '@/services/clusterApi';

export function useClusterTopology() {
  const queryClient = useQueryClient();
  const [isConnected, setIsConnected] = useState(false);

  const query = useQuery({
    queryKey: ['cluster-topology'],
    queryFn: getClusterTopology,
    refetchInterval: 10000, // Refresh every 10 seconds
    staleTime: 30000,
    gcTime: 5 * 60 * 1000,
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    subscribeToTopology((topology) => {
      queryClient.setQueryData<ClusterTopology>(['cluster-topology'], topology);
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
