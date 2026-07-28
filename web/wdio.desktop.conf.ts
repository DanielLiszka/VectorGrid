import { request as httpRequest } from 'node:http'
import { spawn, type ChildProcess } from 'node:child_process'
import fs from 'node:fs'
import net from 'node:net'
import path from 'node:path'
import process from 'node:process'
import {
  desktopAppBinaryPath,
  desktopSmokeAppUrl,
  desktopSmokeDisablePreview,
  ensureDir,
  webRoot,
} from './e2e/desktop/support/appPaths.js'
import {
  appendLogLine,
  createDesktopArtifactContext,
  failureScreenshotPath,
  writeTextFile,
} from './e2e/desktop/support/artifacts.js'
import {
  activateDesktopProfileEnvironment,
  cleanupDesktopProfileIsolation,
  desktopDriverEnvironment,
  finalizeDesktopProfileIsolation,
  prepareDesktopProfileIsolation,
  type DesktopProfileIsolation,
  type DesktopProfileIsolationSummary,
  writeDesktopProfileIsolationBlockedEvidence,
} from './e2e/desktop/support/profileIsolation.js'
import { buildDesktopSmokeApp } from './scripts/desktop/build-smoke-app.ts'
import { verifyDesktopSmokePrereqs } from './scripts/desktop/verify-prereqs.ts'

process.env.VECTORGRID_DESKTOP_SMOKE_RUN_ID =
  String(process.env.VECTORGRID_DESKTOP_SMOKE_RUN_ID || '').trim()
  || new Date().toISOString().replace(/[:.]/g, '-')

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
let desktopProfileIsolation: DesktopProfileIsolation | null = null
let profileIsolationSummary: DesktopProfileIsolationSummary | null = null
const tauriDriverPort = 4444
const previewServerPort = 1420
const vitePreviewEntrypoint = path.resolve(webRoot, 'node_modules', 'vite', 'bin', 'vite.js')
const titlebarNativeControlsSpec = './e2e/desktop/specs/desktop-titlebar-native-controls.spec.ts'
const stableSmokeSpecs = [
  './e2e/desktop/specs/desktop-shell-readiness.spec.ts',
  './e2e/desktop/specs/desktop-workspace-navigation.spec.ts',
  './e2e/desktop/specs/desktop-command-palette.spec.ts',
  './e2e/desktop/specs/desktop-task-center.spec.ts',
  './e2e/desktop/specs/desktop-settings-diagnostics.spec.ts',
  titlebarNativeControlsSpec,
  './e2e/desktop/specs/desktop-app-close.spec.ts',
]
const geometrySmokeSpec = './e2e/desktop/specs/desktop-window-geometry.spec.ts'
const launchBoundsSmokeSpec = './e2e/desktop/specs/desktop-window-launch-bounds.spec.ts'
const nativeWorkflowSmokeSpec = './e2e/desktop/specs/desktop-native-data-flow.spec.ts'
const isolatedCleanStartSpec = './e2e/desktop/specs/desktop-isolated-clean-start.spec.ts'
const nativeRouteMatrixSpec = './e2e/desktop/specs/desktop-native-route-matrix.spec.ts'

type ReleaseProofJourneyEntry = {
  journeyId: string
  spec: string
  evidenceFile: string
  evidenceContract: string
}

function loadReleaseProofJourneyManifest(): ReleaseProofJourneyEntry[] {
  const manifestPath = path.resolve(webRoot, '..', 'scripts', 'release_proof_journeys.json')
  const manifestStat = fs.lstatSync(manifestPath)
  if (!manifestStat.isFile() || manifestStat.isSymbolicLink()) {
    throw new Error(`Release-proof journey manifest must be an ordinary file: ${manifestPath}`)
  }
  const raw = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as {
    schema_version?: unknown
    desktop_suite?: unknown
    journeys?: unknown
  }
  if (
    raw.schema_version !== 1
    || raw.desktop_suite !== 'release-proof'
    || !Array.isArray(raw.journeys)
    || raw.journeys.length === 0
  ) {
    throw new Error(`Release-proof journey manifest has an unsupported schema: ${manifestPath}`)
  }
  const seenIds = new Set<string>()
  const seenSpecs = new Set<string>()
  const seenEvidenceFiles = new Set<string>()
  const seenEvidenceContracts = new Set<string>()
  return raw.journeys.map((value, index) => {
    if (!value || typeof value !== 'object' || Array.isArray(value)) {
      throw new Error(`Release-proof journey ${String(index)} must be an object.`)
    }
    const record = value as Record<string, unknown>
    const journeyId = String(record.id ?? '').trim()
    const repoSpec = String(record.spec ?? '').trim().replace(/\\/g, '/')
    const evidenceFile = String(record.evidence_file ?? '').trim()
    const evidenceContract = String(record.evidence_contract ?? '').trim()
    if (
      !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(journeyId)
      || !repoSpec.startsWith('web/e2e/desktop/specs/')
      || !repoSpec.endsWith('.spec.ts')
      || repoSpec.split('/').includes('..')
      || evidenceFile !== `${journeyId}-proof.json`
      || !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(evidenceContract)
      || seenIds.has(journeyId)
      || seenSpecs.has(repoSpec)
      || seenEvidenceFiles.has(evidenceFile)
      || seenEvidenceContracts.has(evidenceContract)
    ) {
      throw new Error(`Release-proof journey ${String(index)} is unsafe or duplicated.`)
    }
    const absoluteSpec = path.resolve(webRoot, repoSpec.slice('web/'.length))
    const specStat = fs.lstatSync(absoluteSpec)
    if (!specStat.isFile() || specStat.isSymbolicLink()) {
      throw new Error(`Release-proof journey spec must be an ordinary file: ${absoluteSpec}`)
    }
    seenIds.add(journeyId)
    seenSpecs.add(repoSpec)
    seenEvidenceFiles.add(evidenceFile)
    seenEvidenceContracts.add(evidenceContract)
    return {
      journeyId,
      spec: `./${repoSpec.slice('web/'.length)}`,
      evidenceFile,
      evidenceContract,
    }
  })
}

export const releaseProofJourneyManifest = loadReleaseProofJourneyManifest()
const releaseProofSpecs = releaseProofJourneyManifest.map(({ spec }) => spec)
const releaseProofEvidenceFiles = releaseProofJourneyManifest.map(
  ({ evidenceFile }) => evidenceFile,
)
const expandedSmokeSpecs = [
  ...stableSmokeSpecs,
  geometrySmokeSpec,
  launchBoundsSmokeSpec,
  nativeWorkflowSmokeSpec,
]

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

function appendEnvironmentSummary(prereqSummary: string): void {
  const extraLines = [
    `desktopBinary: ${desktopAppBinaryPath}`,
    `skipBuild: ${process.env.VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD === '1'}`,
    `disablePreview: ${desktopSmokeDisablePreview}`,
    `appUrl: ${desktopSmokeAppUrl}`,
  ]
  writeTextFile(artifactContext.envSummaryPath, `${prereqSummary}\n${extraLines.join('\n')}\n`)
}

function selectedSuiteName(): string {
  const suiteIndex = process.argv.findIndex((arg) => arg === '--suite')
  if (suiteIndex >= 0) return String(process.argv[suiteIndex + 1] || '').trim()
  const inlineSuite = process.argv.find((arg) => arg.startsWith('--suite='))
  if (inlineSuite) return inlineSuite.slice('--suite='.length).trim()
  return 'specs'
}

function selectedSuiteSpecCount(suiteName: string): number {
  if (suiteName === 'smoke') return stableSmokeSpecs.length
  if (suiteName === 'geometry') return 3
  if (suiteName === 'workflow') return 1
  if (suiteName === 'clean-start') return 1
  if (suiteName === 'route-matrix') return 1
  if (suiteName === 'release-proof') return releaseProofSpecs.length
  if (suiteName === 'smoke-expanded' || suiteName === 'all') return expandedSmokeSpecs.length
  return stableSmokeSpecs.length
}

function selectedSuiteRequiresProfileIsolation(): boolean {
  return ['clean-start', 'route-matrix', 'release-proof'].includes(selectedSuiteName())
}

function writeDesktopSmokeSummary(exitCode: number): void {
  const suiteName = selectedSuiteName()
  const summary = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    status: exitCode === 0 ? 'passed' : 'failed',
    exitCode,
    suite: suiteName,
    runId: artifactContext.runId,
    specCount: selectedSuiteSpecCount(suiteName),
    profileIsolation: profileIsolationSummary,
    releaseProof: suiteName === 'release-proof'
      ? {
          contract: 'vectorgrid-release-proof-manifest-v1',
          journeys: releaseProofJourneyManifest,
          requiredEvidenceFiles: releaseProofEvidenceFiles,
        }
      : null,
    artifacts: {
      runDir: path.relative(path.resolve(webRoot, '..'), artifactContext.runDir).replace(/\\/g, '/'),
      logsDir: path.relative(path.resolve(webRoot, '..'), artifactContext.logsDir).replace(/\\/g, '/'),
      screenshotsDir: path.relative(path.resolve(webRoot, '..'), artifactContext.screenshotsDir).replace(/\\/g, '/'),
      environment: path.relative(path.resolve(webRoot, '..'), artifactContext.envSummaryPath).replace(/\\/g, '/'),
    },
  }
  const summaryDir = ensureDir(path.resolve(webRoot, '..', 'output', 'wdio'))
  const summaryPath = path.resolve(summaryDir, 'desktop-smoke-summary.json')
  const expandedSummaryPath = path.resolve(summaryDir, 'desktop-smoke-expanded-summary.json')
  writeTextFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`)
  if (suiteName === 'smoke-expanded') {
    writeTextFile(expandedSummaryPath, `${JSON.stringify(summary, null, 2)}\n`)
  } else {
    writeTextFile(expandedSummaryPath, `${JSON.stringify({
      schemaVersion: 1,
      generatedAt: summary.generatedAt,
      status: 'not_run',
      exitCode: null,
      suite: 'smoke-expanded',
      clearedBySuite: suiteName,
      runId: artifactContext.runId,
      specCount: expandedSmokeSpecs.length,
      artifacts: {},
    }, null, 2)}\n`)
  }
}

function registerShutdown(): void {
  if (shutdownRegistered) return
  shutdownRegistered = true
  const cleanup = () => {
    closeTauriDriver()
    closePreviewServer()
    cleanupDesktopProfileIsolation(desktopProfileIsolation)
    desktopProfileIsolation = null
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

function waitForWebDriverReady(port: number, timeoutMs = 20_000): Promise<void> {
  const startedAt = Date.now()
  return new Promise((resolve, reject) => {
    const tryStatus = () => {
      if (Date.now() - startedAt > timeoutMs) {
        reject(new Error(`Expected WebDriver status on port ${String(port)} within ${String(timeoutMs)}ms.`))
        return
      }
      const request = httpRequest(
        {
          host: '127.0.0.1',
          port,
          path: '/status',
          method: 'GET',
        },
        (response) => {
          let body = ''
          response.setEncoding('utf8')
          response.on('data', (chunk) => {
            body += chunk
          })
          response.on('end', () => {
            if (response.statusCode !== 200) {
              setTimeout(tryStatus, 250)
              return
            }
            try {
              const parsed = JSON.parse(body) as { value?: { ready?: boolean } }
              if (parsed.value?.ready === false) {
                setTimeout(tryStatus, 250)
                return
              }
            } catch {
              // Some drivers may not return a strict JSON body immediately; the 200 response is enough.
            }
            resolve()
          })
        },
      )
      request.on('error', () => {
        setTimeout(tryStatus, 250)
      })
      request.end()
    }
    tryStatus()
  })
}

function deleteWebDriverSession(port: number, sessionId: string): Promise<void> {
  return new Promise((resolve) => {
    const request = httpRequest(
      {
        host: '127.0.0.1',
        port,
        path: `/session/${sessionId}`,
        method: 'DELETE',
      },
      (response) => {
        response.resume()
        response.on('end', () => resolve())
      },
    )
    request.on('error', () => resolve())
    request.end()
  })
}

function waitForDesktopSessionReady(timeoutMs = 30_000): Promise<void> {
  const startedAt = Date.now()
  const payload = JSON.stringify({
    capabilities: {
      alwaysMatch: {
        browserName: 'wry',
        'tauri:options': {
          application: desktopAppBinaryPath,
        },
      },
      firstMatch: [{}],
    },
  })
  return new Promise((resolve, reject) => {
    const trySession = () => {
      if (Date.now() - startedAt > timeoutMs) {
        reject(new Error(`Expected WebDriver session creation within ${String(timeoutMs)}ms.`))
        return
      }
      const request = httpRequest(
        {
          host: '127.0.0.1',
          port: tauriDriverPort,
          path: '/session',
          method: 'POST',
          headers: {
            'content-type': 'application/json; charset=utf-8',
            'content-length': String(Buffer.byteLength(payload)),
          },
        },
        (response) => {
          let body = ''
          response.setEncoding('utf8')
          response.on('data', (chunk) => {
            body += chunk
          })
          response.on('end', async () => {
            if (response.statusCode !== 200) {
              setTimeout(trySession, 500)
              return
            }
            try {
              const parsed = JSON.parse(body) as {
                sessionId?: string
                value?: {
                  sessionId?: string
                }
              }
              const sessionId = parsed.value?.sessionId || parsed.sessionId
              if (!sessionId) {
                setTimeout(trySession, 500)
                return
              }
              appendLogLine(artifactContext.driverLogPath, `Warm-up session created: ${sessionId}`)
              await deleteWebDriverSession(tauriDriverPort, sessionId)
              appendLogLine(artifactContext.driverLogPath, `Warm-up session deleted: ${sessionId}`)
              resolve()
            } catch {
              setTimeout(trySession, 500)
            }
          })
        },
      )
      request.on('error', () => {
        setTimeout(trySession, 500)
      })
      request.write(payload)
      request.end()
    }
    trySession()
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
    cwd: desktopProfileIsolation?.userProfile,
    env: {
      ...desktopDriverEnvironment(desktopProfileIsolation),
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
  await waitForWebDriverReady(tauriDriverPort)
  await waitForDesktopSessionReady()
}

export const config: WebdriverIO.Config = {
  hostname: '127.0.0.1',
  port: tauriDriverPort,
  specs: stableSmokeSpecs,
  maxInstances: 1,
  connectionRetryTimeout: 15_000,
  connectionRetryCount: 1,
  specFileRetries: selectedSuiteName() === 'release-proof' ? 0 : 1,
  specFileRetriesDelay: 1,
  specFileRetriesDeferred: true,
  suites: {
    smoke: stableSmokeSpecs,
    geometry: [geometrySmokeSpec, titlebarNativeControlsSpec, launchBoundsSmokeSpec],
    workflow: [nativeWorkflowSmokeSpec],
    'clean-start': [isolatedCleanStartSpec],
    'route-matrix': [nativeRouteMatrixSpec],
    'release-proof': releaseProofSpecs,
    'smoke-expanded': expandedSmokeSpecs,
    all: expandedSmokeSpecs,
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
    timeout: ['release-proof', 'route-matrix'].includes(selectedSuiteName())
      ? 600_000
      : 90_000,
  },
  async onPrepare() {
    process.env.VECTORGRID_DESKTOP_SMOKE_RUN_ID = artifactContext.runId
    if (
      selectedSuiteName() === 'release-proof'
      && process.env.VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE !== '1'
    ) {
      throw new Error(
        'The release-proof suite requires VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE=1 from the exact-package wrapper.',
      )
    }
    try {
      desktopProfileIsolation = prepareDesktopProfileIsolation(artifactContext)
    } catch (error) {
      profileIsolationSummary = writeDesktopProfileIsolationBlockedEvidence(
        artifactContext,
        error,
      )
      throw error
    }
    if (selectedSuiteRequiresProfileIsolation() && !desktopProfileIsolation) {
      const error = new Error(
        `The ${selectedSuiteName()} suite requires a sentinel-owned isolated profile root.`,
      )
      profileIsolationSummary = writeDesktopProfileIsolationBlockedEvidence(
        artifactContext,
        error,
      )
      throw error
    }
    if (desktopProfileIsolation) registerShutdown()
    const prereqs = verifyDesktopSmokePrereqs()
    tauriDriverPath = prereqs.tauriDriverPath
    edgeDriverPath = prereqs.edgeDriverPath
    appendEnvironmentSummary(prereqs.summary)
    if (process.env.VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD !== '1') {
      buildDesktopSmokeApp()
    } else {
      appendLogLine(artifactContext.driverLogPath, 'Skipping smoke build because VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1.')
    }
    if (!fs.existsSync(desktopAppBinaryPath)) {
      throw new Error(`Expected desktop app binary at ${desktopAppBinaryPath} after build.`)
    }
    if (!desktopSmokeDisablePreview) {
      await startPreviewServer()
    } else {
      appendLogLine(
        artifactContext.driverLogPath,
        'Skipping preview bootstrap because VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW=1.',
      )
    }
    activateDesktopProfileEnvironment(desktopProfileIsolation)
    await startTauriDriver()
  },
  afterTest: async function (test, _context, result) {
    if (!result.passed) {
      const screenshotPath = failureScreenshotPath(artifactContext, test.fullTitle || test.title || 'desktop-smoke-failure')
      await browser.saveScreenshot(screenshotPath)
    }
  },
  async onComplete(exitCode) {
    closeTauriDriver()
    closePreviewServer()
    await new Promise((resolve) => setTimeout(resolve, 500))
    let finalExitCode = Number(exitCode || 0)
    let isolationError: unknown = null
    try {
      if (profileIsolationSummary?.status !== 'blocked') {
        profileIsolationSummary = finalizeDesktopProfileIsolation(desktopProfileIsolation)
      }
    } catch (error) {
      finalExitCode = 1
      isolationError = error
    } finally {
      desktopProfileIsolation = null
    }
    writeDesktopSmokeSummary(finalExitCode)
    if (isolationError) throw isolationError
  },
}
