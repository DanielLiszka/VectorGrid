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
const PORT = 4181

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
  const result = await new AxeBuilder({ page })
    .disableRules(['color-contrast'])
    .options(options)
    .analyze()

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
  }

  await scan(page, 'workflow shell')

  await dispatchClick(page, '#workspaceBar button[data-workspace="live"]')
  await page.waitForSelector('[aria-label="Execution runtime status"]', { timeout: 15000 })
  await page.selectOption('#vueExecTarget', 'local_paper')
  await page.fill('#vueExecSymbol', 'AAPL')
  await page.waitForTimeout(200)
  await scan(page, 'execution workspace')

  await dispatchClick(page, '#workspaceBar button[data-workspace="settings"]')
  await page.waitForTimeout(200)
  await scan(page, 'settings workspace')

  await dispatchClick(page, 'button[aria-label=\"Open command palette\"]')
  await page.waitForTimeout(150)
  await scan(page, 'command palette')

  await dispatchClick(page, '.overlayToggle')
  await page.waitForTimeout(150)
  await scan(page, 'task center overlay')

  await browser.close()
  await new Promise((resolve) => server.close(resolve))
  console.log('Accessibility smoke checks passed.')
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
