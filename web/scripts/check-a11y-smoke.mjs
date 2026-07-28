#!/usr/bin/env node
import fs from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import AxeBuilder from '@axe-core/playwright'
import { chromium } from 'playwright'
import {
  dispatchClick,
  dismissStartupOverlayIfPresent,
  installMockedTauri,
  seedAdvancedUiMode,
} from './support/mockedTauriHarness.mjs'

const ROOT = process.cwd()
const DIST_DIR = path.resolve(ROOT, 'dist')
const REPORT_PATH = path.resolve(ROOT, '..', 'output', 'qa', 'a11y-smoke.json')
const PORT = 4181
const INCLUDE_COLOR_CONTRAST =
  process.env.VECTORGRID_A11Y_INCLUDE_CONTRAST === '1' || process.argv.includes('--include-color-contrast')
const INCLUDE_WCAG_22 =
  process.env.VECTORGRID_A11Y_INCLUDE_WCAG_22 === '1' || process.argv.includes('--include-wcag-2-2')
const scannedSurfaces = []

function writeReport(status, error = '') {
  const report = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    status,
    includeColorContrast: INCLUDE_COLOR_CONTRAST,
    includeWcag22: INCLUDE_WCAG_22,
    scannedSurfaces,
    error,
  }
  fs.mkdirSync(path.dirname(REPORT_PATH), { recursive: true })
  fs.writeFileSync(REPORT_PATH, `${JSON.stringify(report, null, 2)}\n`, 'utf8')
}

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

async function scan(page, name, options = {}) {
  const builder = new AxeBuilder({ page }).options(options)
  if (!INCLUDE_COLOR_CONTRAST) {
    builder.disableRules(['color-contrast'])
  }
  const result = await builder.analyze()
  scannedSurfaces.push({
    name,
    violationCount: result.violations.length,
    passes: result.passes.length,
    incomplete: result.incomplete.length,
  })

  if (!result.violations.length) return

  const lines = [`Accessibility violations for ${name}:`]
  for (const violation of result.violations) {
    lines.push(`- ${violation.id}: ${violation.help}`)
    for (const node of violation.nodes.slice(0, 3)) {
      lines.push(`  target: ${node.target.join(' ')}`)
      lines.push(`  summary: ${node.failureSummary || 'n/a'}`)
    }
  }
  throw new Error(lines.join('\n'))
}

async function assertTargetSizes(page, name) {
  const issues = await page.evaluate(() => {
    const interactiveSelector = [
      'button',
      'a[href]',
      'input:not([type="hidden"])',
      'select',
      'textarea',
      '[role="button"]',
      '[role="tab"]',
      '[role="menuitem"]',
      '[role="option"]',
      '[tabindex]:not([tabindex="-1"])',
    ].join(',')
    const rectFor = (element) => {
      const rect = element.getBoundingClientRect()
      let left = rect.left
      let right = rect.right
      let top = rect.top
      let bottom = rect.bottom
      const id = element.getAttribute('id')
      const label = id ? document.querySelector(`label[for="${CSS.escape(id)}"]`) : element.closest('label')
      if (label) {
        const labelRect = label.getBoundingClientRect()
        left = Math.min(left, labelRect.left)
        right = Math.max(right, labelRect.right)
        top = Math.min(top, labelRect.top)
        bottom = Math.max(bottom, labelRect.bottom)
      }
      return { width: right - left, height: bottom - top }
    }
    return Array.from(document.querySelectorAll(interactiveSelector))
      .map((node) => {
        const element = node
        const style = window.getComputedStyle(element)
        const rawRect = element.getBoundingClientRect()
        const disabled =
          element.hasAttribute('disabled') ||
          element.getAttribute('aria-disabled') === 'true' ||
          style.pointerEvents === 'none'
        const visible =
          rawRect.width > 0 &&
          rawRect.height > 0 &&
          style.visibility !== 'hidden' &&
          style.display !== 'none' &&
          !element.closest('[aria-hidden="true"]')
        if (!visible || disabled) return null
        const rect = rectFor(element)
        if (rect.width >= 24 && rect.height >= 24) return null
        const label =
          element.getAttribute('aria-label') ||
          element.getAttribute('title') ||
          element.textContent ||
          element.getAttribute('id') ||
          element.tagName.toLowerCase()
        const id = element.getAttribute('id') || ''
        const type = element.getAttribute('type') || ''
        const classes = element.getAttribute('class') || ''
        const selectorHint = [
          element.tagName.toLowerCase(),
          id ? `#${id}` : '',
          type ? `[type="${type}"]` : '',
          classes ? `.${classes.trim().replace(/\s+/g, '.')}` : '',
        ].join('')
        return `${selectorHint} "${String(label).trim().slice(0, 80)}" ${rect.width.toFixed(1)}x${rect.height.toFixed(1)}`
      })
      .filter(Boolean)
      .slice(0, 12)
  })
  if (issues.length > 0) {
    throw new Error(`WCAG target-size issues for ${name}:\n${issues.map((issue) => `- ${issue}`).join('\n')}`)
  }
}

async function assertKeyboardFocusVisibility(page, name) {
  const issues = []
  const visited = []
  await page.keyboard.press('Home').catch(() => {})
  for (let i = 0; i < 18; i += 1) {
    await page.keyboard.press('Tab')
    await page.waitForTimeout(20)
    const state = await page.evaluate(() => {
      const element = document.activeElement
      if (!(element instanceof HTMLElement) || element === document.body || element === document.documentElement) {
        return null
      }
      const rect = element.getBoundingClientRect()
      const style = window.getComputedStyle(element)
      const outlineWidth = Number.parseFloat(style.outlineWidth || '0')
      const hasOutline = style.outlineStyle !== 'none' && outlineWidth > 0
      const hasShadow = !!style.boxShadow && style.boxShadow !== 'none'
      const visibleRect = rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.right >= 0
      const label =
        element.getAttribute('aria-label') ||
        element.getAttribute('title') ||
        element.textContent ||
        element.getAttribute('id') ||
        element.tagName.toLowerCase()
      return {
        label: String(label).trim().replace(/\s+/g, ' ').slice(0, 80),
        tag: element.tagName.toLowerCase(),
        visibleRect,
        focusVisible: element.matches(':focus-visible'),
        hasVisibleFocusStyle: hasOutline || hasShadow,
      }
    })
    if (!state) continue
    visited.push(`${state.tag} ${state.label}`)
    if (!state.visibleRect || !state.focusVisible || !state.hasVisibleFocusStyle) {
      issues.push(`${state.tag} "${state.label}" focusVisible=${state.focusVisible} styled=${state.hasVisibleFocusStyle}`)
    }
  }
  if (visited.length === 0) {
    throw new Error(`WCAG keyboard focus check for ${name} did not reach any focusable elements.`)
  }
  if (issues.length > 0) {
    throw new Error(`WCAG focus visibility issues for ${name}:\n${issues.slice(0, 8).map((issue) => `- ${issue}`).join('\n')}`)
  }
}

async function wcag22Scan(page, name) {
  if (!INCLUDE_WCAG_22) return
  await assertTargetSizes(page, name)
  await assertKeyboardFocusVisibility(page, name)
}

async function main() {
  if (!fs.existsSync(path.resolve(DIST_DIR, 'index.html'))) {
    throw new Error('Missing web/dist/index.html. Run `npm --prefix web run build` first.')
  }

  const server = await startServer()
  const browser = await chromium.launch({ headless: true })
  const context = await browser.newContext({ viewport: { width: 1600, height: 1000 } })
  await seedAdvancedUiMode(context)
  await installMockedTauri(context, {
    holdStartupOverlay: true,
    commandResults: {
      alpaca_get_credential_status: {
        remember_available: true,
        has_api_key: false,
        has_api_secret: false,
        has_credentials: false,
        state: 'missing',
        storage_backend: 'keyring',
        message: '',
      },
      get_execution_risk_policy: {
        max_order_notional: null,
        max_position_notional: null,
        allow_short: true,
        require_bracket: false,
      },
    },
  })

  const page = await context.newPage()
  await page.goto(`http://127.0.0.1:${PORT}/`, { waitUntil: 'networkidle' })
  await page.waitForSelector('#workspaceBar button[data-workspace="workflow"]', { timeout: 15000 })
  if (await dismissStartupOverlayIfPresent(page)) {
    await scan(page, 'startup overlay')
    await wcag22Scan(page, 'startup overlay')
  }

  await scan(page, 'workflow shell')
  await wcag22Scan(page, 'workflow shell')

  if (await page.locator('#uiModeBar').count()) {
    throw new Error('Advanced-only shell regression: #uiModeBar should not be mounted.')
  }
  if (await page.locator('#guidedShellGrid').count()) {
    throw new Error('Advanced-only shell regression: #guidedShellGrid should not be mounted.')
  }
  await page.waitForSelector('#workspaceBar button[data-workspace="workflow"]', { timeout: 15000 })

  await dispatchClick(page, '#workspaceBar button[data-workspace="live"]')
  await page.waitForSelector('[aria-label="Execution runtime status"]', { timeout: 15000 })
  await page.selectOption('#vueExecTarget', 'local_paper')
  await page.fill('#vueExecSymbol', 'AAPL')
  await page.waitForTimeout(200)
  await scan(page, 'execution workspace')
  await wcag22Scan(page, 'execution workspace')

  await dispatchClick(page, '#workspaceBar button[data-workspace="settings"]')
  await page.waitForTimeout(200)
  await scan(page, 'settings workspace')
  await wcag22Scan(page, 'settings workspace')

  await dispatchClick(page, '#workspaceBar button[data-workspace="workflow"]')
  await dispatchClick(page, '#workflowStepBar button[data-workflow-step="validate"]')
  await page.waitForTimeout(200)
  await scan(page, 'validation workflow')
  await wcag22Scan(page, 'validation workflow')

  await dispatchClick(page, 'button[aria-label=\"Open command palette\"]')
  await page.waitForTimeout(150)
  await scan(page, 'command palette')
  await wcag22Scan(page, 'command palette')

  await page.keyboard.press('Escape')
  await page.waitForSelector('.cmdPaletteDialog', { state: 'detached', timeout: 5000 })
  await dispatchClick(page, '[data-qa-action="shell.taskCenter.open"]')
  await page.waitForTimeout(150)
  await scan(page, 'task center overlay')
  await wcag22Scan(page, 'task center overlay')

  await browser.close()
  await new Promise((resolve) => server.close(resolve))
  writeReport('passed')
  console.log('Accessibility smoke checks passed.')
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error)
  writeReport('failed', message)
  console.error(message)
  process.exit(1)
})
