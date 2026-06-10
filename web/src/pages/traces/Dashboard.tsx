import { Link } from 'react-router-dom';
import { useSessions, parseSummary, useDeleteSession } from '../../api/traces';

function getModel(s: { model: string; summary_json?: string }): string {
  const summary = parseSummary(s.summary_json);
  return summary?.response?.model || summary?.request?.model || s.model;
}

function getTokens(s: { summary_json?: string }): string {
  const summary = parseSummary(s.summary_json);
  const input = summary?.response?.input_tokens;
  const output = summary?.response?.output_tokens;
  if (input !== undefined && output !== undefined) {
    return `${input}/${output}`;
  }
  return '—';
}

export function TraceDashboard() {
  const { data: sessions, isLoading, error } = useSessions();
  const deleteMutation = useDeleteSession();

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

  if (!sessions?.length) {
    return (
      <div className="text-muted-foreground">
        No trace sessions found. Use <code>ys-proxy cl-{'<alias>'}</code> to create one.
      </div>
    );
  }

  return (
    <div>
      <h1 className="text-xl font-semibold mb-4">Trace Sessions</h1>
      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b bg-muted">
              <th className="text-left py-2 px-4">Alias</th>
              <th className="text-left py-2 px-4">Provider</th>
              <th className="text-left py-2 px-4">Model</th>
              <th className="text-left py-2 px-4">Tokens</th>
              <th className="text-left py-2 px-4">Status</th>
              <th className="text-left py-2 px-4">Time</th>
              <th className="text-left py-2 px-4"></th>
            </tr>
          </thead>
          <tbody>
            {sessions.map((s) => (
              <tr key={s.id} className="border-b last:border-b-0 hover:bg-muted/50">
                <td className="py-2 px-4 font-medium">{s.alias}</td>
                <td className="py-2 px-4">{s.provider}</td>
                <td className="py-2 px-4">{getModel(s)}</td>
                <td className="py-2 px-4 text-muted-foreground">{getTokens(s)}</td>
                <td className="py-2 px-4">
                  <StatusBadge status={s.status} />
                </td>
                <td className="py-2 px-4 text-muted-foreground">
                  {new Date(s.started_at).toLocaleString()}
                </td>
                <td className="py-2 px-4">
                  <div className="flex items-center gap-2">
                    <Link
                      to={`/traces/${s.id}`}
                      className="text-xs text-primary hover:underline"
                    >
                      View
                    </Link>
                    <button
                      onClick={() => deleteMutation.mutate(s.id)}
                      disabled={deleteMutation.isPending}
                      className="text-xs text-red-500 hover:text-red-700 disabled:opacity-50"
                    >
                      {deleteMutation.isPending ? '...' : 'Delete'}
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
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
