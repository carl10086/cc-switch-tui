import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useSession, useRecords, parseSummary } from '../../api/traces';
import { StatusBadge } from './Dashboard';
import { TokenBadge } from './TokenBadge';

export function TraceViewer() {
  const { id } = useParams<{ id: string }>();
  const { data: session, isLoading: sessionLoading, error: sessionError } = useSession(id ?? '');
  const { data: records, isLoading: recordsLoading } = useRecords(id ?? '');
  const [showRaw, setShowRaw] = useState(false);

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

  const summary = parseSummary(session.summary_json);
  const request = summary?.request;
  const response = summary?.response;

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
          <div className="flex items-center gap-2">
            <button
              onClick={async () => {
                const resp = await fetch(`/api/traces/sessions/${id}/export/jsonl`);
                const blob = await resp.blob();
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = `trace-${id}.jsonl`;
                a.click();
                URL.revokeObjectURL(url);
              }}
              className="text-xs text-primary hover:underline"
            >
              Export JSONL
            </button>
            <StatusBadge status={session.status} />
          </div>
        </div>
        <div className="text-sm text-muted-foreground space-y-1">
          <p>Provider: {session.provider}</p>
          <p>Model: {response?.model || request?.model || session.model}</p>
          <p>Records: {session.record_count}</p>
          {response?.input_tokens !== undefined && (
            <p className="flex items-center gap-2">
              Tokens: <TokenBadge input={response.input_tokens} output={response.output_tokens ?? 0} />
            </p>
          )}
          <p>Started: {new Date(session.started_at).toLocaleString()}</p>
        </div>
      </div>

      {request?.system && (
        <div className="mb-4">
          <h2 className="text-sm font-semibold text-muted-foreground mb-1">System</h2>
          <div className="bg-muted p-3 rounded text-sm">{request.system}</div>
        </div>
      )}

      {request?.messages && request.messages.length > 0 && (
        <div className="mb-6">
          <h2 className="text-md font-semibold mb-3">Messages</h2>
          <div className="space-y-3">
            {request.messages.map((msg, idx) => (
              <MessageBubble key={idx} role={msg.role} content={msg.content} />
            ))}
            {response?.content && (
              <MessageBubble role="assistant" content={response.content} stopReason={response.stop_reason} />
            )}
          </div>
        </div>
      )}

      <div className="mb-2">
        <button
          onClick={() => setShowRaw(!showRaw)}
          className="text-sm text-muted-foreground hover:text-foreground"
        >
          {showRaw ? '▼' : '▶'} Raw Records ({session.record_count})
        </button>
      </div>

      {showRaw && (
        <div className="space-y-3">
          {recordsLoading ? (
            <div className="text-muted-foreground">Loading records...</div>
          ) : !records?.length ? (
            <div className="text-muted-foreground">No records found.</div>
          ) : (
            records.map((r) => (
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
            ))
          )}
        </div>
      )}
    </div>
  );
}

function MessageBubble({ role, content, stopReason }: { role: string; content: string; stopReason?: string }) {
  const isUser = role === 'user';
  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[80%] rounded-lg px-4 py-2 text-sm ${
          isUser ? 'bg-primary text-primary-foreground' : 'bg-muted'
        }`}
      >
        <div className="text-xs opacity-70 mb-1 capitalize">{role}</div>
        <div className="whitespace-pre-wrap">{content}</div>
        {stopReason && <div className="text-xs opacity-50 mt-1">stop: {stopReason}</div>}
      </div>
    </div>
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
