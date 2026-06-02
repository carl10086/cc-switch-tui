import { useEffect, useMemo, useState } from 'react';
import { ApiError } from '../api/client';
import { useTemplates } from '../api/hooks';
import { instanceSchema, type InstanceFormValues } from '../lib/validate';
import { ModelSelect } from './ModelSelect';
import { OpencodeModelSelect } from './OpencodeModelSelect';
import { SecretInput } from './SecretInput';

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
  const { data: templates, isLoading: templatesLoading } = useTemplates();

  const [values, setValues] = useState<InstanceFormValues>({
    templateId: initial?.templateId ?? '',
    alias: initial?.alias ?? '',
    modelId: initial?.modelId ?? '',
    apiKey: initial?.apiKey ?? '',
    opencodeModelId: initial?.opencodeModelId ?? '',
    kvCacheEnabled: initial?.kvCacheEnabled ?? false,
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  // 加载 templates 后，设置默认 template + model
  useEffect(() => {
    if (!templates || templates.length === 0) return;
    if (values.templateId) return; // already set (e.g. from initial)
    const first = templates[0];
    setValues((v) => ({
      ...v,
      templateId: first.id,
      modelId: v.modelId || first.models[0]?.id || '',
    }));
  }, [templates, values.templateId]);

  // 切换 template 时，若当前 model 不在新 template 的 availableModels 里，替换为第一个
  const currentTemplate = useMemo(
    () => templates?.find((t) => t.id === values.templateId),
    [templates, values.templateId],
  );
  const currentModel = useMemo(
    () => currentTemplate?.models.find((m) => m.id === values.modelId),
    [currentTemplate, values.modelId],
  );

  useEffect(() => {
    if (!currentTemplate) return;
    if (currentTemplate.models.some((m) => m.id === values.modelId)) return;
    setValues((v) => ({
      ...v,
      modelId: currentTemplate.models[0]?.id ?? '',
    }));
  }, [currentTemplate, values.modelId]);

  // 切换 model 时同步设置 opencodeModelId — 仅当用户没手动改过（即仍为空）时才覆盖
  useEffect(() => {
    if (!currentModel) return;
    setValues((v) => {
      if (v.opencodeModelId && v.opencodeModelId !== '') return v;
      return { ...v, opencodeModelId: currentModel.opencodeModelId };
    });
  }, [currentModel]);

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

  // 提取服务端 field error (ApiError.field — 通过 _field hack 标记)
  const enrichedServerError =
    serverError instanceof ApiError &&
    (serverError as ApiError & { _field?: string })._field
      ? serverError
      : null;
  const generalServerError =
    serverError && !enrichedServerError
      ? serverError instanceof ApiError
        ? serverError.message
        : (serverError as Error).message
      : null;
  const aliasFieldFromServer =
    enrichedServerError &&
    (enrichedServerError as ApiError & { _field?: string })._field === 'alias'
      ? 'Alias already exists'
      : '';

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <Field label="Template" error={errors.templateId}>
        <select
          value={values.templateId}
          onChange={(e) => set('templateId', e.target.value)}
          disabled={templatesLoading}
          className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background"
        >
          {templates?.map((t) => (
            <option key={t.id} value={t.id}>{t.displayName}</option>
          ))}
        </select>
      </Field>

      <Field label="Alias" error={errors.alias || aliasFieldFromServer}>
        <input
          value={values.alias}
          onChange={(e) => set('alias', e.target.value)}
          placeholder="cl-mini"
          className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
        />
      </Field>

      <Field label="Model" error={errors.modelId}>
        <ModelSelect
          models={currentTemplate?.models ?? []}
          value={values.modelId}
          onChange={(v) => set('modelId', v)}
        />
      </Field>

      <Field label="API Key" error={errors.apiKey}>
        <SecretInput
          value={values.apiKey}
          onChange={(v) => set('apiKey', v)}
          placeholder="sk-..."
        />
      </Field>

      <Field label="OpenCode Model ID" error={errors.opencodeModelId}>
        <OpencodeModelSelect
          models={currentTemplate?.opencodeModels ?? []}
          value={values.opencodeModelId ?? ''}
          onChange={(v) => set('opencodeModelId', v)}
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
