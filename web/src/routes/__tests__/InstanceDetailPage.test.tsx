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
  modelId: 'MiniMax-M3',
  opencodeModelId: 'MiniMax-M3',
  kvCacheEnabled: false,
  createdAt: '2026-06-01T00:00:00Z',
};

const SAMPLE_TEMPLATES = [
  {
    id: 'minimax',
    displayName: 'MiniMax',
    opencodeProviderId: 'minimax-cn',
    opencodeBaseUrl: 'https://x',
    availableModels: ['MiniMax-M3', 'MiniMax-M2.7-highspeed'],
    models: [
      { id: 'MiniMax-M3', name: 'MiniMax M3', opencodeModelId: 'MiniMax-M3' },
      { id: 'MiniMax-M2.7-highspeed', name: 'MiniMax M2.7 Highspeed', opencodeModelId: 'MiniMax-M2.7-highspeed' },
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
      // First combobox is Model
      const modelSelect = selects[0] as HTMLSelectElement;
      expect(modelSelect.tagName).toBe('SELECT');
      const options = screen.getAllByRole('option');
      // 2 from Model + 3 from OpenCode Model ID (including "— default —") = 5
      expect(options.length).toBeGreaterThanOrEqual(2);
    });
  });

  it('changing model syncs opencodeModelId to new model.opencodeModelId', async () => {
    renderDetail(makeFetchStub());
    // wait for both queries (instance + templates) + form rendered
    await waitFor(() => {
      const selects = screen.getAllByRole('combobox');
      expect(selects.length).toBe(2);
    });
    const selects = screen.getAllByRole('combobox') as HTMLSelectElement[];
    const modelSelect = selects[0];
    const ocSelect = selects[1];

    // 初始：model = M3, opencode = M3
    expect(modelSelect.value).toBe('MiniMax-M3');
    expect(ocSelect.value).toBe('MiniMax-M3');

    // 切 model → m2.7
    const nativeSetter = Object.getOwnPropertyDescriptor(
      window.HTMLSelectElement.prototype,
      'value',
    )!.set as (this: HTMLSelectElement, v: string) => void;
    nativeSetter.call(modelSelect, 'MiniMax-M2.7-highspeed');
    modelSelect.dispatchEvent(new Event('change', { bubbles: true }));

    await waitFor(() => {
      expect(ocSelect.value).toBe('MiniMax-M2.7-highspeed');
    });
  });
});
