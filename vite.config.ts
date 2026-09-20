import { resolve } from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// Two entry documents rather than one router. The overlay is a separate OS window
// that must paint within a frame or two of the hotkey, so it loads only the waveform
// code - none of the history, settings or onboarding bundle.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**', '**/spike/**'] },
  },
  build: {
    target: 'es2022',
    sourcemap: true,
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, 'index.html'),
        overlay: resolve(import.meta.dirname, 'overlay.html'),
      },
    },
  },
  resolve: {
    alias: { '@': resolve(import.meta.dirname, 'src') },
  },
});
