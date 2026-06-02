import { useState } from 'react';
import { useCreateInstance } from '../api/hooks';
import { ApiError } from '../api/client';
import { InstanceForm } from './InstanceForm';

export function InstanceFormDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const create = useCreateInstance();
  const [serverError, setServerError] = useState<unknown>(null);

  if (!open) return null;

  async function handleSubmit(values: Parameters<typeof create.mutateAsync>[0]) {
    setServerError(null);
    try {
      await create.mutateAsync(values);
      onClose();
    } catch (e) {
      if (e instanceof ApiError && e.code === 'ALIAS_CONFLICT') {
        // Set field-specific error to surface in form
        const enriched = new ApiError(
          e.status,
          e.code,
          'Alias already exists. Please choose a different alias.',
        );
        // 简单做法：把 alias 错误塞到 serverError 让 form 显示
        (enriched as ApiError & { _field?: string })._field = 'alias';
        setServerError(enriched);
      } else {
        setServerError(e);
      }
    }
  }

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      onClick={onClose}
    >
      <div
        className="bg-card border border-border rounded-lg p-6 max-w-lg w-full max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-lg font-semibold mb-4">New Instance</h3>
        <InstanceForm
          onSubmit={handleSubmit}
          onCancel={onClose}
          isSubmitting={create.isPending}
          serverError={serverError}
          submitLabel="Create"
        />
      </div>
    </div>
  );
}
