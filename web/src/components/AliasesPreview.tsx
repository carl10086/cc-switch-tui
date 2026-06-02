import { useState } from 'react';
import { isSensitiveKey, maskValue } from '../lib/mask';

/** 匹配 `  export KEY=VALUE` 形式（前置空格容忍缩进） */
const EXPORT_LINE = /^(\s*export\s+)([A-Z_][A-Z0-9_]*)=(.+)$/;

export function AliasesPreview({ content }: { content: string }) {
  const [revealed, setRevealed] = useState(false);
  const lines = content.split('\n');

  return (
    <div>
      <div className="flex items-center justify-end mb-2">
        <button
          type="button"
          onClick={() => setRevealed((r) => !r)}
          data-testid="reveal-toggle"
          className="text-xs px-2 py-1 rounded border border-border hover:bg-muted"
        >
          {revealed ? '🙈 Hide secrets' : '👁 Reveal secrets'}
        </button>
      </div>
      <pre className="bg-card border border-border rounded p-4 text-xs font-mono overflow-x-auto max-h-[60vh] overflow-y-auto whitespace-pre">
        {lines.map((line, i) => (
          <span key={i}>
            {renderLine(line, revealed)}
            {'\n'}
          </span>
        ))}
      </pre>
    </div>
  );
}

function renderLine(line: string, revealed: boolean) {
  const m = line.match(EXPORT_LINE);
  if (!m) return <>{line}</>;
  const [, prefix, key, value] = m;
  if (!isSensitiveKey(key) || revealed) {
    return <>{line}</>;
  }
  return (
    <>
      {prefix}
      {key}={maskValue(value)}
    </>
  );
}
