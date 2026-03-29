import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const supportDir = path.dirname(fileURLToPath(import.meta.url))

export const webRoot = path.resolve(supportDir, '..', '..', '..')
export const repoRoot = path.resolve(webRoot, '..')
const desktopBinaryOverride = String(process.env.VECTORGRID_DESKTOP_SMOKE_BINARY || '').trim()
const appUrlOverride = String(process.env.VECTORGRID_DESKTOP_SMOKE_APP_URL || '').trim()
export const desktopAppBinaryPath = desktopBinaryOverride
  ? (path.isAbsolute(desktopBinaryOverride)
      ? desktopBinaryOverride
      : path.resolve(repoRoot, desktopBinaryOverride))
  : path.resolve(repoRoot, 'target', 'debug', 'ta_desktop_demo_app.exe')
export const desktopArtifactsRoot = path.resolve(repoRoot, 'output', 'wdio', 'desktop-smoke')
export const desktopSmokeAppUrl = appUrlOverride || 'http://127.0.0.1:1420/'

export function ensureDir(dir: string): string {
  fs.mkdirSync(dir, { recursive: true })
  return dir
}
