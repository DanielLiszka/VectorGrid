import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import {
  isReleaseProofRun,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type BrowserStorageEntry = [string, string]

interface RunArtifactProbe {
  kind?: unknown
  ts?: unknown
  data?: {
    csv_path?: unknown
  }
  validation?: {
    req?: unknown
    res?: {
      candidates?: unknown
    }
  }
}

const APP_IDENTIFIER = 'dev.vectoralpha.vectorgrid'
const RUN_ARTIFACT_PREFIX = 'vectorgrid_run_'

function normalizePath(value: unknown): string {
  return String(value || '').trim().replace(/\\/g, '/').toLowerCase()
}

function runsDirectory(): string {
  const localAppData = String(process.env.LOCALAPPDATA || '').trim()
  if (!localAppData) throw new Error('LOCALAPPDATA is required for safe native smoke artifact cleanup.')
  return path.resolve(localAppData, APP_IDENTIFIER, 'runs')
}

function runArtifactFiles(dir: string): Set<string> {
  if (!fs.existsSync(dir)) return new Set()
  return new Set(
    fs.readdirSync(dir)
      .filter((name) => name.startsWith(RUN_ARTIFACT_PREFIX) && name.endsWith('.json')),
  )
}

function createdSmokeArtifact(
  dir: string,
  filesBefore: Set<string>,
  samplePath: string,
  startedAt: number,
): string {
  return createdSmokeArtifacts(dir, filesBefore, samplePath, startedAt)[0] || ''
}

function createdSmokeArtifacts(
  dir: string,
  filesBefore: Set<string>,
  samplePath: string,
  startedAt: number,
): string[] {
  if (!fs.existsSync(dir)) return []
  const matches: string[] = []
  for (const name of fs.readdirSync(dir)) {
    if (filesBefore.has(name) || !name.startsWith(RUN_ARTIFACT_PREFIX) || !name.endsWith('.json')) continue
    const candidate = path.resolve(dir, name)
    if (path.dirname(candidate) !== path.resolve(dir)) continue
    try {
      const artifact = JSON.parse(fs.readFileSync(candidate, 'utf8')) as RunArtifactProbe
      const artifactTime = Date.parse(String(artifact.ts || ''))
      if (artifact.kind !== 'vectorgrid_run_artifact') continue
      if (!Number.isFinite(artifactTime) || artifactTime < startedAt - 5_000) continue
      if (normalizePath(artifact.data?.csv_path) !== normalizePath(samplePath)) continue
      matches.push(candidate)
    } catch {
      // Ignore files that are not the run artifact created by this smoke.
    }
  }
  return matches.sort((left, right) => fs.statSync(right).mtimeMs - fs.statSync(left).mtimeMs)
}

function hasPersistedValidation(candidate: string): boolean {
  if (!candidate || !fs.existsSync(candidate)) return false
  try {
    const artifact = JSON.parse(fs.readFileSync(candidate, 'utf8')) as RunArtifactProbe
    return Boolean(
      artifact.validation?.req
      && artifact.validation?.res
      && Array.isArray(artifact.validation.res.candidates)
      && artifact.validation.res.candidates.length > 0,
    )
  } catch {
    return false
  }
}

async function snapshotBrowserStorage(): Promise<BrowserStorageEntry[]> {
  const serialized = await browser.execute(() => {
    const storage = globalThis.localStorage
    const entries: BrowserStorageEntry[] = []
    for (let index = 0; index < storage.length; index += 1) {
      const key = storage.key(index)
      if (key != null) entries.push([key, storage.getItem(key) || ''])
    }
    return JSON.stringify(entries)
  }) as unknown as string
  return JSON.parse(serialized) as BrowserStorageEntry[]
}

async function restoreBrowserStorage(entries: BrowserStorageEntry[]): Promise<void> {
  await browser.execute((serialized: string) => {
    const savedEntries = JSON.parse(serialized) as BrowserStorageEntry[]
    const storage = globalThis.localStorage
    storage.clear()
    for (const [key, value] of savedEntries) storage.setItem(key, value)
  }, JSON.stringify(entries))
}

async function selectValue(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await field.selectByAttribute('value', value)
}

async function setValue(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await browser.execute((targetSelector: string, nextValue: string) => {
    const page = globalThis as unknown as {
      document: { querySelector: (value: string) => { value: string; dispatchEvent: (event: unknown) => boolean } | null }
      Event: new (type: string, init: { bubbles: boolean }) => unknown
    }
    const element = page.document.querySelector(targetSelector)
    if (!element) throw new Error(`Expected an input for ${targetSelector}.`)
    element.value = nextValue
    element.dispatchEvent(new page.Event('input', { bubbles: true }))
    element.dispatchEvent(new page.Event('change', { bubbles: true }))
  }, selector, value)
  await browser.waitUntil(
    async () => String(await field.getValue()) === value,
    {
      timeout: 3_000,
      interval: 50,
      timeoutMsg: `Input ${selector} did not retain the requested value ${JSON.stringify(value)}.`,
    },
  )
}

async function setCheckbox(selector: string, checked: boolean): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  if ((await field.isSelected()) !== checked) await field.click()
}

describe('desktop native data-to-result workflow', () => {
  it('completes validation and reloads the saved result after a cold app relaunch', async () => {
    await waitForShellReady()
    await openWorkspace('workflow')

    const storageBefore = await snapshotBrowserStorage()
    const artifactDir = runsDirectory()
    const artifactsBefore = runArtifactFiles(artifactDir)
    const startedAt = Date.now()
    let samplePath = ''
    let smokeArtifactPath = ''

    try {
      const conventionalMode = await $(nativeWorkflowSelectors.conventionalMode)
      await conventionalMode.waitForDisplayed({ timeout: 10_000 })
      if ((await conventionalMode.getAttribute('aria-selected')) !== 'true') await conventionalMode.click()

      await (await $(nativeWorkflowSelectors.dataStep)).click()
      const sampleButton = await $(nativeWorkflowSelectors.sampleDatasetButton)
      await sampleButton.waitForClickable({ timeout: 15_000 })
      await sampleButton.click()

      const clearDataset = await $(nativeWorkflowSelectors.clearDatasetButton)
      await clearDataset.waitForEnabled({ timeout: 20_000 })
      samplePath = String(await (await $(nativeWorkflowSelectors.csvPath)).getValue()).trim()
      expect(samplePath).not.toBe('')

      // Keep the native journey deterministic even when the user has persisted broader validation preferences.
      await (await $(nativeWorkflowSelectors.validationStep)).click()
      await setCheckbox(nativeWorkflowSelectors.validationAutoRun, false)
      await setCheckbox(nativeWorkflowSelectors.validationAutoExportXlsx, false)
      await setCheckbox(nativeWorkflowSelectors.validationAutoRevealReport, false)
      await setCheckbox(nativeWorkflowSelectors.validationWalkForward, false)
      await setCheckbox(nativeWorkflowSelectors.validationPerturbations, false)
      await setCheckbox(nativeWorkflowSelectors.validationMonteCarlo, false)
      await setCheckbox(nativeWorkflowSelectors.validationCscv, false)
      await setCheckbox(nativeWorkflowSelectors.validationSpa, false)
      await setCheckbox(nativeWorkflowSelectors.validationReoptWalkForward, false)
      await setValue(nativeWorkflowSelectors.validationCandidates, '1')

      await (await $(nativeWorkflowSelectors.optimizeStep)).click()
      await selectValue(nativeWorkflowSelectors.strategyKind, 'DoubleMa')
      await selectValue(nativeWorkflowSelectors.backend, 'CpuOnly')
      await selectValue(nativeWorkflowSelectors.mode, 'Grid')
      await setValue(nativeWorkflowSelectors.fastStart, '3')
      await setValue(nativeWorkflowSelectors.fastEnd, '4')
      await setValue(nativeWorkflowSelectors.fastStep, '1')
      await setValue(nativeWorkflowSelectors.slowStart, '8')
      await setValue(nativeWorkflowSelectors.slowEnd, '9')
      await setValue(nativeWorkflowSelectors.slowStep, '1')

      const runButton = await $(nativeWorkflowSelectors.runOptimization)
      await runButton.waitForEnabled({ timeout: 10_000 })
      await runButton.click()

      const topPanel = await $(nativeWorkflowSelectors.topResultsPanel)
      await topPanel.waitForDisplayed({ timeout: 45_000 })
      const topRow = await $(nativeWorkflowSelectors.firstTopResult)
      await topRow.waitForDisplayed({ timeout: 45_000 })
      await expect(topRow).toBeDisplayed()

      await browser.waitUntil(
        async () => {
          smokeArtifactPath = createdSmokeArtifact(artifactDir, artifactsBefore, samplePath, startedAt)
          return smokeArtifactPath.length > 0
        },
        {
          timeout: 10_000,
          interval: 200,
          timeoutMsg: 'The native optimization completed without a matching persisted smoke artifact.',
        },
      )

      await (await $(nativeWorkflowSelectors.validationStep)).click()
      await setValue(nativeWorkflowSelectors.validationMinPsr, '0')
      await setValue(nativeWorkflowSelectors.validationMinDsr, '0')
      await setValue(nativeWorkflowSelectors.validationMinCscvRank, '')
      await setValue(nativeWorkflowSelectors.validationMaxPbo, '')
      await setCheckbox(nativeWorkflowSelectors.validationCscv, false)
      const validationRun = await $(nativeWorkflowSelectors.validationRun)
      await validationRun.waitForEnabled({ timeout: 10_000 })
      await validationRun.click()

      const validationRow = await $(nativeWorkflowSelectors.firstValidationResult)
      await validationRow.waitForDisplayed({ timeout: 45_000 })
      await expect(validationRow).toBeDisplayed()
      await browser.waitUntil(
        async () => {
          const persistedPath = createdSmokeArtifacts(
            artifactDir,
            artifactsBefore,
            samplePath,
            startedAt,
          ).find(hasPersistedValidation)
          if (!persistedPath) return false
          smokeArtifactPath = persistedPath
          return true
        },
        {
          timeout: 10_000,
          interval: 200,
          timeoutMsg: 'The native validation completed without updating the persisted smoke artifact.',
        },
      )

      // A new WebDriver session closes the current desktop process and launches the configured
      // binary again. This ensures Library hydration comes from durable state, not live Pinia state.
      await browser.reloadSession()
      await waitForShellReady()
      await openWorkspace('library')
      const refreshSavedRuns = await $(nativeWorkflowSelectors.savedRunsRefresh)
      await refreshSavedRuns.waitForEnabled({ timeout: 10_000 })
      await refreshSavedRuns.click()
      await setValue(nativeWorkflowSelectors.savedRunsSearch, path.basename(smokeArtifactPath))
      await browser.pause(500)

      const savedRun = await $(nativeWorkflowSelectors.savedRunWithValidation)
      await savedRun.waitForDisplayed({ timeout: 15_000 })
      const savedRunText = await savedRun.getText()
      expect(savedRunText.toLowerCase()).toContain('validation')
      await savedRun.click()

      const savedRunDetail = await $(nativeWorkflowSelectors.savedRunDetail)
      await savedRunDetail.waitForDisplayed({ timeout: 15_000 })
      await expect(savedRunDetail).toHaveText(expect.stringContaining('Available'))

      const loadSavedRun = await $(nativeWorkflowSelectors.savedRunLoad)
      await loadSavedRun.waitForEnabled({ timeout: 10_000 })
      await loadSavedRun.click()
      await openWorkspace('workflow')
      await (await $(nativeWorkflowSelectors.dataStep)).click()
      const relaunchedSampleButton = await $(nativeWorkflowSelectors.sampleDatasetButton)
      await relaunchedSampleButton.waitForClickable({ timeout: 15_000 })
      await relaunchedSampleButton.click()
      await (await $(nativeWorkflowSelectors.clearDatasetButton)).waitForEnabled({ timeout: 20_000 })

      // Loading a dataset intentionally clears projected current-run state. Re-apply the durable
      // artifact once its required dataset is active, matching the explicit safe user workflow.
      await openWorkspace('library')
      const relaunchedRefreshSavedRuns = await $(nativeWorkflowSelectors.savedRunsRefresh)
      await relaunchedRefreshSavedRuns.waitForEnabled({ timeout: 10_000 })
      await relaunchedRefreshSavedRuns.click()
      await setValue(nativeWorkflowSelectors.savedRunsSearch, path.basename(smokeArtifactPath))
      const compatibleSavedRun = await $(nativeWorkflowSelectors.savedRunWithValidation)
      await compatibleSavedRun.waitForDisplayed({ timeout: 15_000 })
      await compatibleSavedRun.click()
      const compatibleLoadSavedRun = await $(nativeWorkflowSelectors.savedRunLoad)
      await compatibleLoadSavedRun.waitForEnabled({ timeout: 10_000 })
      await compatibleLoadSavedRun.click()

      await openWorkspace('workflow')
      await (await $(nativeWorkflowSelectors.validationStep)).click()
      await (await $(nativeWorkflowSelectors.firstValidationResult)).waitForDisplayed({ timeout: 15_000 })
      if (isReleaseProofRun()) {
        writeReleaseProofEvidence('native-data-flow-proof.json', {
          contract: 'vectorgrid-exact-package-native-data-flow-v1',
          sample_path: samplePath,
          run_artifact_path: smokeArtifactPath,
          run_artifact_size_bytes: fs.statSync(smokeArtifactPath).size,
          persisted_validation: hasPersistedValidation(smokeArtifactPath),
          cold_process_relaunch: true,
          library_hydration: true,
        })
      }
    } finally {
      const cleanupPaths = samplePath
        ? createdSmokeArtifacts(artifactDir, artifactsBefore, samplePath, startedAt)
        : []
      // A sentinel-owned release-proof profile is removed wholesale by the WDIO
      // teardown. Keep its validated artifact so later exact-package export and
      // recovery specs can prove durable hand-off without touching a real profile.
      if (!process.env.VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT) {
        for (const cleanupPath of cleanupPaths) {
          if (fs.existsSync(cleanupPath)) {
            // Each path passed the prefix, parent, timestamp, kind, and sample-path checks above.
            fs.rmSync(cleanupPath)
          }
        }
      }
      try {
        const clearDataset = await $(nativeWorkflowSelectors.clearDatasetButton)
        if (await clearDataset.isEnabled().catch(() => false)) await clearDataset.click().catch(() => undefined)
        await restoreBrowserStorage(storageBefore)
      } catch {
        // Filesystem cleanup above remains authoritative if session startup/reload itself failed.
      }
    }
  })
})
