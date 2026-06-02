/// TanStack Query hooks for all API endpoints.
/// 每个 hook 暴露 query（GET）或 mutation（POST/PATCH/DELETE）。

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { apiDelete, apiGet, apiPatch, apiPost } from './client';
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

export function useInstance(id: string | undefined) {
  return useQuery({
    queryKey: ['instances', id],
    queryFn: () => apiGet<InstanceDetail>(`/api/instances/${id}`),
    enabled: !!id,
  });
}

// ===== Mutations (POST/PATCH/DELETE) =====

/// POST /api/instances 创建 instance
export function useCreateInstance() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (req: {
      templateId: string;
      alias: string;
      modelId: string;
      apiKey: string;
      opencodeModelId?: string;
      kvCacheEnabled?: boolean;
    }) => apiPost<InstanceDetail>('/api/instances', req),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['instances'] }),
  });
}

/// PATCH /api/instances/:id 改 model/apiKey/opencodeModelId/kvCacheEnabled
/// 注：alias 暂不支持通过 PATCH 修改（需要 delete + recreate）
export function useUpdateInstance(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: {
      modelId?: string;
      apiKey?: string;
      opencodeModelId?: string;
      kvCacheEnabled?: boolean;
    }) => apiPatch<InstanceDetail>(`/api/instances/${id}`, patch),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['instances'] });
    },
  });
}

/// DELETE /api/instances/:id
export function useDeleteInstance() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiDelete(`/api/instances/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['instances'] }),
  });
}

/// POST /api/instances/:id/duplicate
export function useDuplicateInstance() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      apiPost<InstanceDetail>(`/api/instances/${id}/duplicate`, {}),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['instances'] }),
  });
}

// ===== Types =====

/// 详情响应：包含 apiKey
export interface InstanceDetail {
  id: string;
  templateId: string;
  alias: string;
  apiKey: string;
  modelId: string;
  opencodeModelId: string;
  kvCacheEnabled: boolean;
  createdAt: string;
}
