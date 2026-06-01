/// TanStack Query hooks for all API endpoints.
/// 每个 hook 暴露 query（GET）或 mutation（POST/PATCH/DELETE）。

import { useQuery } from '@tanstack/react-query';
import { apiGet } from './client';
import type { HealthResponse } from './types';

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
