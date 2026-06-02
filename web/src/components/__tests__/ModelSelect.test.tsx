import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ModelSelect } from '../ModelSelect';

const MODELS = [
  { id: 'm1', name: 'Model 1', opencodeModelId: 'm1-oc' },
  { id: 'm2', name: 'Model 2', opencodeModelId: 'm2-oc' },
  { id: 'm3', name: 'Model 3', opencodeModelId: 'm3-oc' },
];

describe('ModelSelect', () => {
  it('renders <input> when models array is empty', () => {
    render(<ModelSelect models={[]} value="" onChange={() => {}} />);
    const input = screen.getByRole('textbox');
    expect(input.tagName).toBe('INPUT');
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  });

  it('renders <select> with one option per model when models is non-empty', () => {
    render(<ModelSelect models={MODELS} value="m1" onChange={() => {}} />);
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select.tagName).toBe('SELECT');
    const options = screen.getAllByRole('option');
    expect(options).toHaveLength(3);
    expect(options[0]).toHaveTextContent('Model 1 (m1)');
    expect(options[1]).toHaveTextContent('Model 2 (m2)');
    expect(options[2]).toHaveTextContent('Model 3 (m3)');
  });

  it('option value equals model id (not name, not opencodeModelId)', () => {
    render(<ModelSelect models={MODELS} value="m2" onChange={() => {}} />);
    const options = screen.getAllByRole('option') as HTMLOptionElement[];
    expect(options[0].value).toBe('m1');
    expect(options[1].value).toBe('m2');
  });

  it('input mode: change fires onChange with new value', () => {
    const onChange = vi.fn();
    render(<ModelSelect models={[]} value="" onChange={onChange} placeholder="type here" />);
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'm1' } });
    expect(onChange).toHaveBeenCalledWith('m1');
  });

  it('select mode: change fires onChange with selected model id', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ModelSelect models={MODELS} value="m1" onChange={onChange} />);
    await user.selectOptions(screen.getByRole('combobox'), 'm2');
    expect(onChange).toHaveBeenCalledWith('m2');
  });

  it('controlled value reflects in both input and select', () => {
    const { rerender } = render(<ModelSelect models={[]} value="abc" onChange={() => {}} />);
    expect(screen.getByRole('textbox')).toHaveValue('abc');

    rerender(<ModelSelect models={MODELS} value="m3" onChange={() => {}} />);
    expect(screen.getByRole('combobox')).toHaveValue('m3');
  });
});
