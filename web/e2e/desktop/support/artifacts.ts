import fs from 'node:fs'
import path from 'node:path'
import { desktopArtifactsRoot, ensureDir } from './appPaths.js'

export type DesktopArtifactContext = {
  runId: string
  runDir: string
  logsDir: string
  screenshotsDir: string
  driverLogPath: string
  previewLogPath: string
  envSummaryPath: string
}

function timestampForRunId(value: Date): string {
  return value.toISOString().replace(/[:.]/g, '-')
}

export function fileSafeName(value: string): string {
  return String(value || 'unnamed')
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001F]+/g, '-')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .slice(0, 160)
}

export function createDesktopArtifactContext(runAt = new Date()): DesktopArtifactContext {
  const runId = String(process.env.VECTORGRID_DESKTOP_SMOKE_RUN_ID || '').trim() || timestampForRunId(runAt)
  const runDir = ensureDir(path.resolve(desktopArtifactsRoot, runId))
  const logsDir = ensureDir(path.resolve(runDir, 'logs'))
  const screenshotsDir = ensureDir(path.resolve(runDir, 'screenshots'))
  return {
    runId,
    runDir,
    logsDir,
    screenshotsDir,
    driverLogPath: path.resolve(logsDir, 'tauri-driver.log'),
    previewLogPath: path.resolve(logsDir, 'preview-server.log'),
    envSummaryPath: path.resolve(logsDir, 'environment.txt'),
  }
}

export function appendLogLine(targetPath: string, line: string): void {
  fs.appendFileSync(targetPath, `${line}\n`, 'utf8')
}

export function writeTextFile(targetPath: string, body: string): void {
  ensureDir(path.dirname(targetPath))
  fs.writeFileSync(targetPath, body, 'utf8')
}

export function failureScreenshotPath(ctx: DesktopArtifactContext, title: string): string {
  return path.resolve(ctx.screenshotsDir, `${fileSafeName(title)}.png`)
}
