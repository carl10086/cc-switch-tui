import { useState } from 'react';

interface Props {
  config: Record<string, unknown>;
  /** 初始折叠状态；用户点击后可在内部展开 */
  collapsed?: boolean;
}

/**
 * OpenCode 配置文件 JSON 预览。
 * 默认展开；可传入 `collapsed` 让用户点击展开（节省页面空间）。
 */
export function OpencodeConfigPreview({ config, collapsed = false }: Props) {
  const [expanded, setExpanded] = useState(!collapsed);

  if (!expanded) {
    return (
      <button
        type="button"
        onClick={() => setExpanded(true)}
        className="w-full text-left px-3 py-2 text-xs rounded border border-border bg-card hover:bg-muted text-muted-foreground"
      >
        ▶ Click to expand JSON
      </button>
    );
  }

  return (
    <div>
      <button
        type="button"
        onClick={() => setExpanded(false)}
        className="text-xs text-muted-foreground hover:underline mb-1"
      >
        ▼ Collapse
      </button>
      <pre className="bg-card border border-border rounded p-4 text-xs font-mono overflow-x-auto max-h-[60vh] overflow-y-auto whitespace-pre">
        {JSON.stringify(config, null, 2)}
      </pre>
    </div>
  );
}
