import { useInstances } from '../api/hooks';
import { InstancesTable } from '../components/InstancesTable';
import { ApiErrorBanner } from '../components/ApiErrorBanner';
import { useInstanceFilter } from '../hooks/useInstanceFilter';

export function InstancesPage() {
  const { data, isLoading, isError, error } = useInstances();
  const { query, setQuery, filtered } = useInstanceFilter(data ?? []);

  return (
    <section>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-2xl font-bold">Instances</h2>
        <button
          type="button"
          onClick={() => alert('TODO: open create dialog (S2)')}
          className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          + New
        </button>
      </div>

      {isError && <div className="mb-4"><ApiErrorBanner error={error} /></div>}

      {!isLoading && data && data.length > 0 && (
        <div className="mb-4">
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search alias / template / model…"
            className="w-full max-w-sm px-3 py-1.5 text-sm rounded border border-input bg-background"
          />
        </div>
      )}

      {isLoading ? (
        <div className="text-muted-foreground">Loading…</div>
      ) : (
        <InstancesTable instances={filtered} />
      )}
    </section>
  );
}
