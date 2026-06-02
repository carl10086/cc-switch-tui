import { Link } from 'react-router-dom';
import { useAliasesContent } from '../api/hooks';
import { AliasesPreview } from '../components/AliasesPreview';
import { ApiErrorBanner } from '../components/ApiErrorBanner';

export function AliasesPage() {
  const { data, isLoading, isError, error } = useAliasesContent();

  return (
    <section>
      <h2 className="text-2xl font-bold mb-4">Aliases</h2>

      <p className="text-sm text-muted-foreground mb-4">
        预览 <code className="px-1 py-0.5 rounded bg-muted">~/.cc-switch-tui/aliases.zsh</code> 内容。
        敏感环境变量（KEY/TOKEN/SECRET/PASSWORD/CREDENTIAL）默认脱敏显示，点 Reveal 按钮可临时查看明文。
      </p>

      {isError && <div className="mb-4"><ApiErrorBanner error={error} /></div>}

      {isLoading ? (
        <div className="text-muted-foreground">Loading…</div>
      ) : (
        <AliasesPreview content={data ?? ''} />
      )}

      <p className="mt-4 text-sm text-muted-foreground">
        想写入所有产物？{' '}
        <Link to="/apply" className="text-primary hover:underline font-medium">
          → Apply
        </Link>
      </p>
    </section>
  );
}
