import { describe, expect, it, vi, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { InstanceDetailPage } from '../InstanceDetailPage';

const SAMPLE_INSTANCE = {
  id: 'minimax-cl-mini',
  templateId: 'minimax',
  alias: 'cl-mini',
  apiKey: 'sk-test-1234',
  modelId: 'MiniMax-M3.1-Flash[1m]',
  kvCacheEnabled: false,
  createdAt: '2026-06-01T00:00:00Z',
};

const SAMPLE_TEMPLATES = [
  {
    id: 'minimax',
    displayName: 'MiniMax',
    availableModels: ['MiniMax-M3.1-Flash[1m]'],
    models: [
      { id: 'MiniMax-M3.1-Flash[1m]', name: 'MiniMax M3.1 Flash [1m]' },
    ],
  },
];

function makeFetchStub() {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = typeof input === 'string' ? input : input.toString();
    if (url === '/api/instances/minimax-cl-mini') {
      return new Response(JSON.stringify(SAMPLE_INSTANCE), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }
    if (url === '/api/templates') {
      return new Response(JSON.stringify(SAMPLE_TEMPLATES), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }
    if (url.startsWith('/api/instances/minimax-cl-mini')) {
      return new Response(JSON.stringify(SAMPLE_INSTANCE), {
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

function renderDetail(fetchStub: ReturnType<typeof makeFetchStub>) {
  vi.stubGlobal('fetch', fetchStub);
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/instances/minimax-cl-mini']}>
        <Routes>
          <Route path="/instances/:id" element={<InstanceDetailPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('InstanceDetailPage', () => {
  it('Model field is a <select> (combobox) with options from template.models', async () => {
    renderDetail(makeFetchStub());
    await waitFor(() => {
      const selects = screen.getAllByRole('combobox');
      // 唯一的 combobox 是 Model
      expect(selects.length).toBe(1);
      const modelSelect = selects[0] as HTMLSelectElement;
      expect(modelSelect.tagName).toBe('SELECT');
      expect(modelSelect.value).toBe('MiniMax-M3.1-Flash[1m]');
      const options = screen.getAllByRole('option');
      expect(options.length).toBe(1);
    });
  });
});
