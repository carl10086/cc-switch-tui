/// 统一 fetch wrapper。所有 API 调用都走这里，便于集中处理错误和 headers。

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/// Rust 端 ApiError 响应格式
interface RustErrorBody {
  error: {
    code: string;
    message: string;
    field?: string;
  };
}

export async function apiGet<T>(path: string): Promise<T> {
  return apiFetch<T>('GET', path);
}

export async function apiPost<T>(path: string, body?: unknown): Promise<T> {
  return apiFetch<T>('POST', path, body);
}

export async function apiPatch<T>(path: string, body: unknown): Promise<T> {
  return apiFetch<T>('PATCH', path, body);
}

export async function apiDelete(path: string): Promise<void> {
  const resp = await fetch(path, { method: 'DELETE' });
  if (!resp.ok) {
    throw await toApiError(resp, 'DELETE', path);
  }
}

/// 拿 text/plain 响应（非 JSON）
export async function apiGetText(path: string): Promise<string> {
  const resp = await fetch(path, {
    headers: { Accept: 'text/plain' },
  });
  if (!resp.ok) {
    throw await toApiError(resp, 'GET', path);
  }
  return resp.text();
}

async function apiFetch<T>(method: string, path: string, body?: unknown): Promise<T> {
  const init: RequestInit = {
    method,
    headers: { Accept: 'application/json' },
  };
  if (body !== undefined) {
    (init.headers as Record<string, string>)['Content-Type'] = 'application/json';
    init.body = JSON.stringify(body);
  }
  const resp = await fetch(path, init);
  if (!resp.ok) {
    throw await toApiError(resp, method, path);
  }
  return (await resp.json()) as T;
}

async function toApiError(resp: Response, method: string, path: string): Promise<ApiError> {
  let code = 'UNKNOWN';
  let message = `${method} ${path} returned ${resp.status}`;
  try {
    const body = (await resp.json()) as RustErrorBody;
    code = body.error?.code ?? code;
    message = body.error?.message ?? message;
  } catch {
    // body not JSON — keep default
  }
  return new ApiError(resp.status, code, message);
}
