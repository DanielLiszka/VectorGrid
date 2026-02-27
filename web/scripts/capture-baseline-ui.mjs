#!/usr/bin/env node
import fs from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import { chromium } from 'playwright'

const ROOT = process.cwd()
const DIST_DIR = path.resolve(ROOT, 'dist')
const OUT_DIR = path.resolve(ROOT, '..', 'scripts', 'baselines', 'ui_capture')
const PORT = 4179

const WORKSPACES = ['workflow', 'library', 'aggregate', 'pro', 'settings']
const WORKFLOW_STEPS = ['data', 'optimize', 'validate', 'analyze']
const LEGACY_WORKFLOW_TABS = ['data', 'optimize', 'validate', 'analyze', 'history', 'settings']
const CONTROLS_TABS = ['data', 'opt', 'strategy', 'advanced', 'distributed']
const RESULTS_TABS = ['overview', 'top', 'pareto', 'validate', 'drill', 'heatmap', 'history', 'meta', 'debug']

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

async function dispatchClick(page, selector) {
  const clicked = await page.evaluate((sel) => {
    const el = document.querySelector(sel)
    if (!el) return false
    ;(el instanceof HTMLElement ? el : null)?.click?.()
    return true
  }, selector)
  if (!clicked) {
    throw new Error(`Selector not found for click: ${selector}`)
  }
}

async function dispatchClickIfPresent(page, selector) {
  await page.evaluate((sel) => {
    const el = document.querySelector(sel)
    if (!el) return
    ;(el instanceof HTMLElement ? el : null)?.click?.()
  }, selector)
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
  await context.addInitScript(() => {
    const handlers = {}
    const listen = async (event, cb) => {
      if (!handlers[event]) handlers[event] = []
      handlers[event].push(cb)
      return () => {
        handlers[event] = (handlers[event] || []).filter((fn) => fn !== cb)
      }
    }
    const invoke = async () => null
    window.__TAURI__ = window.__TAURI__ || {}
    window.__TAURI__.event = { listen }
    window.__TAURI__.core = { invoke }
    window.__TAURI__.invoke = invoke
    window.__VTA_TEST_EMIT = (event, payload) => {
      const list = handlers[event] || []
      for (const cb of list) {
        try {
          cb({ event, payload })
        } catch {
          // ignore callback errors in baseline capture harness
        }
      }
    }
  })
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

  await page.waitForSelector('#workspaceBar button[data-workspace="workflow"]', { timeout: 15_000 })
  await clickAndCapture(page, '#workspaceBar button[data-workspace="workflow"]', 'workspace_workflow.png')

  for (const workspace of WORKSPACES) {
    await clickAndCapture(page, `#workspaceBar button[data-workspace="${workspace}"]`, `workspace_${workspace}.png`)
  }
  await dispatchClick(page, '#workspaceBar button[data-workspace="workflow"]')
  await page.waitForTimeout(150)
  await dispatchClickIfPresent(page, '#btnToggleWorkflowAdvanced')
  await page.waitForTimeout(150)
  for (const step of WORKFLOW_STEPS) {
    await clickAndCapture(page, `#workflowStepBar button[data-workflow-step="${step}"]`, `workflow_step_${step}.png`)
  }
  for (const wf of LEGACY_WORKFLOW_TABS) {
    await clickAndCapture(page, `#workflowTabBar button[data-workflow="${wf}"]`, `workflow_tab_${wf}.png`)
  }
  for (const tab of CONTROLS_TABS) {
    await clickAndCapture(page, `#controlsTabBar button[data-tab="${tab}"]`, `controls_tab_${tab}.png`)
  }
  for (const tab of RESULTS_TABS) {
    await clickAndCapture(page, `#resultsTabBar button[data-tab="${tab}"]`, `results_tab_${tab}.png`)
  }

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
