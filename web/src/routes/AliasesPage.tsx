import { useState } from 'react';
import { useAliasesContent, useApplyAliases } from '../api/hooks';
import { AliasesPreview } from '../components/AliasesPreview';
import { ApiErrorBanner } from '../components/ApiErrorBanner';

export function AliasesPage() {
  const { data, isLoading, isError, error } = useAliasesContent();
  const apply = useApplyAliases();
  const [lastApplyResult, setLastApplyResult] = useState<string | null>(null);

  async function handleApply() {
    setLastApplyResult(null);
    try {
      const result = await apply.mutateAsync();
      setLastApplyResult(result.path);
    } catch (e) {
      // shown via mutation.error
    }
  }

  return (
    <section>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-2xl font-bold">Aliases</h2>
        <button
          type="button"
          onClick={handleApply}
          disabled={apply.isPending}
          className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
        >
          {apply.isPending ? 'Applying…' : 'Apply'}
        </button>
      </div>

      <p className="text-sm text-muted-foreground mb-4">
        预览 <code className="px-1 py-0.5 rounded bg-muted">~/.cc-switch-tui/aliases.zsh</code> 内容。点 Apply 会写入该文件（同时也会重新生成 OpenCode 配置文件）。改完后 <code>source ~/.zshrc</code> 生效。
        <br />
        <span className="text-xs">
          敏感环境变量（KEY/TOKEN/SECRET/PASSWORD/CREDENTIAL）默认脱敏显示，点 Reveal 按钮可临时查看明文。
        </span>
      </p>

      {isError && <div className="mb-4"><ApiErrorBanner error={error} /></div>}
      {apply.isError && (
        <div className="mb-4"><ApiErrorBanner error={apply.error} /></div>
      )}

      {lastApplyResult && (
        <div className="mb-4 px-3 py-2 text-sm rounded bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 text-green-800 dark:text-green-200">
          ✓ Written to <code className="font-mono text-xs">{lastApplyResult}</code>
        </div>
      )}

      {isLoading ? (
        <div className="text-muted-foreground">Loading…</div>
      ) : (
        <AliasesPreview content={data ?? ''} />
      )}
    </section>
  );
}
