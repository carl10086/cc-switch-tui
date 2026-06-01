import type { Config } from 'tailwindcss';

// 注意: darkMode: 'class' 会在 S0-T6 加入 dark mode 时启用。
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
