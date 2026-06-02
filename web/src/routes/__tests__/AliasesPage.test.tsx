import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { AliasesPage } from '../AliasesPage';

function renderAliases(fetchStub: ReturnType<typeof vi.fn>) {
  vi.stubGlobal('fetch', fetchStub);
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AliasesPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

const SAMPLE_ALIASES = `export KIMI_API_KEY=sk-test-1234-abcdef
alias cl-mini='...'`;

/** AliasesPage 必须暴露的是“只读预览”——没有 apply 按钮，apply 在 /apply 页 */
describe('AliasesPage', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('does not render an apply button (apply is on /apply page)', async () => {
    const fetchStub = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString();
      if (url === '/api/aliases') {
        return new Response(SAMPLE_ALIASES, {
          status: 200,
          headers: { 'Content-Type': 'text/plain' },
        });
      }
      return new Response('not found', { status: 404 });
    });
    renderAliases(fetchStub);

    await waitFor(() => {
      expect(screen.getByText(/aliases\.zsh/)).toBeInTheDocument();
    });
    // No apply button — user goes to /apply to write files
    expect(screen.queryByRole('button', { name: /^apply$/i })).toBeNull();
  });

  it('shows a link to the /apply page', async () => {
    const fetchStub = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString();
      if (url === '/api/aliases') {
        return new Response(SAMPLE_ALIASES, {
          status: 200,
          headers: { 'Content-Type': 'text/plain' },
        });
      }
      return new Response('not found', { status: 404 });
    });
    renderAliases(fetchStub);

    await waitFor(() => {
      expect(screen.getByRole('link', { name: /apply/i })).toBeInTheDocument();
    });
  });
});
