import { createHash } from 'node:crypto'
import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import {
  isolatedAppDataPath,
  isPathInside,
  releaseProofEvidencePath,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { invokeNative } from '../support/nativeIpc.js'
import {
  startExplorerCloseHarness,
  startSaveDialogHarness,
} from '../support/nativeWindowHarness.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type RunArtifactSummary = {
  path?: unknown
  has_validation?: unknown
  strategy_kind?: unknown
}

type SaveReportResponse = {
  path?: unknown
  bytes?: unknown
  artifact_updated?: unknown
  artifact_update_warning?: unknown
}

type RunArtifact = {
  report_xlsx_path?: unknown
}

function text(value: unknown): string {
  return String(value ?? '').trim()
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

async function prepareOnePairExportRun(exportPath: string): Promise<void> {
  await waitForShellReady()
  await openWorkspace('workflow')
  const conventionalMode = await $(nativeWorkflowSelectors.conventionalMode)
  await conventionalMode.waitForDisplayed({ timeout: 10_000 })
  if ((await conventionalMode.getAttribute('aria-selected')) !== 'true') {
    await conventionalMode.click()
  }
  await (await $(nativeWorkflowSelectors.dataStep)).click()
  const sampleButton = await $(nativeWorkflowSelectors.sampleDatasetButton)
  await sampleButton.waitForClickable({ timeout: 15_000 })
  await sampleButton.click()
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
  await setInput(nativeWorkflowSelectors.fastStart, '3')
  await setInput(nativeWorkflowSelectors.fastEnd, '3')
  await setInput(nativeWorkflowSelectors.fastStep, '1')
  await setInput(nativeWorkflowSelectors.slowStart, '8')
  await setInput(nativeWorkflowSelectors.slowEnd, '8')
  await setInput(nativeWorkflowSelectors.slowStep, '1')

  const advancedTab = await $(nativeWorkflowSelectors.advancedControlsTab)
  await advancedTab.waitForClickable({ timeout: 10_000 })
  await advancedTab.click()
  await setInput('#vueHeatmapBins', '0')
  await setInput('#vueTopK', '1')
  await selectValueIfDisplayed('#vueIncludeAll', 'false')

  const saveDialog = startSaveDialogHarness({
    action: 'save',
    evidenceName: 'export-all-save-dialog.json',
    targetPath: exportPath,
  })
  await saveDialog.ready()
  const browse = await $(nativeWorkflowSelectors.exportAllCsvBrowse)
  await browse.waitForEnabled({ timeout: 10_000 })
  await browse.click()
  await saveDialog.complete()
  await browser.waitUntil(
    async () => text(await (await $(nativeWorkflowSelectors.exportAllCsvPath)).getValue()) === exportPath,
    {
      timeout: 10_000,
      interval: 100,
      timeoutMsg: 'The exact-package save dialog did not populate the export path.',
    },
  )
  const optimizeTab = await $('#controlsTabBar [data-tab="opt"]')
  await optimizeTab.waitForClickable({ timeout: 10_000 })
  await optimizeTab.click()
}

async function revealAndClose(filePath: string, evidenceName: string): Promise<void> {
  const explorer = startExplorerCloseHarness({
    evidenceName,
    expectedDirectory: path.dirname(filePath),
  })
  await explorer.ready()
  await invokeNative('reveal_path', { path: filePath })
  await explorer.complete()
}

describe('desktop exact-package export proof', () => {
  it('exports readable CSV/XLSX artifacts through native paths and contains every opened shell window', async () => {
    const environment = requireReleaseProofEnvironment()
    const exportPath = isolatedAppDataPath('exports', 'release-proof-all-pairs.csv')
    await prepareOnePairExportRun(exportPath)

    const runButton = await $(nativeWorkflowSelectors.runOptimization)
    await runButton.waitForEnabled({ timeout: 10_000 })
    await runButton.click()
    await browser.waitUntil(
      async () => fs.existsSync(exportPath) && fs.statSync(exportPath).isFile(),
      {
        timeout: 60_000,
        interval: 200,
        timeoutMsg: 'The exact-package optimization did not create its issued CSV export.',
      },
    )
    await runButton.waitForEnabled({ timeout: 10_000 })

    expect(isPathInside(exportPath, environment.isolationRoot)).toBe(true)
    const csvText = fs.readFileSync(exportPath, 'utf8')
    const csvLines = csvText
      .split(/\r?\n/g)
      .map((line) => line.trim())
      .filter(Boolean)
    expect(csvLines.length).toBeGreaterThanOrEqual(2)
    expect(csvLines[0]?.toLowerCase()).toContain('fast')
    expect(csvLines[0]?.toLowerCase()).toContain('slow')
    expect(csvLines[1]).toContain('3')
    expect(csvLines[1]).toContain('8')
    const csvEvidencePath = releaseProofEvidencePath('release-proof-all-pairs.csv')
    fs.copyFileSync(exportPath, csvEvidencePath)

    const summaries = await invokeNative<RunArtifactSummary[]>('list_run_artifacts')
    const validated = summaries.find((row) => Boolean(row.has_validation) && text(row.path))
    if (!validated) {
      throw new Error(
        'The release-proof export lane requires the validated artifact produced by the preceding native workflow spec.',
      )
    }
    const validatedArtifactPath = text(validated.path)
    const savedReport = await invokeNative<SaveReportResponse>(
      'save_report_xlsx_from_artifact',
      {
        path: validatedArtifactPath,
        max_rows: 1,
      },
    )
    const reportPath = text(savedReport.path)
    if (!reportPath || !fs.existsSync(reportPath) || !fs.statSync(reportPath).isFile()) {
      throw new Error(`The exact package returned a missing XLSX report: ${reportPath}`)
    }
    expect(isPathInside(reportPath, environment.isolationRoot)).toBe(true)
    expect(Boolean(savedReport.artifact_updated)).toBe(true)
    expect(text(savedReport.artifact_update_warning)).toBe('')
    expect(Number(savedReport.bytes)).toBe(fs.statSync(reportPath).size)

    const xlsxBytes = fs.readFileSync(reportPath)
    expect(xlsxBytes.subarray(0, 4).toString('hex')).toBe('504b0304')
    const centralDirectoryText = xlsxBytes.toString('latin1')
    expect(centralDirectoryText).toContain('[Content_Types].xml')
    expect(centralDirectoryText).toContain('xl/workbook.xml')
    const xlsxEvidencePath = releaseProofEvidencePath('release-proof-report.xlsx')
    fs.copyFileSync(reportPath, xlsxEvidencePath)

    const updatedArtifact = await invokeNative<RunArtifact>('load_run_artifact', {
      path: validatedArtifactPath,
    })
    expect(text(updatedArtifact.report_xlsx_path)).toBe(reportPath)

    await revealAndClose(exportPath, 'export-all-reveal-window.json')
    await revealAndClose(reportPath, 'report-reveal-window.json')

    writeReleaseProofEvidence('export-proof.json', {
      contract: 'vectorgrid-exact-package-export-v1',
      schemaVersion: 1,
      exactBinary: environment.desktopBinary,
      csv: {
        sourcePath: exportPath,
        evidencePath: csvEvidencePath,
        bytes: fs.statSync(csvEvidencePath).size,
        sha256: sha256(csvEvidencePath),
        rows: csvLines.length - 1,
      },
      xlsx: {
        sourcePath: reportPath,
        evidencePath: xlsxEvidencePath,
        bytes: fs.statSync(xlsxEvidencePath).size,
        sha256: sha256(xlsxEvidencePath),
      },
      validatedArtifactPath,
      nativeWindowsContained: true,
    })
  })
})
