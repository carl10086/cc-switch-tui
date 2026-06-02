import { useEffect, useState } from 'react';
import { useNavigate, useParams, Link } from 'react-router-dom';
import {
  useDeleteInstance,
  useDuplicateInstance,
  useInstance,
  useTemplates,
  useUpdateInstance,
  type InstanceDetail,
} from '../api/hooks';
import { ApiErrorBanner } from '../components/ApiErrorBanner';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { ModelSelect } from '../components/ModelSelect';
import { OpencodeModelSelect } from '../components/OpencodeModelSelect';
import { SecretInput } from '../components/SecretInput';
import { useUnsavedGuard } from '../hooks/useUnsavedGuard';

export function InstanceDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: instance, isLoading, isError, error } = useInstance(id);
  const { data: templates } = useTemplates();
  const update = useUpdateInstance(id ?? '');
  const deleteInst = useDeleteInstance();
  const duplicate = useDuplicateInstance();

  const [draft, setDraft] = useState<Partial<InstanceDetail>>({});
  const [dirty, setDirty] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [serverError, setServerError] = useState<unknown>(null);

  // 同步远端 → 本地草稿
  useEffect(() => {
    if (instance) {
      setDraft({
        modelId: instance.modelId,
        apiKey: instance.apiKey,
        opencodeModelId: instance.opencodeModelId,
        kvCacheEnabled: instance.kvCacheEnabled,
      });
      setDirty(false);
    }
  }, [instance]);

  // 切换 model 时，若 opencodeModelId 仍指向旧 model 的 id（或为空），自动同步到新 model 的 opencodeModelId。
  // 这和 InstanceForm 的行为一致 — 详见 .ys-powers/specs/2026-06-02-fix-m1-ux-bugs-design.md
  useEffect(() => {
    if (!templates || !instance) return;
    const currentTemplate = templates.find((t) => t.id === instance.templateId);
    if (!currentTemplate) return;
    const newModel = currentTemplate.models.find((m) => m.id === draft.modelId);
    if (!newModel) return;
    const ocId = draft.opencodeModelId ?? '';
    const oldModelIds = new Set(currentTemplate.models.map((m) => m.opencodeModelId));
    if (ocId === '' || oldModelIds.has(ocId)) {
      if (ocId !== newModel.opencodeModelId) {
        setDraft((d) => ({ ...d, opencodeModelId: newModel.opencodeModelId }));
        setDirty(true);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft.modelId]);

  useUnsavedGuard(dirty);

  if (isLoading) return <div className="text-muted-foreground">Loading…</div>;
  if (isError)
    return (
      <div>
        <ApiErrorBanner error={error} />
        <Link to="/" className="text-sm text-muted-foreground hover:underline">
          ← Back to list
        </Link>
      </div>
    );
  if (!instance) return null;

  function set<K extends keyof InstanceDetail>(key: K, value: InstanceDetail[K]) {
    setDraft((d) => ({ ...d, [key]: value }));
    setDirty(true);
    setServerError(null);
  }

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    if (!dirty) return;
    setServerError(null);
    try {
      await update.mutateAsync(draft);
      setDirty(false);
    } catch (e) {
      setServerError(e);
    }
  }

  async function handleDelete() {
    if (!id) return;
    try {
      await deleteInst.mutateAsync(id);
      navigate('/');
    } catch (e) {
      setServerError(e);
    }
  }

  async function handleDuplicate() {
    if (!id) return;
    try {
      const dup = await duplicate.mutateAsync(id);
      navigate(`/instances/${dup.id}`);
    } catch (e) {
      setServerError(e);
    }
  }

  return (
    <section className="max-w-2xl">
      <div className="flex items-center justify-between mb-4">
        <div>
          <Link to="/" className="text-xs text-muted-foreground hover:underline">
            ← Instances
          </Link>
          <h2 className="text-2xl font-bold font-mono mt-1">{instance.alias}</h2>
          <div className="text-xs text-muted-foreground mt-1">
            {instance.templateId} · id: <span className="font-mono">{instance.id}</span>
          </div>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={handleDuplicate}
            className="px-3 py-1.5 text-sm rounded border border-border hover:bg-muted"
          >
            Duplicate
          </button>
          <button
            type="button"
            onClick={() => setConfirmDelete(true)}
            className="px-3 py-1.5 text-sm rounded border border-red-300 text-red-700 hover:bg-red-50 dark:border-red-800 dark:text-red-300 dark:hover:bg-red-950"
          >
            Delete
          </button>
        </div>
      </div>

      {serverError ? (
        <div className="mb-4"><ApiErrorBanner error={serverError} /></div>
      ) : null}

      <form onSubmit={handleSave} className="space-y-4">
        <Field label="Template">
          <div className="px-3 py-1.5 text-sm rounded border border-input bg-muted text-muted-foreground font-mono">
            {instance.templateId}
          </div>
        </Field>

        <Field label="Model">
          <ModelSelect
            models={templates?.find((t) => t.id === instance.templateId)?.models ?? []}
            value={draft.modelId ?? ''}
            onChange={(v) => set('modelId', v)}
          />
        </Field>

        <Field label="API Key">
          <SecretInput
            value={draft.apiKey ?? ''}
            onChange={(v) => set('apiKey', v)}
          />
        </Field>

        <Field label="OpenCode Model ID">
          <OpencodeModelSelect
            models={templates?.find((t) => t.id === instance.templateId)?.opencodeModels ?? []}
            value={draft.opencodeModelId ?? ''}
            onChange={(v) => set('opencodeModelId', v)}
          />
        </Field>

        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={draft.kvCacheEnabled ?? false}
            onChange={(e) => set('kvCacheEnabled', e.target.checked)}
            className="rounded"
          />
          <span>Enable KV Cache optimization</span>
        </label>

        <div className="flex justify-end gap-2 pt-2">
          <Link
            to="/"
            className="px-3 py-1.5 text-sm rounded border border-border hover:bg-muted"
          >
            Cancel
          </Link>
          <button
            type="submit"
            disabled={!dirty || update.isPending}
            className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
          >
            {update.isPending ? 'Saving…' : 'Save'}
          </button>
        </div>

        {instance.alias && dirty && (
          <div className="text-xs text-muted-foreground">
            Unsaved changes. Closing this tab will prompt to confirm.
          </div>
        )}
      </form>

      <ConfirmDialog
        open={confirmDelete}
        title="Delete instance?"
        message={`This will permanently delete "${instance.alias}".\nThis cannot be undone.`}
        confirmLabel="Delete"
        destructive
        onConfirm={handleDelete}
        onCancel={() => setConfirmDelete(false)}
      />
    </section>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label className="block text-xs font-medium text-muted-foreground mb-1">
        {label}
      </label>
      {children}
    </div>
  );
}
