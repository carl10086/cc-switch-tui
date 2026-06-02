import { useState } from 'react';

export function SecretInput({
  value,
  onChange,
  placeholder,
  id,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  id?: string;
}) {
  const [visible, setVisible] = useState(false);
  return (
    <div className="flex gap-2">
      <input
        id={id}
        type={visible ? 'text' : 'password'}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="flex-1 px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
        autoComplete="off"
        spellCheck={false}
      />
      <button
        type="button"
        onClick={() => setVisible(!visible)}
        className="px-3 text-xs rounded border border-border bg-muted text-muted-foreground hover:bg-muted/70"
      >
        {visible ? 'Hide' : 'Show'}
      </button>
    </div>
  );
}
