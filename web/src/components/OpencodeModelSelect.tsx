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
 * OpenCode Model ID 字段的 select/input 切换组件。
 * 当模板有 models 列表时显示下拉（带 "use model id" 默认选项）；
 * 否则降级为 input（兼容老 template 或没有 models 字段的情况）。
 */
export function OpencodeModelSelect({ models, value, onChange, placeholder }: Props) {
  if (models.length === 0) {
    return (
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder ?? 'defaults to model'}
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
      <option value="">— default (use model id) —</option>
      {models.map((m) => (
        <option key={m.id} value={m.opencodeModelId}>
          {m.name} → {m.opencodeModelId}
        </option>
      ))}
    </select>
  );
}
