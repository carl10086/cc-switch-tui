import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { ApplyPage } from '../ApplyPage';

// Mock api client hooks via a global fetch stub.
function makeFetchStub(
  handlers: Record<string, () => unknown>,
  overrides: Record<string, (url: string) => Response> = {},
) {
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === 'string' ? input : input.toString();
    const method = (init?.method ?? 'GET').toUpperCase();
    const key = `${method} ${url}`;
    if (overrides[key]) return overrides[key](url);
    const handler = handlers[key];
    if (!handler) {
      return new Response(JSON.stringify({ error: `unmocked ${key}` }), {
        status: 500,
        headers: { 'Content-Type': 'application/json' },
      });
    }
    const body = handler();
    return new Response(typeof body === 'string' ? body : JSON.stringify(body), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
  });
}

function renderApply(fetchStub: ReturnType<typeof makeFetchStub>) {
  vi.stubGlobal('fetch', fetchStub);
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <ApplyPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

const SAMPLE_INSTANCES = [
  { id: 'minimax-cl-mini', templateId: 'minimax', alias: 'cl-mini', modelId: 'm1', opencodeModelId: '', kvCacheEnabled: false },
];

const SAMPLE_TEMPLATES = [
  { id: 'minimax', displayName: 'MiniMax', opencodeProviderId: 'minimax', opencodeBaseUrl: 'https://x', availableModels: [], models: [] },
];

const SAMPLE_ALIASES = `export KIMI_API_KEY=sk-test-1234-abcdef
export ANTHROPIC_BASE_URL=https://x
alias cl-mini='...'`;

const SAMPLE_OPENCODE = { provider: { name: 'minimax' }, model: 'm1' };

describe('ApplyPage', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('renders product list with total count and apply button', async () => {
    const fetchStub = makeFetchStub({
      'GET /api/instances': () => SAMPLE_INSTANCES,
      'GET /api/templates': () => SAMPLE_TEMPLATES,
      'GET /api/aliases': () => SAMPLE_ALIASES,
      'GET /api/opencode-config/minimax-cl-mini': () => SAMPLE_OPENCODE,
    });
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getByText(/aliases\.zsh/)).toBeInTheDocument();
    });
    // 1 aliases + 1 opencode config = 2 files
    await waitFor(() => {
      const btn = screen.getByRole('button', { name: /apply all/i });
      expect(btn).toHaveTextContent(/2 files/i);
    });
  });

  it('aliases preview is expanded by default; opencode JSON is collapsed', async () => {
    const fetchStub = makeFetchStub({
      'GET /api/instances': () => SAMPLE_INSTANCES,
      'GET /api/templates': () => SAMPLE_TEMPLATES,
      'GET /api/aliases': () => SAMPLE_ALIASES,
      'GET /api/opencode-config/minimax-cl-mini': () => SAMPLE_OPENCODE,
    });
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getByText(/sk-/)).toBeInTheDocument();
    });
    // aliases content visible (masked)
    const text = document.body.textContent ?? '';
    expect(text).toMatch(/sk-\*+/); // masked
    // opencode JSON NOT visible (collapsed)
    expect(text).not.toContain('"provider"');
  });

  it('clicking apply posts to /api/aliases/apply and shows success toast', async () => {
    const fetchStub = makeFetchStub({
      'GET /api/instances': () => SAMPLE_INSTANCES,
      'GET /api/templates': () => SAMPLE_TEMPLATES,
      'GET /api/aliases': () => SAMPLE_ALIASES,
      'GET /api/opencode-config/minimax-cl-mini': () => SAMPLE_OPENCODE,
      'POST /api/aliases/apply': () => ({ path: '/home/x/.cc-switch-tui/aliases.zsh' }),
    });
    const user = userEvent.setup();
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /apply all/i })).toBeInTheDocument();
    });
    await user.click(screen.getByRole('button', { name: /apply all/i }));

    await waitFor(() => {
      expect(screen.getByText(/wrote/i)).toBeInTheDocument();
    });
    expect(fetchStub).toHaveBeenCalledWith(
      expect.stringContaining('/api/aliases/apply'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('shows error banner when apply fails', async () => {
    const fetchStub = makeFetchStub(
      {
        'GET /api/instances': () => SAMPLE_INSTANCES,
        'GET /api/templates': () => SAMPLE_TEMPLATES,
        'GET /api/aliases': () => SAMPLE_ALIASES,
        'GET /api/opencode-config/minimax-cl-mini': () => SAMPLE_OPENCODE,
      },
      {
        'POST /api/aliases/apply': () =>
          new Response(
            JSON.stringify({ error: { code: 'INTERNAL', message: 'disk full' } }),
            { status: 500, headers: { 'Content-Type': 'application/json' } },
          ),
      },
    );
    const user = userEvent.setup();
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /apply all/i })).toBeInTheDocument();
    });
    await user.click(screen.getByRole('button', { name: /apply all/i }));

    await waitFor(() => {
      expect(screen.getByText(/apply failed/i)).toBeInTheDocument();
    });
  });
});
