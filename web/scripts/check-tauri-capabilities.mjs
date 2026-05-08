#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(scriptDir, '..', '..')
const tauriDir = path.resolve(repoRoot, 'src-tauri')
const reportPath = path.resolve(repoRoot, 'output', 'qa', 'tauri-capabilities.json')

const EXPECTED_MAIN_PERMISSIONS = [
  'core:default',
  'core:window:allow-start-dragging',
  'core:window:allow-minimize',
  'core:window:allow-toggle-maximize',
  'core:window:allow-close',
  'clipboard-manager:allow-write-text',
  'updater:default',
  'process:allow-restart',
  'log:default',
  'notification:default',
  'window-state:default',
]

const EXPECTED_DEV_PERMISSIONS = [
  'core:default',
  'log:default',
]

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'))
}

function setDiff(actual, expected) {
  const actualSet = new Set(actual)
  const expectedSet = new Set(expected)
  return {
    missing: expected.filter((item) => !actualSet.has(item)),
    extra: actual.filter((item) => !expectedSet.has(item)),
  }
}

function hasDirective(csp, directive, value) {
  const parts = String(csp || '').split(';').map((part) => part.trim()).filter(Boolean)
  const match = parts.find((part) => part === directive || part.startsWith(`${directive} `))
  if (!match) return false
  return match.split(/\s+/g).slice(1).includes(value)
}

const issues = []
const tauriConfig = readJson(path.resolve(tauriDir, 'tauri.conf.json'))
const mainCapability = readJson(path.resolve(tauriDir, 'capabilities', 'main.json'))
const devCapability = readJson(path.resolve(tauriDir, 'capabilities', 'dev.json'))

const configuredCapabilities = tauriConfig?.app?.security?.capabilities ?? []
if (!configuredCapabilities.includes('main-window')) {
  issues.push('tauri.conf.json must include the main-window capability.')
}
if (configuredCapabilities.some((item) => item !== 'main-window')) {
  issues.push(`Production Tauri config has unexpected capabilities: ${configuredCapabilities.join(', ')}`)
}

if (mainCapability.identifier !== 'main-window') {
  issues.push('main capability identifier must stay main-window.')
}
if (!Array.isArray(mainCapability.windows) || mainCapability.windows.length !== 1 || mainCapability.windows[0] !== 'main') {
  issues.push('main-window capability must apply only to the main window label.')
}
const mainPermissionDiff = setDiff(mainCapability.permissions ?? [], EXPECTED_MAIN_PERMISSIONS)
for (const permission of mainPermissionDiff.missing) issues.push(`main-window missing permission ${permission}`)
for (const permission of mainPermissionDiff.extra) issues.push(`main-window has unexpected permission ${permission}`)

if (devCapability.identifier !== 'dev-window') {
  issues.push('dev capability identifier must stay dev-window.')
}
if (!Array.isArray(devCapability.windows) || devCapability.windows.length !== 1 || devCapability.windows[0] !== 'dev-*') {
  issues.push('dev-window capability must remain scoped to dev-* windows.')
}
const devPermissionDiff = setDiff(devCapability.permissions ?? [], EXPECTED_DEV_PERMISSIONS)
for (const permission of devPermissionDiff.missing) issues.push(`dev-window missing permission ${permission}`)
for (const permission of devPermissionDiff.extra) issues.push(`dev-window has unexpected permission ${permission}`)

const productionCsp = tauriConfig?.app?.security?.csp ?? ''
if (String(productionCsp).includes("'unsafe-eval'")) {
  issues.push('Production CSP must not include unsafe-eval.')
}
if (String(productionCsp).includes("'unsafe-inline'") && !String(productionCsp).includes("style-src 'self' 'unsafe-inline'")) {
  issues.push('Production CSP unsafe-inline usage must remain limited to style-src.')
}
if (!hasDirective(productionCsp, 'object-src', "'none'")) {
  issues.push("Production CSP must keep object-src 'none'.")
}
if (!hasDirective(productionCsp, 'base-uri', "'self'")) {
  issues.push("Production CSP must keep base-uri 'self'.")
}
if (!hasDirective(productionCsp, 'frame-ancestors', "'self'")) {
  issues.push("Production CSP must keep frame-ancestors 'self'.")
}

fs.mkdirSync(path.dirname(reportPath), { recursive: true })
fs.writeFileSync(
  reportPath,
  `${JSON.stringify(
    {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      configuredCapabilities,
      mainPermissions: mainCapability.permissions ?? [],
      devPermissions: devCapability.permissions ?? [],
      productionCsp,
      issues,
    },
    null,
    2,
  )}\n`,
  'utf8',
)

if (issues.length > 0) {
  console.error('[tauri-capabilities] Capability drift detected:')
  for (const issue of issues) console.error(`- ${issue}`)
  process.exit(1)
}

console.log('[tauri-capabilities] Capability and production CSP drift checks passed.')
console.log(`[tauri-capabilities] Report written to ${path.relative(path.resolve(repoRoot, 'web'), reportPath).replace(/\\/g, '/')}`)
