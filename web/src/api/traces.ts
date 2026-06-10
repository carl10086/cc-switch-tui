import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiDelete } from './client';

export interface TraceSession {
  id: string;
  started_at: string;
  updated_at: string;
  date_key: string;
  alias: string;
  provider: string;
  model: string;
  status: string;
  record_count: number;
  summary_json?: string;
}

export interface TraceRecord {
  session_id: string;
  record_index: number;
  turn: number | null;
  timestamp: string | null;
  direction: string;
  payload_json: string;
}

export interface SessionSummary {
  request?: {
    model?: string;
    messages?: Array<{ role: string; content: string }>;
    max_tokens?: number;
    system?: string;
  };
  response?: {
    content?: string;
    stop_reason?: string;
    input_tokens?: number;
    output_tokens?: number;
    model?: string;
  };
}

export function parseSummary(json?: string): SessionSummary | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as SessionSummary;
  } catch {
    return null;
  }
}

interface ListSessionsResponse {
  sessions: TraceSession[];
  total: number;
}

export function useSessions() {
  return useQuery({
    queryKey: ['trace-sessions'],
    queryFn: async () => {
      const resp = await apiGet<ListSessionsResponse>('/api/traces/sessions');
      return resp.sessions;
    },
    refetchInterval: 5_000,
  });
}

export function useSession(id: string) {
  return useQuery({
    queryKey: ['trace-session', id],
    queryFn: () => apiGet<TraceSession>(`/api/traces/sessions/${id}`),
    enabled: !!id,
  });
}

interface GetRecordsResponse {
  records: TraceRecord[];
}

export function useRecords(id: string) {
  return useQuery({
    queryKey: ['trace-records', id],
    queryFn: async () => {
      const resp = await apiGet<GetRecordsResponse>(`/api/traces/sessions/${id}/records`);
      return resp.records;
    },
    enabled: !!id,
  });
}

export function useDeleteSession() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiDelete(`/api/traces/sessions/${id}`),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['trace-sessions'] });
    },
  });
}

export function useClearAllSessions() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => apiDelete('/api/traces/sessions'),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['trace-sessions'] });
    },
  });
}
