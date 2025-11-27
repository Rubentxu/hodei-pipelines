import { useQuery, useMutation } from '@tanstack/react-query';
import { developerApi } from '../services/developerApi';

export function useApiDocumentation() {
  return useQuery({
    queryKey: ['api-documentation'],
    queryFn: () => developerApi.getApiDocumentation(),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

export function useCodeExamples() {
  return useQuery({
    queryKey: ['code-examples'],
    queryFn: () => developerApi.getCodeExamples(),
    staleTime: 10 * 60 * 1000, // 10 minutes
  });
}

export function useTestEndpoint() {
  return useMutation({
    mutationFn: (params: Parameters<typeof developerApi.testEndpoint>[0]) =>
      developerApi.testEndpoint(params),
  });
}

export function useSdkDownloads() {
  return useQuery({
    queryKey: ['sdk-downloads'],
    queryFn: () => developerApi.getSdkDownloads(),
    staleTime: 30 * 60 * 1000, // 30 minutes
  });
}

export function useTutorials() {
  return useQuery({
    queryKey: ['tutorials'],
    queryFn: () => developerApi.getTutorials(),
    staleTime: 10 * 60 * 1000, // 10 minutes
  });
}
