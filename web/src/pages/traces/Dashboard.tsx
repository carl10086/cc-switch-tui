import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useSessions, parseSummary, useDeleteSession, useClearAllSessions } from '../../api/traces';
import { TokenBadge } from './TokenBadge';

interface SessionCardProps {
  session: {
    id: string;
    alias: string;
    provider: string;
    model: string;
    status: string;
    record_count: number;
    started_at: string;
    summary_json?: string;
  };
  onDelete: (id: string) => void;
  isDeleting: boolean;
}

function SessionCard({ session, onDelete, isDeleting }: SessionCardProps) {
  const summary = parseSummary(session.summary_json);
  const input = summary?.response?.input_tokens;
  const output = summary?.response?.output_tokens;

  return (
    <div className="border rounded-lg p-4 hover:border-primary/50 transition-colors bg-card">
      <div className="flex items-start justify-between mb-3">
        <div>
          <h3 className="font-semibold text-base">{session.alias}</h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            {session.provider} · {summary?.response?.model || summary?.request?.model || session.model}
          </p>
        </div>
        <StatusBadge status={session.status} />
      </div>

      <div className="flex items-center gap-3 mb-3">
        {input !== undefined && output !== undefined && (
          <TokenBadge input={input} output={output} />
        )}
        <span className="text-xs text-muted-foreground">
          {session.record_count} records
        </span>
      </div>

      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {new Date(session.started_at).toLocaleString()}
        </span>
        <div className="flex items-center gap-2">
          <Link
            to={`/traces/${session.id}`}
            className="text-xs text-primary hover:underline"
          >
            View
          </Link>
          <button
            onClick={() => onDelete(session.id)}
            disabled={isDeleting}
            className="text-xs text-red-500 hover:text-red-700 disabled:opacity-50"
          >
            {isDeleting ? '...' : 'Delete'}
          </button>
        </div>
      </div>
    </div>
  );
}

export function TraceDashboard() {
  const { data: sessions, isLoading, error } = useSessions();
  const deleteMutation = useDeleteSession();
  const clearAllMutation = useClearAllSessions();
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [dateFilter, setDateFilter] = useState<string>('all');
  const [query, setQuery] = useState('');

  const dateOptions = useMemo(() => {
    const today = new Date().toISOString().slice(0, 10);
    const yesterday = new Date(Date.now() - 86400000).toISOString().slice(0, 10);
    return [
      { key: 'all', label: 'All' },
      { key: today, label: 'Today' },
      { key: yesterday, label: 'Yesterday' },
    ];
  }, []);

  const filtered = useMemo(() => {
    if (!sessions) return [];
    return sessions.filter((s) => {
      const matchAlias = s.alias.toLowerCase().includes(query.toLowerCase());
      const matchDate = dateFilter === 'all' || s.date_key === dateFilter;
      return matchAlias && matchDate;
    });
  }, [sessions, query, dateFilter]);

  if (isLoading) {
    return <div className="text-muted-foreground">Loading sessions...</div>;
  }

  if (error) {
    return (
      <div className="text-red-500">
        Error: {error instanceof Error ? error.message : String(error)}
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold">Trace Sessions</h1>
        <div className="flex items-center gap-3">
          <input
            type="text"
            placeholder="Search alias..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="text-sm border rounded px-3 py-1.5 w-48 bg-background"
          />
          {sessions && sessions.length > 0 && (
            <button
              onClick={() => {
                if (confirm(`Delete all ${sessions.length} sessions? This cannot be undone.`)) {
                  clearAllMutation.mutate();
                }
              }}
              disabled={clearAllMutation.isPending}
              className="text-xs text-red-500 hover:text-red-700 disabled:opacity-50 border border-red-200 rounded px-3 py-1.5"
            >
              {clearAllMutation.isPending ? 'Clearing...' : 'Clear All'}
            </button>
          )}
        </div>
      </div>

      <div className="flex gap-6">
        {/* 左侧日期筛选 */}
        <div className="w-32 shrink-0 space-y-1">
          <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
            Date
          </h2>
          {dateOptions.map((opt) => (
            <button
              key={opt.key}
              onClick={() => setDateFilter(opt.key)}
              className={`w-full text-left text-sm px-3 py-1.5 rounded transition-colors ${
                dateFilter === opt.key
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:bg-muted'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {/* 右侧 session 列表 */}
        <div className="flex-1">
          {!filtered.length ? (
            <div className="text-muted-foreground">
              {query || dateFilter !== 'all'
                ? 'No matching sessions.'
                : 'No trace sessions found. Use ys-proxy cl-&lt;alias&gt; to create one.'}
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {filtered.map((s) => (
                <SessionCard
                  key={s.id}
                  session={s}
                  onDelete={(id) => {
                    setDeletingId(id);
                    deleteMutation.mutate(id, {
                      onSettled: () => setDeletingId(null),
                    });
                  }}
                  isDeleting={deletingId === s.id}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export function StatusBadge({ status }: { status: string }) {
  const STATUS_COLORS: Record<string, string> = {
    complete: 'bg-green-100 text-green-700',
    error: 'bg-red-100 text-red-700',
  };
  const color = STATUS_COLORS[status] ?? 'bg-yellow-100 text-yellow-700';

  return (
    <span className={`inline-block px-2 py-0.5 rounded-full text-xs ${color}`}>
      {status}
    </span>
  );
}
