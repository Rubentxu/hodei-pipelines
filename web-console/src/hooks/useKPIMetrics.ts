import { observabilityApi } from "@/services/observabilityApi";
import { ObservabilityMetricsResponse } from "@/types";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

export function useKPIMetrics() {
  const queryClient = useQueryClient();
  const [isConnected, setIsConnected] = useState(false);

  const query = useQuery({
    queryKey: ["kpi-metrics"],
    queryFn: () => observabilityApi.getMetrics(),
    refetchInterval: 5000, // Poll every 5 seconds
    staleTime: 30000,
    gcTime: 5 * 60 * 1000, // Keep in cache for 5 minutes
  });

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    observabilityApi.streamMetrics((data) => {
      queryClient.setQueryData<ObservabilityMetricsResponse>(["kpi-metrics"], (old) => {
        return old ? { ...old, ...data } : ({ ...data } as ObservabilityMetricsResponse);
      });
      setIsConnected(true);
    }).then((eventSource) => {
      cleanup = () => eventSource.close();
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
