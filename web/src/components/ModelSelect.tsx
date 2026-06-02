import type { TemplateModel } from '../api/types';

interface Props {
  /** 模板的 models 列表；如果为空则降级为 input */
  models: TemplateModel[];
  value: string;
  onChange: (value: string) => void;
  /** 为空时的占位符（仅 input 模式用） */
  placeholder?: string;
}

/**
 * Model 字段的 select/input 切换组件。
 * 当 template.models 非空时显示下拉（option 显示 `name (id)`），为空时降级为 input。
 * 和 OpencodeModelSelect 形态平行。
 */
export function ModelSelect({ models, value, onChange, placeholder }: Props) {
  if (models.length === 0) {
    return (
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder ?? 'MiniMax-M3'}
        className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
      />
    );
  }
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
    >
      {models.map((m) => (
        <option key={m.id} value={m.id}>{m.name} ({m.id})</option>
      ))}
    </select>
  );
}
