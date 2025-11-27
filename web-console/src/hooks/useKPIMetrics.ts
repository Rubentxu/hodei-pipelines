import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { getKPIMetrics, subscribeToMetrics } from "@/services/observabilityApi";
import { KPIMetrics } from "@/types";

export function useKPIMetrics() {
  const queryClient = useQueryClient();
  const [isConnected, setIsConnected] = useState(false);

  const query = useQuery({
    queryKey: ["kpi-metrics"],
    queryFn: getKPIMetrics,
    refetchInterval: 5000, // Poll every 5 seconds
    staleTime: 30000,
    gcTime: 5 * 60 * 1000, // Keep in cache for 5 minutes
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    subscribeToMetrics((data) => {
      queryClient.setQueryData<KPIMetrics>(["kpi-metrics"], (old) => {
        return old ? { ...old, ...data } : ({ ...data } as KPIMetrics);
      });
      setIsConnected(true);
    }).then((unsubscribeFn) => {
      cleanup = unsubscribeFn;
    });

    const connectionTimeout = setTimeout(() => {
      setIsConnected(false);
    }, 10000);

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
