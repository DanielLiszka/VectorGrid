import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const webRoot = path.resolve(scriptDir, '..', '..')
const repoRoot = path.resolve(webRoot, '..')
const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm'

function runOrThrow(command: string, args: string[], cwd: string): void {
  const isWindowsCmd = process.platform === 'win32' && command.toLowerCase().endsWith('.cmd')
  const spawnCommand = isWindowsCmd ? 'cmd.exe' : command
  const spawnArgs = isWindowsCmd ? ['/d', '/s', '/c', command, ...args] : args
  const result = spawnSync(spawnCommand, spawnArgs, {
    cwd,
    stdio: 'inherit',
    shell: false,
  })
  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(' ')} (cwd=${cwd})`)
  }
}

export function buildDesktopSmokeApp(): void {
  runOrThrow(npmCommand, ['run', 'build'], webRoot)
  runOrThrow('cargo', ['build', '-p', 'ta_desktop_demo_app'], repoRoot)
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1].replace(/\\/g, '/')}`).href) {
  buildDesktopSmokeApp()
}
