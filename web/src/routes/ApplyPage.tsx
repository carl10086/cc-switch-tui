import { useEffect, useState } from 'react';
import {
  useAliasesContent,
  useApplyAliases,
  useInstances,
  useOpencodeConfig,
} from '../api/hooks';
import { AliasesPreview } from '../components/AliasesPreview';
import { ApiErrorBanner } from '../components/ApiErrorBanner';
import { OpencodeConfigPreview } from '../components/OpencodeConfigPreview';

export function ApplyPage() {
  const { data: aliasesData, isLoading: aliasesLoading, isError: aliasesError, error: aliasesErr } =
    useAliasesContent();
  const { data: instances, isLoading: instancesLoading, isError: instancesError, error: instancesErr } =
    useInstances();
  const apply = useApplyAliases();

  const [toast, setToast] = useState<string | null>(null);
  const [serverError, setServerError] = useState<unknown>(null);

  // 3 秒后自动消失
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 3000);
    return () => clearTimeout(t);
  }, [toast]);

  const instanceCount = instances?.length ?? 0;
  const totalFiles = 1 + instanceCount; // aliases.zsh + N opencode configs

  async function handleApply() {
    setServerError(null);
    setToast(null);
    try {
      await apply.mutateAsync();
      setToast(`✓ Wrote ${totalFiles} files`);
    } catch (e) {
      setServerError(e);
    }
  }

  if (instancesError || aliasesError) {
    return (
      <section>
        <h2 className="text-2xl font-bold mb-4">Apply</h2>
        <ApiErrorBanner error={instancesError ? instancesErr : aliasesErr} />
      </section>
    );
  }

  return (
    <section>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-2xl font-bold">Apply</h2>
        <button
          type="button"
          onClick={handleApply}
          disabled={apply.isPending || instancesLoading || aliasesLoading}
          data-testid="apply-all"
          className="px-4 py-2 text-sm rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
        >
          {apply.isPending ? 'Applying…' : `Apply All (${totalFiles} files)`}
        </button>
      </div>

      <p className="text-sm text-muted-foreground mb-4">
        即将写入 {totalFiles} 个文件到 <code className="px-1 py-0.5 rounded bg-muted">~/.cc-switch-tui/</code>。
        点 Apply 后 <code>source ~/.zshrc</code> 生效。
      </p>

      {toast && (
        <div
          data-testid="apply-toast"
          className="mb-4 px-3 py-2 text-sm rounded bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 text-green-800 dark:text-green-200"
        >
          {toast}
        </div>
      )}

      {serverError != null && (
        <div className="mb-4" data-testid="apply-error">
          <div className="text-sm font-semibold text-red-700 dark:text-red-300 mb-1">
            Apply failed:
          </div>
          <ApiErrorBanner error={serverError} />
        </div>
      )}

      <div className="space-y-4">
        <ProductBlock title="~/.cc-switch-tui/aliases.zsh" defaultOpen>
          {aliasesLoading ? (
            <div className="text-muted-foreground text-sm">Loading…</div>
          ) : (
            <AliasesPreview content={aliasesData ?? ''} />
          )}
        </ProductBlock>

        {instancesLoading ? (
          <div className="text-muted-foreground text-sm">Loading instances…</div>
        ) : (
          instances?.map((inst) => (
            <ProductBlock
              key={inst.id}
              title={`~/.cc-switch-tui/opencode/${inst.alias}.json`}
            >
              <OpencodeConfigInstance id={inst.id} />
            </ProductBlock>
          ))
        )}
      </div>
    </section>
  );
}

function ProductBlock({
  title,
  defaultOpen = false,
  children,
}: {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  return (
    <details open={defaultOpen} className="border border-border rounded">
      <summary className="px-3 py-2 text-sm font-mono cursor-pointer hover:bg-muted select-none">
        {title}
      </summary>
      <div className="p-3 border-t border-border">{children}</div>
    </details>
  );
}

function OpencodeConfigInstance({ id }: { id: string }) {
  const { data, isLoading, isError, error } = useOpencodeConfig(id);
  if (isLoading) return <div className="text-muted-foreground text-sm">Loading…</div>;
  if (isError) return <ApiErrorBanner error={error} />;
  if (!data) return <div className="text-muted-foreground text-sm">No config</div>;
  return <OpencodeConfigPreview config={data} collapsed />;
}
