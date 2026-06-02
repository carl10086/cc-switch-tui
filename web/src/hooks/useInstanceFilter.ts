import { useMemo, useState } from 'react';
import type { Instance } from '../api/types';

/// 客户端过滤：按 alias / templateId / modelId 模糊匹配（大小写不敏感）
export function useInstanceFilter(instances: Instance[]) {
  const [query, setQuery] = useState('');
  const filtered = useMemo(() => {
    if (!query.trim()) return instances;
    const q = query.toLowerCase();
    return instances.filter(
      (i) =>
        i.alias.toLowerCase().includes(q) ||
        i.templateId.toLowerCase().includes(q) ||
        i.modelId.toLowerCase().includes(q),
    );
  }, [instances, query]);
  return { query, setQuery, filtered };
}
