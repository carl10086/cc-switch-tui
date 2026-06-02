import { describe, expect, it, vi, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { CopyButton } from '../CopyButton';

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('CopyButton', () => {
  it('renders the default label', () => {
    render(<CopyButton text="hello" />);
    expect(screen.getByRole('button')).toHaveTextContent(/copy/i);
  });

  it('copies text via clipboard and shows "Copied!" feedback', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { ...globalThis.navigator, clipboard: { writeText } });

    render(<CopyButton text="hello world" />);
    screen.getByRole('button').click();

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('hello world');
    });
    await waitFor(() => {
      expect(screen.getByRole('button')).toHaveTextContent(/copied/i);
    });
  });

  it('resets to idle after 1.5s', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { ...globalThis.navigator, clipboard: { writeText } });

    render(<CopyButton text="x" />);
    screen.getByRole('button').click();

    await waitFor(() => {
      expect(screen.getByRole('button')).toHaveTextContent(/copied/i);
    });
    await waitFor(
      () => {
        expect(screen.getByRole('button')).toHaveTextContent(/^copy$/i);
      },
      { timeout: 2000 },
    );
  });

  it('shows "Copy failed" when clipboard API throws', async () => {
    const writeText = vi.fn().mockRejectedValue(new Error('denied'));
    vi.stubGlobal('navigator', { ...globalThis.navigator, clipboard: { writeText } });

    render(<CopyButton text="x" />);
    screen.getByRole('button').click();

    await waitFor(() => {
      expect(screen.getByRole('button')).toHaveTextContent(/failed/i);
    });
  });

  it('shows "Copy failed" when clipboard API is unavailable', async () => {
    vi.stubGlobal('navigator', { ...globalThis.navigator, clipboard: undefined });

    render(<CopyButton text="x" />);
    screen.getByRole('button').click();

    await waitFor(() => {
      expect(screen.getByRole('button')).toHaveTextContent(/failed/i);
    });
  });

  it('uses custom label when provided', () => {
    render(<CopyButton text="x" label="Copy aliases" />);
    expect(screen.getByRole('button')).toHaveTextContent(/copy aliases/i);
  });
});
