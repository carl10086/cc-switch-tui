import { useEffect, useRef } from 'react';

/// 监听 beforeunload 事件 + Router 内的导航（如 react-router 的 unstable_usePrompt）。
/// 简单实现：仅 beforeunload。
/// 完整实现需要 react-router 的 useBlocker（v6+），但用 unstable_ 前缀。
/// M1 用浏览器原生 beforeunload 即可。
export function useUnsavedGuard(when: boolean, message = 'You have unsaved changes. Leave anyway?') {
  const messageRef = useRef(message);
  messageRef.current = message;

  useEffect(() => {
    if (!when) return;
    function handler(e: BeforeUnloadEvent) {
      e.preventDefault();
      e.returnValue = messageRef.current;
      return messageRef.current;
    }
    window.addEventListener('beforeunload', handler);
    return () => window.removeEventListener('beforeunload', handler);
  }, [when]);
}
