import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        pin: resolve(__dirname, 'pin.html'),
        overlay: resolve(__dirname, 'overlay.html'),
        preview: resolve(__dirname, 'preview.html'),
        pinhandle: resolve(__dirname, 'pinhandle.html'),
        scroll_toolbar: resolve(__dirname, 'scroll_toolbar.html'),
        scroll_frame: resolve(__dirname, 'scroll_frame.html'),
      },
    },
  },
});
