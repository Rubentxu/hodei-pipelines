import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getPipelines, createPipeline, updatePipeline, deletePipeline, executePipeline, Pipeline } from '@/services/pipelineApi';
import { PipelineTask } from '@/types';

export function usePipelines() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['pipelines'],
    queryFn: getPipelines,
    staleTime: 30000,
  });

  const createMutation = useMutation({
    mutationFn: (data: { name: string; description?: string; tasks: PipelineTask[] }) =>
      createPipeline(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: any }) =>
      updatePipeline(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deletePipeline(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });

  const executeMutation = useMutation({
    mutationFn: (id: string) => executePipeline(id),
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
    queryFn: () => getPipelines().then(pipelines =>
      pipelines.find(p => p.id === id)
    ),
    enabled: !!id,
  });
}
