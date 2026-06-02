import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { OpencodePage } from '../OpencodePage';

function renderOpencode(fetchStub: ReturnType<typeof vi.fn>) {
  vi.stubGlobal('fetch', fetchStub);
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <OpencodePage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

const SAMPLE_INSTANCES = [
  { id: 'minimax-cl-mini', templateId: 'minimax', alias: 'cl-mini', modelId: 'm1', opencodeModelId: '', kvCacheEnabled: false },
];

const SAMPLE_OPENCODE = { provider: { name: 'minimax' }, model: 'm1' };

/** OpencodePage 必须是“只读预览”——没有 apply 按钮 */
describe('OpencodePage', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('does not render an apply button (apply is on /apply page)', async () => {
    const fetchStub = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString();
      if (url === '/api/instances') {
        return new Response(JSON.stringify(SAMPLE_INSTANCES), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      }
      if (url === '/api/opencode-config/minimax-cl-mini') {
        return new Response(JSON.stringify(SAMPLE_OPENCODE), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      }
      return new Response('not found', { status: 404 });
    });
    renderOpencode(fetchStub);

    await waitFor(() => {
      expect(screen.getByText('cl-mini')).toBeInTheDocument();
    });
    expect(screen.queryByRole('button', { name: /^apply$/i })).toBeNull();
  });

  it('shows a link to the /apply page', async () => {
    const fetchStub = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString();
      if (url === '/api/instances') {
        return new Response(JSON.stringify(SAMPLE_INSTANCES), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      }
      if (url === '/api/opencode-config/minimax-cl-mini') {
        return new Response(JSON.stringify(SAMPLE_OPENCODE), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        });
      }
      return new Response('not found', { status: 404 });
    });
    renderOpencode(fetchStub);

    await waitFor(() => {
      expect(screen.getByRole('link', { name: /→ apply/i })).toBeInTheDocument();
    });
  });
});
