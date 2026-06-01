import { useHealth } from './api/hooks';

export default function App() {
  const { data, isLoading, isError, error } = useHealth();

  return (
    <div className="min-h-screen bg-background text-foreground p-8">
      <h1 className="text-3xl font-bold">Hello cc-switch</h1>
      <p className="text-sm text-muted-foreground mt-2">
        S0-T5 接入 /api/health 显示后端状态。
      </p>
      <div className="mt-6 p-4 rounded border border-border bg-card text-card-foreground">
        <div className="text-sm font-semibold mb-1">Backend status</div>
        {isLoading && <div className="text-muted-foreground">Checking…</div>}
        {isError && (
          <div className="text-red-600">
            error: {(error as Error).message}
          </div>
        )}
        {data && (
          <div>
            <span className="inline-block px-2 py-1 rounded bg-green-100 text-green-800 text-xs font-mono">
              {data.status}
            </span>
            <span className="ml-3 text-sm text-muted-foreground">
              v{data.version} · db: {data.dbPath}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
