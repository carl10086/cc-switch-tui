/// TanStack Query hooks for all API endpoints.
/// 每个 hook 暴露 query（GET）或 mutation（POST/PATCH/DELETE）。

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPost } from './client';
import type { HealthResponse, Instance } from './types';

// ===== Queries (GET) =====

export function useHealth() {
  return useQuery({
    queryKey: ['health'],
    queryFn: () => apiGet<HealthResponse>('/api/health'),
    staleTime: 0,
    retry: 1,
    refetchOnWindowFocus: false,
  });
}

export function useInstances() {
  return useQuery({
    queryKey: ['instances'],
    queryFn: () => apiGet<Instance[]>('/api/instances'),
  });
}

// ===== Mutations (POST/PATCH/DELETE) =====

/// POST /api/instances 创建 instance
export function useCreateInstance() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (req: Omit<Instance, 'id' | 'opencodeModelId' | 'kvCacheEnabled'> & {
      opencodeModelId?: string;
      kvCacheEnabled?: boolean;
    }) => apiPost<Instance>('/api/instances', req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['instances'] });
    },
  });
}
