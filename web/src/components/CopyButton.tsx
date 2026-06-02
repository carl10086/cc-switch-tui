import { useEffect, useState } from 'react';

type State = 'idle' | 'copied' | 'failed';

const RESET_MS = 1500;

/**
 * 复制文本到剪贴板，附带 1.5s 状态反馈。
 * 失败原因：clipboard API 不可用（HTTP/老浏览器）或用户拒绝授权。
 */
export function CopyButton({ text, label = 'Copy' }: { text: string; label?: string }) {
  const [state, setState] = useState<State>('idle');

  useEffect(() => {
    if (state === 'idle') return;
    const t = setTimeout(() => setState('idle'), RESET_MS);
    return () => clearTimeout(t);
  }, [state]);

  async function handle() {
    try {
      if (!navigator.clipboard) throw new Error('clipboard unavailable');
      await navigator.clipboard.writeText(text);
      setState('copied');
    } catch {
      setState('failed');
    }
  }

  const text_ = state === 'copied' ? '✓ Copied' : state === 'failed' ? '✗ Failed' : label;
  const className =
    state === 'copied'
      ? 'text-xs px-2 py-1 rounded border border-green-300 bg-green-50 text-green-800 dark:bg-green-950 dark:text-green-200 dark:border-green-800'
      : state === 'failed'
        ? 'text-xs px-2 py-1 rounded border border-red-300 bg-red-50 text-red-800 dark:bg-red-950 dark:text-red-200 dark:border-red-800'
        : 'text-xs px-2 py-1 rounded border border-border hover:bg-muted';

  return (
    <button type="button" onClick={handle} className={className} aria-live="polite">
      {text_}
    </button>
  );
}
