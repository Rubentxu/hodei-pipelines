import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getRoles,
  createRole,
  updateRole,
  deleteRole,
  Role
} from '@/services/securityApi';

export function useRoles() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['security', 'roles'],
    queryFn: getRoles,
    staleTime: 60000,
  });

  const createMutation = useMutation({
    mutationFn: (role: Partial<Role>) => createRole(role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'roles'] });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, updates }: { id: string; updates: Partial<Role> }) =>
      updateRole(id, updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'roles'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteRole(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'roles'] });
    },
  });

  return {
    roles: query.data || [],
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    createRole: createMutation.mutate,
    updateRole: updateMutation.mutate,
    deleteRole: deleteMutation.mutate,
    isCreating: createMutation.isPending,
    isUpdating: updateMutation.isPending,
    isDeleting: deleteMutation.isPending,
  };
}
