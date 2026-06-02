import { useRef, useState } from 'react';
import { useImportConfig, type ExportedConfig } from '../api/hooks';
import { ApiErrorBanner } from '../components/ApiErrorBanner';

export function ConfigPage() {
  const [exportError, setExportError] = useState<unknown>(null);
  const [importPreview, setImportPreview] = useState<ExportedConfig | null>(null);
  const [importError, setImportError] = useState<unknown>(null);
  const [importResult, setImportResult] = useState<{ created: number; skipped: number; skippedAliases: string[] } | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const importMut = useImportConfig();

  async function handleExport() {
    setExportError(null);
    try {
      const resp = await fetch('/api/config/export');
      if (!resp.ok) throw new Error(`export failed: ${resp.status}`);
      // 触发浏览器下载
      const blob = await resp.blob();
      const cd = resp.headers.get('content-disposition') ?? '';
      const m = cd.match(/filename=([^;]+)/);
      const filename = m ? m[1].trim() : `cc-switch-config-${Date.now()}.json`;
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch (e) {
      setExportError(e);
    }
  }

  async function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    setImportError(null);
    setImportResult(null);
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const parsed = JSON.parse(text) as ExportedConfig;
      if (typeof parsed.version !== 'number' || !Array.isArray(parsed.instances)) {
        throw new Error('Invalid config file: missing version or instances array');
      }
      setImportPreview(parsed);
    } catch (e) {
      setImportError(e);
      setImportPreview(null);
    }
  }

  async function handleImport() {
    if (!importPreview) return;
    setImportError(null);
    setImportResult(null);
    try {
      const result = await importMut.mutateAsync(importPreview);
      setImportResult(result);
      setImportPreview(null);
      if (fileInputRef.current) fileInputRef.current.value = '';
    } catch (e) {
      setImportError(e);
    }
  }

  return (
    <section className="max-w-2xl space-y-8">
      <div>
        <h2 className="text-2xl font-bold mb-2">Export</h2>
        <p className="text-sm text-muted-foreground mb-4">
          下载当前所有 instances 为 JSON 文件。<strong>不含 apiKey</strong>（安全考虑 — 分享的 config 不应包含密钥）。
        </p>
        <button
          type="button"
          onClick={handleExport}
          className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          Download config
        </button>
        {exportError ? <div className="mt-2"><ApiErrorBanner error={exportError} /></div> : null}
      </div>

      <div>
        <h2 className="text-2xl font-bold mb-2">Import</h2>
        <p className="text-sm text-muted-foreground mb-4">
          上传 JSON 文件。Merge 模式：已存在的 alias 跳过（不覆盖）。导入后用户需手动补填 apiKey。
        </p>

        <input
          ref={fileInputRef}
          type="file"
          accept="application/json,.json"
          onChange={handleFileChange}
          className="block w-full text-sm file:mr-3 file:py-1.5 file:px-3 file:rounded file:border-0 file:bg-muted file:text-foreground file:cursor-pointer"
        />

        {importError ? (
          <div className="mt-2"><ApiErrorBanner error={importError} /></div>
        ) : null}

        {importPreview && (
          <div className="mt-4 p-3 border border-border rounded bg-muted/40">
            <div className="text-sm font-semibold mb-1">
              Preview: {importPreview.instances.length} instance(s), version {importPreview.version}
            </div>
            <ul className="text-xs space-y-0.5 max-h-40 overflow-y-auto">
              {importPreview.instances.map((i) => (
                <li key={i.id} className="font-mono text-muted-foreground">
                  {i.templateId} / {i.alias} ({i.id})
                </li>
              ))}
            </ul>
            <div className="mt-3 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => {
                  setImportPreview(null);
                  if (fileInputRef.current) fileInputRef.current.value = '';
                }}
                className="px-3 py-1.5 text-sm rounded border border-border hover:bg-muted"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleImport}
                disabled={importMut.isPending}
                className="px-3 py-1.5 text-sm rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
              >
                {importMut.isPending ? 'Importing…' : 'Import'}
              </button>
            </div>
          </div>
        )}

        {importResult && (
          <div className="mt-4 px-3 py-2 text-sm rounded bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 text-green-800 dark:text-green-200">
            ✓ Imported {importResult.created} instance(s){importResult.skipped > 0 ? `, skipped ${importResult.skipped} (${importResult.skippedAliases.join(', ')})` : ''}
          </div>
        )}
      </div>
    </section>
  );
}
