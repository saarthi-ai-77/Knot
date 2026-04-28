/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: ['class'],
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        // Knot custom colors
        'knot-bg': '#0a0a0f',
        'knot-panel': '#0d1117',
        'knot-panel-hover': '#161b22',
        'knot-border': '#1e2433',
        'knot-cyan': '#00d4ff',
        'knot-cyan-dim': 'rgba(0, 212, 255, 0.06)',
        'knot-amber': '#f59e0b',
        'knot-emerald': '#10b981',
        'knot-red': '#ef4444',
        'knot-purple': '#a855f7',
        'knot-muted': '#6b7280',
        'knot-text': '#9ca3af',
        'knot-text-bright': '#e5e7eb',
      },
      fontFamily: {
        mono: ['"JetBrains Mono"', 'ui-monospace', 'SFMono-Regular', 'Consolas', 'monospace'],
        heading: ['"Syne"', 'system-ui', 'sans-serif'],
      },
      transitionTimingFunction: {
        'knot': 'cubic-bezier(0.4, 0, 0.2, 1)',
      },
      transitionDuration: {
        '120': '120ms',
      },
      animation: {
        'pulse-green': 'pulse-green 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
      keyframes: {
        'pulse-green': {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.5' },
        },
      },
    },
  },
  plugins: [],
}
