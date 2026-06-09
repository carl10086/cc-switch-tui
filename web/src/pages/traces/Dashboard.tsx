import { useSessions } from '../../api/traces';

export function TraceDashboard() {
  const { data: sessions, isLoading, error } = useSessions();

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
              <th className="text-left py-2 px-4">Status</th>
              <th className="text-left py-2 px-4">Records</th>
              <th className="text-left py-2 px-4">Time</th>
            </tr>
          </thead>
          <tbody>
            {sessions.map((s) => (
              <tr key={s.id} className="border-b last:border-b-0 hover:bg-muted/50">
                <td className="py-2 px-4 font-medium">{s.alias}</td>
                <td className="py-2 px-4">{s.provider}</td>
                <td className="py-2 px-4">{s.model}</td>
                <td className="py-2 px-4">
                  <StatusBadge status={s.status} />
                </td>
                <td className="py-2 px-4">{s.record_count}</td>
                <td className="py-2 px-4 text-muted-foreground">
                  {new Date(s.started_at).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const color =
    status === 'complete'
      ? 'bg-green-100 text-green-700'
      : status === 'error'
        ? 'bg-red-100 text-red-700'
        : 'bg-yellow-100 text-yellow-700';

  return (
    <span className={`inline-block px-2 py-0.5 rounded-full text-xs ${color}`}>
      {status}
    </span>
  );
}
