#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'

const ROOT = process.cwd()
const CANONICAL_PATH = path.resolve(ROOT, 'src-tauri', 'tauri.conf.json')
const MIRROR_PATH = path.resolve(ROOT, 'tauri.conf.json')
const CANONICAL_WINDOWS_PATH = path.resolve(ROOT, 'src-tauri', 'tauri.windows.conf.json')
const MIRROR_WINDOWS_PATH = path.resolve(ROOT, 'tauri.windows.conf.json')

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function stableValue(value) {
  if (Array.isArray(value)) {
    return value.map(stableValue)
  }

  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, stableValue(value[key])]),
    )
  }

  return value ?? null
}

function stableWindowShape(windowConfig) {
  return stableValue(windowConfig ?? {})
}

function comparableShape(config) {
  return {
    productName: config.productName ?? null,
    version: config.version ?? null,
    identifier: config.identifier ?? null,
    build: {
      beforeDevCommand: config.build?.beforeDevCommand ?? null,
      beforeBuildCommand: config.build?.beforeBuildCommand ?? null,
      devUrl: config.build?.devUrl ?? null,
    },
    app: {
      withGlobalTauri: config.app?.withGlobalTauri ?? null,
      macOSPrivateApi: config.app?.macOSPrivateApi ?? null,
      security: {
        capabilities: config.app?.security?.capabilities ?? [],
        csp: config.app?.security?.csp ?? null,
        devCsp: config.app?.security?.devCsp ?? null,
      },
      windows: Array.isArray(config.app?.windows) ? config.app.windows.map(stableWindowShape) : [],
    },
  }
}

function normalizeBundleResources(configPath, resources) {
  if (!resources) return null
  const configDir = path.dirname(configPath)
  if (Array.isArray(resources)) {
    return resources.map((value) => ({
      source: path.resolve(configDir, value),
      target: null,
    }))
  }
  if (typeof resources === 'object') {
    return Object.entries(resources)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([source, target]) => ({
        source: path.resolve(configDir, source),
        target: target ?? null,
      }))
  }
  return resources
}

function validateResourceSourcesExist(label, resources, failures) {
  if (!Array.isArray(resources)) return
  for (const entry of resources) {
    if (!entry || typeof entry !== 'object') continue
    const source = typeof entry.source === 'string' ? entry.source : ''
    if (!source) continue
    if (!fs.existsSync(source)) {
      failures.push(`${label} references missing bundle resource source: ${source}`)
    }
  }
}

function readOptionalJson(filePath) {
  return fs.existsSync(filePath) ? readJson(filePath) : null
}

const canonical = readJson(CANONICAL_PATH)
const mirror = readJson(MIRROR_PATH)

const expectedCanonicalDist = '../web/dist'
const expectedMirrorDist = './web/dist'
const failures = []

if (canonical.build?.frontendDist !== expectedCanonicalDist) {
  failures.push(`Expected canonical frontendDist to be ${expectedCanonicalDist}, received ${canonical.build?.frontendDist}`)
}

if (mirror.build?.frontendDist !== expectedMirrorDist) {
  failures.push(`Expected root mirror frontendDist to be ${expectedMirrorDist}, received ${mirror.build?.frontendDist}`)
}

const canonicalShape = comparableShape(canonical)
const mirrorShape = comparableShape(mirror)

if (JSON.stringify(canonicalShape) !== JSON.stringify(mirrorShape)) {
  failures.push('Canonical and root mirror Tauri configs differ on shared fields.')
  failures.push(`Canonical: ${JSON.stringify(canonicalShape, null, 2)}`)
  failures.push(`Mirror: ${JSON.stringify(mirrorShape, null, 2)}`)
}

const canonicalWindows = readOptionalJson(CANONICAL_WINDOWS_PATH)
const mirrorWindows = readOptionalJson(MIRROR_WINDOWS_PATH)

if (Boolean(canonicalWindows) !== Boolean(mirrorWindows)) {
  failures.push('Canonical and root mirror Windows Tauri configs are out of sync on presence.')
} else if (canonicalWindows && mirrorWindows) {
  const canonicalWindowsShape = {
    schema: canonicalWindows.$schema ?? null,
    bundle: {
      resources: normalizeBundleResources(
        CANONICAL_WINDOWS_PATH,
        canonicalWindows.bundle?.resources ?? null,
      ),
    },
  }
  const mirrorWindowsShape = {
    schema: mirrorWindows.$schema ?? null,
    bundle: {
      resources: normalizeBundleResources(
        MIRROR_WINDOWS_PATH,
        mirrorWindows.bundle?.resources ?? null,
      ),
    },
  }
  if (JSON.stringify(canonicalWindowsShape) !== JSON.stringify(mirrorWindowsShape)) {
    failures.push('Canonical and root mirror Windows Tauri configs differ on shared fields.')
    failures.push(`Canonical windows: ${JSON.stringify(canonicalWindowsShape, null, 2)}`)
    failures.push(`Mirror windows: ${JSON.stringify(mirrorWindowsShape, null, 2)}`)
  }
  validateResourceSourcesExist(
    'Canonical Windows Tauri config',
    canonicalWindowsShape.bundle.resources,
    failures,
  )
  validateResourceSourcesExist(
    'Root mirror Windows Tauri config',
    mirrorWindowsShape.bundle.resources,
    failures,
  )
}

if (failures.length > 0) {
  console.error(failures.join('\n\n'))
  process.exit(1)
}

console.log('Tauri config mirrors are in sync, including optional Windows bundle resources.')
