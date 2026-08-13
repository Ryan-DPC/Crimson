import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
// base './' so production assets resolve inside the Tauri webview (embedded
// frontendDist), not against an absolute http://localhost origin.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // Tauri uses Chromium on Windows — es2021 is fine and keeps bundles smaller.
    target: 'es2021',
  },
  base: './',
})
