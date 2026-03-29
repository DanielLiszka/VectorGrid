#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'

const ROOT = process.cwd()
const CANONICAL_PATH = path.resolve(ROOT, 'src-tauri', 'tauri.conf.json')
const MIRROR_PATH = path.resolve(ROOT, 'tauri.conf.json')

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function stableWindowShape(windowConfig) {
  return {
    label: windowConfig?.label ?? null,
    title: windowConfig?.title ?? null,
    create: windowConfig?.create ?? null,
    width: windowConfig?.width ?? null,
    height: windowConfig?.height ?? null,
    transparent: windowConfig?.transparent ?? null,
    visible: windowConfig?.visible ?? null,
  }
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

if (failures.length > 0) {
  console.error(failures.join('\n\n'))
  process.exit(1)
}

console.log('Tauri config mirror is in sync with src-tauri/tauri.conf.json')
