import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useInstances, useOpencodeConfig } from '../api/hooks';
import { ApiErrorBanner } from '../components/ApiErrorBanner';
import { OpencodeConfigPreview } from '../components/OpencodeConfigPreview';

export function OpencodePage() {
  const { data: instances, isLoading, isError, error } = useInstances();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // 默认选第一个
  const activeId = selectedId ?? instances?.[0]?.id ?? null;
  const {
    data: config,
    isLoading: configLoading,
    isError: configError,
    error: configErr,
  } = useOpencodeConfig(activeId ?? undefined);

  return (
    <section>
      <h2 className="text-2xl font-bold mb-4">OpenCode</h2>
      <p className="text-sm text-muted-foreground mb-4">
        预览每个 instance 对应的 OpenCode 配置 JSON。写入操作在{' '}
        <Link to="/apply" className="text-primary hover:underline font-medium">Apply</Link> 页面。
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
                onClick={() => setSelectedId(i.id)}
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
            <div className="text-xs text-muted-foreground mb-2">
              <Link
                to={`/instances/${activeId}`}
                className="hover:underline font-mono"
              >
                {activeId}
              </Link>
            </div>
          )}

          {configError && <ApiErrorBanner error={configErr} />}

          {configLoading ? (
            <div className="text-muted-foreground text-sm">Loading…</div>
          ) : config ? (
            <OpencodeConfigPreview config={config} />
          ) : (
            <div className="text-muted-foreground text-sm">No instance selected</div>
          )}

          <p className="mt-4 text-sm text-muted-foreground">
            想写入所有产物？{' '}
            <Link to="/apply" className="text-primary hover:underline font-medium">
              → Apply
            </Link>
          </p>
        </div>
      </div>
    </section>
  );
}
