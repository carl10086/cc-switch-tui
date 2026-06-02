import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OpencodeConfigPreview } from '../OpencodeConfigPreview';

const SAMPLE = {
  provider: { name: 'minimax', baseURL: 'https://api.minimaxi.com/anthropic' },
  model: 'MiniMax-M2.7',
};

describe('OpencodeConfigPreview', () => {
  it('renders JSON when not collapsed', () => {
    render(<OpencodeConfigPreview config={SAMPLE} />);
    const text = document.body.textContent ?? '';
    expect(text).toContain('"provider"');
    expect(text).toContain('MiniMax-M2.7');
  });

  it('hides JSON when collapsed', () => {
    render(<OpencodeConfigPreview config={SAMPLE} collapsed />);
    const text = document.body.textContent ?? '';
    expect(text).not.toContain('"provider"');
    expect(text).not.toContain('MiniMax-M2.7');
  });

  it('toggles collapsed state on click', async () => {
    const user = userEvent.setup();
    render(<OpencodeConfigPreview config={SAMPLE} collapsed />);
    const toggle = screen.getByRole('button');
    expect(document.body.textContent ?? '').not.toContain('"provider"');
    await user.click(toggle);
    expect(document.body.textContent ?? '').toContain('"provider"');
  });
});
