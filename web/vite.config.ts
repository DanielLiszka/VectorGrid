import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { visualizer } from 'rollup-plugin-visualizer'

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [
    vue(),
    ...(mode === 'analyze'
      ? [
          visualizer({
            filename: 'dist/bundle-stats.html',
            gzipSize: true,
            brotliSize: true,
            open: false,
          }),
        ]
      : []),
  ],
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
          if (normalized.includes('lightweight-charts.production')) return 'vendor-charts'
          if (
            normalized.includes('/node_modules/vue/')
            || normalized.includes('/node_modules/@vue/')
            || normalized.includes('/node_modules/pinia/')
          ) {
            return 'vendor-framework'
          }
          if (
            normalized.includes('/node_modules/reka-ui/')
            || normalized.includes('/node_modules/@floating-ui/')
            || normalized.includes('/node_modules/aria-hidden/')
          ) {
            return 'vendor-ui'
          }
          if (
            normalized.includes('/node_modules/@vueuse/')
            || normalized.includes('/node_modules/fuse.js/')
            || normalized.includes('/node_modules/lucide-vue-next/')
            || normalized.includes('/node_modules/zod/')
          ) {
            return 'vendor-utilities'
          }
          return undefined
        },
      },
    },
  },
}))
