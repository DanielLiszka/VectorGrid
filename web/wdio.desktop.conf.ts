import { spawn, type ChildProcess } from 'node:child_process'
import fs from 'node:fs'
import net from 'node:net'
import path from 'node:path'
import process from 'node:process'
import { desktopAppBinaryPath, ensureDir, webRoot } from './e2e/desktop/support/appPaths.js'
import {
  appendLogLine,
  createDesktopArtifactContext,
  failureScreenshotPath,
  writeTextFile,
} from './e2e/desktop/support/artifacts.js'
import { buildDesktopSmokeApp } from './scripts/desktop/build-smoke-app.ts'
import { verifyDesktopSmokePrereqs } from './scripts/desktop/verify-prereqs.ts'

const artifactContext = createDesktopArtifactContext()
const outputDir = ensureDir(path.resolve(artifactContext.logsDir, 'wdio'))
let tauriDriverProcess: ChildProcess | null = null
let tauriDriverLogStream: fs.WriteStream | null = null
let previewServerProcess: ChildProcess | null = null
let previewServerLogStream: fs.WriteStream | null = null
let shutdownRegistered = false
let tauriDriverExpectedExit = false
let previewServerExpectedExit = false
let tauriDriverPath = ''
let edgeDriverPath = ''
const tauriDriverPort = 4444
const previewServerPort = 1420
const vitePreviewEntrypoint = path.resolve(webRoot, 'node_modules', 'vite', 'bin', 'vite.js')

type TauriDesktopCapability = WebdriverIO.Capabilities & {
  'tauri:options': {
    application: string
  }
}

function closeTauriDriver(): void {
  tauriDriverExpectedExit = true
  tauriDriverLogStream?.end()
  tauriDriverLogStream = null
  if (!tauriDriverProcess) return
  tauriDriverProcess.kill()
  tauriDriverProcess = null
}

function closePreviewServer(): void {
  previewServerExpectedExit = true
  previewServerLogStream?.end()
  previewServerLogStream = null
  if (!previewServerProcess) return
  previewServerProcess.kill()
  previewServerProcess = null
}

function registerShutdown(): void {
  if (shutdownRegistered) return
  shutdownRegistered = true
  const cleanup = () => {
    closeTauriDriver()
    closePreviewServer()
  }
  process.on('exit', cleanup)
  process.on('SIGINT', cleanup)
  process.on('SIGTERM', cleanup)
  process.on('SIGHUP', cleanup)
  process.on('SIGBREAK', cleanup)
}

function waitForPortReady(port: number, timeoutMs = 15_000): Promise<void> {
  const startedAt = Date.now()
  return new Promise((resolve, reject) => {
    const tryConnect = () => {
      if (Date.now() - startedAt > timeoutMs) {
        reject(new Error(`Expected a local server on port ${String(port)} within ${String(timeoutMs)}ms.`))
        return
      }
      const socket = net.createConnection({ host: '127.0.0.1', port })
      socket.once('connect', () => {
        socket.destroy()
        resolve()
      })
      socket.once('error', () => {
        socket.destroy()
        setTimeout(tryConnect, 200)
      })
    }
    tryConnect()
  })
}

async function startPreviewServer(): Promise<void> {
  previewServerExpectedExit = false
  previewServerLogStream = fs.createWriteStream(artifactContext.previewLogPath, { flags: 'a' })
  appendLogLine(artifactContext.previewLogPath, `Starting preview server from ${webRoot}`)
  previewServerProcess = spawn(
    process.execPath,
    [vitePreviewEntrypoint, 'preview', '--host', '127.0.0.1', '--port', String(previewServerPort), '--strictPort'],
    {
      cwd: webRoot,
      env: {
        ...process.env,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  )
  const previewProcess = previewServerProcess
  if (!previewProcess?.stdout || !previewProcess?.stderr) {
    throw new Error('preview server failed to expose stdout/stderr streams.')
  }
  previewProcess.stdout.on('data', (chunk) => {
    previewServerLogStream?.write(chunk)
    process.stdout.write(chunk)
  })
  previewProcess.stderr.on('data', (chunk) => {
    previewServerLogStream?.write(chunk)
    process.stderr.write(chunk)
  })
  previewProcess.on('error', (error) => {
    appendLogLine(artifactContext.previewLogPath, `preview server error: ${String(error)}`)
  })
  previewProcess.on('exit', (code) => {
    previewServerLogStream?.end()
    previewServerLogStream = null
    if (!previewServerExpectedExit) {
      appendLogLine(artifactContext.previewLogPath, `preview server exited unexpectedly with code ${String(code)}`)
    }
  })
  await waitForPortReady(previewServerPort, 20_000)
}

async function startTauriDriver(): Promise<void> {
  registerShutdown()
  tauriDriverExpectedExit = false
  tauriDriverLogStream = fs.createWriteStream(artifactContext.driverLogPath, { flags: 'a' })
  appendLogLine(artifactContext.driverLogPath, `Starting tauri-driver from ${tauriDriverPath}`)
  appendLogLine(artifactContext.driverLogPath, `Resolved EdgeDriver ${edgeDriverPath}`)
  const nextPath = [path.dirname(edgeDriverPath), process.env.PATH || ''].filter(Boolean).join(path.delimiter)
  tauriDriverProcess = spawn(tauriDriverPath, [], {
    env: {
      ...process.env,
      PATH: nextPath,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  const driverProcess = tauriDriverProcess
  if (!driverProcess?.stdout || !driverProcess?.stderr) {
    throw new Error('tauri-driver failed to expose stdout/stderr streams.')
  }
  driverProcess.stdout.on('data', (chunk) => {
    tauriDriverLogStream?.write(chunk)
    process.stdout.write(chunk)
  })
  driverProcess.stderr.on('data', (chunk) => {
    tauriDriverLogStream?.write(chunk)
    process.stderr.write(chunk)
  })
  driverProcess.on('error', (error) => {
    appendLogLine(artifactContext.driverLogPath, `tauri-driver error: ${String(error)}`)
  })
  driverProcess.on('exit', (code) => {
    tauriDriverLogStream?.end()
    tauriDriverLogStream = null
    if (!tauriDriverExpectedExit) {
      appendLogLine(artifactContext.driverLogPath, `tauri-driver exited unexpectedly with code ${String(code)}`)
    }
  })
  await waitForPortReady(tauriDriverPort)
}

export const config: WebdriverIO.Config = {
  hostname: '127.0.0.1',
  port: tauriDriverPort,
  specs: ['./e2e/desktop/specs/**/*.spec.ts'],
  maxInstances: 1,
  suites: {
    smoke: ['./e2e/desktop/specs/**/*.spec.ts'],
  },
  capabilities: [
    {
      maxInstances: 1,
      browserName: 'wry',
      'tauri:options': {
        application: desktopAppBinaryPath,
      },
    } as TauriDesktopCapability,
  ],
  logLevel: 'info',
  outputDir,
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 90_000,
  },
  async onPrepare() {
    process.env.VECTORGRID_DESKTOP_SMOKE_RUN_ID = artifactContext.runId
    const prereqs = verifyDesktopSmokePrereqs()
    tauriDriverPath = prereqs.tauriDriverPath
    edgeDriverPath = prereqs.edgeDriverPath
    writeTextFile(artifactContext.envSummaryPath, prereqs.summary)
    if (process.env.VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD !== '1') {
      buildDesktopSmokeApp()
    } else {
      appendLogLine(artifactContext.driverLogPath, 'Skipping smoke build because VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1.')
    }
    if (!fs.existsSync(desktopAppBinaryPath)) {
      throw new Error(`Expected desktop app binary at ${desktopAppBinaryPath} after build.`)
    }
    await startPreviewServer()
    await startTauriDriver()
  },
  afterTest: async function (test, _context, result) {
    if (!result.passed) {
      const screenshotPath = failureScreenshotPath(artifactContext, test.fullTitle || test.title || 'desktop-smoke-failure')
      await browser.saveScreenshot(screenshotPath)
    }
  },
  onComplete() {
    closeTauriDriver()
    closePreviewServer()
  },
}
