import { useEffect, useState } from 'react';
import { useAliasesContent, useApplyAliases } from '../api/hooks';
import { AliasesPreview } from '../components/AliasesPreview';
import { ApiErrorBanner } from '../components/ApiErrorBanner';
import { ArtifactCard } from '../components/ArtifactCard';

const ALIASES_PATH = '~/.cc-switch-tui/aliases.zsh';

type ApplyState = 'idle' | 'loading' | 'success' | 'error';

export function ApplyPage() {
  const { data: aliasesData, isLoading: aliasesLoading, isError: aliasesError, error: aliasesErr } =
    useAliasesContent();
  const apply = useApplyAliases();

  const [state, setState] = useState<ApplyState>('idle');
  const [serverError, setServerError] = useState<unknown>(null);

  // 成功后 1.5s 回 idle
  useEffect(() => {
    if (state !== 'success') return;
    const t = setTimeout(() => setState('idle'), 1500);
    return () => clearTimeout(t);
  }, [state]);

  // Cmd/Ctrl+Enter 触发 apply
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        void handleApply();
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Apply 只写入一个文件：aliases.zsh（OpenCode 集成已废弃，不再生成 opencode 配置）
  const TOTAL_FILES = 1;

  async function handleApply() {
    setState('loading');
    setServerError(null);
    try {
      await apply.mutateAsync();
      setState('success');
      window.scrollTo({ top: 0, behavior: 'smooth' });
    } catch (e) {
      setServerError(e);
      setState('error');
    }
  }

  if (aliasesError) {
    return (
      <section>
        <h2 className="text-2xl font-bold mb-4">Apply</h2>
        <ApiErrorBanner error={aliasesErr} />
      </section>
    );
  }

  return (
    <section>
      <h2 className="text-2xl font-bold mb-1">Apply</h2>
      <p className="text-sm text-muted-foreground mb-4">
        即将写入 <strong>{TOTAL_FILES}</strong> 个文件到{' '}
        <code className="px-1 py-0.5 rounded bg-muted">~/.cc-switch-tui/</code>。
        点 Apply 后 <code>source ~/.zshrc</code> 生效（<kbd className="px-1 py-0.5 rounded border border-border bg-muted text-xs">⌘</kbd>+<kbd className="px-1 py-0.5 rounded border border-border bg-muted text-xs">Enter</kbd>）。
      </p>

      {/* Sticky 顶栏 */}
      <div
        className="sticky top-0 z-10 -mx-6 px-6 py-3 bg-background/80 backdrop-blur border-b border-border mb-4"
        data-testid="apply-sticky"
      >
        <div className="flex items-center justify-between gap-4">
          <div className="text-sm text-muted-foreground">
            {state === 'success' ? (
              <span className="text-green-700 dark:text-green-300 font-medium">
                ✓ Wrote {TOTAL_FILES} files
              </span>
            ) : state === 'error' ? (
              <span className="text-red-700 dark:text-red-300 font-medium">Apply failed</span>
            ) : (
              <span>将写入 {TOTAL_FILES} 个文件</span>
            )}
          </div>
          <ApplyButton
            state={state}
            disabled={aliasesLoading}
            onClick={handleApply}
          />
        </div>

        {state === 'error' && serverError != null && (
          <div className="mt-3" data-testid="apply-error">
            <div className="text-xs font-semibold text-red-700 dark:text-red-300 mb-1">
              Apply failed:
            </div>
            <ApiErrorBanner error={serverError} />
            <button
              type="button"
              onClick={handleApply}
              data-testid="apply-retry"
              className="mt-2 text-xs px-3 py-1.5 rounded border border-red-300 bg-red-50 text-red-700 hover:bg-red-100 dark:bg-red-950 dark:text-red-200 dark:border-red-800"
            >
              Retry
            </button>
          </div>
        )}
      </div>

      <div className="space-y-3">
        <ArtifactCard
          title="aliases.zsh"
          path={ALIASES_PATH}
          sizeBytes={aliasesData ? new Blob([aliasesData]).size : undefined}
          defaultOpen
          copyText={aliasesData ?? ''}
          copyLabel="Copy"
        >
          {aliasesLoading ? (
            <div className="text-muted-foreground text-sm">Loading…</div>
          ) : (
            <AliasesPreview content={aliasesData ?? ''} />
          )}
        </ArtifactCard>
      </div>
    </section>
  );
}

const APPLY_BTN_BASE =
  'inline-flex items-center gap-2 px-4 py-2 text-sm rounded font-medium transition-colors';
const APPLY_BTN_STATE: Record<ApplyState, { className: string; label: string }> = {
  idle: { className: 'bg-primary text-primary-foreground hover:opacity-90', label: '⚡ Apply all' },
  loading: { className: 'bg-primary text-primary-foreground opacity-80 cursor-wait', label: 'Writing…' },
  success: { className: 'bg-green-600 text-white', label: '✓ Done' },
  error: { className: 'bg-red-600 text-white hover:bg-red-700', label: 'Retry' },
};

function ApplyButton({
  state,
  disabled,
  onClick,
}: {
  state: ApplyState;
  disabled: boolean;
  onClick: () => void;
}) {
  const { className, label } = APPLY_BTN_STATE[state];

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={state === 'loading' || disabled}
      data-testid="apply-all"
      data-state={state}
      className={`${APPLY_BTN_BASE} ${className}`}
    >
      {state === 'loading' && (
        <span className="inline-block w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin" />
      )}
      {label}
    </button>
  );
}

