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

export function useSessions() {
  return useQuery({
    queryKey: ['trace-sessions'],
    queryFn: () => apiGet<TraceSession[]>('/api/traces/sessions'),
  });
}
