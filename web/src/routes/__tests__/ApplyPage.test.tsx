import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { ApplyPage } from '../ApplyPage';

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
  { id: 'minimax-cl-pro', templateId: 'minimax', alias: 'cl-pro', modelId: 'm2', opencodeModelId: '', kvCacheEnabled: false },
];

const SAMPLE_TEMPLATES = [
  { id: 'minimax', displayName: 'MiniMax', opencodeProviderId: 'minimax', opencodeBaseUrl: 'https://x', availableModels: [], models: [] },
];

const SAMPLE_ALIASES = `export KIMI_API_KEY=sk-test-1234-abcdef
export ANTHROPIC_BASE_URL=https://x
alias cl-mini='...'`;

const SAMPLE_OPENCODE_CL_MINI = { provider: { name: 'minimax' }, model: 'm1' };
const SAMPLE_OPENCODE_CL_PRO = { provider: { name: 'minimax' }, model: 'm2' };

const DEFAULT_HANDLERS = {
  'GET /api/instances': () => SAMPLE_INSTANCES,
  'GET /api/templates': () => SAMPLE_TEMPLATES,
  'GET /api/aliases': () => SAMPLE_ALIASES,
  'GET /api/opencode-config/minimax-cl-mini': () => SAMPLE_OPENCODE_CL_MINI,
  'GET /api/opencode-config/minimax-cl-pro': () => SAMPLE_OPENCODE_CL_PRO,
};

describe('ApplyPage', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('renders sticky apply bar with file count', async () => {
    const fetchStub = makeFetchStub(DEFAULT_HANDLERS);
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getByTestId('apply-sticky')).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByTestId('apply-all')).toHaveTextContent(/apply all/i);
    });
    // 1 aliases + 2 opencode configs = 3 files
    const sticky = screen.getByTestId('apply-sticky');
    expect(sticky.textContent).toContain('3');
  });

  it('renders 3 ArtifactCards (1 aliases + 2 opencode)', async () => {
    const fetchStub = makeFetchStub(DEFAULT_HANDLERS);
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getAllByTestId('artifact-card')).toHaveLength(3);
    });
  });

  it('aliases card is open by default with content visible', async () => {
    const fetchStub = makeFetchStub(DEFAULT_HANDLERS);
    renderApply(fetchStub);

    await waitFor(() => {
      // aliases content (masked) is visible
      expect(document.body.textContent ?? '').toMatch(/sk-\*+/);
    });
    // opencode JSON NOT visible (collapsed by default)
    expect(document.body.textContent ?? '').not.toContain('"provider"');
  });

  it('expanding opencode card shows JSON', async () => {
    const fetchStub = makeFetchStub(DEFAULT_HANDLERS);
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getAllByTestId('artifact-card')).toHaveLength(3);
    });

    // 点击 opencode 卡片 toggle 按钮（"cl-mini.json" 标题）
    const cards = screen.getAllByTestId('artifact-card');
    const clMiniCard = cards[1];
    const toggle = clMiniCard.querySelector('button[aria-label="toggle artifact"]') as HTMLElement;
    toggle.click();

    await waitFor(() => {
      expect(document.body.textContent ?? '').toContain('"provider"');
    });
  });

  it('clicking apply transitions to loading then success', async () => {
    const fetchStub = makeFetchStub(DEFAULT_HANDLERS, {
      'POST /api/aliases/apply': () =>
        new Response(JSON.stringify({ path: '/home/x/.cc-switch-tui/aliases.zsh' }), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
    });
    renderApply(fetchStub);

    // 等数据加载完（卡片出现 = queries 完成 → 按钮 disabled 解除）
    await waitFor(() => {
      expect(screen.getAllByTestId('artifact-card')).toHaveLength(3);
    });
    screen.getByTestId('apply-all').click();

    // 立即进入 success（resolve 快）
    await waitFor(() => {
      expect(screen.getByTestId('apply-all').getAttribute('data-state')).toBe('success');
    });
  });

  it('shows error banner and Retry button on failure', async () => {
    const fetchStub = makeFetchStub(DEFAULT_HANDLERS, {
      'POST /api/aliases/apply': () =>
        new Response(
          JSON.stringify({ error: { code: 'INTERNAL', message: 'disk full' } }),
          { status: 500, headers: { 'Content-Type': 'application/json' } },
        ),
    });
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getAllByTestId('artifact-card')).toHaveLength(3);
    });
    screen.getByTestId('apply-all').click();

    await waitFor(() => {
      expect(screen.getByTestId('apply-error')).toBeInTheDocument();
    });
    expect(screen.getByTestId('apply-retry')).toBeInTheDocument();
  });

  it('Cmd+Enter triggers apply', async () => {
    const fetchStub = makeFetchStub(DEFAULT_HANDLERS, {
      'POST /api/aliases/apply': () =>
        new Response(JSON.stringify({ path: '/p' }), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
    });
    renderApply(fetchStub);

    await waitFor(() => {
      expect(screen.getAllByTestId('artifact-card')).toHaveLength(3);
    });

    // 不点按钮，直接 Cmd+Enter
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', metaKey: true }));

    await waitFor(() => {
      expect(fetchStub).toHaveBeenCalledWith(
        expect.stringContaining('/api/aliases/apply'),
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });
});
