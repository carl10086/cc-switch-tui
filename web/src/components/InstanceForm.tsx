import { useState } from 'react';
import { ApiError } from '../api/client';
import { instanceSchema, type InstanceFormValues } from '../lib/validate';
import { SecretInput } from './SecretInput';

/// 临时硬编码的 templates + models 列表。
/// S4 会换成从 /api/templates 拉取。
const HARDCODED_TEMPLATES = [
  { id: 'minimax', name: 'MiniMax', defaultModel: 'MiniMax-M3' },
  { id: 'kimi', name: 'Kimi', defaultModel: 'kimi-for-coding' },
] as const;

export function InstanceForm({
  onSubmit,
  onCancel,
  isSubmitting,
  serverError,
  initial,
  submitLabel = 'Create',
}: {
  onSubmit: (values: InstanceFormValues) => Promise<void> | void;
  onCancel?: () => void;
  isSubmitting?: boolean;
  serverError?: unknown;
  initial?: Partial<InstanceFormValues>;
  submitLabel?: string;
}) {
  const [values, setValues] = useState<InstanceFormValues>({
    templateId: initial?.templateId ?? 'minimax',
    alias: initial?.alias ?? '',
    modelId: initial?.modelId ?? 'MiniMax-M3',
    apiKey: initial?.apiKey ?? '',
    opencodeModelId: initial?.opencodeModelId ?? '',
    kvCacheEnabled: initial?.kvCacheEnabled ?? false,
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  function set<K extends keyof InstanceFormValues>(key: K, value: InstanceFormValues[K]) {
    setValues((v) => ({ ...v, [key]: value }));
    setErrors((e) => ({ ...e, [key]: '' }));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setErrors({});
    const result = instanceSchema.safeParse(values);
    if (!result.success) {
      const fieldErrors: Record<string, string> = {};
      for (const issue of result.error.issues) {
        const k = issue.path[0];
        if (typeof k === 'string' && !fieldErrors[k]) fieldErrors[k] = issue.message;
      }
      setErrors(fieldErrors);
      return;
    }
    await onSubmit(result.data);
  }

  // 提取服务端 field error (ApiError.field)
  const serverFieldError =
    serverError instanceof ApiError && serverError.code === 'VALIDATION_ERROR' &&
    typeof (serverError as ApiError & { _field?: string })._field === 'string'
      ? ((serverError as unknown) as { _field?: string })._field
      : undefined;
  const generalServerError =
    serverError && !(serverError instanceof ApiError)
      ? (serverError as Error).message
      : serverError instanceof ApiError && serverError.code !== 'VALIDATION_ERROR'
        ? serverError.message
        : undefined;

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <Field label="Template" error={errors.templateId}>
        <select
          value={values.templateId}
          onChange={(e) => set('templateId', e.target.value)}
          className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background"
        >
          {HARDCODED_TEMPLATES.map((t) => (
            <option key={t.id} value={t.id}>{t.name}</option>
          ))}
        </select>
      </Field>

      <Field label="Alias" error={errors.alias || serverFieldError === 'alias' ? 'Alias already exists' : ''}>
        <input
          value={values.alias}
          onChange={(e) => set('alias', e.target.value)}
          placeholder="cl-mini"
          className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
        />
      </Field>

      <Field label="Model" error={errors.modelId}>
        <input
          value={values.modelId}
          onChange={(e) => set('modelId', e.target.value)}
          placeholder="MiniMax-M3"
          className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
        />
      </Field>

      <Field label="API Key" error={errors.apiKey}>
        <SecretInput
          value={values.apiKey}
          onChange={(v) => set('apiKey', v)}
          placeholder="sk-..."
        />
      </Field>

      <Field label="OpenCode Model ID (optional)" error={errors.opencodeModelId}>
        <input
          value={values.opencodeModelId ?? ''}
          onChange={(e) => set('opencodeModelId', e.target.value)}
          placeholder="defaults to model"
          className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
        />
      </Field>

      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={values.kvCacheEnabled}
          onChange={(e) => set('kvCacheEnabled', e.target.checked)}
          className="rounded"
        />
        <span>Enable KV Cache optimization</span>
      </label>

      {generalServerError && (
        <div className="text-sm text-red-600">{generalServerError}</div>
      )}

      <div className="flex justify-end gap-2 pt-2">
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            className="px-3 py-1.5 text-sm rounded border border-border hover:bg-muted"
          >
            Cancel
          </button>
        )}
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
        >
          {isSubmitting ? 'Saving…' : submitLabel}
        </button>
      </div>
    </form>
  );
}

function Field({
  label,
  error,
  children,
}: {
  label: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label className="block text-xs font-medium text-muted-foreground mb-1">
        {label}
      </label>
      {children}
      {error && <div className="text-xs text-red-600 mt-1">{error}</div>}
    </div>
  );
}
