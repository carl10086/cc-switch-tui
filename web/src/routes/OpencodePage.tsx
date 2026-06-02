import { useState } from 'react';
import { Link } from 'react-router-dom';
import {
  useApplyOpencodeConfig,
  useInstances,
  useOpencodeConfig,
} from '../api/hooks';
import { ApiErrorBanner } from '../components/ApiErrorBanner';

export function OpencodePage() {
  const { data: instances, isLoading, isError, error } = useInstances();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [lastApplyResult, setLastApplyResult] = useState<string | null>(null);

  // 默认选第一个
  const activeId = selectedId ?? instances?.[0]?.id ?? null;
  const { data: config, isLoading: configLoading, isError: configError, error: configErr } =
    useOpencodeConfig(activeId ?? undefined);
  const apply = useApplyOpencodeConfig();

  async function handleApply() {
    if (!activeId) return;
    setLastApplyResult(null);
    try {
      const result = await apply.mutateAsync(activeId);
      setLastApplyResult(result.path);
    } catch {
      // error shown via apply.error
    }
  }

  return (
    <section>
      <h2 className="text-2xl font-bold mb-4">OpenCode</h2>
      <p className="text-sm text-muted-foreground mb-4">
        预览每个 instance 对应的 OpenCode 配置 JSON。Apply 写入 <code className="px-1 py-0.5 rounded bg-muted">~/.cc-switch-tui/opencode/{'{alias}'}.json</code>。
      </p>

      {isError && <ApiErrorBanner error={error} />}

      <div className="grid grid-cols-1 md:grid-cols-[200px_1fr] gap-4">
        {/* 左侧 instance 列表 */}
        <aside className="space-y-1">
          {isLoading ? (
            <div className="text-muted-foreground text-sm">Loading…</div>
          ) : (
            instances?.map((i) => (
              <button
                key={i.id}
                type="button"
                onClick={() => {
                  setSelectedId(i.id);
                  setLastApplyResult(null);
                }}
                className={`w-full text-left px-2 py-1.5 text-sm rounded ${
                  activeId === i.id
                    ? 'bg-primary text-primary-foreground'
                    : 'hover:bg-muted'
                }`}
              >
                <div className="font-mono">{i.alias}</div>
                <div className="text-xs opacity-70">{i.templateId}</div>
              </button>
            ))
          )}
        </aside>

        {/* 右侧 JSON */}
        <div>
          {activeId && (
            <div className="flex items-center justify-between mb-2">
              <div className="text-xs text-muted-foreground">
                <Link
                  to={`/instances/${activeId}`}
                  className="hover:underline font-mono"
                >
                  {activeId}
                </Link>
              </div>
              <button
                type="button"
                onClick={handleApply}
                disabled={apply.isPending || !config}
                className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
              >
                {apply.isPending ? 'Applying…' : 'Apply'}
              </button>
            </div>
          )}

          {apply.isError && <div className="mb-2"><ApiErrorBanner error={apply.error} /></div>}
          {lastApplyResult && (
            <div className="mb-2 px-3 py-2 text-sm rounded bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 text-green-800 dark:text-green-200">
              ✓ Written to <code className="font-mono text-xs">{lastApplyResult}</code>
            </div>
          )}
          {configError && <ApiErrorBanner error={configErr} />}

          {configLoading ? (
            <div className="text-muted-foreground text-sm">Loading…</div>
          ) : config ? (
            <pre className="bg-card border border-border rounded p-4 text-xs font-mono overflow-x-auto max-h-[60vh] overflow-y-auto whitespace-pre">
              {JSON.stringify(config, null, 2)}
            </pre>
          ) : (
            <div className="text-muted-foreground text-sm">No instance selected</div>
          )}
        </div>
      </div>
    </section>
  );
}
