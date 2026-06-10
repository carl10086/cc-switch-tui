import { useQuery } from '@tanstack/react-query';
import { apiGet } from './client';

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

export function useSessions() {
  return useQuery({
    queryKey: ['trace-sessions'],
    queryFn: () => apiGet<TraceSession[]>('/api/traces/sessions'),
  });
}

export function useSession(id: string) {
  return useQuery({
    queryKey: ['trace-session', id],
    queryFn: () => apiGet<TraceSession>(`/api/traces/sessions/${id}`),
    enabled: !!id,
  });
}

export function useRecords(id: string) {
  return useQuery({
    queryKey: ['trace-records', id],
    queryFn: () => apiGet<TraceRecord[]>(`/api/traces/sessions/${id}/records`),
    enabled: !!id,
  });
}
