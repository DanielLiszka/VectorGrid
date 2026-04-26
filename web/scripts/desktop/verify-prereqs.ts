import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..', '..')
const repoRoot = path.resolve(webRoot, '..')

function commandExists(command: string, explicitPath?: string): string | null {
  if (explicitPath && fs.existsSync(explicitPath)) return explicitPath
  if (process.platform === 'win32') {
    const resolved = spawnSync('where.exe', [command], { encoding: 'utf8' })
    if (resolved.status === 0) {
      const candidate = String(resolved.stdout || '')
        .split(/\r?\n/)
        .map((line) => line.trim())
        .find((line) => line.length > 0)
      if (candidate) return candidate
    }
  } else {
    const resolved = spawnSync('which', [command], { encoding: 'utf8' })
    if (resolved.status === 0) {
      const candidate = String(resolved.stdout || '').trim()
      if (candidate) return candidate
    }
  }
  return null
}

function versionOf(executable: string, args = ['--version']): string {
  const result = spawnSync(executable, args, { encoding: 'utf8' })
  if (result.status !== 0) {
    return `unavailable (${String(result.stderr || result.stdout || '').trim() || 'failed to execute'})`
  }
  return String(result.stdout || result.stderr || '').trim() || 'unknown'
}

export function verifyDesktopSmokePrereqs(): {
  tauriDriverPath: string
  edgeDriverPath: string
  summary: string
} {
  const edgeDriverOverride = String(process.env.VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER || '').trim()
  const tauriDriverFallback = path.resolve(
    os.homedir(),
    '.cargo',
    'bin',
    process.platform === 'win32' ? 'tauri-driver.exe' : 'tauri-driver',
  )
  const edgeDriverFallbacks = [
    edgeDriverOverride,
    path.resolve(repoRoot, process.platform === 'win32' ? 'msedgedriver.exe' : 'msedgedriver'),
    path.resolve(webRoot, process.platform === 'win32' ? 'msedgedriver.exe' : 'msedgedriver'),
  ].filter(Boolean)
  const tauriDriverPath = commandExists('tauri-driver', tauriDriverFallback)
  const edgeDriverPath = edgeDriverFallbacks
    .map((candidate) =>
      commandExists(
        process.platform === 'win32' ? 'msedgedriver.exe' : 'msedgedriver',
        candidate,
      ),
    )
    .find((candidate): candidate is string => Boolean(candidate))
    ?? commandExists(process.platform === 'win32' ? 'msedgedriver.exe' : 'msedgedriver')

  if (!tauriDriverPath) {
    throw new Error(
      'Desktop smoke requires tauri-driver. Install it with "cargo install tauri-driver --locked".',
    )
  }
  if (!edgeDriverPath) {
    throw new Error(
      'Desktop smoke requires msedgedriver.exe. Install a driver that matches the local Edge version and place it on PATH, in the repo root, or set VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER.',
    )
  }

  const summaryLines = [
    `repoRoot: ${repoRoot}`,
    `webRoot: ${webRoot}`,
    `platform: ${process.platform}`,
    `node: ${process.version}`,
    `cargo: ${versionOf('cargo')}`,
    `tauri-driver: ${versionOf(tauriDriverPath, ['--help'])}`,
    `msedgedriver: ${versionOf(edgeDriverPath)}`,
  ]

  return {
    tauriDriverPath,
    edgeDriverPath,
    summary: summaryLines.join('\n'),
  }
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1].replace(/\\/g, '/')}`).href) {
  const { summary } = verifyDesktopSmokePrereqs()
  process.stdout.write(`${summary}\n`)
}
