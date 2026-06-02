import { useState } from 'react';
import { CopyButton } from './CopyButton';

interface Props {
  title: string;
  path: string;
  sizeBytes?: number;
  defaultOpen?: boolean;
  copyText?: string;
  copyLabel?: string;
  children: React.ReactNode;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

/**
 * 单个产物卡片：标题 + 路径 + (字节数) + (复制按钮) + 可折叠内容。
 * 用 `grid-template-rows: 0fr/1fr` 配 transition 实现平滑折叠。
 */
export function ArtifactCard({
  title,
  path,
  sizeBytes,
  defaultOpen = false,
  copyText,
  copyLabel,
  children,
}: Props) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <section
      data-testid="artifact-card"
      data-open={open}
      className="border border-border rounded-lg overflow-hidden bg-card hover:shadow-md transition-shadow"
    >
      <header className="flex items-center gap-3 px-4 py-3">
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          aria-label="toggle artifact"
          aria-expanded={open}
          className="flex-1 min-w-0 text-left flex items-center gap-3"
        >
          <span
            className="text-muted-foreground text-sm select-none"
            aria-hidden
          >
            {open ? '▾' : '▸'}
          </span>
          <span className="font-mono text-sm font-medium truncate">{title}</span>
          {sizeBytes !== undefined && (
            <span className="text-xs text-muted-foreground shrink-0">
              {formatSize(sizeBytes)}
            </span>
          )}
        </button>
        {copyText !== undefined && (
          <CopyButton text={copyText} label={copyLabel ?? 'Copy'} />
        )}
      </header>

      <div
        className="grid transition-[grid-template-rows] duration-200 ease-out"
        style={{ gridTemplateRows: open ? '1fr' : '0fr' }}
      >
        <div className="overflow-hidden">
          <div className="px-4 py-3 border-t border-border">
            <div className="text-xs text-muted-foreground mb-2 truncate" title={path}>
              {path}
            </div>
            {!open && (
              <div className="text-xs text-muted-foreground italic">
                Click to expand
              </div>
            )}
            {open && <div data-testid="artifact-body">{children}</div>}
          </div>
        </div>
      </div>
    </section>
  );
}
