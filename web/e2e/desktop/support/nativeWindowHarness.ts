import { spawn, type ChildProcess } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { repoRoot } from './appPaths.js'
import {
  isPathInside,
  releaseProofEvidencePath,
  requireReleaseProofEnvironment,
} from './releaseProofEvidence.js'

type HarnessStatus = {
  status?: unknown
  mode?: unknown
  message?: unknown
  target_path?: unknown
  expected_directory?: unknown
  window_handle?: unknown
}

export type NativeWindowHarnessHandle = {
  ready: () => Promise<void>
  complete: () => Promise<HarnessStatus>
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function parseEvidence(evidencePath: string): HarnessStatus {
  if (!fs.existsSync(evidencePath)) return {}
  try {
    return JSON.parse(fs.readFileSync(evidencePath, 'utf8')) as HarnessStatus
  } catch {
    return {}
  }
}

function waitForReady(
  child: ChildProcess,
  evidencePath: string,
  timeoutMs: number,
): Promise<void> {
  const startedAt = Date.now()
  return new Promise((resolve, reject) => {
    const poll = () => {
      const status = text(parseEvidence(evidencePath).status).toLowerCase()
      if (status === 'ready') {
        resolve()
        return
      }
      if (status === 'failed') {
        reject(new Error(`Native window harness failed before ready: ${text(parseEvidence(evidencePath).message)}`))
        return
      }
      if (child.exitCode != null) {
        reject(new Error(`Native window harness exited before ready with code ${String(child.exitCode)}.`))
        return
      }
      if (Date.now() - startedAt > timeoutMs) {
        child.kill()
        reject(new Error(`Native window harness did not become ready within ${String(timeoutMs)}ms.`))
        return
      }
      setTimeout(poll, 50)
    }
    poll()
  })
}

function waitForComplete(
  child: ChildProcess,
  evidencePath: string,
  stdout: string[],
  stderr: string[],
  timeoutMs: number,
): Promise<HarnessStatus> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      child.kill()
      reject(new Error(`Native window harness exceeded ${String(timeoutMs)}ms.`))
    }, timeoutMs)
    child.once('error', (error) => {
      clearTimeout(timer)
      reject(error)
    })
    child.once('exit', (code) => {
      clearTimeout(timer)
      const evidence = parseEvidence(evidencePath)
      if (code !== 0 || text(evidence.status).toLowerCase() !== 'passed') {
        reject(new Error(
          [
            `Native window harness failed with code ${String(code)}.`,
            text(evidence.message),
            stderr.join('').trim(),
            stdout.join('').trim(),
          ].filter(Boolean).join(' '),
        ))
        return
      }
      resolve(evidence)
    })
  })
}

function startHarness(
  mode: 'OpenDialog' | 'SaveDialog' | 'CancelDialog' | 'WaitCloseExplorer',
  options: {
    evidenceName: string
    targetPath?: string
    expectedDirectory?: string
    timeoutSeconds?: number
  },
): NativeWindowHarnessHandle {
  const environment = requireReleaseProofEnvironment()
  const evidencePath = releaseProofEvidencePath(options.evidenceName)
  const isolationRoot = environment.isolationRoot
  const candidate = options.targetPath || options.expectedDirectory || ''
  if (!candidate || !path.isAbsolute(candidate) || !isPathInside(candidate, isolationRoot)) {
    throw new Error(`Native window harness target escaped the isolated profile: ${candidate}`)
  }
  if (fs.existsSync(evidencePath)) fs.rmSync(evidencePath)

  const scriptPath = path.resolve(repoRoot, 'scripts', 'desktop_native_window_harness.ps1')
  const args = [
    '-NoProfile',
    '-NonInteractive',
    '-ExecutionPolicy',
    'Bypass',
    '-File',
    scriptPath,
    '-Mode',
    mode,
    '-EvidencePath',
    evidencePath,
    '-TimeoutSeconds',
    String(options.timeoutSeconds ?? 30),
  ]
  if (options.targetPath) args.push('-TargetPath', path.resolve(options.targetPath))
  if (options.expectedDirectory) {
    args.push('-ExpectedDirectory', path.resolve(options.expectedDirectory))
  }

  const child = spawn('powershell.exe', args, {
    cwd: environment.isolationRoot,
    env: { ...process.env },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  })
  const stdout: string[] = []
  const stderr: string[] = []
  child.stdout?.on('data', (chunk) => stdout.push(String(chunk)))
  child.stderr?.on('data', (chunk) => stderr.push(String(chunk)))
  const completion = waitForComplete(
    child,
    evidencePath,
    stdout,
    stderr,
    (options.timeoutSeconds ?? 30) * 1_000 + 5_000,
  )
  // The caller intentionally waits for the helper's "ready" evidence before
  // triggering the native window. Attach a handler immediately so an early
  // helper failure cannot become an unhandled rejection during that interval;
  // complete() still returns the original rejecting promise to the spec.
  void completion.catch(() => undefined)
  return {
    ready: () => waitForReady(child, evidencePath, 10_000),
    complete: () => completion,
  }
}

export function startOpenDialogHarness(options: {
  evidenceName: string
  targetPath: string
}): NativeWindowHarnessHandle {
  return startHarness('OpenDialog', options)
}

export function startSaveDialogHarness(options: {
  action: 'save' | 'cancel'
  evidenceName: string
  targetPath: string
}): NativeWindowHarnessHandle {
  return startHarness(
    options.action === 'save' ? 'SaveDialog' : 'CancelDialog',
    {
      evidenceName: options.evidenceName,
      targetPath: options.targetPath,
    },
  )
}

export function startExplorerCloseHarness(options: {
  evidenceName: string
  expectedDirectory: string
}): NativeWindowHarnessHandle {
  return startHarness('WaitCloseExplorer', options)
}
