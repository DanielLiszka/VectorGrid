import { configDefaults, defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { visualizer } from 'rollup-plugin-visualizer'
import checker from 'vite-plugin-checker'
import { minify } from 'terser'
import type { Plugin } from 'vite'

const baseTestExclude = [
  ...configDefaults.exclude,
  'e2e/**',
  'scripts/**/*.test.mjs',
]

const jsdomTestInclude = [
  'src/App.test.ts',
  'src/main.test.ts',
  'src/preflight.test.ts',
  'src/components/**/*.test.ts',
  'src/composables/useLightweightChart.test.ts',
  'src/composables/usePersistedMatrixColumnState.test.ts',
  'src/composables/useShellMount.test.ts',
  'src/composables/useTaskCenterVisibility.test.ts',
  'src/composables/useTaskLifecycleSummary.test.ts',
  'src/services/analysisService.test.ts',
  'src/services/artifactHistoryPerf.test.ts',
  'src/services/desktopProductization.test.ts',
  'src/services/executionChartService.test.ts',
  'src/services/optionsExpressionService.test.ts',
  'src/services/validationReportPerf.test.ts',
  'src/stores/historyStore.search.test.ts',
  'src/tauri/errorPathQa.test.ts',
  'src/tauri/instrumentation.mockIPC.test.ts',
  'src/tauri/invoke.mockIPC.test.ts',
  'src/utils/core/runArtifactReview.test.ts',
  'src/visualizations/*.test.ts',
]

function secondaryTerserPass(pass: number): Plugin {
  return {
    name: `secondary-terser-pass-${pass}`,
    enforce: 'post',
    renderChunk: {
      order: 'post',
      async handler(code) {
        const result = await minify(code, {
          ecma: 2022,
          module: true,
          compress: {
            hoist_funs: true,
            hoist_vars: true,
            passes: 3,
          },
          format: {
            comments: false,
          },
        })
        return result.code ? { code: result.code, map: null } : null
      },
    },
  }
}

export default defineConfig(({ mode }) => ({
  plugins: [
    vue(),
    ...(mode === 'development'
      ? [
          checker({
            vueTsc: {
              tsconfigPath: 'tsconfig.app.json',
            },
          }),
        ]
      : []),
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
    ...(mode === 'production'
      ? [secondaryTerserPass(2), secondaryTerserPass(3), secondaryTerserPass(4)]
      : []),
  ],
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
  },
  test: {
    exclude: baseTestExclude,
    projects: [
      {
        extends: true,
        test: {
          name: 'node',
          environment: 'node',
          exclude: [...baseTestExclude, ...jsdomTestInclude],
        },
      },
      {
        extends: true,
        test: {
          name: 'jsdom',
          environment: 'jsdom',
          include: jsdomTestInclude,
          exclude: baseTestExclude,
        },
      },
    ],
  },
  build: {
    emptyOutDir: true,
    outDir: 'dist',
    // Keep small UI glyphs inline, but emit larger SVG illustrations separately
    // so they do not inflate JavaScript parse/evaluation work as base64 data URIs.
    assetsInlineLimit: 1024,
    sourcemap: mode === 'analyze',
    target: 'es2022',
    minify: 'terser',
    terserOptions: {
      ecma: 2022,
      compress: {
        hoist_funs: true,
        hoist_vars: true,
        passes: 3,
      },
      format: {
        comments: false,
      },
    },
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
            normalized.includes('/node_modules/@tauri-apps/api/')
            || normalized.includes('/node_modules/@tauri-apps/plugin-')
          ) {
            return 'vendor-tauri'
          }
          if (
            normalized.includes('/node_modules/@vueuse/')
            || normalized.includes('/node_modules/fuse.js/')
            || normalized.includes('/node_modules/@lucide/vue/')
            || normalized.includes('/node_modules/zod/')
          ) {
            return 'vendor-utilities'
          }
          if (normalized.includes('/src/stores/')) {
            return 'app-state'
          }
          return undefined
        },
      },
    },
  },
}))
