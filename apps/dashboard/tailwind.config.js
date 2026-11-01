/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        sans: [
          'Inter',
          'ui-sans-serif',
          'system-ui',
          '-apple-system',
          'Segoe UI',
          'Roboto',
          'Helvetica Neue',
          'Arial',
          'sans-serif'
        ],
        mono: [
          'JetBrains Mono',
          'ui-monospace',
          'SFMono-Regular',
          'Menlo',
          'Consolas',
          'Liberation Mono',
          'monospace'
        ]
      },
      colors: {
        brand: {
          50: '#ecfdf7',
          100: '#d1faec',
          200: '#a7f3da',
          300: '#6ee7c3',
          400: '#34d3a6',
          500: '#10b98c',
          600: '#059673',
          700: '#04785f',
          800: '#065f4c',
          900: '#064e40',
          950: '#022c25'
        },
        ink: {
          50: '#f6f7f9',
          100: '#eceef2',
          200: '#d5d9e2',
          300: '#b0b8c9',
          400: '#8591ab',
          500: '#667391',
          600: '#515c78',
          700: '#424b62',
          800: '#2a3142',
          850: '#1d2331',
          900: '#151a25',
          950: '#0b0e15'
        }
      },
      boxShadow: {
        card: '0 1px 2px 0 rgb(16 24 40 / 0.04), 0 1px 3px 0 rgb(16 24 40 / 0.06)',
        pop: '0 12px 32px -8px rgb(16 24 40 / 0.18), 0 4px 8px -4px rgb(16 24 40 / 0.08)',
        glow: '0 0 0 4px rgb(16 185 140 / 0.15)'
      },
      keyframes: {
        'fade-in': {
          from: { opacity: '0', transform: 'translateY(4px)' },
          to: { opacity: '1', transform: 'translateY(0)' }
        },
        'scale-in': {
          from: { opacity: '0', transform: 'scale(0.97)' },
          to: { opacity: '1', transform: 'scale(1)' }
        },
        ping2: {
          '75%, 100%': { transform: 'scale(2.2)', opacity: '0' }
        }
      },
      animation: {
        'fade-in': 'fade-in 180ms ease-out',
        'scale-in': 'scale-in 160ms ease-out',
        ping2: 'ping2 1.8s cubic-bezier(0, 0, 0.2, 1) infinite'
      }
    }
  },
  plugins: []
};
