import { useEffect, useState } from 'react';
import {
  useDiagnostics,
  useSettings,
  useUpdateSettings,
  type Settings,
} from '../api/hooks';
import { ApiErrorBanner } from '../components/ApiErrorBanner';

export function SettingsPage() {
  const { data: settings, isLoading, isError, error } = useSettings();
  const { data: diag, isError: diagIsError, error: diagError } = useDiagnostics();
  const update = useUpdateSettings();
  const [draft, setDraft] = useState<Settings | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (settings) {
      setDraft(settings);
      setSaved(false);
    }
  }, [settings]);

  async function handleSave() {
    if (!draft) return;
    try {
      await update.mutateAsync(draft);
      setSaved(true);
    } catch {
      // shown via update.error
    }
  }

  if (isLoading || !settings || !draft) {
    return <div className="text-muted-foreground">Loading…</div>;
  }
  if (isError) {
    return <ApiErrorBanner error={error} />;
  }

  return (
    <section className="max-w-2xl space-y-8">
      <div>
        <h2 className="text-2xl font-bold mb-2">Settings</h2>
        <p className="text-sm text-muted-foreground mb-4">
          注：设置当前是 <strong>in-memory</strong>，重启 binary 后重置为默认。
        </p>

        <div className="space-y-4">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={draft.autoOpenBrowser}
              onChange={(e) => {
                setDraft({ ...draft, autoOpenBrowser: e.target.checked });
                setSaved(false);
              }}
              className="rounded"
            />
            <span>Auto-open browser on launch</span>
          </label>

          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1">
              Default template
            </label>
            <select
              value={draft.defaultTemplate ?? ''}
              onChange={(e) => {
                setDraft({
                  ...draft,
                  defaultTemplate: e.target.value === '' ? null : e.target.value,
                });
                setSaved(false);
              }}
              className="w-full max-w-xs px-3 py-1.5 text-sm rounded border border-input bg-background"
            >
              <option value="">(none)</option>
              <option value="minimax">minimax</option>
              <option value="kimi">kimi</option>
            </select>
          </div>

          {update.isError && <ApiErrorBanner error={update.error} />}

          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={handleSave}
              disabled={update.isPending}
              className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
            >
              {update.isPending ? 'Saving…' : 'Save'}
            </button>
            {saved && (
              <span className="text-xs text-green-700 dark:text-green-400">✓ Saved</span>
            )}
          </div>
        </div>
      </div>

      <div>
        <h2 className="text-2xl font-bold mb-2">Diagnostics</h2>
        {diagIsError ? (
          <ApiErrorBanner error={diagError} />
        ) : diag ? (
          <div className="border border-border rounded">
            <Row label="Status">
              <span
                className={`inline-block px-2 py-0.5 rounded text-xs font-mono ${
                  diag.status === 'ok'
                    ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                    : diag.status === 'warn'
                    ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                    : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                }`}
              >
                {diag.status}
              </span>
            </Row>
            <Row label="DB path">
              <code className="text-xs">{diag.dbPath}</code>
              {diag.dbWritable ? <Badge ok>writable</Badge> : <Badge>NOT writable</Badge>}
            </Row>
            <Row label="Aliases path">
              <code className="text-xs">{diag.aliasesPath}</code>
              {diag.aliasesWritable ? <Badge ok>writable</Badge> : <Badge>NOT writable</Badge>}
            </Row>
            <Row label="OpenCode dir">
              <code className="text-xs">{diag.opencodeDir}</code>
              {diag.opencodeDirWritable ? <Badge ok>writable</Badge> : <Badge>NOT writable</Badge>}
            </Row>
            <Row label=".zshrc">
              <code className="text-xs">{diag.zshrcPath}</code>
              {diag.zshrcWritable ? <Badge ok>writable</Badge> : <Badge>NOT writable</Badge>}
            </Row>
            <Row label="Instance count">{diag.instanceCount}</Row>
            <Row label="Template count">{diag.templateCount}</Row>
          </div>
        ) : (
          <div className="text-muted-foreground">Loading…</div>
        )}
      </div>
    </section>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3 px-3 py-2 border-b border-border last:border-b-0 text-sm">
      <span className="w-40 text-muted-foreground text-xs">{label}</span>
      <div className="flex-1 flex items-center gap-2">{children}</div>
    </div>
  );
}

function Badge({ children, ok }: { children: React.ReactNode; ok?: boolean }) {
  return (
    <span
      className={`inline-block px-1.5 py-0.5 rounded text-[10px] font-mono ${
        ok
          ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
          : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
      }`}
    >
      {children}
    </span>
  );
}
