import { createHash } from 'node:crypto'
import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import {
  isolatedAppDataPath,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { invokeNative } from '../support/nativeIpc.js'
import { startSaveDialogHarness } from '../support/nativeWindowHarness.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type RunArtifactSummary = {
  path?: unknown
  has_validation?: unknown
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizedPath(value: string): string {
  const full = path.resolve(value)
  return process.platform === 'win32' ? full.toLowerCase() : full
}

function sha256(filePath: string): string {
  return createHash('sha256').update(fs.readFileSync(filePath)).digest('hex')
}

async function selectValueIfDisplayed(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  if (!(await field.isExisting()) || !(await field.isDisplayed())) return
  await field.selectByAttribute('value', value)
}

async function selectValueRequired(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await field.selectByAttribute('value', value)
}

async function setInput(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await browser.execute((targetSelector: string, nextValue: string) => {
    const page = globalThis as unknown as {
      document: {
        querySelector: (value: string) => {
          value: string
          dispatchEvent: (event: unknown) => boolean
        } | null
      }
      Event: new (type: string, init: { bubbles: boolean }) => unknown
    }
    const element = page.document.querySelector(targetSelector)
    if (!element) throw new Error(`Expected an input for ${targetSelector}.`)
    element.value = nextValue
    element.dispatchEvent(new page.Event('input', { bubbles: true }))
    element.dispatchEvent(new page.Event('change', { bubbles: true }))
  }, selector, value)
  await browser.waitUntil(
    async () => text(await field.getValue()) === value,
    {
      timeout: 3_000,
      interval: 50,
      timeoutMsg: `${selector} did not retain ${JSON.stringify(value)}.`,
    },
  )
}

async function setCheckbox(selector: string, checked: boolean): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  if ((await field.isSelected()) !== checked) await field.click()
}

async function loadSampleAndOpenOptimize(): Promise<void> {
  await waitForShellReady()
  await openWorkspace('workflow')
  const conventionalMode = await $(nativeWorkflowSelectors.conventionalMode)
  await conventionalMode.waitForDisplayed({ timeout: 10_000 })
  if ((await conventionalMode.getAttribute('aria-selected')) !== 'true') {
    await conventionalMode.click()
  }
  await (await $(nativeWorkflowSelectors.dataStep)).click()
  const sample = await $(nativeWorkflowSelectors.sampleDatasetButton)
  await sample.waitForClickable({ timeout: 15_000 })
  await sample.click()
  await (await $(nativeWorkflowSelectors.clearDatasetButton))
    .waitForEnabled({ timeout: 20_000 })
  await (await $(nativeWorkflowSelectors.validationStep)).click()
  await setCheckbox(nativeWorkflowSelectors.validationAutoRun, false)
  await setCheckbox(nativeWorkflowSelectors.validationAutoExportXlsx, false)
  await setCheckbox(nativeWorkflowSelectors.validationAutoRevealReport, false)
  await (await $(nativeWorkflowSelectors.optimizeStep)).click()
  await selectValueRequired(nativeWorkflowSelectors.strategyKind, 'DoubleMa')
  await selectValueRequired(nativeWorkflowSelectors.backend, 'CpuOnly')
  await selectValueRequired(nativeWorkflowSelectors.mode, 'Grid')
}

async function setRanges(
  fastStart: number,
  fastEnd: number,
  slowStart: number,
  slowEnd: number,
): Promise<void> {
  await setInput(nativeWorkflowSelectors.fastStart, String(fastStart))
  await setInput(nativeWorkflowSelectors.fastEnd, String(fastEnd))
  await setInput(nativeWorkflowSelectors.fastStep, '1')
  await setInput(nativeWorkflowSelectors.slowStart, String(slowStart))
  await setInput(nativeWorkflowSelectors.slowEnd, String(slowEnd))
  await setInput(nativeWorkflowSelectors.slowStep, '1')
}

async function openAdvanced(): Promise<void> {
  const tab = await $(nativeWorkflowSelectors.advancedControlsTab)
  await tab.waitForClickable({ timeout: 10_000 })
  await tab.click()
}

async function openOptimizeControls(): Promise<void> {
  const tab = await $('#controlsTabBar [data-tab="opt"]')
  await tab.waitForClickable({ timeout: 10_000 })
  await tab.click()
}

async function chooseExportPath(
  targetPath: string,
  evidenceName: string,
  action: 'save' | 'cancel',
): Promise<void> {
  const dialog = startSaveDialogHarness({
    action,
    evidenceName,
    targetPath,
  })
  await dialog.ready()
  const browse = await $(nativeWorkflowSelectors.exportAllCsvBrowse)
  await browse.waitForEnabled({ timeout: 10_000 })
  await browse.click()
  await dialog.complete()
}

async function optimizationStatus(): Promise<string> {
  const status = await $('.optPanelVueRoot [role="status"]')
  return text(await status.getText())
}

async function waitForOptimizationFailure(): Promise<string> {
  const runButton = await $(nativeWorkflowSelectors.runOptimization)
  await browser.waitUntil(
    async () => {
      const status = (await optimizationStatus()).toLowerCase()
      return await runButton.isEnabled() && status.includes('optimization failed')
    },
    {
      timeout: 60_000,
      interval: 100,
      timeoutMsg: 'The unwritable export did not return to an actionable failure state.',
    },
  )
  return optimizationStatus()
}

async function waitForNewRunArtifact(existingPaths: ReadonlySet<string>): Promise<string> {
  let artifactPath = ''
  await browser.waitUntil(
    async () => {
      const rows = await invokeNative<RunArtifactSummary[]>('list_run_artifacts')
      artifactPath = rows
        .map((row) => text(row.path))
        .find((candidate) => candidate && !existingPaths.has(candidate)) || ''
      return artifactPath.length > 0
    },
    {
      timeout: 60_000,
      interval: 250,
      timeoutMsg: 'The post-recovery run did not persist a new artifact.',
    },
  )
  return artifactPath
}

describe('desktop exact-package recovery proof', () => {
  it('recovers from dialog cancellation, write failure, run cancellation, and process restart', async () => {
    const environment = requireReleaseProofEnvironment()
    const validated = (await invokeNative<RunArtifactSummary[]>('list_run_artifacts'))
      .find((row) => Boolean(row.has_validation) && text(row.path))
    if (!validated) {
      throw new Error('Recovery proof requires the durable validated artifact from the native workflow.')
    }
    const protectedArtifactPath = text(validated.path)
    const protectedArtifactHash = sha256(protectedArtifactPath)

    await loadSampleAndOpenOptimize()
    await setRanges(3, 3, 8, 8)
    await openAdvanced()
    await setInput('#vueHeatmapBins', '0')
    await setInput('#vueTopK', '1')
    await selectValueIfDisplayed('#vueIncludeAll', 'false')

    const cancelledTarget = isolatedAppDataPath('exports', 'cancelled-selection.csv')
    const pathBeforeCancel = text(
      await (await $(nativeWorkflowSelectors.exportAllCsvPath)).getValue(),
    )
    await chooseExportPath(
      cancelledTarget,
      'recovery-cancel-save-dialog.json',
      'cancel',
    )
    await browser.waitUntil(
      async () => (await $('.advancedPanelVueRoot .status')).getText()
        .then((value) => value.toLowerCase().includes('cancelled')),
      {
        timeout: 10_000,
        interval: 100,
        timeoutMsg: 'Save-dialog cancellation did not produce an actionable status.',
      },
    )
    expect(text(await (await $(nativeWorkflowSelectors.exportAllCsvPath)).getValue()))
      .toBe(pathBeforeCancel)

    const blockedPath = isolatedAppDataPath('exports', 'blocked-destination.csv')
    await chooseExportPath(
      blockedPath,
      'recovery-blocked-save-dialog.json',
      'save',
    )
    await browser.waitUntil(
      async () => normalizedPath(text(
        await (await $(nativeWorkflowSelectors.exportAllCsvPath)).getValue(),
      )) === normalizedPath(blockedPath),
      {
        timeout: 10_000,
        interval: 100,
        timeoutMsg: 'The blocked export path was not retained after the native dialog.',
      },
    )
    if (fs.existsSync(blockedPath)) fs.rmSync(blockedPath, { recursive: true, force: true })
    fs.mkdirSync(blockedPath, { recursive: true })
    await openOptimizeControls()
    const runButton = await $(nativeWorkflowSelectors.runOptimization)
    await runButton.waitForEnabled({ timeout: 10_000 })
    await runButton.click()
    const failureStatus = await waitForOptimizationFailure()
    expect(failureStatus.toLowerCase()).toMatch(/failed|directory|export|write/)
    expect(sha256(protectedArtifactPath)).toBe(protectedArtifactHash)
    fs.rmSync(blockedPath, { recursive: true, force: true })

    await openAdvanced()
    const recoveredExportPath = isolatedAppDataPath('exports', 'recovered-export.csv')
    await chooseExportPath(
      recoveredExportPath,
      'recovery-success-save-dialog.json',
      'save',
    )
    await openOptimizeControls()
    await runButton.waitForEnabled({ timeout: 10_000 })
    await runButton.click()
    await browser.waitUntil(
      async () => fs.existsSync(recoveredExportPath)
        && fs.statSync(recoveredExportPath).isFile()
        && fs.statSync(recoveredExportPath).size > 0,
      {
        timeout: 60_000,
        interval: 200,
        timeoutMsg: 'A valid export did not succeed after the blocked destination failure.',
      },
    )
    await runButton.waitForEnabled({ timeout: 10_000 })
    expect(sha256(protectedArtifactPath)).toBe(protectedArtifactHash)

    // Clear the now-consumed export path before exercising cancellation.
    await openAdvanced()
    await setInput(nativeWorkflowSelectors.exportAllCsvPath, '')
    await openOptimizeControls()
    await setRanges(1, 600, 601, 1_200)
    await runButton.waitForEnabled({ timeout: 10_000 })
    await runButton.click()
    const cancelButton = await $(nativeWorkflowSelectors.cancelOptimization)
    await browser.waitUntil(
      async () => {
        const label = text(await cancelButton.getText()).toLowerCase()
        return await cancelButton.isEnabled() && label.includes('stop')
      },
      {
        timeout: 10_000,
        interval: 50,
        timeoutMsg: 'The bounded cancellation run never exposed Stop run.',
      },
    )
    await cancelButton.click()
    await browser.waitUntil(
      async () => await runButton.isEnabled()
        && (await optimizationStatus()).toLowerCase().includes('cancelled'),
      {
        timeout: 30_000,
        interval: 100,
        timeoutMsg: 'Cancellation did not restore the optimization controls.',
      },
    )
    expect(sha256(protectedArtifactPath)).toBe(protectedArtifactHash)

    // Close the exact product while a bounded CPU request is active, then prove
    // a cold foreground session can load data and complete a fresh native run.
    await setRanges(1, 700, 701, 1_400)
    await runButton.waitForEnabled({ timeout: 10_000 })
    await runButton.click()
    await browser.waitUntil(
      async () => {
        const label = text(await cancelButton.getText()).toLowerCase()
        return await cancelButton.isEnabled() && label.includes('stop')
      },
      {
        timeout: 10_000,
        interval: 50,
        timeoutMsg: 'The shutdown recovery run never became active.',
      },
    )
    await browser.reloadSession()
    await waitForShellReady()
    expect(sha256(protectedArtifactPath)).toBe(protectedArtifactHash)

    const beforeRecoveryRows = await invokeNative<RunArtifactSummary[]>('list_run_artifacts')
    const beforeRecoveryPaths = new Set(
      beforeRecoveryRows.map((row) => text(row.path)).filter(Boolean),
    )
    await loadSampleAndOpenOptimize()
    await setRanges(3, 3, 8, 8)
    const recoveredRunButton = await $(nativeWorkflowSelectors.runOptimization)
    await recoveredRunButton.waitForEnabled({ timeout: 10_000 })
    await recoveredRunButton.click()
    const recoveryArtifactPath = await waitForNewRunArtifact(beforeRecoveryPaths)
    await recoveredRunButton.waitForEnabled({ timeout: 10_000 })
    expect(fs.existsSync(recoveryArtifactPath)).toBe(true)
    expect(sha256(protectedArtifactPath)).toBe(protectedArtifactHash)

    writeReleaseProofEvidence('recovery-proof.json', {
      contract: 'vectorgrid-exact-package-recovery-v1',
      schemaVersion: 1,
      exactBinary: environment.desktopBinary,
      protectedArtifactPath,
      protectedArtifactSha256: protectedArtifactHash,
      cancelledDialogPreservedPath: true,
      blockedDestinationFailedClosed: true,
      subsequentExportSucceeded: true,
      cancellationRecovered: true,
      coldRestartRecovered: true,
      recoveryArtifactPath,
    })
  })
})
