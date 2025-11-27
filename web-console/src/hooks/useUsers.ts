import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getUsers,
  createUser,
  updateUser,
  deleteUser,
  toggleUserStatus,
  User
} from '@/services/securityApi';

export function useUsers() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['security', 'users'],
    queryFn: getUsers,
    staleTime: 30000,
  });

  const createMutation = useMutation({
    mutationFn: (user: Partial<User>) => createUser(user),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'users'] });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, updates }: { id: string; updates: Partial<User> }) =>
      updateUser(id, updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'users'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteUser(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'users'] });
    },
  });

  const toggleStatusMutation = useMutation({
    mutationFn: (id: string) => toggleUserStatus(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['security', 'users'] });
    },
  });

  return {
    users: query.data || [],
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    createUser: createMutation.mutate,
    updateUser: updateMutation.mutate,
    deleteUser: deleteMutation.mutate,
    toggleUserStatus: toggleStatusMutation.mutate,
    isCreating: createMutation.isPending,
    isUpdating: updateMutation.isPending,
    isDeleting: deleteMutation.isPending,
    isToggling: toggleStatusMutation.isPending,
  };
}
