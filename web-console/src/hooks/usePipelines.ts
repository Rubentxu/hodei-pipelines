import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { CreatePipelineRequest, pipelineApi, UpdatePipelineRequest } from '../services/pipelineApi';

export function usePipelines() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['pipelines'],
    queryFn: async () => {
      const response = await pipelineApi.listPipelines();
      return response.items || [];
    },
    staleTime: 30000,
  });

  const createMutation = useMutation({
    mutationFn: (data: CreatePipelineRequest) =>
      pipelineApi.createPipeline(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdatePipelineRequest }) =>
      pipelineApi.updatePipeline(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => pipelineApi.deletePipeline(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });

  const executeMutation = useMutation({
    mutationFn: (id: string) => pipelineApi.executePipeline(id),
  });

  return {
    pipelines: query.data || [],
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    createPipeline: createMutation.mutate,
    updatePipeline: updateMutation.mutate,
    deletePipeline: deleteMutation.mutate,
    executePipeline: executeMutation.mutate,
    isCreating: createMutation.isPending,
    isUpdating: updateMutation.isPending,
    isDeleting: deleteMutation.isPending,
    isExecuting: executeMutation.isPending,
  };
}

export function usePipeline(id: string) {
  return useQuery({
    queryKey: ['pipelines', id],
    queryFn: () => pipelineApi.getPipeline(id),
    enabled: !!id,
  });
}
