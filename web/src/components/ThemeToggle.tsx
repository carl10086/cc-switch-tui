import { useTheme } from '../hooks/useTheme';

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  return (
    <div className="inline-flex rounded-md border border-border bg-card p-0.5 text-xs">
      <button
        type="button"
        onClick={() => setTheme('light')}
        className={`px-2 py-1 rounded ${
          theme === 'light' ? 'bg-primary text-primary-foreground' : ''
        }`}
        aria-label="Light theme"
      >
        ☀️
      </button>
      <button
        type="button"
        onClick={() => setTheme('system')}
        className={`px-2 py-1 rounded ${
          theme === 'system' ? 'bg-primary text-primary-foreground' : ''
        }`}
        aria-label="System theme"
      >
        🖥️
      </button>
      <button
        type="button"
        onClick={() => setTheme('dark')}
        className={`px-2 py-1 rounded ${
          theme === 'dark' ? 'bg-primary text-primary-foreground' : ''
        }`}
        aria-label="Dark theme"
      >
        🌙
      </button>
    </div>
  );
}
