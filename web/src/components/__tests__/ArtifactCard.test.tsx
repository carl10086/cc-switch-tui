import { describe, expect, it, vi, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ArtifactCard } from '../ArtifactCard';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('ArtifactCard', () => {
  it('renders collapsed by default with title and path', () => {
    render(
      <ArtifactCard title="aliases.zsh" path="~/.cc-switch-tui/aliases.zsh">
        <pre>export K=1</pre>
      </ArtifactCard>,
    );
    expect(screen.getByText('aliases.zsh')).toBeInTheDocument();
    expect(screen.getByText('~/.cc-switch-tui/aliases.zsh')).toBeInTheDocument();
    // children not visible when collapsed
    expect(screen.queryByText('export K=1')).not.toBeInTheDocument();
    expect(screen.getByText(/click to expand/i)).toBeInTheDocument();
  });

  it('renders expanded when defaultOpen=true', () => {
    render(
      <ArtifactCard title="x" path="/p" defaultOpen>
        <pre>visible content</pre>
      </ArtifactCard>,
    );
    expect(screen.getByText('visible content')).toBeInTheDocument();
  });

  it('toggles expand/collapse on header click', async () => {
    const user = userEvent.setup();
    render(
      <ArtifactCard title="x" path="/p">
        <pre>payload</pre>
      </ArtifactCard>,
    );
    expect(screen.queryByText('payload')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /toggle artifact/i }));
    expect(screen.getByText('payload')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /toggle artifact/i }));
    expect(screen.queryByText('payload')).not.toBeInTheDocument();
  });

  it('renders size badge when sizeBytes provided', () => {
    render(
      <ArtifactCard title="x" path="/p" sizeBytes={240}>
        <></>
      </ArtifactCard>,
    );
    expect(screen.getByText(/240 B/)).toBeInTheDocument();
  });

  it('renders size in KB when >= 1024', () => {
    render(
      <ArtifactCard title="x" path="/p" sizeBytes={2048}>
        <></>
      </ArtifactCard>,
    );
    expect(screen.getByText(/2\.0 KB/)).toBeInTheDocument();
  });

  it('copies text via CopyButton when provided', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { ...globalThis.navigator, clipboard: { writeText } });
    render(
      <ArtifactCard title="x" path="/p" copyText="to-copy" defaultOpen>
        <></>
      </ArtifactCard>,
    );

    screen.getByRole('button', { name: /copy/i }).click();
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('to-copy');
    });
  });
});
