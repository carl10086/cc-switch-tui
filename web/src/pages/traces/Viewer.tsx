import { useParams, Link } from 'react-router-dom';
import { useSession, useRecords } from '../../api/traces';

export function TraceViewer() {
  const { id } = useParams<{ id: string }>();
  const { data: session, isLoading: sessionLoading, error: sessionError } = useSession(id ?? '');
  const { data: records, isLoading: recordsLoading } = useRecords(id ?? '');

  if (sessionLoading) {
    return <div className="text-muted-foreground">Loading session...</div>;
  }

  if (sessionError || !session) {
    return (
      <div className="text-red-500">
        Error: {sessionError instanceof Error ? sessionError.message : 'Session not found'}
      </div>
    );
  }

  return (
    <div>
      <div className="mb-4">
        <Link to="/traces" className="text-sm text-muted-foreground hover:text-foreground">
          ← Back to sessions
        </Link>
      </div>

      <div className="border rounded-lg p-4 mb-6">
        <div className="flex items-center justify-between mb-2">
          <h1 className="text-lg font-semibold">{session.alias}</h1>
          <StatusBadge status={session.status} />
        </div>
        <div className="text-sm text-muted-foreground space-y-1">
          <p>Provider: {session.provider}</p>
          <p>Model: {session.model}</p>
          <p>Records: {session.record_count}</p>
          <p>Started: {new Date(session.started_at).toLocaleString()}</p>
        </div>
      </div>

      <h2 className="text-md font-semibold mb-3">Records</h2>

      {recordsLoading ? (
        <div className="text-muted-foreground">Loading records...</div>
      ) : !records?.length ? (
        <div className="text-muted-foreground">No records found.</div>
      ) : (
        <div className="space-y-3">
          {records.map((r) => (
            <div key={r.record_index} className="border rounded-lg p-3">
              <div className="flex items-center gap-2 mb-2">
                <DirectionBadge direction={r.direction} />
                <span className="text-xs text-muted-foreground">
                  {r.timestamp ? new Date(r.timestamp).toLocaleString() : '—'}
                </span>
              </div>
              <pre className="text-xs bg-muted p-2 rounded overflow-auto max-h-96">
                {formatPayload(r.payload_json)}
              </pre>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
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

function DirectionBadge({ direction }: { direction: string }) {
  const isRequest = direction === 'request';
  return (
    <span
      className={`inline-block px-2 py-0.5 rounded-full text-xs ${
        isRequest ? 'bg-blue-100 text-blue-700' : 'bg-purple-100 text-purple-700'
      }`}
    >
      {direction}
    </span>
  );
}

function formatPayload(json: string): string {
  try {
    return JSON.stringify(JSON.parse(json), null, 2);
  } catch {
    return json;
  }
}
