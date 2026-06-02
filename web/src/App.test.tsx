import { describe, expect, it, vi, afterEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import App from './App';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const SAMPLE_TEMPLATES = [
  { id: 't1', displayName: 'T1', opencodeProviderId: 'p1', opencodeBaseUrl: 'https://x', availableModels: [], models: [] },
];

function makeFetchStub() {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = typeof input === 'string' ? input : input.toString();
    if (url === '/api/templates') {
      return new Response(JSON.stringify(SAMPLE_TEMPLATES), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }
    return new Response(JSON.stringify({ error: `unmocked ${url}` }), {
      status: 500,
      headers: { 'Content-Type': 'application/json' },
    });
  });
}

describe('App templates prefetch', () => {
  it('prefetches /api/templates on mount', async () => {
    // ThemeToggle calls window.matchMedia — jsdom doesn't implement it
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    );
    const fetchStub = makeFetchStub();
    vi.stubGlobal('fetch', fetchStub);

    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
    });

    render(
      <QueryClientProvider client={qc}>
        <MemoryRouter>
          <App />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(fetchStub).toHaveBeenCalledWith('/api/templates', expect.anything());
    });
  });
});
