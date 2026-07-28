import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import { sha256File } from '../support/nativeFeatureFixtures.js'
import { invokeNative } from '../support/nativeIpc.js'
import {
  isPathInside,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

const CAMPAIGN_FIXTURE_PREFIX = 'release-proof-campaign'

type RunArtifactSummary = {
  path?: unknown
  strategy_kind?: unknown
  has_validation?: unknown
}

type RunArtifact = {
  data?: {
    csv_path?: unknown
  }
  validation?: {
    req?: {
      cross_ticker?: {
        max_datasets?: unknown
        mode?: unknown
        datasets?: Array<{
          csv_path?: unknown
          label?: unknown
        }>
      } | null
    }
    res?: {
      cross_ticker?: {
        mode?: unknown
        datasets?: Array<{
          csv_path?: unknown
          label?: unknown
          bars?: unknown
        }>
      } | null
      candidates?: unknown[]
    }
  }
}

type PolygonCacheInfo = {
  cache_dir?: unknown
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

async function setFormValue(selector: string, value: string): Promise<void> {
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
    if (!element) throw new Error(`Expected a form control for ${targetSelector}.`)
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
  await browser.waitUntil(
    async () => (await field.isSelected()) === checked,
    {
      timeout: 3_000,
      interval: 50,
      timeoutMsg: `${selector} did not become ${checked ? 'checked' : 'unchecked'}.`,
    },
  )
}

async function selectCrossDataset(label: string): Promise<void> {
  const checkbox = await $(
    `//div[@aria-label="Cross-ticker validation datasets"]//label[.//div[normalize-space()="${label}"]]//input[@type="checkbox"]`,
  )
  await checkbox.waitForDisplayed({ timeout: 10_000 })
  await checkbox.waitForEnabled({ timeout: 10_000 })
  if (!(await checkbox.isSelected())) await checkbox.click()
  await browser.waitUntil(
    async () => checkbox.isSelected(),
    {
      timeout: 3_000,
      interval: 50,
      timeoutMsg: `Cross-ticker dataset ${label} did not stay selected.`,
    },
  )
}

async function newRunArtifactPath(
  before: Set<string>,
  validatedOnly = false,
): Promise<string> {
  const rows = await invokeNative<RunArtifactSummary[]>('list_run_artifacts')
  return text(
    rows.find((row) => (
      text(row.strategy_kind) === 'DoubleMa'
      && text(row.path)
      && !before.has(text(row.path))
      && (!validatedOnly || Boolean(row.has_validation))
    ))?.path,
  )
}

describe('desktop exact-package cross-ticker validation proof', () => {
  it('validates one finalist against two isolated Polygon-catalog datasets', async () => {
    const environment = requireReleaseProofEnvironment()
    const cache = await invokeNative<PolygonCacheInfo>('polygon_get_cache_info')
    const cacheDir = text(cache.cache_dir)
    if (!cacheDir || !isPathInside(cacheDir, environment.isolationRoot)) {
      throw new Error(`Polygon cache escaped the isolated release profile: ${cacheDir}`)
    }
    const fixtureRoot = path.resolve(cacheDir, CAMPAIGN_FIXTURE_PREFIX)
    const crossPaths = [
      path.resolve(
        fixtureRoot,
        `${CAMPAIGN_FIXTURE_PREFIX}-01-valid.csv`,
      ),
      path.resolve(
        fixtureRoot,
        `${CAMPAIGN_FIXTURE_PREFIX}-03-valid.csv`,
      ),
    ]
    for (const csvPath of crossPaths) {
      if (
        !isPathInside(csvPath, environment.isolationRoot)
        || !fs.existsSync(csvPath)
        || !fs.statSync(csvPath).isFile()
      ) {
        throw new Error(
          `Cross-ticker proof requires the valid fixture from the preceding campaign journey: ${csvPath}`,
        )
      }
    }
    const artifactRowsBefore = await invokeNative<RunArtifactSummary[]>(
      'list_run_artifacts',
    )
    const artifactPathsBefore = new Set(
      artifactRowsBefore.map((row) => text(row.path)).filter(Boolean),
    )

    await waitForShellReady()
    await openWorkspace('workflow')
    const conventional = await $(nativeWorkflowSelectors.conventionalMode)
    await conventional.waitForDisplayed({ timeout: 10_000 })
    if ((await conventional.getAttribute('aria-selected')) !== 'true') {
      await conventional.click()
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
    await setCheckbox(nativeWorkflowSelectors.validationWalkForward, false)
    await setCheckbox(nativeWorkflowSelectors.validationPerturbations, false)
    await setCheckbox(nativeWorkflowSelectors.validationMonteCarlo, false)
    await setCheckbox(nativeWorkflowSelectors.validationCscv, false)
    await setCheckbox(nativeWorkflowSelectors.validationSpa, false)
    await setCheckbox(nativeWorkflowSelectors.validationReoptWalkForward, false)
    await setFormValue(nativeWorkflowSelectors.validationCandidates, '1')

    await (await $(nativeWorkflowSelectors.optimizeStep)).click()
    await setFormValue(nativeWorkflowSelectors.strategyKind, 'DoubleMa')
    await setFormValue(nativeWorkflowSelectors.backend, 'CpuOnly')
    await setFormValue(nativeWorkflowSelectors.mode, 'Grid')
    await setFormValue(nativeWorkflowSelectors.fastStart, '3')
    await setFormValue(nativeWorkflowSelectors.fastEnd, '3')
    await setFormValue(nativeWorkflowSelectors.fastStep, '1')
    await setFormValue(nativeWorkflowSelectors.slowStart, '8')
    await setFormValue(nativeWorkflowSelectors.slowEnd, '8')
    await setFormValue(nativeWorkflowSelectors.slowStep, '1')
    const run = await $(nativeWorkflowSelectors.runOptimization)
    await run.waitForEnabled({ timeout: 10_000 })
    await run.click()
    await (await $(nativeWorkflowSelectors.firstTopResult))
      .waitForDisplayed({ timeout: 45_000 })
    let artifactPath = ''
    await browser.waitUntil(
      async () => {
        artifactPath = await newRunArtifactPath(artifactPathsBefore)
        return Boolean(artifactPath)
      },
      {
        timeout: 10_000,
        interval: 200,
        timeoutMsg: 'Cross-ticker setup did not persist its bounded optimization.',
      },
    )

    await (await $(nativeWorkflowSelectors.validationStep)).click()
    await setFormValue(nativeWorkflowSelectors.validationMinPsr, '0')
    await setFormValue(nativeWorkflowSelectors.validationMinDsr, '0')
    await setFormValue(nativeWorkflowSelectors.validationMinCscvRank, '')
    await setFormValue(nativeWorkflowSelectors.validationMaxPbo, '')
    const refreshCross = await $('[data-qa-action="validation.cross.refresh"]')
    await refreshCross.waitForEnabled({ timeout: 10_000 })
    await refreshCross.click()
    const crossGroup = await $('[aria-label="Cross-ticker validation datasets"]')
    await browser.waitUntil(
      async () => {
        const groupText = text(await crossGroup.getText())
        return crossPaths.every((csvPath) => (
          groupText.includes(path.basename(csvPath, '.csv'))
        ))
      },
      {
        timeout: 15_000,
        interval: 100,
        timeoutMsg: 'The native Polygon catalog did not expose both cross-ticker fixtures.',
      },
    )
    const selectNone = await $('[data-qa-action="validation.cross.selectNone"]')
    if (await selectNone.isEnabled().catch(() => false)) {
      await selectNone.click()
      await browser.pause(100)
    }
    await setFormValue('#vueValCrossMax', '2')
    await setFormValue('#vueValCrossMode', 'lightweight')
    await selectCrossDataset(`${CAMPAIGN_FIXTURE_PREFIX}-01-valid`)
    await selectCrossDataset(`${CAMPAIGN_FIXTURE_PREFIX}-03-valid`)
    await setCheckbox(
      '//label[contains(., "Cross-ticker validation")]//input[@type="checkbox"]',
      true,
    )
    await expect(crossGroup).toHaveText(expect.stringContaining('01-valid'))
    await expect(crossGroup).toHaveText(expect.stringContaining('03-valid'))

    const validationRun = await $(nativeWorkflowSelectors.validationRun)
    await validationRun.waitForEnabled({ timeout: 10_000 })
    await validationRun.click()
    await (await $(nativeWorkflowSelectors.firstValidationResult))
      .waitForDisplayed({ timeout: 60_000 })
    await validationRun.waitForEnabled({ timeout: 10_000 })
    await browser.waitUntil(
      async () => {
        artifactPath = await newRunArtifactPath(artifactPathsBefore, true)
        return Boolean(artifactPath)
      },
      {
        timeout: 10_000,
        interval: 200,
        timeoutMsg: 'Cross-ticker validation did not persist its artifact.',
      },
    )

    const persisted = await invokeNative<RunArtifact>(
      'load_run_artifact',
      { path: artifactPath },
    )
    const requestCross = persisted.validation?.req?.cross_ticker
    const resultCross = persisted.validation?.res?.cross_ticker
    expect(text(requestCross?.mode)).toBe('lightweight')
    expect(Number(requestCross?.max_datasets)).toBe(2)
    expect(requestCross?.datasets?.length ?? 0).toBe(2)
    expect(text(resultCross?.mode)).toBe('lightweight')
    expect(resultCross?.datasets?.length ?? 0).toBe(2)
    const resultPaths = (resultCross?.datasets ?? [])
      .map((entry) => text(entry.csv_path))
      .sort()
    expect(resultPaths).toEqual([...crossPaths].sort())

    writeReleaseProofEvidence('cross-ticker-proof.json', {
      contract: 'vectorgrid-cross-ticker-validation-v1',
      mode: 'lightweight',
      candidate_limit: 1,
      dataset_count: 2,
      artifact_path: artifactPath,
      datasets: (resultCross?.datasets ?? []).map((entry) => ({
        label: text(entry.label),
        csv_path: text(entry.csv_path),
        bars: Number(entry.bars),
        sha256: sha256File(text(entry.csv_path)),
      })),
      result_candidate_count: persisted.validation?.res?.candidates?.length ?? 0,
      provider_network_used: false,
    })
  })
})
