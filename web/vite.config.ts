import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    target: 'es2022',
    chunkSizeWarningLimit: 650,
    rollupOptions: {
      output: {
        manualChunks(id) {
          const normalized = id.replace(/\\/g, '/')
          if (normalized.includes('/src/legacy/template.html')) return 'legacy-template'
          if (normalized.includes('lightweight-charts.standalone.production.mjs')) return 'vendor-charts'
          if (normalized.includes('/src/legacy/core/')) return 'legacy-core'
          if (normalized.includes('/src/legacy/')) return 'legacy-app'
          return undefined
        },
      },
    },
  },
})
