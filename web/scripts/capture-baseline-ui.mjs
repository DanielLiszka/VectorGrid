#!/usr/bin/env node
import fs from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import { chromium } from 'playwright'
import {
  dispatchClick,
  dispatchClickIfPresent,
  dismissStartupOverlayIfPresent,
  installMockedTauri,
  seedAdvancedUiMode,
} from './support/mockedTauriHarness.mjs'

const ROOT = process.cwd()
const DIST_DIR = path.resolve(ROOT, 'dist')
const OUT_DIR = path.resolve(ROOT, '..', 'scripts', 'baselines', 'ui_capture')
const PORT = 4179

const WORKSPACES = ['workflow', 'library', 'aggregate', 'pro', 'settings']
const WORKFLOW_STEPS = ['data', 'optimize', 'validate', 'analyze', 'finalize']
const AGGREGATE_TABS = ['meta', 'batch', 'saved', 'compare']
const OPTIMIZE_CONTROLS_TABS = ['opt', 'strategy', 'advanced']
const OPTIMIZE_RESULTS_TABS = ['overview', 'top', 'pareto', 'heatmap']
const ANALYZE_RESULTS_TABS = ['overview', 'top', 'pareto', 'drill', 'heatmap']

function mimeType(file) {
  if (file.endsWith('.html')) return 'text/html; charset=utf-8'
  if (file.endsWith('.js')) return 'text/javascript; charset=utf-8'
  if (file.endsWith('.css')) return 'text/css; charset=utf-8'
  if (file.endsWith('.json')) return 'application/json; charset=utf-8'
  if (file.endsWith('.svg')) return 'image/svg+xml'
  if (file.endsWith('.png')) return 'image/png'
  if (file.endsWith('.jpg') || file.endsWith('.jpeg')) return 'image/jpeg'
  if (file.endsWith('.ico')) return 'image/x-icon'
  return 'application/octet-stream'
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true })
}

function rel(file) {
  return path.relative(path.resolve(ROOT, '..'), file).replace(/\\/g, '/')
}

function startServer() {
  const server = http.createServer((req, res) => {
    const urlPath = String(req.url || '/').split('?')[0] || '/'
    const clean = urlPath === '/' ? '/index.html' : urlPath
    const filePath = path.resolve(DIST_DIR, `.${clean}`)
    if (!filePath.startsWith(DIST_DIR)) {
      res.statusCode = 403
      res.end('Forbidden')
      return
    }
    if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
      res.statusCode = 404
      res.end('Not found')
      return
    }
    res.statusCode = 200
    res.setHeader('Content-Type', mimeType(filePath))
    fs.createReadStream(filePath).pipe(res)
  })
  return new Promise((resolve) => {
    server.listen(PORT, '127.0.0.1', () => resolve(server))
  })
}

async function clickAndCapture(page, selector, fileName) {
  await dispatchClick(page, selector)
  await page.waitForTimeout(150)
  await page.screenshot({
    path: path.resolve(OUT_DIR, fileName),
    fullPage: true,
  })
}

async function selectWorkspace(page, workspace) {
  await dispatchClick(page, `#workspaceBar button[data-workspace="${workspace}"]`)
  await page.waitForTimeout(150)
}

async function selectWorkflowStep(page, step) {
  await selectWorkspace(page, 'workflow')
  await dispatchClick(page, `#workflowStepBar button[data-workflow-step="${step}"]`)
  await page.waitForTimeout(150)
}

async function main() {
  if (!fs.existsSync(path.resolve(DIST_DIR, 'index.html'))) {
    throw new Error('Missing web/dist/index.html. Run `npm --prefix web run build` first.')
  }

  ensureDir(OUT_DIR)
  const startedAt = Date.now()
  const server = await startServer()
  const baseUrl = `http://127.0.0.1:${PORT}/`

  const browser = await chromium.launch({ headless: true })
  const context = await browser.newContext({ viewport: { width: 1720, height: 1080 } })
  await seedAdvancedUiMode(context)
  await installMockedTauri(context, { holdStartupOverlay: true, emitTestEvents: true })
  const page = await context.newPage()

  const navStart = Date.now()
  await page.goto(baseUrl, { waitUntil: 'domcontentloaded' })
  const domContentLoadedMs = Date.now() - navStart
  await page.waitForLoadState('networkidle')
  const ttiMs = Date.now() - navStart
  const jsHeapUsedBytes = await page.evaluate(() => {
    const perf = performance
    const mem = perf && 'memory' in perf ? perf.memory : null
    return mem && typeof mem.usedJSHeapSize === 'number' ? mem.usedJSHeapSize : null
  })
  let startupCapture = 'shell_bootstrap'

  await page.waitForSelector('#workspaceBar button[data-workspace="workflow"]', { timeout: 15_000 })
  const overlayVisible = await dismissStartupOverlayIfPresent(page)
  if (overlayVisible) {
    startupCapture = 'startup_overlay'
    await page.screenshot({
      path: path.resolve(OUT_DIR, 'startup_overlay_intro.png'),
      fullPage: true,
    })
  } else {
    await page.screenshot({
      path: path.resolve(OUT_DIR, 'startup_shell_bootstrap.png'),
      fullPage: true,
    })
  }
  await clickAndCapture(page, '#workspaceBar button[data-workspace="workflow"]', 'workspace_workflow.png')

  for (const workspace of WORKSPACES) {
    await clickAndCapture(page, `#workspaceBar button[data-workspace="${workspace}"]`, `workspace_${workspace}.png`)
  }
  await selectWorkspace(page, 'aggregate')
  if (await page.locator('#metaAggregateSurface [data-aggregate-tab]').count()) {
    for (const tab of AGGREGATE_TABS) {
      await clickAndCapture(
        page,
        `#metaAggregateSurface button[data-aggregate-tab="${tab}"]`,
        `aggregate_tab_${tab}.png`,
      )
    }
  }
  for (const step of WORKFLOW_STEPS) {
    await selectWorkflowStep(page, step)
    await page.screenshot({
      path: path.resolve(OUT_DIR, `workflow_step_${step}.png`),
      fullPage: true,
    })
  }

  await selectWorkflowStep(page, 'optimize')
  for (const tab of OPTIMIZE_CONTROLS_TABS) {
    await clickAndCapture(page, `#controlsTabBar button[data-tab="${tab}"]`, `workflow_optimize_controls_${tab}.png`)
  }

  await selectWorkflowStep(page, 'optimize')
  for (const tab of OPTIMIZE_RESULTS_TABS) {
    await clickAndCapture(page, `#resultsTabBar button[data-tab="${tab}"]`, `workflow_optimize_results_${tab}.png`)
  }

  await selectWorkflowStep(page, 'analyze')
  for (const tab of ANALYZE_RESULTS_TABS) {
    await clickAndCapture(page, `#resultsTabBar button[data-tab="${tab}"]`, `workflow_analyze_results_${tab}.png`)
  }

  await selectWorkspace(page, 'settings')
  const chartResponsiveness = await page.evaluate(() => {
    const emit = window.__VTA_TEST_EMIT
    if (typeof emit !== 'function') return null
    const symbolInput = document.getElementById('execSymbol')
    const symbol = String((symbolInput && 'value' in symbolInput ? symbolInput.value : '') || 'AAPL')
      .trim()
      .toUpperCase()
    const baseOpenMs = Date.now()
    const eventCount = 200
    const started = performance.now()
    for (let i = 0; i < eventCount; i += 1) {
      const open = 100 + i * 0.01
      emit('polygon_live_bar', {
        symbol,
        time_open_unix_ms: baseOpenMs + i * 60_000,
        open,
        high: open + 0.2,
        low: open - 0.2,
        close: open + 0.05,
      })
    }
    const totalMs = performance.now() - started
    const statusText = String(document.getElementById('execLiveStatus')?.textContent || '').trim()
    return {
      events: eventCount,
      total_ms: Number(totalMs.toFixed(3)),
      avg_ms_per_event: Number((totalMs / eventCount).toFixed(4)),
      status: statusText,
    }
  })

  await page.screenshot({
    path: path.resolve(OUT_DIR, 'fullpage_final.png'),
    fullPage: true,
  })

  const metrics = {
    generated_at: new Date().toISOString(),
    app_cold_start_ms: domContentLoadedMs,
    time_to_interactive_ms: ttiMs,
    js_heap_used_bytes_at_idle: jsHeapUsedBytes,
    startup_capture_mode: startupCapture,
    chart_live_feed_responsiveness: chartResponsiveness,
    capture_runtime_ms: Date.now() - startedAt,
    note: 'Synthetic live-bar event throughput captured via mocked Tauri event emitter.',
  }
  fs.writeFileSync(path.resolve(OUT_DIR, 'perf_metrics.json'), `${JSON.stringify(metrics, null, 2)}\n`, 'utf8')

  await browser.close()
  await new Promise((resolve) => server.close(resolve))

  console.log(`UI baseline screenshots captured under ${rel(OUT_DIR)}`)
  console.log(`Metrics written to ${rel(path.resolve(OUT_DIR, 'perf_metrics.json'))}`)
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err))
  process.exit(1)
})
