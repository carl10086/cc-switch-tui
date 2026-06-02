import { ApiError } from '../api/client';

export function ApiErrorBanner({ error }: { error: unknown }) {
  if (!error) return null;
  const message =
    error instanceof ApiError
      ? `${error.code}: ${error.message}`
      : (error as Error).message;
  return (
    <div className="bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-800 text-red-800 dark:text-red-200 px-4 py-2 rounded text-sm">
      {message}
    </div>
  );
}
